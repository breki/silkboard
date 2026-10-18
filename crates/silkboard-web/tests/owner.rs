//! Owner authentication is independent of project publisher credentials.

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode},
};
use serde_json::{Value, json};
use silkboard_web::owner::set_password_from_reader;
use silkboard_web::{create_owner_router, progress::ProgressStore};
use std::path::Path;
use tower::ServiceExt;

const ORIGIN: &str = "https://silkboard.example";
const PASSWORD: &str = "a private owner passphrase";

fn router(store: ProgressStore) -> Router {
    create_owner_router(Path::new("missing"), store, ORIGIN).unwrap()
}

async fn request(
    app: Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (StatusCode, HeaderMap, Value) {
    let mut request = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .oneshot(request.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, headers, value)
}

async fn login(app: Router, password: &str) -> (StatusCode, HeaderMap, Value) {
    request(
        app,
        "POST",
        "/api/auth/login",
        &[("origin", ORIGIN), ("content-type", "application/json")],
        &json!({"password":password}).to_string(),
    )
    .await
}

async fn cookie(app: Router) -> String {
    let (status, headers, _) = login(app, PASSWORD).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let value = headers["set-cookie"].to_str().unwrap();
    for attribute in [
        "Secure",
        "HttpOnly",
        "SameSite=Strict",
        "Path=/",
        "Max-Age=43200",
    ] {
        assert!(value.contains(attribute));
    }
    assert!(!value.contains("Domain="));
    value.split(';').next().unwrap().to_owned()
}

async fn read_session(
    app: Router,
    session: &str,
) -> (StatusCode, HeaderMap, Value) {
    request(app, "GET", "/api/owner/session", &[("cookie", session)], "").await
}

#[tokio::test]
async fn owner_namespace_requires_browser_authentication() {
    let store = ProgressStore::open_memory().unwrap();
    let publisher = store.issue_token("silkboard", "agent").unwrap();
    let router = create_owner_router(
        Path::new("missing"),
        store,
        "https://silkboard.example",
    )
    .unwrap();
    for path in ["/api/owner/session", "/api/owner/projects"] {
        let response = router
            .clone()
            .oneshot(
                Request::get(path)
                    .header("authorization", format!("Bearer {publisher}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn login_logout_and_forged_requests() {
    let store = ProgressStore::open_memory().unwrap();
    set_password_from_reader(&store, PASSWORD.as_bytes()).unwrap();
    let app = router(store);
    assert_eq!(
        login(app.clone(), "a wrong owner passphrase").await.0,
        StatusCode::UNAUTHORIZED
    );
    let session = cookie(app.clone()).await;
    let (status, headers, info) = read_session(app.clone(), &session).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(info["authenticated"], true);
    assert_eq!(info.as_object().unwrap().len(), 2);
    let csrf = info["csrf_token"].as_str().unwrap();
    assert_eq!(csrf.len(), 64);
    for extra in [
        vec![],
        vec![("origin", ORIGIN)],
        vec![("origin", ORIGIN), ("x-silkboard-csrf", "wrong")],
        vec![
            ("origin", "https://evil.example"),
            ("x-silkboard-csrf", csrf),
        ],
    ] {
        let mut headers = vec![("cookie", session.as_str())];
        headers.extend(extra);
        assert_eq!(
            request(app.clone(), "POST", "/api/owner/logout", &headers, "")
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    for extra in [
        vec![("origin", "https://evil.example")],
        vec![("sec-fetch-site", "cross-site")],
        vec![("origin", ORIGIN), ("origin", ORIGIN)],
    ] {
        let mut headers = vec![("cookie", session.as_str())];
        headers.extend(extra);
        assert_eq!(
            request(app.clone(), "GET", "/api/owner/session", &headers, "")
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        request(
            app.clone(),
            "GET",
            "/api/owner/session",
            &[("cookie", &format!("{session}; {session}"))],
            ""
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            app.clone(),
            "GET",
            "/api/owner/unknown",
            &[("cookie", &session)],
            ""
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (status, headers, _) = request(
        app.clone(),
        "POST",
        "/api/owner/logout",
        &[
            ("cookie", &session),
            ("origin", ORIGIN),
            ("x-silkboard-csrf", csrf),
        ],
        "",
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(
        headers["set-cookie"]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    assert_eq!(
        read_session(app, &session).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn login_origin_body_and_attempt_budget_are_bounded() {
    let app = router(ProgressStore::open_memory().unwrap());
    for origin in [
        None,
        Some("null"),
        Some("https://evil.example"),
        Some("https://silkboard.example.evil"),
    ] {
        let mut headers = vec![("content-type", "application/json")];
        if let Some(origin) = origin {
            headers.push(("origin", origin));
        }
        assert_eq!(
            request(
                app.clone(),
                "POST",
                "/api/auth/login",
                &headers,
                &json!({"password":PASSWORD}).to_string()
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    for body in [
        "{",
        r#"{"password":12}"#,
        r#"{"password":"secret","extra":true}"#,
        &"x".repeat(5000),
    ] {
        let (status, headers, error) = request(
            app.clone(),
            "POST",
            "/api/auth/login",
            &[("origin", ORIGIN), ("content-type", "application/json")],
            body,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(headers["cache-control"], "no-store");
        assert!(!error.to_string().contains("secret"));
    }
    for _ in 0..5 {
        assert_eq!(
            login(app.clone(), PASSWORD).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    let (status, headers, _) = login(app, PASSWORD).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(headers["retry-after"], "60");
}

#[test]
fn rejects_invalid_origins_and_password_input() {
    struct Broken;
    impl std::io::Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("private input failure"))
        }
    }
    let store = ProgressStore::open_memory().unwrap();
    for origin in [
        "http://silkboard.example",
        "https://silkboard.example/",
        "https://silkboard.example/path",
        "https://silkboard.example?query",
        "https://user@silkboard.example",
        "https://silkboard.example/#fragment",
        "",
        "https://",
        "https://SILKBOARD.example",
        "https://silkboard.example:443",
        "https://silkboard.example:bad",
    ] {
        assert!(
            create_owner_router(Path::new("missing"), store.clone(), origin)
                .is_err(),
            "{origin}"
        );
    }
    for password in [
        String::new(),
        "short".into(),
        "x".repeat(1025),
        format!("{}\r\nignored", "x".repeat(1024)),
        "password with\nnewline".into(),
    ] {
        assert!(set_password_from_reader(&store, password.as_bytes()).is_err());
    }
    assert!(set_password_from_reader(&store, Broken).is_err());
    set_password_from_reader(&store, format!("{PASSWORD}\r\n").as_bytes())
        .unwrap();
}

struct Database(std::path::PathBuf);
impl Database {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "silkboard-owner-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                &dir,
                std::fs::Permissions::from_mode(0o700),
            )
            .unwrap();
        }
        Self(dir.join("data.sqlite"))
    }
    fn sql(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(&self.0).unwrap()
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        std::fs::remove_dir_all(self.0.parent().unwrap()).unwrap();
    }
}

#[test]
fn binary_password_setup_never_prints_or_accepts_password_arguments() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let db = Database::new();
    let binary = env!("CARGO_BIN_EXE_silkboard-web");
    let mut process = Command::new(binary)
        .env("SILKBOARD_DATABASE", &db.0)
        .args(["owner", "set-password"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(PASSWORD.as_bytes())
        .unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains(PASSWORD));
    for args in [
        vec!["owner"],
        vec!["owner", "set-password", "unexpected-argument"],
    ] {
        let output = Command::new(binary)
            .env("SILKBOARD_DATABASE", &db.0)
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
    }
    let output = Command::new(binary)
        .env_remove("SILKBOARD_DATABASE")
        .args(["owner", "set-password"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let output = Command::new(binary)
        .env_remove("SILKBOARD_DATABASE")
        .env("SILKBOARD_OWNER_ORIGIN", ORIGIN)
        .env("SILKBOARD_FRONTEND", db.0.parent().unwrap())
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[tokio::test]
async fn corrupt_hash_and_unexpected_work_factors_fail_closed() {
    let db = Database::new();
    let store = ProgressStore::open(&db.0).unwrap();
    set_password_from_reader(&store, PASSWORD.as_bytes()).unwrap();
    let original: String = db
        .sql()
        .query_row("SELECT password_hash FROM owner", [], |r| r.get(0))
        .unwrap();
    let app = router(store);
    for hash in [
        "invalid PHC".into(),
        original.replace("m=65536", "m=65537"),
        original.replace("argon2id", "argon2i"),
    ] {
        db.sql()
            .execute("UPDATE owner SET password_hash = ?1", [hash])
            .unwrap();
        let (status, _, error) = login(app.clone(), PASSWORD).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error["error"], "owner authentication unavailable");
    }
    assert_eq!(login(app, "short").await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn password_reset_restart_expiry_and_storage_failure() {
    let db = Database::new();
    let store = ProgressStore::open(&db.0).unwrap();
    set_password_from_reader(&store, PASSWORD.as_bytes()).unwrap();
    let app = router(store.clone());
    let session = cookie(app).await;
    let hash: String = db
        .sql()
        .query_row("SELECT password_hash FROM owner", [], |r| r.get(0))
        .unwrap();
    assert!(hash.starts_with("$argon2id$v=19$m=65536,t=3,p=1$"));
    assert!(!hash.contains(PASSWORD));
    let token_hash: Vec<u8> = db
        .sql()
        .query_row("SELECT token_hash FROM owner_sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(token_hash.len(), 32);
    assert_ne!(token_hash, session.split('=').nth(1).unwrap().as_bytes());
    drop(store);
    let store = ProgressStore::open(&db.0).unwrap();
    let app = router(store.clone());
    assert_eq!(read_session(app.clone(), &session).await.0, StatusCode::OK);
    assert!(set_password_from_reader(&store, b"short".as_slice()).is_err());
    assert_eq!(read_session(app.clone(), &session).await.0, StatusCode::OK);
    set_password_from_reader(
        &store,
        "a new private owner passphrase".as_bytes(),
    )
    .unwrap();
    assert_eq!(
        read_session(app.clone(), &session).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        login(app.clone(), PASSWORD).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        login(app.clone(), "a new private owner passphrase").await.0,
        StatusCode::NO_CONTENT
    );
    db.sql()
        .execute("UPDATE owner_sessions SET expires_at = 0", [])
        .unwrap();
    let active = login(app.clone(), "a new private owner passphrase").await;
    assert_eq!(active.0, StatusCode::NO_CONTENT);
    assert_eq!(
        db.sql()
            .query_row("SELECT count(*) FROM owner_sessions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let current = active.1["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    db.sql()
        .execute("UPDATE owner_sessions SET expires_at = 0", [])
        .unwrap();
    assert_eq!(
        request(
            app.clone(),
            "GET",
            "/api/owner/session",
            &[("cookie", current)],
            ""
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    db.sql().execute("DROP TABLE owner_sessions", []).unwrap();
    assert_eq!(
        request(app, "GET", "/api/owner/session", &[("cookie", current)], "")
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn migration_preserves_publishers_and_browser_cookies_cannot_publish() {
    let db = Database::new();
    // Actual schema-1 fixture: no owner tables before opening the new binary.
    db.sql()
        .execute_batch(include_str!("../src/progress/schema.sql"))
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&db.0, std::fs::Permissions::from_mode(0o600))
            .unwrap();
    }
    db.sql()
        .execute("INSERT INTO projects VALUES ('existing')", [])
        .unwrap();
    let store = ProgressStore::open(&db.0).unwrap();
    assert_eq!(
        db.sql()
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        db.sql()
            .query_row("SELECT id FROM projects", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "existing"
    );
    let publisher = store.issue_token("silkboard", "agent").unwrap();
    set_password_from_reader(&store, PASSWORD.as_bytes()).unwrap();
    let app = router(store);
    let session = cookie(app.clone()).await;
    let body=json!({"event_id":"event","hostname":"dev","tool":"codex","session_id":"native","sequence":1,"message":"Running tests"}).to_string();
    let path = "/api/projects/silkboard/progress";
    assert_eq!(
        request(
            app.clone(),
            "POST",
            path,
            &[("content-type", "application/json"), ("cookie", &session)],
            &body
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            app.clone(),
            "POST",
            path,
            &[
                ("content-type", "application/json"),
                ("authorization", &format!("Bearer {publisher}"))
            ],
            &body
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            app.clone(),
            "GET",
            "/api/owner/session",
            &[("cookie", &format!("__Host-silkboard-owner={publisher}"))],
            ""
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    for path in ["/health", "/api/status"] {
        assert_eq!(
            request(app.clone(), "GET", path, &[], "").await.0,
            StatusCode::OK
        );
    }
}
