//! Validation shared by the build script and its behavioral tests.

/// Validate explicit release identity, leaving development builds unknown.
pub fn validate_identity<'a>(
    version: &str,
    release: bool,
    tag: Option<&'a str>,
    commit: Option<&'a str>,
) -> Result<(&'a str, &'a str), &'static str> {
    match (tag, commit) {
        (None, None) if !release => Ok(("unknown", "unknown")),
        (Some(tag), Some(commit))
            if tag == format!("v{version}")
                && commit.len() == 40
                && commit.bytes().all(|byte| byte.is_ascii_hexdigit()) =>
        {
            Ok((tag, commit))
        }
        _ => Err("set SILKBOARD_RELEASE_TAG=v<package version> and \
             SILKBOARD_COMMIT=<40-digit Git SHA>; release builds require both"),
    }
}
