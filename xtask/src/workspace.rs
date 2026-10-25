//! Runtime workspace selection for helpers reused across local worktrees.
use std::path::{Path, PathBuf};

/// Locate the calling Silkboard workspace without a compiled-in checkout path.
///
/// # Errors
/// Returns an error outside a workspace containing the repository's xtask.
pub fn root(start: &Path) -> Result<PathBuf, String> {
    for dir in start.ancestors() {
        let manifest = dir.join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&manifest)
            .map_err(|error| format!("read {}: {error}", manifest.display()))?;
        let value: toml::Value = text.parse().map_err(|error| {
            format!("parse {}: {error}", manifest.display())
        })?;
        if value.get("workspace").is_some() {
            if dir.join("xtask/Cargo.toml").is_file() {
                return Ok(dir.to_path_buf());
            }
            return Err("calling workspace has no xtask/Cargo.toml".into());
        }
    }
    Err("run xtask from inside the Silkboard workspace".into())
}
