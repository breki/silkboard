//! Persistent, publish-only progress HTTP contracts.

use std::path::Path;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use silkboard_web::{create_progress_router, progress::ProgressStore};
use tower::ServiceExt;

fn event() -> Value {
    json!({"event_id":"event-1", "hostname":"dev", "tool":"codex",
        "session_id":"native-1", "sequence":1, "message":"Running tests"})
}

fn initialized_store() -> ProgressStore {
    let store = ProgressStore::open_memory().unwrap();
    silkboard_web::owner::set_password_from_reader(
        &store,
        b"private ingestion test password".as_slice(),
    )
    .unwrap();
    store
}

async fn publish(
    router: Router,
    token: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = router
        .oneshot(
            Request::post("/api/projects/silkboard/progress")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn accepts_only_scoped_publishers_and_acknowledges_exact_retries() {
    let store = initialized_store();
    let token = store.issue_token("silkboard", "dev").unwrap();
    let foreign = store.issue_token("other", "dev").unwrap();
    let router = create_progress_router(Path::new("missing"), store.clone());
    assert_eq!(
        publish(router.clone(), "invalid", event()).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        publish(router.clone(), &foreign, event()).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, ack) = publish(router.clone(), &token, event()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ack["event_id"], "event-1");
    assert_eq!(ack.as_object().unwrap().len(), 2);
    assert_eq!(publish(router.clone(), &token, event()).await.1, ack);
    let mut conflict = event();
    conflict["message"] = json!("Different message");
    assert_eq!(
        publish(router.clone(), &token, conflict).await.0,
        StatusCode::CONFLICT
    );
    store.revoke_token("silkboard", "dev").unwrap();
    assert_eq!(
        publish(router, &token, event()).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn validates_payloads_and_keeps_feed_unreadable() {
    let store = initialized_store();
    let token = store.issue_token("silkboard", "test").unwrap();
    let router = create_progress_router(Path::new("missing"), store);
    for (field, value) in [
        ("message", json!("")),
        ("message", json!(" \t ")),
        ("message", json!("line\nbreak")),
        ("message", json!("x".repeat(2001))),
        ("hostname", json!("bad host")),
        ("tool", json!("")),
        ("session_id", json!("x".repeat(257))),
        ("subagent_id", json!("")),
        ("parent_session_id", json!("parent-without-child")),
        ("session_name", json!("x".repeat(121))),
        ("sequence", json!(0)),
        ("sequence", json!(-1)),
    ] {
        let mut body = event();
        body[field] = value;
        assert_eq!(
            publish(router.clone(), &token, body).await.0,
            StatusCode::BAD_REQUEST,
            "{field}"
        );
    }
    for (body, expected) in [
        ("{", StatusCode::BAD_REQUEST),
        (r#"{"unknown":"field"}"#, StatusCode::UNPROCESSABLE_ENTITY),
        (&"x".repeat(9000), StatusCode::PAYLOAD_TOO_LARGE),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::post("/api/projects/silkboard/progress")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    for path in [
        "/api/projects/silkboard/progress",
        "/api/projects/silkboard/sessions",
        "/api/projects/silkboard/events",
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::get(path)
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(matches!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_FOUND
        ));
    }
    let response = router
        .clone()
        .oneshot(
            Request::post("/api/projects/silkboard/progress")
                .header("content-type", "application/json")
                .body(Body::from(event().to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    for path in ["/health", "/api/status"] {
        let response = router
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn concurrent_retries_are_atomic() {
    let store = initialized_store();
    let token = store.issue_token("silkboard", "test").unwrap();
    let router = create_progress_router(Path::new("missing"), store);
    let (first, second) = tokio::join!(
        publish(router.clone(), &token, event()),
        publish(router, &token, event())
    );
    assert_eq!(first.0, StatusCode::OK);
    assert_eq!(first, second);
}
