//! Independently versioned additive tables compatible with base schema 3.
use crate::progress::ProgressError;
use rusqlite::{Connection, OptionalExtension};

pub(crate) fn validate_optional(
    conn: &Connection,
) -> Result<bool, ProgressError> {
    let names:Vec<String>=conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name IN ('plan_schema','task_plans','plan_snapshots','plan_revisions','plan_step_history')")?.query_map([],|r|r.get(0))?.collect::<Result<_,_>>()?;
    if names.is_empty() {
        return Ok(false);
    }
    if names.len() != 5 {
        return Err(ProgressError::Invalid);
    }
    let version: Option<i64> = conn
        .query_row(
            "SELECT version FROM plan_schema WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let count: i64 =
        conn.query_row("SELECT count(*) FROM plan_schema", [], |r| r.get(0))?;
    match version {
        Some(1) if count == 1 => {}
        Some(n) if n > 1 => return Err(ProgressError::NewerSchema),
        _ => return Err(ProgressError::Invalid),
    }
    for sql in [
        "SELECT project,task_id,latest_sequence,received_at,summary FROM task_plans LIMIT 0",
        "SELECT project,task_id,sequence,plan_revision,writer_generation,payload,summary,received_at FROM plan_snapshots LIMIT 0",
        "SELECT project,task_id,revision,latest_sequence,summary FROM plan_revisions LIMIT 0",
        "SELECT project,task_id,step_id,sequence,state FROM plan_step_history LIMIT 0",
    ] {
        conn.prepare(sql)?;
    }
    Ok(true)
}
pub(crate) fn initialize(conn: &Connection) -> Result<(), ProgressError> {
    if !validate_optional(conn)? {
        conn.execute_batch(include_str!("schema.sql"))?;
    }
    Ok(())
}
