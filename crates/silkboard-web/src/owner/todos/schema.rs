//! Independently versioned additive checklist storage and snapshot admission.
use crate::progress::ProgressError;
use rusqlite::{Connection, OptionalExtension};
pub(crate) fn validate_optional(
    conn: &Connection,
) -> Result<bool, ProgressError> {
    let count:i64=conn.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('todo_schema','todos')",[],|r|r.get(0))?;
    if count == 0 {
        return Ok(false);
    }
    if count != 2 {
        return Err(ProgressError::Invalid);
    }
    let version: Option<(i64, i64)> = conn
        .query_row(
            "SELECT version,revision FROM todo_schema WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match version {
        Some((1, n)) if n >= 0 => {}
        Some((n, _)) if n > 1 => return Err(ProgressError::NewerSchema),
        _ => return Err(ProgressError::Invalid),
    }
    let count: i64 =
        conn.query_row("SELECT count(*) FROM todo_schema", [], |r| r.get(0))?;
    if count != 1 {
        return Err(ProgressError::Invalid);
    }
    conn.prepare("SELECT id,parent_id,text,completed FROM todos LIMIT 0")?;
    // One SQLite statement keeps hierarchy validation consistent during edits.
    let (count, depth, reachable): (i64, i64, i64) = conn.query_row(
        "WITH RECURSIVE tree(id,depth) AS (
           SELECT id,1 FROM todos WHERE parent_id IS NULL
           UNION ALL SELECT t.id,p.depth+1 FROM todos t
           JOIN tree p ON t.parent_id=p.id WHERE p.depth<6 LIMIT 1001
         ) SELECT (SELECT count(*) FROM todos),coalesce(max(depth),0),count(*) FROM tree",
        [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    if count > 1000 || depth > 5 || reachable != count {
        return Err(ProgressError::Invalid);
    }
    Ok(true)
}
pub(crate) fn initialize(conn: &Connection) -> Result<(), ProgressError> {
    if !validate_optional(conn)? {
        conn.execute_batch(include_str!("schema.sql"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn component_is_optional_and_refuses_partial_or_future_schema() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(!validate_optional(&conn).unwrap());
        initialize(&conn).unwrap();
        assert!(validate_optional(&conn).unwrap());
        initialize(&conn).unwrap();
        conn.execute("UPDATE todo_schema SET version=2", [])
            .unwrap();
        assert!(matches!(initialize(&conn), Err(ProgressError::NewerSchema)));
        conn.execute("UPDATE todo_schema SET version=1", [])
            .unwrap();
        conn.execute("DELETE FROM todo_schema", []).unwrap();
        assert!(validate_optional(&conn).is_err());
        conn.execute("DROP TABLE todo_schema", []).unwrap();
        assert!(validate_optional(&conn).is_err());
    }
    #[test]
    fn invalid_hierarchy_and_excessive_size_are_refused() {
        let conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        for id in 1..=6 {
            conn.execute(
                "INSERT INTO todos(id,parent_id,text) VALUES(?1,?2,'Task')",
                rusqlite::params![
                    id,
                    if id == 1 { None } else { Some(id - 1) }
                ],
            )
            .unwrap();
        }
        assert!(validate_optional(&conn).is_err());
        for id in (1..=6).rev() {
            conn.execute("DELETE FROM todos WHERE id=?1", [id]).unwrap();
        }
        for id in 1..=1001 {
            conn.execute("INSERT INTO todos(id,text) VALUES(?1,'Task')", [id])
                .unwrap();
        }
        assert!(validate_optional(&conn).is_err());
    }
}
