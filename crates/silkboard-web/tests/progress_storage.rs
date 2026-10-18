//! `SQLite` migrations, identities, retry ordering and operator boundaries.

use rusqlite::Connection;
use serde_json::json;
use silkboard_web::progress::{
    ProgressError, ProgressEvent, ProgressStore, publisher_command,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "silkboard-progress-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        Self(dir.join("progress.sqlite"))
    }
    fn connection(&self) -> Connection {
        Connection::open(&self.0).unwrap()
    }
    fn count(&self, table: &str) -> i64 {
        self.connection()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
                r.get(0)
            })
            .unwrap()
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        fs::remove_dir_all(self.0.parent().unwrap()).unwrap();
    }
}

fn event(id: &str, sequence: i64) -> ProgressEvent {
    serde_json::from_value(json!({"event_id":id,"hostname":"dev", "tool":"codex", "session_id":"native", "sequence":sequence,"message":"Running tests"})).unwrap()
}

#[test]
fn migration_restart_identities_and_out_of_order_history() {
    let db = Database::new();
    let store = ProgressStore::open(&db.0).unwrap();
    let token = store.issue_token("silkboard", "first").unwrap();
    let foreign = store.issue_token("other", "first").unwrap();
    let second = event("second", 2);
    let ack = store.publish("silkboard", &token, &second).unwrap();
    assert!(ack.received_at > 0);
    drop(store);
    let store = ProgressStore::open(&db.0).unwrap();
    assert_eq!(
        store
            .publish("silkboard", &token, &second)
            .unwrap()
            .received_at,
        ack.received_at
    );
    store
        .publish("silkboard", &token, &event("first", 1))
        .unwrap();
    let mut sequences = db
        .connection()
        .prepare("SELECT sequence FROM events ORDER BY sequence")
        .unwrap()
        .query_map([], |r| r.get::<_, i64>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(sequences, [1, 2]);
    sequences.clear();
    assert!(matches!(
        store.publish("silkboard", &token, &event("collision", 1)),
        Err(ProgressError::Conflict)
    ));
    assert_eq!(db.count("events"), 2);
    for (index, field) in ["hostname", "tool", "session_id", "subagent_id"]
        .iter()
        .enumerate()
    {
        let mut value =
            serde_json::to_value(event(&format!("independent-{index}"), 1))
                .unwrap();
        value[*field] = json!("another");
        if *field == "subagent_id" {
            value["parent_session_id"] = json!("native");
        }
        let independent = serde_json::from_value(value).unwrap();
        store.publish("silkboard", &token, &independent).unwrap();
    }
    store
        .publish("other", &foreign, &event("first", 1))
        .unwrap();
    assert_eq!(db.count("sessions"), 6);
    assert_eq!(db.count("events"), 7);
    let mut renamed = event("renamed", 3);
    renamed.session_name = Some("Investigate test failures".into());
    store.publish("silkboard", &token, &renamed).unwrap();
    assert_eq!(db.count("sessions"), 6);
    assert_eq!(
        db.connection()
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    store.revoke_token("silkboard", "first").unwrap();
    drop(store);
    assert!(matches!(
        ProgressStore::open(&db.0).unwrap().publish(
            "silkboard",
            &token,
            &second
        ),
        Err(ProgressError::Unauthorized)
    ));
    assert_eq!(db.count("events"), 8);
}

#[test]
fn credentials_are_hashed_scoped_and_cannot_be_silently_replaced() {
    let db = Database::new();
    let store = ProgressStore::open(&db.0).unwrap();
    for invalid in ["", "Wrong", "a/b", &"a".repeat(65)] {
        assert!(matches!(
            store.issue_token(invalid, "label"),
            Err(ProgressError::Invalid)
        ));
    }
    assert!(matches!(
        store.issue_token("project", "has space"),
        Err(ProgressError::Invalid)
    ));
    let token = store.issue_token("project", "label").unwrap();
    assert_eq!(token.len(), 64);
    let hash: Vec<u8> = db
        .connection()
        .query_row("SELECT token_hash FROM publishers", [], |r| r.get(0))
        .unwrap();
    assert_eq!(hash.len(), 32);
    assert_ne!(hash, token.as_bytes());
    assert!(matches!(
        store.issue_token("project", "label"),
        Err(ProgressError::Conflict)
    ));
    store.revoke_token("project", "label").unwrap();
    assert!(matches!(
        store.issue_token("project", "label"),
        Err(ProgressError::Conflict)
    ));
    assert_eq!(db.count("publishers"), 1);
    assert!(matches!(
        store.publish("other", &token, &event("event", 1)),
        Err(ProgressError::Unauthorized)
    ));
    assert!(matches!(
        store.publish("project", "wrong", &event("event", 1)),
        Err(ProgressError::Unauthorized)
    ));
    assert!(matches!(
        store.revoke_token("Wrong", "label"),
        Err(ProgressError::Invalid)
    ));
    assert!(matches!(
        store.revoke_token("project", "bad label"),
        Err(ProgressError::Invalid)
    ));
}

#[test]
fn parent_metadata_is_immutable_and_failed_events_leave_no_partial_session() {
    let db = Database::new();
    let store = ProgressStore::open(&db.0).unwrap();
    let token = store.issue_token("project", "label").unwrap();
    let mut child = event("child", 1);
    child.subagent_id = Some("child".into());
    child.parent_session_id = Some("parent".into());
    store.publish("project", &token, &child).unwrap();
    child.event_id = "changed-parent".into();
    child.sequence = 2;
    child.parent_session_id = Some("another".into());
    assert!(matches!(
        store.publish("project", &token, &child),
        Err(ProgressError::Conflict)
    ));
    child.message = String::new();
    child.session_id = "new-session".into();
    assert!(matches!(
        store.publish("project", &token, &child),
        Err(ProgressError::Invalid)
    ));
    assert_eq!(db.count("events"), 1);
    assert_eq!(db.count("sessions"), 1);
}

#[test]
fn newer_or_broken_schema_is_rejected_without_resetting_data() {
    let db = Database::new();
    let store = ProgressStore::open(&db.0).unwrap();
    store.issue_token("project", "label").unwrap();
    drop(store);
    db.connection()
        .pragma_update(None, "user_version", 3)
        .unwrap();
    assert!(matches!(
        ProgressStore::open(&db.0),
        Err(ProgressError::NewerSchema)
    ));
    assert_eq!(db.count("publishers"), 1);
    db.connection()
        .pragma_update(None, "user_version", 0)
        .unwrap();
    assert!(matches!(
        ProgressStore::open(&db.0),
        Err(ProgressError::Unavailable)
    ));
    assert_eq!(db.count("publishers"), 1);
}

#[test]
fn operator_writes_private_files_and_revokes_without_token_arguments() {
    let db = Database::new();
    let token_file = db.0.with_file_name("token");
    let issue = vec![
        "issue".into(),
        "project".into(),
        "label".into(),
        token_file.to_str().unwrap().into(),
    ];
    publisher_command(&db.0, &issue).unwrap();
    let token = fs::read_to_string(&token_file).unwrap();
    assert_eq!(token.trim().len(), 64);
    assert!(publisher_command(&db.0, &issue).is_err());
    assert_eq!(fs::read_to_string(&token_file).unwrap(), token);
    let store = ProgressStore::open(&db.0).unwrap();
    store
        .publish("project", token.trim(), &event("event", 1))
        .unwrap();
    publisher_command(
        &db.0,
        &["revoke".into(), "project".into(), "label".into()],
    )
    .unwrap();
    assert!(matches!(
        store.publish("project", token.trim(), &event("next", 2)),
        Err(ProgressError::Unauthorized)
    ));
    assert!(publisher_command(&db.0, &[]).is_err());
    fs::remove_file(&token_file).unwrap();
    assert!(publisher_command(&db.0, &issue).is_err());
    assert!(!token_file.exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(fs::metadata(&db.0).unwrap().mode() & 0o777, 0o600);
    }
}

#[cfg(unix)]
#[test]
fn rejects_public_storage_and_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let db = Database::new();
    fs::set_permissions(
        db.0.parent().unwrap(),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    assert!(matches!(
        ProgressStore::open(&db.0),
        Err(ProgressError::Invalid)
    ));
    fs::set_permissions(
        db.0.parent().unwrap(),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    ProgressStore::open(&db.0).unwrap();
    fs::set_permissions(&db.0, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        ProgressStore::open(&db.0),
        Err(ProgressError::Invalid)
    ));
    let link = db.0.with_file_name("link");
    symlink(&db.0, &link).unwrap();
    assert!(matches!(
        ProgressStore::open(&link),
        Err(ProgressError::Invalid)
    ));
    assert!(matches!(
        ProgressStore::open(&db.0.join("not-a-dir")),
        Err(ProgressError::Invalid)
    ));
    assert!(matches!(
        ProgressStore::open(&db.0.with_file_name("missing").join("db")),
        Err(ProgressError::Unavailable)
    ));
}

#[test]
fn separate_connections_serialize_duplicate_events_and_storage_errors_fail_closed()
 {
    let db = Database::new();
    let first = ProgressStore::open(&db.0).unwrap();
    let second = ProgressStore::open(&db.0).unwrap();
    let token = first.issue_token("project", "label").unwrap();
    std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            first.publish("project", &token, &event("same", 1)).unwrap()
        });
        let b = scope.spawn(|| {
            second
                .publish("project", &token, &event("same", 1))
                .unwrap()
        });
        assert_eq!(
            a.join().unwrap().received_at,
            b.join().unwrap().received_at
        );
    });
    assert_eq!(db.count("events"), 1);
    db.connection().execute("DROP TABLE events", []).unwrap();
    let error = first
        .publish("project", &token, &event("new", 2))
        .unwrap_err();
    assert!(matches!(error, ProgressError::Unavailable));
    assert_eq!(error.to_string(), "progress storage unavailable");
    assert!(!format!("{error:?}").contains(&token));
}

#[test]
fn binary_operator_and_startup_enforce_configuration() {
    use std::process::Command;
    let db = Database::new();
    let binary = env!("CARGO_BIN_EXE_silkboard-web");
    for args in [
        vec!["unknown"],
        vec!["publisher", "issue", "project", "label", "file"],
    ] {
        let output = Command::new(binary)
            .env_remove("SILKBOARD_DATABASE")
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    let token_file = db.0.with_file_name("cli.token");
    let output = Command::new(binary)
        .env("SILKBOARD_DATABASE", &db.0)
        .args([
            "publisher",
            "issue",
            "project",
            "label",
            token_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let token = fs::read_to_string(&token_file).unwrap();
    assert!(!String::from_utf8_lossy(&output.stderr).contains(token.trim()));
    let store = ProgressStore::open(&db.0).unwrap();
    store
        .publish("project", token.trim(), &event("cli", 1))
        .unwrap();
    let assets = db.0.with_file_name("assets");
    fs::create_dir(&assets).unwrap();
    fs::write(assets.join("index.html"), "status").unwrap();
    let output = Command::new(binary)
        .env("SILKBOARD_DATABASE", assets.join("private.sqlite"))
        .env("SILKBOARD_FRONTEND", &assets)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("outside the static")
    );
    assert!(!assets.join("private.sqlite").exists());
    let output = Command::new(binary)
        .env(
            "SILKBOARD_DATABASE",
            db.0.with_file_name("missing").join("db"),
        )
        .env("SILKBOARD_FRONTEND", &assets)
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn operator_rejects_database_and_sidecar_output_collisions() {
    for suffix in ["", "-journal", "-wal", "-shm"] {
        let db = Database::new();
        let output = format!("{}{suffix}", db.0.display());
        let args = ["issue".into(), "project".into(), "label".into(), output];
        assert!(publisher_command(&db.0, &args).is_err());
        ProgressStore::open(&db.0).unwrap();
        assert_eq!(db.count("publishers"), 0);
    }
    let db = Database::new();
    let alias =
        db.0.parent()
            .unwrap()
            .join(".")
            .join("progress.sqlite-journal");
    assert!(
        publisher_command(
            &db.0,
            &[
                "issue".into(),
                "project".into(),
                "label".into(),
                alias.to_str().unwrap().into()
            ]
        )
        .is_err()
    );
    assert_eq!(db.count("publishers"), 0);
}
