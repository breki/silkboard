//! Bounded owner-only projections; never return credential or raw payload rows.

use super::{AuthError, OwnerAuth};
use crate::progress::{ProgressStore, validate_project};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

const PAGE_SIZE: usize = 20;

#[derive(Serialize)]
struct Page<T> {
    items: Vec<T>,
    next_cursor: Option<String>,
}

fn page<T>(mut items: Vec<T>, cursor: impl FnOnce(&T) -> String) -> Page<T> {
    let more = items.len() > PAGE_SIZE;
    items.truncate(PAGE_SIZE);
    let next_cursor = items.last().filter(|_| more).map(cursor);
    Page { items, next_cursor }
}

#[derive(Serialize)]
struct Project {
    id: String,
}

#[derive(Serialize)]
struct ProgressMessage {
    event_id: String,
    sequence: String,
    received_at: i64,
    message: String,
}

#[derive(Serialize)]
struct AgentSession {
    id: String,
    hostname: String,
    tool: String,
    session_id: String,
    subagent_id: Option<String>,
    parent_session_id: Option<String>,
    session_name: Option<String>,
    latest: ProgressMessage,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct After {
    after: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Before {
    before: Option<String>,
}

pub(super) fn routes() -> Router<OwnerAuth> {
    Router::new()
        .route("/projects", get(projects))
        .route("/projects/{project}/sessions", get(sessions))
        .route(
            "/projects/{project}/sessions/{session}/events",
            get(history),
        )
}

fn positive(value: &str) -> Result<i64, AuthError> {
    let parsed = value.parse::<i64>().map_err(|_| AuthError::Invalid)?;
    if parsed <= 0 || value != parsed.to_string() {
        return Err(AuthError::Invalid);
    }
    Ok(parsed)
}

async fn projects(
    State(auth): State<OwnerAuth>,
    Query(query): Query<After>,
) -> Result<Json<Page<Project>>, AuthError> {
    if let Some(after) = &query.after {
        validate_project(after).map_err(|_| AuthError::Invalid)?;
    }
    let result = auth
        .query(move |store| {
            let conn = store.0.lock().map_err(|_| AuthError::Unavailable)?;
            let mut statement = conn.prepare(
                "SELECT id FROM projects
                 WHERE id > coalesce(?1,'') ORDER BY id LIMIT 21",
            )?;
            let items = statement
                .query_map([query.after], |r| Ok(Project { id: r.get(0)? }))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(page(items, |p| p.id.clone()))
        })
        .await?;
    Ok(Json(result))
}

async fn sessions(
    State(auth): State<OwnerAuth>,
    Path(project): Path<String>,
    Query(query): Query<Before>,
) -> Result<Json<Page<AgentSession>>, AuthError> {
    validate_project(&project).map_err(|_| AuthError::Invalid)?;
    let before = query.before.as_deref().map(positive).transpose()?;
    let result = auth
        .query(move |store| list_sessions(store, &project, before))
        .await?;
    Ok(Json(result))
}

fn list_sessions(
    store: &ProgressStore,
    project: &str,
    before: Option<i64>,
) -> Result<Page<AgentSession>, AuthError> {
    let conn = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    let exists = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?1)",
        [project],
        |r| r.get::<_, bool>(0),
    )?;
    if !exists {
        return Err(AuthError::NotFound);
    }
    let mut statement = conn.prepare(
        "SELECT s.id,s.hostname,s.tool,s.native_id,
                NULLIF(s.child_id,''),s.parent_id,
           (SELECT session_name FROM events named
            WHERE named.session=s.id AND session_name IS NOT NULL
            ORDER BY sequence DESC LIMIT 1),
           e.event_id,e.sequence,e.received_at,e.message
         FROM sessions s JOIN events e ON e.session=s.id
           AND e.sequence=(SELECT MAX(sequence) FROM events
                           WHERE session=s.id)
         WHERE s.project=?1 AND (?2 IS NULL OR s.id < ?2)
         ORDER BY s.id DESC LIMIT 21",
    )?;
    let items = statement
        .query_map(params![project, before], |r| {
            Ok(AgentSession {
                id: r.get::<_, i64>(0)?.to_string(),
                hostname: r.get(1)?,
                tool: r.get(2)?,
                session_id: r.get(3)?,
                subagent_id: r.get(4)?,
                parent_session_id: r.get(5)?,
                session_name: r.get(6)?,
                latest: ProgressMessage {
                    event_id: r.get(7)?,
                    sequence: r.get::<_, i64>(8)?.to_string(),
                    received_at: r.get(9)?,
                    message: r.get(10)?,
                },
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(page(items, |s| s.id.clone()))
}

async fn history(
    State(auth): State<OwnerAuth>,
    Path((project, session)): Path<(String, String)>,
    Query(query): Query<Before>,
) -> Result<Json<Page<ProgressMessage>>, AuthError> {
    validate_project(&project).map_err(|_| AuthError::Invalid)?;
    let session = positive(&session)?;
    let before = query.before.as_deref().map(positive).transpose()?;
    let result = auth
        .query(move |store| {
            let conn = store.0.lock().map_err(|_| AuthError::Unavailable)?;
            let found = conn
                .query_row(
                    "SELECT id FROM sessions WHERE project=?1 AND id=?2",
                    params![project, session],
                    |r| r.get::<_, i64>(0),
                )
                .optional()?;
            if found.is_none() {
                return Err(AuthError::NotFound);
            }
            let mut statement = conn.prepare(
                "SELECT event_id,sequence,received_at,message FROM events
                 WHERE project=?1 AND session=?2
                   AND (?3 IS NULL OR sequence < ?3)
                 ORDER BY sequence DESC LIMIT 21",
            )?;
            let items = statement
                .query_map(params![project, session, before], |r| {
                    Ok(ProgressMessage {
                        event_id: r.get(0)?,
                        sequence: r.get::<_, i64>(1)?.to_string(),
                        received_at: r.get(2)?,
                        message: r.get(3)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(page(items, |e| e.sequence.clone()))
        })
        .await?;
    Ok(Json(result))
}
