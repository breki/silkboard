//! Owner-managed checklist; mutations are revision checked and reauthenticated.
pub(crate) mod schema;
use super::{AuthError, OwnerAuth, http::OwnerSession, storage};
use crate::progress::ProgressStore;
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
#[derive(Serialize)]
struct Todo {
    id: String,
    parent_id: Option<String>,
    text: String,
    completed: bool,
}
#[derive(Serialize)]
struct Checklist {
    revision: String,
    items: Vec<Todo>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Add {
    revision: String,
    parent_id: Option<String>,
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Edit {
    revision: String,
    text: String,
    completed: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Delete {
    revision: String,
}
fn number(value: &str, zero: bool) -> Result<i64, AuthError> {
    let n = value.parse::<i64>().map_err(|_| AuthError::Invalid)?;
    if n < 0 || (!zero && n == 0) || n.to_string() != value {
        return Err(AuthError::Invalid);
    }
    Ok(n)
}
fn text(value: &str) -> Result<String, AuthError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 1000
        || value.chars().any(|c| {
            c.is_control() || matches!(c, '\u{2028}' | '\u{2029}' | '\u{feff}')
        })
    {
        return Err(AuthError::Invalid);
    }
    Ok(value.to_owned())
}
fn snapshot(conn: &Connection) -> Result<Checklist, AuthError> {
    let revision: i64 = conn.query_row(
        "SELECT revision FROM todo_schema WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    let mut query = conn.prepare(
        "SELECT id,parent_id,text,completed FROM todos ORDER BY id LIMIT 1001",
    )?;
    let items = query
        .query_map([], |r| {
            Ok(Todo {
                id: r.get::<_, i64>(0)?.to_string(),
                parent_id: r.get::<_, Option<i64>>(1)?.map(|v| v.to_string()),
                text: r.get(2)?,
                completed: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if items.len() > 1000 {
        return Err(AuthError::Unavailable);
    }
    Ok(Checklist {
        revision: revision.to_string(),
        items,
    })
}
fn mutate(
    store: &ProgressStore,
    session: &str,
    revision: &str,
    change: impl FnOnce(&Connection) -> Result<(), AuthError>,
) -> Result<Checklist, AuthError> {
    let revision = number(revision, true)?;
    let mut conn = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    storage::authenticate_on(&tx, session)?;
    let current: i64 = tx.query_row(
        "SELECT revision FROM todo_schema WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    if current != revision {
        return Err(AuthError::Conflict);
    }
    let next = current.checked_add(1).ok_or(AuthError::Unavailable)?;
    change(&tx)?;
    tx.execute(
        "UPDATE todo_schema SET revision=?1 WHERE singleton=1",
        [next],
    )?;
    let result = snapshot(&tx)?;
    tx.commit()?;
    Ok(result)
}
pub(super) fn routes() -> Router<OwnerAuth> {
    Router::new()
        .route("/todos", get(list).post(add))
        .route("/todos/{id}", axum::routing::patch(edit).delete(delete))
}
async fn list(
    State(auth): State<OwnerAuth>,
) -> Result<Json<Checklist>, AuthError> {
    Ok(Json(
        auth.query(|store| {
            let mut conn =
                store.0.lock().map_err(|_| AuthError::Unavailable)?;
            let tx = conn.transaction()?;
            let result = snapshot(&tx)?;
            tx.commit()?;
            Ok(result)
        })
        .await?,
    ))
}
async fn add(
    State(auth): State<OwnerAuth>,
    Extension(session): Extension<OwnerSession>,
    input: Result<Json<Add>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<Checklist>), AuthError> {
    let Json(input) = input.map_err(|_| AuthError::Invalid)?;
    let task = text(&input.text)?;
    let parent = input
        .parent_id
        .as_deref()
        .map(|p| number(p, false))
        .transpose()?;
    let result = auth
        .query(move |store| {
            mutate(store, &session.0, &input.revision, |tx| {
                add_on(tx, parent, &task)
            })
        })
        .await?;
    Ok((StatusCode::CREATED, Json(result)))
}
fn add_on(
    conn: &Connection,
    parent: Option<i64>,
    task: &str,
) -> Result<(), AuthError> {
    let count: i64 =
        conn.query_row("SELECT count(*) FROM todos", [], |r| r.get(0))?;
    if count >= 1000 {
        return Err(AuthError::Invalid);
    }
    if let Some(parent) = parent {
        let exists = conn
            .query_row("SELECT id FROM todos WHERE id=?1", [parent], |r| {
                r.get::<_, i64>(0)
            })
            .optional()?;
        if exists.is_none() {
            return Err(AuthError::NotFound);
        }
        let depth: i64 = conn.query_row(
            "WITH RECURSIVE ancestors(id,parent_id) AS (
                SELECT id,parent_id FROM todos WHERE id=?1
                UNION ALL SELECT t.id,t.parent_id FROM todos t
                JOIN ancestors a ON a.parent_id=t.id
             ) SELECT count(*) FROM ancestors",
            [parent],
            |r| r.get(0),
        )?;
        if depth >= 5 {
            return Err(AuthError::Invalid);
        }
    }
    conn.execute(
        "INSERT INTO todos(parent_id,text) VALUES(?1,?2)",
        params![parent, task],
    )?;
    Ok(())
}
async fn edit(
    State(auth): State<OwnerAuth>,
    Extension(session): Extension<OwnerSession>,
    Path(id): Path<String>,
    input: Result<Json<Edit>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Checklist>, AuthError> {
    let Json(input) = input.map_err(|_| AuthError::Invalid)?;
    let id = number(&id, false)?;
    let task = text(&input.text)?;
    Ok(Json(
        auth.query(move |store| {
            mutate(store, &session.0, &input.revision, |tx| {
                if tx.execute(
                    "UPDATE todos SET text=?1,completed=?2 WHERE id=?3",
                    params![task, input.completed, id],
                )? == 0
                {
                    return Err(AuthError::NotFound);
                }
                Ok(())
            })
        })
        .await?,
    ))
}
async fn delete(
    State(auth): State<OwnerAuth>,
    Extension(session): Extension<OwnerSession>,
    Path(id): Path<String>,
    input: Result<Json<Delete>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Checklist>, AuthError> {
    let Json(input) = input.map_err(|_| AuthError::Invalid)?;
    let id = number(&id, false)?;
    Ok(Json(
        auth.query(move |store| {
            mutate(store, &session.0, &input.revision, |tx| {
                let parent: Option<i64> = tx
                    .query_row(
                        "SELECT parent_id FROM todos WHERE id=?1",
                        [id],
                        |r| r.get(0),
                    )
                    .optional()?
                    .ok_or(AuthError::NotFound)?;
                tx.execute(
                    "UPDATE todos SET parent_id=?1 WHERE parent_id=?2",
                    params![parent, id],
                )?;
                tx.execute("DELETE FROM todos WHERE id=?1", [id])?;
                Ok(())
            })
        })
        .await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capacity_and_snapshot_limits_never_truncate_existing_todos() {
        let store = ProgressStore::open_memory().unwrap();
        let conn = store.0.lock().unwrap();
        for _ in 0..1000 {
            add_on(&conn, None, "Task").unwrap();
        }
        assert!(matches!(
            add_on(&conn, None, "Too many"),
            Err(AuthError::Invalid)
        ));
        assert_eq!(snapshot(&conn).unwrap().items.len(), 1000);
        conn.execute("INSERT INTO todos(text) VALUES('Oversize')", [])
            .unwrap();
        assert!(matches!(snapshot(&conn), Err(AuthError::Unavailable)));
    }
    #[test]
    fn failed_change_rolls_back_and_revoked_session_cannot_mutate() {
        let store = ProgressStore::open_memory().unwrap();
        let password = b"private checklist transaction test password";
        super::super::set_password_from_reader(&store, password.as_slice())
            .unwrap();
        let session =
            storage::login(&store, std::str::from_utf8(password).unwrap())
                .unwrap();
        let result = mutate(&store, &session, "0", |conn| {
            add_on(conn, None, "Must roll back")?;
            Err(AuthError::Invalid)
        });
        assert!(matches!(result, Err(AuthError::Invalid)));
        assert_eq!(snapshot(&store.0.lock().unwrap()).unwrap().items.len(), 0);
        storage::logout(&store, &session).unwrap();
        assert!(matches!(
            mutate(&store, &session, "0", |_| Ok(())),
            Err(AuthError::Unauthorized)
        ));
    }
}
