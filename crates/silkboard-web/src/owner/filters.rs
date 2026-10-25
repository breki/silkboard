//! Shared validation for owner list filter values.
use super::AuthError;
use crate::progress::validate_project;

pub(crate) fn validate(
    project: Option<&str>,
    agent: Option<&str>,
) -> Result<(), AuthError> {
    if let Some(project) = project {
        validate_project(project).map_err(|_| AuthError::Invalid)?;
    }
    if agent.is_some_and(|v| {
        v.is_empty() || v.len() > 513 || v.chars().any(char::is_control)
    }) {
        return Err(AuthError::Invalid);
    }
    Ok(())
}
