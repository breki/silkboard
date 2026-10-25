//! Validate explicit build metadata without claiming signature verification.

/// Render a tagged identity, or explicitly identify a development build.
pub fn identity(
    version: &str,
    tag: Option<&str>,
    commit: Option<&str>,
) -> Result<String, &'static str> {
    if let Some(commit) = commit
        && (commit.len() != 40
            || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err("SILKBOARD_COMMIT must be a 40-digit Git SHA");
    }
    match (tag, commit) {
        (Some(tag), Some(commit)) if tag == format!("v{version}") => {
            Ok(format!("tag {tag}; commit {commit}"))
        }
        (None, commit) => Ok(format!(
            "development; commit {}",
            commit.unwrap_or("unknown")
        )),
        _ => Err(
            "SILKBOARD_RELEASE_TAG must match the package version and requires SILKBOARD_COMMIT",
        ),
    }
}
