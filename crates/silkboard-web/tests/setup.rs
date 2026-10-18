//! First-owner authorization and publisher initialization boundary.
use axum::Router;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use silkboard_web::owner::{prepare_setup, set_password_from_reader};
use silkboard_web::{
    create_owner_router, create_progress_router, progress::ProgressStore,
};
use std::path::Path;
use tower::ServiceExt;

const ORIGIN: &str = "https://silkboard.example";
const PASSWORD: &str = "a private owner passphrase";

struct Fixture {
    directory: std::path::PathBuf,
    store: ProgressStore,
}
impl Fixture {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let directory = std::env::temp_dir().join(format!(
            "silkboard-setup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(
            &directory,
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        let store =
            ProgressStore::open(&directory.join("data.sqlite")).unwrap();
        Self { directory, store }
    }
    fn sql(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.directory.join("data.sqlite")).unwrap()
    }
    fn app(&self) -> Router {
        create_owner_router(Path::new("missing"), self.store.clone(), ORIGIN)
            .unwrap()
    }
    fn credential(&self) -> String {
        prepare_setup(&self.store, ORIGIN)
            .unwrap()
            .strip_prefix(&format!("{ORIGIN}/setup#setup="))
            .unwrap()
            .to_owned()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

async fn register(app: Router, credential: &str, password: &str) -> StatusCode {
    app.oneshot(
        Request::post("/api/auth/setup")
            .header("origin", ORIGIN)
            .header("content-type", "application/json")
            .header("x-silkboard-setup", "1")
            .body(Body::from(
                json!({"credential":credential,"password":password})
                    .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap()
    .status()
}

#[tokio::test]
async fn setup_state_and_page_are_uncacheable_and_failure_is_safe() {
    let f = Fixture::new();
    for expected in ["waiting", "available", "closed"] {
        if expected == "available" {
            f.credential();
        }
        if expected == "closed" {
            set_password_from_reader(&f.store, PASSWORD.as_bytes()).unwrap();
        }
        let response = f
            .app()
            .oneshot(
                Request::get("/api/auth/setup").body(Body::empty()).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            json!({"state":expected})
        );
    }
    let response = f
        .app()
        .oneshot(Request::get("/setup").body(Body::empty()).unwrap())
        .await
        .unwrap();
    for (name, value) in [
        ("cache-control", "no-store"),
        ("referrer-policy", "no-referrer"),
        ("x-frame-options", "DENY"),
    ] {
        assert_eq!(response.headers()[name], value);
    }
    assert!(
        response.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("form-action 'none'")
    );
    f.sql().execute("DROP TABLE owner_setup", []).unwrap();
    let response = f
        .app()
        .oneshot(Request::get("/api/auth/setup").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        register(f.app(), "a".repeat(64).as_str(), PASSWORD).await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn setup_rotates_expires_and_durably_consumes_the_hash_only_credential() {
    let f = Fixture::new();
    let old = f.credential();
    let token = f.credential();
    assert_eq!(old.len(), 64);
    let (hash, expires): (Vec<u8>, i64) = f
        .sql()
        .query_row("SELECT token_hash, expires_at FROM owner_setup", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(hash.len(), 32);
    assert_ne!(hash, token.as_bytes());
    let now: i64 = f
        .sql()
        .query_row("SELECT unixepoch()", [], |r| r.get(0))
        .unwrap();
    assert!((3590..=3600).contains(&(expires - now)));
    assert_eq!(
        register(f.app(), &old, PASSWORD).await,
        StatusCode::UNAUTHORIZED
    );
    f.sql()
        .execute("UPDATE owner_setup SET expires_at=0", [])
        .unwrap();
    assert_eq!(
        register(f.app(), &token, PASSWORD).await,
        StatusCode::UNAUTHORIZED
    );
    let token = f.credential();
    assert_eq!(
        register(f.app(), &token, "short").await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        f.sql()
            .query_row("SELECT count(*) FROM owner", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        register(f.app(), &token, PASSWORD).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(register(f.app(), &token, PASSWORD).await, StatusCode::GONE);
    assert!(prepare_setup(&f.store, ORIGIN).is_err());
    let stored: (bool, bool) = f
        .sql()
        .query_row(
            "SELECT closed,token_hash IS NULL FROM owner_setup",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(stored, (true, true));
    let reopened =
        ProgressStore::open(&f.directory.join("data.sqlite")).unwrap();
    assert!(prepare_setup(&reopened, ORIGIN).is_err());
    set_password_from_reader(
        &reopened,
        b"a replacement private password".as_slice(),
    )
    .unwrap();
    assert!(prepare_setup(&reopened, ORIGIN).is_err());
    // Even removal of the password record cannot silently reopen closed setup.
    f.sql().execute("DELETE FROM owner", []).unwrap();
    assert!(prepare_setup(&reopened, ORIGIN).is_err());
}

#[tokio::test]
async fn concurrent_claims_create_exactly_one_owner_and_enable_publishing() {
    let f = Fixture::new();
    let token = f.credential();
    let second = ProgressStore::open(&f.directory.join("data.sqlite")).unwrap();
    let second_app =
        create_owner_router(Path::new("missing"), second.clone(), ORIGIN)
            .unwrap();
    let (a, b) = tokio::join!(
        register(f.app(), &token, PASSWORD),
        register(second_app, &token, "a different private password")
    );
    assert!(matches!(
        (a, b),
        (StatusCode::NO_CONTENT, StatusCode::GONE)
            | (StatusCode::GONE, StatusCode::NO_CONTENT)
    ));
    assert_eq!(
        f.sql()
            .query_row("SELECT count(*) FROM owner", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    let publisher = second.issue_token("silkboard", "after").unwrap();
    let event=serde_json::from_value(json!({"event_id":"enabled","hostname":"dev","tool":"codex","session_id":"native","sequence":1,"message":"Owner registered"})).unwrap();
    second.publish("silkboard", &publisher, &event).unwrap();
}

#[tokio::test]
async fn setup_requires_same_origin_json_and_non_simple_header_with_a_bounded_budget()
 {
    let f = Fixture::new();
    let publisher = f
        .store
        .issue_token("silkboard", "cannot-claim-owner")
        .unwrap();
    assert_eq!(
        register(f.app(), &publisher, PASSWORD).await,
        StatusCode::UNAUTHORIZED
    );
    let token = f.credential();
    let app = f.app();
    for (origin, header, site) in [
        ("https://evil.example", "1", "same-origin"),
        (ORIGIN, "wrong", "same-origin"),
        (ORIGIN, "1", "cross-site"),
        ("null", "1", "none"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/auth/setup")
                    .header("origin", origin)
                    .header("x-silkboard-setup", header)
                    .header("sec-fetch-site", site)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"credential":token,"password":PASSWORD})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    for body in [
        "{".to_owned(),
        "x".repeat(4097),
        json!({"credential":token,"password":PASSWORD,"extra":true})
            .to_string(),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/auth/setup")
                    .header("origin", ORIGIN)
                    .header("x-silkboard-setup", "1")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    for _ in 0..5 {
        assert_eq!(
            register(app.clone(), "wrong", PASSWORD).await,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        register(app, &token, PASSWORD).await,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[test]
fn initialized_schema_two_owners_migrate_closed_and_prepare_cli_uses_private_output()
 {
    let f = Fixture::new();
    set_password_from_reader(&f.store, PASSWORD.as_bytes()).unwrap();
    f.sql()
        .execute_batch("DROP TABLE owner_setup; PRAGMA user_version=2;")
        .unwrap();
    let reopened =
        ProgressStore::open(&f.directory.join("data.sqlite")).unwrap();
    assert!(prepare_setup(&reopened, ORIGIN).is_err());
    assert_eq!(
        f.sql()
            .query_row("SELECT closed FROM owner_setup", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let fresh = Fixture::new();
    let output =
        std::process::Command::new(env!("CARGO_BIN_EXE_silkboard-web"))
            .env("SILKBOARD_DATABASE", fresh.directory.join("data.sqlite"))
            .env("SILKBOARD_OWNER_ORIGIN", ORIGIN)
            .args(["owner", "prepare-setup"])
            .output()
            .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let link = String::from_utf8(output.stdout).unwrap();
    assert!(link.starts_with(&format!("{ORIGIN}/setup#setup=")));
    assert_eq!(
        link.trim().len(),
        format!("{ORIGIN}/setup#setup=").len() + 64
    );
    assert!(prepare_setup(&fresh.store, "http://invalid.example").is_err());
}

#[tokio::test]
async fn publishing_requires_a_successfully_initialized_owner_in_every_router()
{
    let store = ProgressStore::open_memory().unwrap();
    let token = store.issue_token("silkboard", "before-setup").unwrap();
    for app in [
        create_progress_router(Path::new("missing"), store.clone()),
        create_owner_router(
            Path::new("missing"),
            store.clone(),
            "https://silkboard.example",
        )
        .unwrap(),
    ] {
        let response = app.oneshot(Request::post("/api/projects/silkboard/progress")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"event_id":"blocked","hostname":"dev","tool":"codex","session_id":"native","sequence":1,"message":"Must not be stored"}"#)).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}

#[tokio::test]
async fn network_access_without_setup_credential_cannot_claim_owner() {
    let app = create_owner_router(
        Path::new("missing"),
        ProgressStore::open_memory().unwrap(),
        "https://silkboard.example",
    )
    .unwrap();
    let response = app.oneshot(Request::post("/api/auth/setup")
        .header("origin", "https://silkboard.example")
        .header("content-type", "application/json")
        .header("x-silkboard-setup", "1")
        .body(Body::from(r#"{"credential":"wrong","password":"a private owner passphrase"}"#)).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
