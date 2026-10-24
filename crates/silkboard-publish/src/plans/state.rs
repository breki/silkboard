//! Current local plan and high-water marks, not an outbound request queue.
use super::Handoff;
use crate::{Error, model::Config, state};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use silkboard_plan::{Counter, Snapshot, valid_task_id};
use std::{fs::File, path::PathBuf};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Working {
    schema_version: u8,
    snapshot: Snapshot,
}
fn increment(value: Counter) -> Result<Counter, Error> {
    Counter::new(value.get().checked_add(1).ok_or(Error::State)?)
        .map_err(|_| Error::State)
}
fn locked(config: &Config, id: &str) -> Result<(File, PathBuf), Error> {
    use std::fmt::Write;
    if !valid_task_id(id) {
        return Err(Error::Input);
    }
    let identity = serde_json::to_vec(&(&config.endpoint, &config.project, id))
        .map_err(|_| Error::Input)?;
    let mut key = String::from("plan-");
    for b in Sha256::digest(identity) {
        write!(key, "{b:02x}").map_err(|_| Error::State)?;
    }
    let lock = state::lock(&config.state_dir, &key)?;
    Ok((lock, config.state_dir.join(format!("{key}.json"))))
}
fn load(path: &std::path::Path, id: &str) -> Result<Option<Snapshot>, Error> {
    match path.symlink_metadata() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Error::State),
        Ok(_) => {}
    }
    let w: Working = serde_json::from_slice(&state::read_private(
        path,
        silkboard_plan::MAX_SNAPSHOT_BYTES as u64 + 64,
    )?)
    .map_err(|_| Error::State)?;
    if w.schema_version != 1 || w.snapshot.task_id != id {
        return Err(Error::State);
    }
    Ok(Some(w.snapshot.normalized().map_err(|_| Error::State)?))
}
fn save(path: &std::path::Path, snapshot: &Snapshot) -> Result<(), Error> {
    let bytes = serde_json::to_vec(&Working {
        schema_version: 1,
        snapshot: snapshot.clone(),
    })
    .map_err(|_| Error::State)?;
    state::replace(path, &bytes)
}
pub(super) fn allocate(
    config: &Config,
    mut s: Snapshot,
    start: bool,
) -> Result<Snapshot, Error> {
    let (_lock, path) = locked(config, &s.task_id)?;
    if let Some(old) = load(&path, &s.task_id)? {
        if start {
            return Err(Error::Conflict);
        }
        if !s.reporter.same_identity(&old.reporter) {
            return Err(Error::Conflict);
        }
        s.sequence = increment(old.sequence)?;
        s.writer_generation = old.writer_generation;
        s.plan_revision = if s.same_definition(&old) {
            old.plan_revision
        } else {
            increment(old.plan_revision)?
        };
    } else if !start {
        return Err(Error::State);
    }
    let s = s.normalized().map_err(|_| Error::Input)?;
    save(&path, &s)?;
    Ok(s)
}
pub(super) fn handoff(config: &Config, h: Handoff) -> Result<(), Error> {
    if h.schema_version != 1 {
        return Err(Error::Input);
    }
    let (_lock, path) = locked(config, &h.task_id)?;
    let mut s = load(&path, &h.task_id)?.ok_or(Error::State)?;
    // Both supplied identities use the shared field rules. Readable names do
    // not distinguish writers; the caller must name a different native owner.
    let old_reporter = s.reporter.clone();
    let mut from = s.clone();
    from.reporter = h.from;
    let from = from.normalized().map_err(|_| Error::Input)?;
    s.reporter = h.to;
    let mut s = s.normalized().map_err(|_| Error::Input)?;
    if !from.reporter.same_identity(&old_reporter)
        || from.reporter.same_identity(&s.reporter)
    {
        return Err(Error::Conflict);
    }
    s.writer_generation = increment(s.writer_generation)?;
    let s = s.normalized().map_err(|_| Error::Input)?;
    save(&path, &s)
}
