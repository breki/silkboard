//! Additive schema migration, isolated recovery and restart persistence.
use rusqlite::Connection;
use serde_json::json;
use silkboard_web::{
    owner::set_password_from_reader,
    plans,
    progress::{ProgressError, ProgressStore, database_command},
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "silkboard-plan-data-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&p).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        }
        Self(p)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn snapshot() -> plans::Snapshot {
    serde_json::from_value(json!({"schema_version":1,"task_id":"11111111-1111-4111-8111-111111111111","sequence":"1","plan_revision":"1","writer_generation":"1","reporter":{"hostname":"dev","tool":"codex","session_id":"native"},"title":"Stored plan","steps":[{"id":"a","title":"Work","depends_on":[],"state":"active"}]})).unwrap()
}
#[test]
fn legacy_rows_and_plan_history_survive_open_backup_restore_and_restart() {
    let d = Directory::new();
    let live = d.0.join("live.sqlite");
    let backup = d.0.join("backup.sqlite");
    let restored = d.0.join("restored.sqlite");
    let store = ProgressStore::open(&live).unwrap();
    set_password_from_reader(
        &store,
        b"private plan recovery password".as_slice(),
    )
    .unwrap();
    let token = store.issue_token("silkboard", "agent").unwrap();
    let event=serde_json::from_value(json!({"event_id":"legacy","hostname":"dev","tool":"codex","session_id":"native","sequence":1,"message":"Existing progress"})).unwrap();
    store.publish("silkboard", &token, &event).unwrap();
    drop(store);
    // A schema-3 installation before plan support has none of these tables.
    let conn = Connection::open(&live).unwrap();
    conn.execute_batch("DROP TABLE plan_step_history;DROP TABLE plan_revisions;DROP TABLE plan_snapshots;DROP TABLE task_plans;DROP TABLE plan_schema;").unwrap();
    let old: Vec<(String, Vec<u8>, i64)> = conn
        .prepare("SELECT label,token_hash,revoked FROM publishers")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    drop(conn);
    let store = ProgressStore::open(&live).unwrap();
    let ack = plans::publish(&store, "silkboard", &token, snapshot()).unwrap();
    drop(store);
    let store = ProgressStore::open(&live).unwrap();
    assert_eq!(
        plans::publish(&store, "silkboard", &token, snapshot()).unwrap(),
        ack
    );
    let conn = Connection::open(&live).unwrap();
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        conn.query_row("SELECT version FROM plan_schema", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let after: Vec<(String, Vec<u8>, i64)> = conn
        .prepare("SELECT label,token_hash,revoked FROM publishers")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(old, after);
    assert_eq!(
        conn.query_row("SELECT count(*) FROM events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
            .optional()
            .unwrap(),
        None
    );
    drop(conn);
    database_command(
        &live,
        &["backup".into(), backup.to_str().unwrap().into()],
    )
    .unwrap();
    database_command(
        &restored,
        &["restore".into(), backup.to_str().unwrap().into()],
    )
    .unwrap();
    let recovered = ProgressStore::open(&restored).unwrap();
    assert!(matches!(
        plans::publish(&recovered, "silkboard", &token, snapshot()),
        Err(ProgressError::Unauthorized)
    ));
    let conn = Connection::open(&restored).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM plan_snapshots", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM owner", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let new_token = recovered.issue_token("silkboard", "replacement").unwrap();
    let mut next = snapshot();
    next.sequence = "2".parse().unwrap();
    assert!(plans::publish(&recovered, "silkboard", &new_token, next).is_ok());
}
use rusqlite::OptionalExtension;
#[test]
fn unknown_or_partial_plan_components_fail_without_overwriting_data() {
    let d = Directory::new();
    let db = d.0.join("db.sqlite");
    drop(ProgressStore::open(&db).unwrap());
    let conn = Connection::open(&db).unwrap();
    conn.execute("UPDATE plan_schema SET version=2", [])
        .unwrap();
    drop(conn);
    let before = fs::read(&db).unwrap();
    assert!(matches!(
        ProgressStore::open(&db),
        Err(ProgressError::NewerSchema)
    ));
    assert_eq!(before, fs::read(&db).unwrap());
    let dest = d.0.join("refused.sqlite");
    assert!(matches!(
        database_command(
            &db,
            &["backup".into(), dest.to_str().unwrap().into()]
        ),
        Err(ProgressError::NewerSchema)
    ));
    assert!(!dest.exists());
    let conn = Connection::open(&db).unwrap();
    conn.execute("UPDATE plan_schema SET version=1", [])
        .unwrap();
    conn.execute("DROP TABLE plan_step_history", []).unwrap();
    drop(conn);
    assert!(ProgressStore::open(&db).is_err());
}

#[test]
fn simultaneous_writer_handoffs_have_one_winner_and_keep_received_history() {
    use std::sync::{Arc, Barrier};
    let d = Directory::new();
    let db = d.0.join("race.sqlite");
    let store = ProgressStore::open(&db).unwrap();
    set_password_from_reader(&store, b"private race test password".as_slice())
        .unwrap();
    let token = store.issue_token("silkboard", "agent").unwrap();
    plans::publish(&store, "silkboard", &token, snapshot()).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|i| {
            // Separate SQLite connections exercise transactional serialization,
            // rather than relying only on the process-local store mutex.
            let store = ProgressStore::open(&db).unwrap();
            let token = token.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut s = snapshot();
                s.sequence = "2".parse().unwrap();
                s.writer_generation = "2".parse().unwrap();
                s.reporter.session_id = format!("handoff-{i}");
                barrier.wait();
                plans::publish(&store, "silkboard", &token, s)
            })
        })
        .collect();
    let results: Vec<_> =
        threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(ProgressError::Conflict)))
            .count(),
        1
    );
    let conn = Connection::open(&db).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM plan_snapshots", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        conn.query_row("SELECT latest_sequence FROM task_plans", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
}
