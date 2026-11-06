//! Additive ordering metadata, compatible with writes from the previous app.
use crate::progress::ProgressError;
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub(super) struct Placement {
    pub id: i64,
    pub parent: Option<i64>,
}
fn read(conn: &Connection) -> Result<Vec<Placement>, ProgressError> {
    let mut query=conn.prepare("SELECT id,parent_id,position FROM todo_layout ORDER BY position LIMIT 1001")?;
    let rows = query.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, Option<i64>>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    let mut result = Vec::new();
    let mut ancestors = Vec::new();
    let mut seen = HashSet::new();
    for row in rows {
        let (id, parent, position) = row?;
        if id <= 0
            || !seen.insert(id)
            || position
                != i64::try_from(result.len())
                    .map_err(|_| ProgressError::Invalid)?
            || result.len() >= 1000
        {
            return Err(ProgressError::Invalid);
        }
        if let Some(parent) = parent {
            let index = ancestors
                .iter()
                .position(|id| *id == parent)
                .ok_or(ProgressError::Invalid)?;
            ancestors.truncate(index + 1);
        } else {
            ancestors.clear();
        }
        ancestors.push(id);
        if ancestors.len() > 5 {
            return Err(ProgressError::Invalid);
        }
        result.push(Placement { id, parent });
    }
    Ok(result)
}
pub(super) fn validate_optional(
    conn: &Connection,
    revision: i64,
) -> Result<bool, ProgressError> {
    let tables:i64=conn.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('todo_layout_schema','todo_layout')",[],|r|r.get(0))?;
    if tables == 0 {
        return Ok(false);
    }
    if tables != 2 {
        return Err(ProgressError::Invalid);
    }
    let component: Option<(i64, i64)> = conn
        .query_row(
            "SELECT version,revision FROM todo_layout_schema WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let stored = match component {
        Some((1, n)) if n >= 0 && n <= revision => n,
        Some((n, _)) if n > 1 => return Err(ProgressError::NewerSchema),
        _ => return Err(ProgressError::Invalid),
    };
    if conn.query_row("SELECT count(*) FROM todo_layout_schema", [], |r| {
        r.get::<_, i64>(0)
    })? != 1
    {
        return Err(ProgressError::Invalid);
    }
    read(conn)?;
    if stored == revision {
        let missing:i64=conn.query_row("SELECT
          (SELECT count(*) FROM todos t LEFT JOIN todo_layout l ON t.id=l.id WHERE l.id IS NULL)+
          (SELECT count(*) FROM todo_layout l LEFT JOIN todos t ON t.id=l.id WHERE t.id IS NULL)+
          (SELECT count(*) FROM todos t JOIN todo_layout l ON t.id=l.id WHERE t.parent_id IS NOT (CASE WHEN l.parent_id<t.id THEN l.parent_id ELSE NULL END))",[],|r|r.get(0))?;
        if missing != 0 {
            return Err(ProgressError::Invalid);
        }
    }
    Ok(true)
}
pub(super) fn write(
    conn: &Connection,
    items: &[Placement],
) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM todo_layout", [])?;
    for (position, item) in items.iter().enumerate() {
        conn.execute(
            "INSERT INTO todo_layout(id,parent_id,position) VALUES(?1,?2,?3)",
            params![
                item.id,
                item.parent,
                i64::try_from(position).expect("bounded checklist")
            ],
        )?;
    }
    Ok(())
}
pub(super) fn project_legacy(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute("UPDATE todos SET parent_id=(SELECT CASE WHEN l.parent_id<todos.id THEN l.parent_id ELSE NULL END FROM todo_layout l WHERE l.id=todos.id)",[])?;
    Ok(())
}
pub(super) fn initialize(
    conn: &Connection,
    revision: i64,
) -> Result<(), ProgressError> {
    let present = validate_optional(conn, revision)?;
    if present
        && conn.query_row(
            "SELECT revision FROM todo_layout_schema",
            [],
            |r| r.get::<_, i64>(0),
        )? == revision
    {
        return Ok(());
    }
    conn.execute_batch("SAVEPOINT todo_layout_upgrade")?;
    let result = (|| -> Result<(), ProgressError> {
        let old = if present {
            read(conn)?
        } else {
            conn.execute_batch(include_str!("layout.sql"))?;
            Vec::new()
        };
        let items = reconcile(conn, &old)?;
        write(conn, &items)?;
        project_legacy(conn)?;
        conn.execute("UPDATE todo_layout_schema SET revision=?1", [revision])?;
        validate_optional(conn, revision)?;
        Ok(())
    })();
    if let Err(error) = result {
        conn.execute_batch(
            "ROLLBACK TO todo_layout_upgrade; RELEASE todo_layout_upgrade",
        )?;
        return Err(error);
    }
    conn.execute_batch("RELEASE todo_layout_upgrade")?;
    Ok(())
}
fn reconcile(
    conn: &Connection,
    old: &[Placement],
) -> Result<Vec<Placement>, ProgressError> {
    let mut query =
        conn.prepare("SELECT id,parent_id FROM todos ORDER BY id")?;
    let live = query
        .query_map([], |r| {
            Ok(Placement {
                id: r.get(0)?,
                parent: r.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let live_ids: HashSet<_> = live.iter().map(|item| item.id).collect();
    let old_ids: HashMap<_, _> =
        old.iter().map(|item| (item.id, item.parent)).collect();
    let mut result = Vec::new();
    for item in old.iter().filter(|item| live_ids.contains(&item.id)) {
        let mut parent = item.parent;
        while let Some(id) = parent.filter(|id| !live_ids.contains(id)) {
            parent = *old_ids.get(&id).ok_or(ProgressError::Invalid)?;
        }
        result.push(Placement {
            id: item.id,
            parent,
        });
    }
    for item in live.iter().filter(|item| !old_ids.contains_key(&item.id)) {
        let parents: HashMap<_, _> =
            result.iter().map(|item| (item.id, item.parent)).collect();
        let mut depths = HashMap::new();
        for placed in &result {
            let depth = if let Some(parent) = placed.parent {
                *depths.get(&parent).ok_or(ProgressError::Invalid)? + 1
            } else {
                1
            };
            depths.insert(placed.id, depth);
        }
        let mut parent = item.parent;
        while let Some(id) = parent {
            if *depths.get(&id).ok_or(ProgressError::Invalid)? < 5 {
                break;
            }
            parent = *parents.get(&id).ok_or(ProgressError::Invalid)?;
        }
        let position = if let Some(parent) = parent {
            let start = result
                .iter()
                .position(|item| item.id == parent)
                .ok_or(ProgressError::Invalid)?
                + 1;
            let depth = depths[&parent];
            start
                + result[start..]
                    .iter()
                    .take_while(|item| depths[&item.id] > depth)
                    .count()
        } else {
            result.len()
        };
        result.insert(
            position,
            Placement {
                id: item.id,
                parent,
            },
        );
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seeded() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        super::super::schema::initialize(&conn).unwrap();
        conn.execute_batch("INSERT INTO todos(id,text) VALUES(1,'Parent'),(2,'Child'),(3,'Other');UPDATE todo_schema SET revision=3;").unwrap();
        initialize(&conn, 3).unwrap();
        conn
    }
    #[test]
    fn old_writer_deletion_promotes_children_without_losing_document_order() {
        let conn = seeded();
        write(
            &conn,
            &[
                Placement {
                    id: 3,
                    parent: None,
                },
                Placement {
                    id: 1,
                    parent: Some(3),
                },
                Placement {
                    id: 2,
                    parent: Some(1),
                },
            ],
        )
        .unwrap();
        project_legacy(&conn).unwrap();
        conn.execute_batch("UPDATE todos SET parent_id=NULL WHERE parent_id=1;DELETE FROM todos WHERE id=1;UPDATE todo_schema SET revision=4;").unwrap();
        assert!(validate_optional(&conn, 4).unwrap());
        initialize(&conn, 4).unwrap();
        assert_eq!(
            read(&conn)
                .unwrap()
                .iter()
                .map(|item| (item.id, item.parent))
                .collect::<Vec<_>>(),
            [(3, None), (2, Some(3))]
        );
        assert_eq!(
            conn.query_row("SELECT parent_id FROM todos WHERE id=2", [], |r| r
                .get::<_, Option<i64>>(0))
                .unwrap(),
            None
        );
        assert_eq!(
            conn.query_row("SELECT version FROM todo_schema", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn legacy_additions_preserve_old_rows_and_respect_the_editor_depth_bound() {
        let conn = Connection::open_in_memory().unwrap();
        super::super::schema::initialize(&conn).unwrap();
        for id in 1..=5 {
            conn.execute("INSERT INTO todos(id,text) VALUES(?1,'Task')", [id])
                .unwrap();
        }
        conn.execute("UPDATE todo_schema SET revision=5", [])
            .unwrap();
        initialize(&conn, 5).unwrap();
        let items: Vec<_> = (1..=5)
            .rev()
            .map(|id| Placement {
                id,
                parent: if id == 5 { None } else { Some(id + 1) },
            })
            .collect();
        write(&conn, &items).unwrap();
        project_legacy(&conn).unwrap();
        conn.execute_batch("INSERT INTO todos(id,parent_id,text) VALUES(6,1,'Legacy child'),(7,6,'Legacy grandchild');UPDATE todo_schema SET revision=7;").unwrap();
        initialize(&conn, 7).unwrap();
        let items = read(&conn).unwrap();
        assert_eq!(
            items.iter().map(|item| item.id).collect::<Vec<_>>(),
            [5, 4, 3, 2, 1, 6, 7]
        );
        assert_eq!(items[5].parent, Some(2));
        assert_eq!(items[6].parent, Some(2));
        assert!(validate_optional(&conn, 7).unwrap());
    }
    #[test]
    fn partial_future_and_corrupt_metadata_are_not_silently_repaired() {
        let conn = seeded();
        conn.execute("UPDATE todo_layout_schema SET version=2", [])
            .unwrap();
        assert!(matches!(
            validate_optional(&conn, 3),
            Err(ProgressError::NewerSchema)
        ));
        conn.execute("UPDATE todo_layout_schema SET version=1,revision=4", [])
            .unwrap();
        assert!(validate_optional(&conn, 3).is_err());
        conn.execute("UPDATE todo_layout_schema SET revision=3", [])
            .unwrap();
        conn.execute("UPDATE todo_layout SET position=position+10", [])
            .unwrap();
        assert!(validate_optional(&conn, 3).is_err());
        conn.execute("UPDATE todo_layout SET position=position-10", [])
            .unwrap();
        conn.execute("UPDATE todo_layout SET parent_id=999 WHERE id=2", [])
            .unwrap();
        assert!(validate_optional(&conn, 3).is_err());
        conn.execute("UPDATE todo_layout SET parent_id=NULL", [])
            .unwrap();
        conn.execute("DELETE FROM todo_layout WHERE id=2", [])
            .unwrap();
        assert!(validate_optional(&conn, 3).is_err());
        conn.execute("DROP TABLE todo_layout", []).unwrap();
        assert!(validate_optional(&conn, 3).is_err());
    }
}
