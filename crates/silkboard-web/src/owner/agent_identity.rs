//! Shared owner-facing native identity metadata.
use rusqlite::Row;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct AgentIdentity {
    pub(super) hostname: String,
    pub(super) tool: String,
    pub(super) session_id: String,
    pub(super) subagent_id: Option<String>,
    pub(super) parent_session_id: Option<String>,
    pub(super) session_name: Option<String>,
}

impl AgentIdentity {
    pub(super) fn read(row: &Row<'_>, start: usize) -> rusqlite::Result<Self> {
        Ok(Self {
            hostname: row.get(start)?,
            tool: row.get(start + 1)?,
            session_id: row.get(start + 2)?,
            subagent_id: row.get(start + 3)?,
            parent_session_id: row.get(start + 4)?,
            session_name: row.get(start + 5)?,
        })
    }
}
