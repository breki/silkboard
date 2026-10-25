//! Wire types and bounded input invariants.

use super::ProgressError;
use serde::{Deserialize, Serialize};

/// A short progress message. Session-local sequences establish history order,
/// independent of delivery order; reuse the entire event unchanged on retry.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressEvent {
    /// Stable project-wide event identifier, at most 256 ASCII bytes.
    pub event_id: String,
    /// Publisher-reported hostname, descriptive rather than attested identity.
    pub hostname: String,
    /// AI tool, e.g. `codex` or `claude`.
    pub tool: String,
    /// Native session ID supplied by the tool.
    pub session_id: String,
    /// Native child ID; absent for the primary session.
    pub subagent_id: Option<String>,
    /// Optional native parent-session metadata, only for subagents.
    pub parent_session_id: Option<String>,
    /// Optional short readable name, at most 120 UTF-8 bytes.
    pub session_name: Option<String>,
    /// Positive sequence unique within the full session identity.
    pub sequence: i64,
    /// Markdown text, at most 2,000 UTF-8 bytes; LF, CR and tabs are allowed.
    pub message: String,
    /// Optional detailed Markdown, at most 8,000 UTF-8 bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Explicit workflow task ID in this event's project, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

/// Delivery acknowledgement, never dashboard content or instructions.
#[derive(Debug, Serialize)]
pub struct Acknowledgement {
    /// The caller's accepted event ID.
    pub event_id: String,
    /// Original server receipt time, Unix milliseconds; unchanged on retry.
    pub received_at: i64,
}

impl ProgressEvent {
    pub(super) fn validate(&self) -> Result<(), ProgressError> {
        for value in
            [&self.event_id, &self.hostname, &self.tool, &self.session_id]
                .into_iter()
                .chain(self.subagent_id.as_ref())
                .chain(self.parent_session_id.as_ref())
        {
            identity(value)?;
        }
        if self.sequence <= 0
            || (self.parent_session_id.is_some() && self.subagent_id.is_none())
        {
            return Err(ProgressError::Invalid);
        }
        if self
            .task_id
            .as_deref()
            .is_some_and(|id| !silkboard_plan::valid_task_id(id))
        {
            return Err(ProgressError::Invalid);
        }
        text(&self.message, 2000, true)?;
        if let Some(description) = &self.description {
            text(description, 8000, true)?;
        }
        if let Some(name) = &self.session_name {
            text(name, 120, false)?;
        }
        Ok(())
    }
}

pub(crate) fn project(value: &str) -> Result<(), ProgressError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err(ProgressError::Invalid);
    }
    Ok(())
}

pub(crate) fn identity(value: &str) -> Result<(), ProgressError> {
    if value.is_empty()
        || value.len() > 256
        || !value.bytes().all(|c| c.is_ascii_graphic())
    {
        return Err(ProgressError::Invalid);
    }
    Ok(())
}

fn text(
    value: &str,
    limit: usize,
    multiline: bool,
) -> Result<(), ProgressError> {
    if value.trim().is_empty()
        || value.len() > limit
        || value.chars().any(|c| {
            c.is_control() && !(multiline && matches!(c, '\n' | '\r' | '\t'))
        })
    {
        return Err(ProgressError::Invalid);
    }
    Ok(())
}
