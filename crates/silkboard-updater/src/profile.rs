//! Validated operator-owned admission policy; no candidate-controlled commands.

use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::BTreeSet, fs, os::unix::fs::MetadataExt, path::Path};

/// Fixed configuration location for the installed Silkboard adapter.
pub const INSTALLED_POLICY: &str = "/etc/silkboard/deployment.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    schema: u32,
    application: String,
    repository: String,
    repository_id: u64,
    workflow_id: u64,
    workflow_path: String,
    ci_path: String,
    required_jobs: Vec<String>,
}

/// A validated strict GitHub/SSH-tag policy. It cannot disable either gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Policy(Definition);

impl<'de> Deserialize<'de> for Policy {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let definition = Definition::deserialize(deserializer)?;
        Self::validated(definition).map_err(serde::de::Error::custom)
    }
}

impl Policy {
    fn validated(definition: Definition) -> Result<Self, String> {
        let repository: Vec<_> = definition.repository.split('/').collect();
        let segment = |s: &str| {
            !s.is_empty()
                && s != "."
                && s != ".."
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        };
        let workflow = |path: &str| {
            path.strip_prefix(".github/workflows/").is_some_and(|file| {
                segment(file)
                    && !file.contains("..")
                    && matches!(
                        Path::new(file).extension().and_then(|s| s.to_str()),
                        Some("yml" | "yaml")
                    )
            })
        };
        if definition.schema != 1
            || definition.application.len() > 64
            || !segment(&definition.application)
            || definition.repository.len() > 255
            || repository.len() != 2
            || !repository.iter().all(|s| segment(s))
            || definition.repository_id == 0
            || definition.workflow_id == 0
            || !workflow(&definition.workflow_path)
            || !workflow(&definition.ci_path)
            || definition.required_jobs.is_empty()
            || definition.required_jobs.len() > 100
            || definition.required_jobs.iter().any(|job| {
                job.is_empty()
                    || job.len() > 200
                    || job.trim() != job
                    || job.chars().any(char::is_control)
            })
            || definition
                .required_jobs
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != definition.required_jobs.len()
        {
            return Err("invalid deployment policy".into());
        }
        Ok(Self(definition))
    }

    /// Parse a strict policy, rejecting unknown fields and unsupported schemas.
    ///
    /// # Errors
    /// Returns invalid JSON, shape or policy errors.
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 65_536 {
            return Err("deployment policy is too large".into());
        }
        serde_json::from_slice(bytes)
            .map_err(|e| format!("deployment policy: {e}"))
    }

    /// Canonical policy of the existing Silkboard deployment.
    #[must_use]
    pub fn silkboard() -> Self {
        Self(Definition {
            schema: 1,
            application: "silkboard".into(),
            repository: "breki/silkboard".into(),
            repository_id: 1_417_054_470,
            workflow_id: 387_356_305,
            workflow_path: ".github/workflows/release.yml".into(),
            ci_path: ".github/workflows/ci.yml".into(),
            required_jobs: [
                "Signed stable tag and exact revision",
                "gates / Full validation (Linux)",
                "Build the checked release revision",
            ]
            .map(str::to_owned)
            .into(),
        })
    }

    /// Load only the fixed protected policy; absence preserves legacy defaults.
    ///
    /// # Errors
    /// Rejects insecure ownership/modes, symlinks and malformed policy.
    pub fn installed() -> Result<Self, String> {
        load(Path::new(INSTALLED_POLICY), 0)
    }

    /// Deployment namespace bound into persistent state.
    #[must_use]
    pub fn application(&self) -> &str {
        &self.0.application
    }
    /// GitHub owner/repository, never a caller-provided URL.
    #[must_use]
    pub fn repository(&self) -> &str {
        &self.0.repository
    }
    /// Independent numeric repository identity.
    #[must_use]
    pub fn repository_id(&self) -> u64 {
        self.0.repository_id
    }
    /// Independently registered release workflow.
    #[must_use]
    pub fn workflow_id(&self) -> u64 {
        self.0.workflow_id
    }
    /// Exact release workflow path.
    #[must_use]
    pub fn workflow_path(&self) -> &str {
        &self.0.workflow_path
    }
    /// Exact same-revision reusable CI path.
    #[must_use]
    pub fn ci_path(&self) -> &str {
        &self.0.ci_path
    }
    /// Required exact successful job set, including rejection of additional jobs.
    #[must_use]
    pub fn required_jobs(&self) -> &[String] {
        &self.0.required_jobs
    }
}

fn load(path: &Path, owner: u32) -> Result<Policy, String> {
    let parent_path = path.parent().ok_or("missing policy parent")?;
    let parent = match fs::symlink_metadata(parent_path) {
        Ok(parent) => parent,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Policy::silkboard());
        }
        Err(error) => return Err(error.to_string()),
    };
    if !parent.is_dir() || parent.uid() != owner || parent.mode() & 0o022 != 0 {
        return Err(
            "deployment policy parent must be a protected root-owned directory"
                .into(),
        );
    }
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Policy::silkboard());
        }
        Err(error) => return Err(error.to_string()),
    };
    if !metadata.is_file()
        || metadata.uid() != owner
        || metadata.mode() & 0o022 != 0
        || metadata.len() > 65_536
    {
        return Err("deployment policy must be a protected root-owned regular file under 64 KiB".into());
    }
    Policy::from_json(&fs::read(path).map_err(|e| e.to_string())?)
}

#[cfg(test)]
mod tests;
