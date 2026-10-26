//! Latest received activity per full native identity across both report histories.
use super::{AuthError, OwnerAuth, agent_identity::AgentIdentity};
use crate::progress::{ProgressStore, validate_identity, validate_project};
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    received_at: i64,
    project: String,
    hostname: String,
    tool: String,
    session_id: String,
    subagent_id: String,
}
impl Cursor {
    fn parse(value: &str) -> Result<Self, AuthError> {
        if value.len() > 4096 {
            return Err(AuthError::Invalid);
        }
        let cursor: Self =
            serde_json::from_str(value).map_err(|_| AuthError::Invalid)?;
        if !(0..=8_640_000_000_000_000).contains(&cursor.received_at) {
            return Err(AuthError::Invalid);
        }
        validate_project(&cursor.project).map_err(|_| AuthError::Invalid)?;
        for value in [&cursor.hostname, &cursor.tool, &cursor.session_id]
            .into_iter()
            .chain(
                (!cursor.subagent_id.is_empty()).then_some(&cursor.subagent_id),
            )
        {
            validate_identity(value).map_err(|_| AuthError::Invalid)?;
        }
        Ok(cursor)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Before {
    before: Option<String>,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Activity {
    Event {
        event_id: String,
        message: String,
    },
    Workflow {
        task_id: String,
        sequence: String,
        title: String,
        state: String,
    },
}
#[derive(Serialize)]
struct Agent {
    project: String,
    #[serde(flatten)]
    identity: AgentIdentity,
    received_at: i64,
    latest: Activity,
}
#[derive(Serialize)]
struct Page {
    items: Vec<Agent>,
    next_cursor: Option<String>,
}
pub(super) fn routes() -> Router<OwnerAuth> {
    Router::new().route("/agents", get(agents))
}
async fn agents(
    State(auth): State<OwnerAuth>,
    query: Result<Query<Before>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Page>, AuthError> {
    let Query(query) = query.map_err(|_| AuthError::Invalid)?;
    let before = query.before.as_deref().map(Cursor::parse).transpose()?;
    Ok(Json(
        auth.query(move |store| list(store, before.as_ref()))
            .await?,
    ))
}
fn list(
    store: &ProgressStore,
    before: Option<&Cursor>,
) -> Result<Page, AuthError> {
    let conn = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    let mut statement = conn.prepare(include_str!("agents.sql"))?;
    let mut items = statement
        .query_map(
            params![
                before.map_or(i64::MAX, |c| c.received_at),
                before.map_or("", |c| c.project.as_str()),
                before.map_or("", |c| c.hostname.as_str()),
                before.map_or("", |c| c.tool.as_str()),
                before.map_or("", |c| c.session_id.as_str()),
                before.map_or("", |c| c.subagent_id.as_str()),
            ],
            |row| {
                let latest = if row.get::<_, i64>(8)? == 1 {
                    Activity::Workflow {
                        task_id: row.get(9)?,
                        sequence: row.get::<_, i64>(10)?.to_string(),
                        title: row.get(11)?,
                        state: row.get(12)?,
                    }
                } else {
                    Activity::Event {
                        event_id: row.get(9)?,
                        message: row.get(11)?,
                    }
                };
                Ok(Agent {
                    project: row.get(0)?,
                    identity: AgentIdentity::read(row, 1)?,
                    received_at: row.get(7)?,
                    latest,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let more = items.len() > 20;
    items.truncate(20);
    let next_cursor = items.last().filter(|_| more).map(|item| {
        serde_json::json!({
            "received_at": item.received_at,
            "project": item.project,
            "hostname": item.identity.hostname,
            "tool": item.identity.tool,
            "session_id": item.identity.session_id,
            "subagent_id": item.identity.subagent_id.as_deref().unwrap_or("")
        })
        .to_string()
    });
    Ok(Page { items, next_cursor })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    fn event(store: &ProgressStore, key: [&str; 5], id: &str, at: i64) {
        let conn = store.0.lock().unwrap();
        conn.execute("INSERT OR IGNORE INTO projects VALUES (?1)", [key[0]])
            .unwrap();
        conn.execute("INSERT OR IGNORE INTO sessions(project,hostname,tool,native_id,child_id) VALUES (?1,?2,?3,?4,?5)",params![key[0],key[1],key[2],key[3],key[4]]).unwrap();
        let session:i64=conn.query_row("SELECT id FROM sessions WHERE project=?1 AND hostname=?2 AND tool=?3 AND native_id=?4 AND child_id=?5",params![key[0],key[1],key[2],key[3],key[4]],|r|r.get(0)).unwrap();
        conn.execute("INSERT INTO events(project,event_id,session,sequence,message,session_name,payload,received_at) VALUES (?1,?2,?3,(SELECT coalesce(max(sequence),0)+1 FROM events WHERE session=?3),?2,'Same name','{}',?4)",params![key[0],id,session,at]).unwrap();
    }
    fn workflow(conn: &Connection, key: [&str; 5], at: i64) {
        conn.execute(
            "INSERT OR IGNORE INTO projects(id) VALUES (?1)",
            [key[0]],
        )
        .unwrap();
        let summary=serde_json::json!({"reporter":{"hostname":key[1],"tool":key[2],"session_id":key[3],"subagent_id":if key[4].is_empty(){None}else{Some(key[4])},"session_name":"Same name"},"title":"Reported workflow","state":"active"}).to_string();
        let task = "11111111-1111-4111-8111-111111111111";
        conn.execute_batch("BEGIN; PRAGMA defer_foreign_keys=ON;")
            .unwrap();
        conn.execute(
            "INSERT INTO task_plans VALUES (?1,?2,1,?3,?4)",
            params![key[0], task, at, summary],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO plan_snapshots VALUES (?1,?2,1,1,1,'{}',?3,?4)",
            params![key[0], task, summary, at],
        )
        .unwrap();
        conn.execute_batch("COMMIT").unwrap();
    }
    #[test]
    fn all_identity_parts_and_complete_history_precede_pagination() {
        let store = ProgressStore::open_memory().unwrap();
        for n in 0..25 {
            event(
                &store,
                ["alpha", "dev", "codex", &format!("native-{n:02}"), ""],
                &format!("e-{n}"),
                100,
            );
        }
        for n in 0..30 {
            event(
                &store,
                ["alpha", "dev", "codex", "native-00", ""],
                &format!("busy-{n}"),
                200 + n,
            );
        }
        for key in [
            ["bravo", "dev", "codex", "native-00", ""],
            ["alpha", "other", "codex", "native-00", ""],
            ["alpha", "dev", "claude", "native-00", ""],
            ["alpha", "dev", "codex", "native-00", "child-a"],
            ["alpha", "dev", "codex", "native-00", "child-b"],
        ] {
            event(
                &store,
                key,
                &format!("unique-{}-{}-{}-{}", key[0], key[1], key[2], key[4]),
                100,
            );
        }
        let first = list(&store, None).unwrap();
        assert_eq!(first.items.len(), 20);
        assert_eq!(first.items[0].received_at, 229);
        let cursor =
            Cursor::parse(first.next_cursor.as_deref().unwrap()).unwrap();
        store.0.lock().unwrap().execute_batch("VACUUM").unwrap();
        let last = list(&store, Some(&cursor)).unwrap();
        assert_eq!(last.items.len(), 8);
        assert!(last.next_cursor.is_none());
        let keys: std::collections::HashSet<_> = first
            .items
            .iter()
            .chain(&last.items)
            .map(|a| {
                (
                    &a.project,
                    &a.identity.hostname,
                    &a.identity.tool,
                    &a.identity.session_id,
                    &a.identity.subagent_id,
                )
            })
            .collect();
        assert_eq!(keys.len(), 28);
    }
    #[test]
    fn children_never_fill_pages_or_override_main_activity() {
        let store = ProgressStore::open_memory().unwrap();
        workflow(
            &store.0.lock().unwrap(),
            ["alpha", "dev", "codex", "workflow-child", "child"],
            2000,
        );
        let children_only = list(&store, None).unwrap();
        assert!(children_only.items.is_empty());
        assert!(children_only.next_cursor.is_none());
        event(
            &store,
            ["alpha", "dev", "codex", "root", ""],
            "main-event",
            100,
        );
        for n in 0..45 {
            event(
                &store,
                ["alpha", "dev", "codex", "root", &format!("child-{n}")],
                &format!("child-event-{n}"),
                1000 + n,
            );
        }
        let result = list(&store, None).unwrap();
        assert_eq!(result.items.len(), 1);
        assert!(result.next_cursor.is_none());
        let main = &result.items[0];
        assert_eq!(main.identity.session_id, "root");
        assert_eq!(main.received_at, 100);
        assert!(
            matches!(&main.latest, Activity::Event{event_id,..} if event_id == "main-event")
        );
        assert_eq!(
            store
                .0
                .lock()
                .unwrap()
                .query_row("SELECT count(*) FROM events", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            46
        );
    }
    #[test]
    fn workflow_wins_ties_but_later_received_event_wins_even_with_lower_sequence()
     {
        let store = ProgressStore::open_memory().unwrap();
        let key = ["alpha", "dev", "codex", "native", ""];
        event(&store, key, "earlier", 100);
        workflow(&store.0.lock().unwrap(), key, 100);
        assert!(matches!(
            list(&store, None).unwrap().items[0].latest,
            Activity::Workflow { .. }
        ));
        store
            .0
            .lock()
            .unwrap()
            .execute("UPDATE events SET sequence=10", [])
            .unwrap();
        event(&store, key, "late", 101);
        store
            .0
            .lock()
            .unwrap()
            .execute("UPDATE events SET sequence=1 WHERE event_id='late'", [])
            .unwrap();
        assert!(
            matches!(&list(&store,None).unwrap().items[0].latest,Activity::Event{event_id,..} if event_id=="late")
        );
    }
    #[test]
    fn cursor_rejects_invalid_scope_and_timestamps() {
        let valid = serde_json::json!({"received_at":0,"project":"alpha","hostname":"dev","tool":"codex","session_id":"native","subagent_id":""});
        assert!(Cursor::parse(&valid.to_string()).is_ok());
        for (field, value) in [
            ("received_at", serde_json::json!(-1)),
            ("project", serde_json::json!("BAD")),
            ("hostname", serde_json::json!("")),
            ("tool", serde_json::json!("bad tool")),
            ("session_id", serde_json::json!("\n")),
            ("subagent_id", serde_json::json!("bad child")),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            assert!(Cursor::parse(&invalid.to_string()).is_err());
        }
        assert!(Cursor::parse("bad").is_err());
        assert!(Cursor::parse(&"x".repeat(4097)).is_err());
    }
}
