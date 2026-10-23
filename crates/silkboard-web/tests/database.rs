//! Private operator snapshots and isolated recovery, never production fixtures.
use rusqlite::Connection;
use silkboard_web::{owner::set_password_from_reader, progress::ProgressStore};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "silkboard-recovery-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        Self(path)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn command(
    database: &Path,
    operation: &str,
    snapshot: &Path,
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_silkboard-web"))
        .env("SILKBOARD_DATABASE", database)
        .env_remove("SILKBOARD_OWNER_ORIGIN")
        .args(["database", operation])
        .arg(snapshot)
        .output()
        .unwrap()
}
fn seed(path: &Path) -> (ProgressStore, String) {
    let store = ProgressStore::open(path).unwrap();
    set_password_from_reader(
        &store,
        b"private recovery fixture password".as_slice(),
    )
    .unwrap();
    let token = store.issue_token("silkboard", "original").unwrap();
    let event=serde_json::from_value(serde_json::json!({"event_id":"saved","hostname":"dev","tool":"codex","session_id":"native","sequence":1,"message":"Preserve this progress"})).unwrap();
    store.publish("silkboard", &token, &event).unwrap();
    let conn = Connection::open(path).unwrap();
    conn.execute(
        "INSERT INTO owner_sessions VALUES (x'0102',unixepoch()+3600)",
        [],
    )
    .unwrap();
    (store, token)
}
#[test]
fn snapshot_restore_preserves_history_but_drops_restored_authority() {
    let f = Fixture::new();
    let live = f.path("live.sqlite");
    let backup = f.path("backup.sqlite");
    let restored = f.path("restored.sqlite");
    let (_store, token) = seed(&live);
    let result = command(&live, "backup", &backup);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&result.stderr).contains(&token));
    let before = fs::read(&backup).unwrap();
    assert!(!command(&live, "backup", &backup).status.success());
    assert_eq!(fs::read(&backup).unwrap(), before);
    assert!(command(&restored, "restore", &backup).status.success());
    let connection = Connection::open(&restored).unwrap();
    for (table, count) in [
        ("events", 1),
        ("sessions", 1),
        ("owner", 1),
        ("owner_sessions", 0),
    ] {
        assert_eq!(
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
                    r.get::<_, i64>(0)
                })
                .unwrap(),
            count
        );
    }
    assert!(
        connection
            .query_row("SELECT revoked FROM publishers", [], |r| r
                .get::<_, bool>(0))
            .unwrap()
    );
    assert!(connection.query_row("SELECT closed AND token_hash IS NULL AND expires_at=0 FROM owner_setup",[],|r|r.get::<_,bool>(0)).unwrap());
    assert_eq!(fs::read(&backup).unwrap(), before);
    assert!(!command(&restored, "restore", &backup).status.success());
    assert!(
        !Connection::open(&live)
            .unwrap()
            .query_row("SELECT revoked FROM publishers", [], |r| r
                .get::<_, bool>(0))
            .unwrap()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&backup).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&restored).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
#[test]
fn snapshot_includes_committed_wal_without_changing_source() {
    let f = Fixture::new();
    let live = f.path("wal.sqlite");
    let backup = f.path("snapshot.sqlite");
    let (_store, _token) = seed(&live);
    let connection = Connection::open(&live).unwrap();
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    connection
        .execute("UPDATE events SET message='Committed in WAL'", [])
        .unwrap();
    assert!(f.path("wal.sqlite-wal").exists());
    assert!(command(&live, "backup", &backup).status.success());
    let copy = Connection::open(&backup).unwrap();
    assert_eq!(
        copy.query_row("SELECT message FROM events", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "Committed in WAL"
    );
}
#[test]
fn bad_sources_paths_and_operations_fail_without_overwriting() {
    let f = Fixture::new();
    let live = f.path("live.sqlite");
    let output = f.path("output.sqlite");
    let (_store, _token) = seed(&live);
    for (source, operation) in
        [(f.path("absent"), "backup"), (live.clone(), "unknown")]
    {
        assert!(!command(&source, operation, &output).status.success());
        assert!(!output.exists());
    }
    assert!(!f.path("absent").exists());
    fs::write(f.path("corrupt"), b"not a SQLite database").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            f.path("corrupt"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }
    assert!(
        !command(&output, "restore", &f.path("corrupt"))
            .status
            .success()
    );
    assert!(!output.exists());
    let connection = Connection::open(&live).unwrap();
    connection.pragma_update(None, "user_version", 999).unwrap();
    assert!(!command(&live, "backup", &output).status.success());
    assert!(!output.exists());
    connection.pragma_update(None, "user_version", 3).unwrap();
    assert!(
        !command(&live, "backup", &f.path("live.sqlite-journal"))
            .status
            .success()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        symlink(&live, f.path("link")).unwrap();
        assert!(!command(&f.path("link"), "backup", &output).status.success());
        assert!(!command(&live, "backup", &f.path("link")).status.success());
        fs::set_permissions(&live, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(!command(&live, "backup", &output).status.success());
        fs::set_permissions(&live, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&f.0, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(!command(&live, "backup", &output).status.success());
    }
}

#[test]
fn concurrent_exports_publish_one_complete_snapshot_and_clean_temporary_files()
{
    let f = Fixture::new();
    let live = f.path("live.sqlite");
    let backup = f.path("race.sqlite");
    let (_store, _token) = seed(&live);
    let results = std::thread::scope(|scope| {
        let a = scope.spawn(|| command(&live, "backup", &backup));
        let b = scope.spawn(|| command(&live, "backup", &backup));
        [
            a.join().unwrap().status.success(),
            b.join().unwrap().status.success(),
        ]
    });
    assert_eq!(results.iter().filter(|&&ok| ok).count(), 1);
    assert_eq!(
        Connection::open(&backup)
            .unwrap()
            .query_row("SELECT count(*) FROM events", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(fs::read_dir(&f.0).unwrap().all(|p| {
        !p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".silkboard-snapshot-")
    }));
}
#[test]
fn restore_clears_pending_setup_and_rejects_inconsistent_sources() {
    let f = Fixture::new();
    let live = f.path("live.sqlite");
    let backup = f.path("backup.sqlite");
    let restored = f.path("restored.sqlite");
    let store = ProgressStore::open(&live).unwrap();
    let _private_link = silkboard_web::owner::prepare_setup(
        &store,
        "https://silkboard.example",
    )
    .unwrap();
    assert!(command(&live, "backup", &backup).status.success());
    assert!(command(&restored, "restore", &backup).status.success());
    let connection = Connection::open(&restored).unwrap();
    assert!(connection.query_row("SELECT NOT closed AND token_hash IS NULL AND expires_at=0 FROM owner_setup",[],|r|r.get::<_,bool>(0)).unwrap());
    let restored_store = ProgressStore::open(&restored).unwrap();
    assert!(
        silkboard_web::owner::prepare_setup(
            &restored_store,
            "https://silkboard.example"
        )
        .is_ok()
    );
    connection.execute("DROP TABLE events", []).unwrap();
    assert!(
        !command(&restored, "backup", &f.path("bad.sqlite"))
            .status
            .success()
    );
    assert!(!f.path("bad.sqlite").exists());
    Connection::open(&backup)
        .unwrap()
        .pragma_update(None, "user_version", 2)
        .unwrap();
    assert!(
        !command(&f.path("old.sqlite"), "restore", &backup)
            .status
            .success()
    );
}

#[tokio::test]
async fn restored_password_still_logs_in_but_old_browser_and_publisher_do_not()
{
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let f = Fixture::new();
    let live = f.path("live.sqlite");
    let backup = f.path("backup.sqlite");
    let restored = f.path("restored.sqlite");
    let (store, token) = seed(&live);
    let login = || {
        Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("origin", "https://silkboard.example")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"password":"private recovery fixture password"}"#,
            ))
            .unwrap()
    };
    let app = silkboard_web::create_owner_router(
        &f.0,
        store,
        "https://silkboard.example",
    )
    .unwrap();
    let response = app.oneshot(login()).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    assert!(command(&live, "backup", &backup).status.success());
    assert!(command(&restored, "restore", &backup).status.success());
    let app = silkboard_web::create_owner_router(
        &f.0,
        ProgressStore::open(&restored).unwrap(),
        "https://silkboard.example",
    )
    .unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::get("/api/owner/projects")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = app.clone().oneshot(login()).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let response=app.oneshot(Request::builder().method("POST").uri("/api/projects/silkboard/progress").header("authorization",format!("Bearer {token}")).header("content-type","application/json").body(Body::from(r#"{"event_id":"restored","hostname":"dev","tool":"codex","session_id":"native","sequence":2,"message":"Must be rejected"}"#)).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn restore_refuses_leftover_destination_companions_without_touching_them() {
    let f = Fixture::new();
    let live = f.path("live.sqlite");
    let backup = f.path("backup.sqlite");
    let (_store, _token) = seed(&live);
    assert!(command(&live, "backup", &backup).status.success());
    for suffix in ["-journal", "-wal", "-shm", "-reserved"] {
        let output = f.path("restored.sqlite");
        let companion = f.path(&format!("restored.sqlite{suffix}"));
        fs::write(
            &companion,
            b"Retained companion from an interrupted operation",
        )
        .unwrap();
        let before = fs::read(&companion).unwrap();
        assert!(
            !command(&output, "restore", &backup).status.success(),
            "restore must refuse existing {suffix}"
        );
        assert!(!output.exists());
        assert_eq!(fs::read(&companion).unwrap(), before);
        fs::remove_file(companion).unwrap();
    }
    assert!(
        command(&f.path("restored.sqlite"), "restore", &backup)
            .status
            .success()
    );
}

#[test]
fn global_feed_index_is_added_without_changing_existing_data_or_schema_version()
{
    let f = Fixture::new();
    let database = f.path("existing.sqlite");
    let (store, _token) = seed(&database);
    drop(store);
    let conn = Connection::open(&database).unwrap();
    conn.execute_batch("DROP INDEX events_feed; DROP INDEX events_named")
        .unwrap();
    let before: String = conn
        .query_row("SELECT payload FROM events", [], |r| r.get(0))
        .unwrap();
    drop(conn);
    let _store = ProgressStore::open(&database).unwrap();
    let conn = Connection::open(&database).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        conn.query_row("SELECT payload FROM events", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        before
    );
    assert_eq!(conn.query_row("SELECT count(*) FROM sqlite_master WHERE type='index' AND name IN ('events_feed','events_named')",[],|r|r.get::<_,i64>(0)).unwrap(),2);
}
