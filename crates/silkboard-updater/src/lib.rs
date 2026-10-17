//! Verified release selection and durable upgrade recovery.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub mod evidence;
pub mod linux;

/// A stable, canonical numeric release version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version([u64; 3]);

impl Version {
    /// Parse `vMAJOR.MINOR.PATCH`, excluding aliases and prereleases.
    #[must_use]
    pub fn parse(tag: &str) -> Option<Self> {
        let parts: Vec<_> = tag.strip_prefix('v')?.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let mut numbers = [0; 3];
        for (index, part) in parts.iter().enumerate() {
            if part.is_empty()
                || !part.bytes().all(|b| b.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
            {
                return None;
            }
            numbers[index] = part.parse().ok()?;
        }
        Some(Self(numbers))
    }
}

/// A validated stable release and its full Git commit identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    tag: String,
    commit: String,
}

impl Release {
    /// Validate a release identity read from an external boundary.
    ///
    /// # Errors
    /// Rejects noncanonical tags and non-SHA-1 commit identities.
    pub fn new(tag: &str, commit: &str) -> Result<Self, String> {
        if Version::parse(tag).is_none()
            || commit.len() != 40
            || !commit
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("invalid release identity".into());
        }
        Ok(Self {
            tag: tag.into(),
            commit: commit.into(),
        })
    }

    /// The signed stable release name.
    #[must_use]
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// The exact full commit identifier.
    #[must_use]
    pub fn commit(&self) -> &str {
        &self.commit
    }

    /// Validate a signed tag's direct commit target and embedded name.
    /// Signature verification is a separate mandatory backend operation.
    ///
    /// # Errors
    /// Rejects lightweight, indirect, aliased, malformed, or unsigned objects.
    pub fn from_tag(tag: &str, object: &str) -> Result<Self, String> {
        let mut lines = object.lines();
        let commit = lines
            .next()
            .and_then(|s| s.strip_prefix("object "))
            .ok_or("missing direct tag target")?;
        if lines.next() != Some("type commit")
            || lines.next() != Some(format!("tag {tag}").as_str())
            || !object.contains("\n-----BEGIN SSH SIGNATURE-----\n")
            || !object.ends_with("-----END SSH SIGNATURE-----\n")
        {
            return Err("tag must bind its name and direct commit with an SSH signature".into());
        }
        Self::new(tag, commit)
    }
}

/// An interrupted or uncommitted upgrade always restores `previous`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Previously healthy release, retained during the upgrade.
    pub previous: Release,
    /// Candidate blocked until successful completion or explicit retry.
    pub candidate: Release,
}

/// Root-owned durable updater state; pending transactions survive reboot.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    /// Failed release names remain blocked even if their refs move.
    pub blocked: BTreeSet<String>,
    /// Written and synced before any build or activation.
    pub pending: Option<Transaction>,
}

/// Deployment boundaries implemented by the installed, operator-owned updater.
pub trait Backend {
    /// Read and validate root-owned persistent state.
    ///
    /// # Errors
    /// Returns boundary failures without treating unavailable evidence as success.
    fn load(&mut self) -> Result<State, String>;
    /// Atomically write and sync persistent state before returning.
    ///
    /// # Errors
    /// Returns boundary failures without treating unavailable evidence as success.
    fn save(&mut self, state: &State) -> Result<(), String>;
    /// Read the current root-owned release identity.
    ///
    /// # Errors
    /// Returns boundary failures without treating unavailable evidence as success.
    fn current(&mut self) -> Result<Release, String>;
    /// Fetch current remote tags, removing deleted local refs.
    ///
    /// # Errors
    /// Returns boundary failures without treating unavailable evidence as success.
    fn refresh(&mut self) -> Result<Vec<String>, String>;
    /// Return only a trusted signed tag with passing exact release checks.
    /// Pending or rejected tags return None; transport failures return errors.
    ///
    /// # Errors
    /// Returns boundary failures without treating unavailable evidence as success.
    fn admit(&mut self, tag: &str) -> Result<Option<Release>, String>;
    /// Build unprivileged and snapshot regular artifacts into root-owned staging.
    ///
    /// # Errors
    /// Returns boundary failures without treating unavailable evidence as success.
    fn build(&mut self, release: &Release) -> Result<(), String>;
    /// Atomically switch current and restart the service.
    ///
    /// # Errors
    /// Returns boundary failures without treating unavailable evidence as success.
    fn activate(&mut self, release: &Release) -> Result<(), String>;
    /// Check both health endpoints and exact running release identity.
    ///
    /// # Errors
    /// Returns boundary failures without treating unavailable evidence as success.
    fn healthy(&mut self, release: &Release) -> Result<(), String>;
}

/// Poll once, or recover a persisted transaction before considering updates.
///
/// # Errors
/// Returns transport, persistence, build, health, or recovery failures. A failed
/// recovery retains the durable transaction for the next invocation.
pub fn poll(backend: &mut impl Backend) -> Result<(), String> {
    let mut state = backend.load()?;
    if state.pending.is_some() {
        recover(backend, &mut state)?;
        return Ok(());
    }
    let current = backend.current()?;
    backend.healthy(&current)?;
    let version =
        Version::parse(current.tag()).ok_or("invalid current version")?;
    let mut tags: Vec<_> = backend
        .refresh()?
        .into_iter()
        .filter_map(|tag| Version::parse(&tag).map(|v| (v, tag)))
        .filter(|(v, tag)| *v > version && !state.blocked.contains(tag))
        .collect();
    tags.sort_by(|a, b| b.cmp(a));
    for (_, tag) in tags {
        let Some(candidate) = backend.admit(&tag)? else {
            continue;
        };
        state.blocked.insert(tag);
        state.pending = Some(Transaction {
            previous: current.clone(),
            candidate: candidate.clone(),
        });
        backend.save(&state)?;
        let result = upgrade(backend, &candidate);
        if let Err(error) = result {
            recover(backend, &mut state).map_err(|rollback| {
                format!("{error}; recovery failed: {rollback}")
            })?;
            return Err(format!(
                "{} blocked: {error}; restored {}",
                candidate.tag(),
                current.tag()
            ));
        }
        state.pending = None;
        state.blocked.remove(candidate.tag());
        backend.save(&state)?;
        println!("Activated {} ({})", candidate.tag(), candidate.commit());
        return Ok(());
    }
    println!("No newer eligible release; current {}", current.tag());
    Ok(())
}

fn upgrade(
    backend: &mut impl Backend,
    candidate: &Release,
) -> Result<(), String> {
    backend.build(candidate)?;
    // Re-fetch before activation: deletion, replacement, or failed reruns revoke
    // this attempt. The already staged source never changes the admitted identity.
    backend.refresh()?;
    if backend.admit(candidate.tag())?.as_ref() != Some(candidate) {
        return Err("candidate eligibility changed during build".into());
    }
    backend.activate(candidate)?;
    backend.healthy(candidate)
}

fn recover(
    backend: &mut impl Backend,
    state: &mut State,
) -> Result<(), String> {
    let transaction = state
        .pending
        .as_ref()
        .ok_or("missing recovery transaction")?;
    println!(
        "Restoring {} after incomplete {}",
        transaction.previous.tag(),
        transaction.candidate.tag()
    );
    backend.activate(&transaction.previous)?;
    backend.healthy(&transaction.previous)?;
    state.pending = None;
    backend.save(state)
}

/// Remove a failed-tag block explicitly, without forcing deployment/downgrade.
///
/// # Errors
/// Rejects invalid tags, incomplete recovery, and state persistence failures.
pub fn retry(backend: &mut impl Backend, tag: &str) -> Result<(), String> {
    if Version::parse(tag).is_none() {
        return Err("invalid stable tag".into());
    }
    let mut state = backend.load()?;
    if state.pending.is_some() {
        return Err("run poll to complete recovery first".into());
    }
    state.blocked.remove(tag);
    backend.save(&state)
}
