//! Bounded owner-only projections; never return credential or raw payload rows.

use super::{AuthError, OwnerAuth, agent_identity::AgentIdentity};
use crate::progress::{ProgressStore, validate_identity, validate_project};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use rusqlite::{OptionalExtension, Row, params};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow_task_id: Option<String>,
}

#[derive(Serialize)]
struct AgentSession {
    id: String,
    #[serde(flatten)]
    identity: AgentIdentity,
    latest: ProgressMessage,
}

impl ProgressMessage {
    fn read(row: &Row<'_>, start: usize) -> rusqlite::Result<Self> {
        Ok(Self {
            event_id: row.get(start)?,
            sequence: row.get::<_, i64>(start + 1)?.to_string(),
            received_at: row.get(start + 2)?,
            message: row.get(start + 3)?,
            workflow_task_id: row.get(start + 4)?,
            description: row.get(start + 5)?,
        })
    }
}

#[derive(Serialize)]
struct FeedEvent {
    project: String,
    #[serde(flatten)]
    identity: AgentIdentity,
    #[serde(flatten)]
    message: ProgressMessage,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventCursor {
    received_at: i64,
    project: String,
    event_id: String,
}

impl EventCursor {
    fn parse(value: &str) -> Result<Self, AuthError> {
        if value.len() > 2048 {
            return Err(AuthError::Invalid);
        }
        let cursor: Self =
            serde_json::from_str(value).map_err(|_| AuthError::Invalid)?;
        if !(0..=8_640_000_000_000_000).contains(&cursor.received_at) {
            return Err(AuthError::Invalid);
        }
        validate_project(&cursor.project).map_err(|_| AuthError::Invalid)?;
        validate_identity(&cursor.event_id).map_err(|_| AuthError::Invalid)?;
        Ok(cursor)
    }
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

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct EventQuery {
    before: Option<String>,
    project: Option<String>,
    agent: Option<String>,
    from: Option<i64>,
    to: Option<i64>,
    hostname: Option<String>,
    tool: Option<String>,
    session_id: Option<String>,
    subagent_id: Option<String>,
}
impl EventQuery {
    fn validate(&self) -> Result<(), AuthError> {
        super::filters::validate(
            self.project.as_deref(),
            self.agent.as_deref(),
        )?;
        let fields = [
            &self.hostname,
            &self.tool,
            &self.session_id,
            &self.subagent_id,
        ];
        if fields.iter().any(|field| field.is_some()) {
            if self.project.is_none()
                || fields.iter().any(|field| field.is_none())
            {
                return Err(AuthError::Invalid);
            }
            for value in [&self.hostname, &self.tool, &self.session_id]
                .into_iter()
                .flatten()
            {
                validate_identity(value).map_err(|_| AuthError::Invalid)?;
            }
            if let Some(child) = self
                .subagent_id
                .as_deref()
                .filter(|child| !child.is_empty())
            {
                validate_identity(child).map_err(|_| AuthError::Invalid)?;
            }
        }
        if self
            .from
            .into_iter()
            .chain(self.to)
            .any(|v| !(0..=8_640_000_000_000_000).contains(&v))
            || matches!((self.from, self.to), (Some(a), Some(b)) if a > b)
        {
            return Err(AuthError::Invalid);
        }
        Ok(())
    }
}

pub(super) fn routes() -> Router<OwnerAuth> {
    Router::new()
        .route("/projects", get(projects))
        .route("/events", get(events))
        .route("/projects/{project}/sessions", get(sessions))
        .route(
            "/projects/{project}/sessions/{session}/events",
            get(history),
        )
}

async fn events(
    State(auth): State<OwnerAuth>,
    query: Result<Query<EventQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Page<FeedEvent>>, AuthError> {
    let Query(query) = query.map_err(|_| AuthError::Invalid)?;
    query.validate()?;
    let cursor = query
        .before
        .as_deref()
        .map(EventCursor::parse)
        .transpose()?;
    let result = auth
        .query(move |store| list_events(store, cursor.as_ref(), &query))
        .await?;
    Ok(Json(result))
}

fn list_events(
    store: &ProgressStore,
    before: Option<&EventCursor>,
    filters: &EventQuery,
) -> Result<Page<FeedEvent>, AuthError> {
    let conn = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    let mut statement = conn.prepare(
        "SELECT e.project,s.hostname,s.tool,s.native_id,
                NULLIF(s.child_id,''),s.parent_id,
           (SELECT session_name FROM events named
            WHERE named.session=s.id AND named.session_name IS NOT NULL
              AND named.sequence<=e.sequence
            ORDER BY named.sequence DESC LIMIT 1),
           e.event_id,e.sequence,e.received_at,e.message,
           (SELECT p.task_id FROM task_plans p WHERE p.project=e.project
            AND p.task_id=json_extract(e.payload,'$.task_id')),
           json_extract(e.payload,'$.description')
         FROM events e JOIN sessions s ON s.id=e.session
         WHERE (e.received_at,e.project,e.event_id)<(?1,?2,?3)
           AND (?4 IS NULL OR e.project=?4)
           AND (?5 IS NULL OR instr(lower(s.tool || ' ' || s.hostname),lower(?5))>0)
           AND (?6 IS NULL OR e.received_at>=?6)
           AND (?7 IS NULL OR e.received_at<=?7)
           AND (?8 IS NULL OR (s.hostname=?8 AND s.tool=?9 AND s.native_id=?10 AND s.child_id=?11))
         ORDER BY e.received_at DESC,e.project DESC,e.event_id DESC
         LIMIT 21",
    )?;
    let items = statement
        .query_map(
            params![
                before.map_or(i64::MAX, |c| c.received_at),
                before.map_or("", |c| c.project.as_str()),
                before.map_or("", |c| c.event_id.as_str()),
                filters.project,
                filters.agent,
                filters.from,
                filters.to,
                filters.hostname,
                filters.tool,
                filters.session_id,
                filters.subagent_id,
            ],
            |row| {
                Ok(FeedEvent {
                    project: row.get(0)?,
                    identity: AgentIdentity::read(row, 1)?,
                    message: ProgressMessage::read(row, 7)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(page(items, |event| {
        serde_json::json!({
            "received_at": event.message.received_at,
            "project": event.project,
            "event_id": event.message.event_id,
        })
        .to_string()
    }))
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
           e.event_id,e.sequence,e.received_at,e.message,
           (SELECT p.task_id FROM task_plans p WHERE p.project=e.project
            AND p.task_id=json_extract(e.payload,'$.task_id')),
           json_extract(e.payload,'$.description')
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
                identity: AgentIdentity::read(r, 1)?,
                latest: ProgressMessage::read(r, 7)?,
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
                "SELECT e.event_id,e.sequence,e.received_at,e.message,
                   (SELECT p.task_id FROM task_plans p WHERE p.project=e.project
                    AND p.task_id=json_extract(e.payload,'$.task_id')),
                   json_extract(e.payload,'$.description')
                 FROM events e WHERE e.project=?1 AND e.session=?2
                   AND (?3 IS NULL OR e.sequence < ?3)
                 ORDER BY e.sequence DESC LIMIT 21",
            )?;
            let items = statement
                .query_map(params![project, session, before], |r| {
                    ProgressMessage::read(r, 0)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(page(items, |e| e.sequence.clone()))
        })
        .await?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tied_receipt_times_paginate_stably_even_after_vacuum() {
        let store = ProgressStore::open_memory().unwrap();
        {
            let conn = store.0.lock().unwrap();
            for (session, project) in [(1, "alpha"), (2, "bravo")] {
                conn.execute("INSERT INTO projects VALUES (?1)", [project])
                    .unwrap();
                conn.execute("INSERT INTO sessions VALUES (?1,?2,'dev','codex','native','',NULL)",params![session,project]).unwrap();
                for index in 1..=13 {
                    conn.execute("INSERT INTO events(project,event_id,session,sequence,message,payload,received_at) VALUES (?1,?2,?3,?4,'A report','{}',1000)",params![project,format!("{index:02}"),session,index]).unwrap();
                }
            }
            conn.execute_batch(
                "UPDATE events SET session_name='Earlier name' WHERE sequence=1;
                 UPDATE events SET session_name='Later name' WHERE sequence=10",
            ).unwrap();
        }
        let filtered = list_events(
            &store,
            None,
            &EventQuery {
                project: Some("alpha".into()),
                agent: Some("CODEX dev".into()),
                from: Some(1000),
                to: Some(1000),
                ..EventQuery::default()
            },
        )
        .unwrap();
        assert_eq!(filtered.items.len(), 13);
        assert!(filtered.next_cursor.is_none());
        let first = list_events(&store, None, &EventQuery::default()).unwrap();
        assert_eq!(first.items.len(), 20);
        assert_eq!(first.items[0].project, "bravo");
        assert_eq!(
            first.items[0].identity.session_name.as_deref(),
            Some("Later name")
        );
        let earlier = first
            .items
            .iter()
            .find(|item| {
                item.project == "bravo" && item.message.event_id == "09"
            })
            .unwrap();
        assert_eq!(
            earlier.identity.session_name.as_deref(),
            Some("Earlier name")
        );
        let cursor =
            EventCursor::parse(first.next_cursor.as_deref().unwrap()).unwrap();
        store.0.lock().unwrap().execute_batch("VACUUM").unwrap();
        let last =
            list_events(&store, Some(&cursor), &EventQuery::default()).unwrap();
        assert_eq!(last.items.len(), 6);
        assert_eq!(last.items.last().unwrap().message.event_id, "01");
        assert_eq!(last.items.last().unwrap().project, "alpha");
        assert!(last.next_cursor.is_none());
    }
}
