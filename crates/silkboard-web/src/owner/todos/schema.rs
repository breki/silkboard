//! Independently versioned additive checklist storage and snapshot admission.
use crate::progress::ProgressError;
use rusqlite::{Connection, OptionalExtension};
pub(crate) fn validate_optional(
    conn: &Connection,
) -> Result<bool, ProgressError> {
    let count:i64=conn.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('todo_schema','todos')",[],|r|r.get(0))?;
    if count == 0 {
        let layout:i64=conn.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('todo_layout_schema','todo_layout')",[],|r|r.get(0))?;
        return if layout == 0 {
            Ok(false)
        } else {
            Err(ProgressError::Invalid)
        };
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
    let mut query = conn.prepare("SELECT id,text,completed FROM todos")?;
    let rows = query.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    for row in rows {
        let (id, text, completed) = row?;
        if id <= 0 || !super::valid_text(&text) || !matches!(completed, 0 | 1) {
            return Err(ProgressError::Invalid);
        }
    }
    let revision = version.expect("validated component").1;
    super::layout::validate_optional(conn, revision)?;
    Ok(true)
}
pub(crate) fn initialize(conn: &Connection) -> Result<(), ProgressError> {
    if !validate_optional(conn)? {
        conn.execute_batch(include_str!("schema.sql"))?;
    }
    let revision =
        conn.query_row("SELECT revision FROM todo_schema", [], |r| {
            r.get::<_, i64>(0)
        })?;
    super::layout::initialize(conn, revision)?;
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

#[cfg(test)]
mod editor_migration_tests {
    use super::*;
    fn legacy() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;
          CREATE TABLE todo_schema(singleton INTEGER PRIMARY KEY,version INTEGER NOT NULL,revision INTEGER NOT NULL);
          INSERT INTO todo_schema VALUES(1,1,7);
          CREATE TABLE todos(id INTEGER PRIMARY KEY AUTOINCREMENT,parent_id INTEGER REFERENCES todos(id) ON DELETE RESTRICT CHECK(parent_id<id),text TEXT NOT NULL,completed INTEGER NOT NULL DEFAULT 0);
          INSERT INTO todos(id,parent_id,text,completed) VALUES(1,NULL,'First',0),(2,NULL,'Second',1),(3,1,'Child',1),(9,NULL,'Removed',0);
          DELETE FROM todos WHERE id=9;").unwrap();
        conn
    }
    #[test]
    fn migration_keeps_identity_completion_revision_and_visible_order() {
        let conn = legacy();
        initialize(&conn).unwrap();
        let component: (i64, i64) = conn
            .query_row("SELECT version,revision FROM todo_schema", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(component, (1, 7));
        let mut query=conn.prepare("SELECT t.id,l.parent_id,t.text,t.completed FROM todos t JOIN todo_layout l ON t.id=l.id ORDER BY l.position").unwrap();
        let rows = query
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Option<i64>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, bool>(3)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![
                (1, None, "First".into(), false),
                (3, Some(1), "Child".into(), true),
                (2, None, "Second".into(), true)
            ]
        );
        conn.execute("INSERT INTO todos(text) VALUES('New')", [])
            .unwrap();
        assert_eq!(conn.last_insert_rowid(), 10);
        conn.execute("UPDATE todo_schema SET revision=revision+1", [])
            .unwrap();
        initialize(&conn).unwrap();
    }
    #[test]
    fn migration_of_an_empty_list_preserves_deleted_id_high_water() {
        let conn = legacy();
        conn.execute_batch(
            "UPDATE todos SET parent_id=NULL;DELETE FROM todos;",
        )
        .unwrap();
        initialize(&conn).unwrap();
        conn.execute("INSERT INTO todos(text) VALUES('New')", [])
            .unwrap();
        assert_eq!(conn.last_insert_rowid(), 10);
    }
    #[test]
    fn failed_migration_restores_the_old_schema_and_hierarchy() {
        let conn = legacy();
        conn.execute_batch("CREATE TABLE other(value INTEGER);CREATE INDEX todo_layout_parent ON other(value);").unwrap();
        assert!(initialize(&conn).is_err());
        assert_eq!(
            conn.query_row("SELECT version FROM todo_schema", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row("SELECT parent_id FROM todos WHERE id=3", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row("SELECT count(*) FROM todos", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            3
        );
        assert!(conn.prepare("SELECT position FROM todos").is_err());
    }
}
