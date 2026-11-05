//! HTTP contracts for the status application.

use std::path::Path;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use silkboard_web::create_router;
use tower::ServiceExt;

#[tokio::test]
async fn health_and_status_report_backend_build_identity() {
    for (path, expected) in [
        ("/health", json!({"status": "ok"})),
        (
            "/api/status",
            json!({
                "health": "ok",
                "version": env!("CARGO_PKG_VERSION"),
                "release_tag": env!("SILKBOARD_BUILD_TAG"),
                "commit": env!("SILKBOARD_BUILD_COMMIT"),
            }),
        ),
    ] {
        let response = create_router(Path::new("missing-frontend"))
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "application/json");
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), expected);
    }
}

#[tokio::test]
async fn unknown_api_and_mutation_requests_are_rejected() {
    for (method, path, expected) in [
        ("GET", "/api/tasks", StatusCode::NOT_FOUND),
        ("POST", "/api/status", StatusCode::METHOD_NOT_ALLOWED),
        ("POST", "/health", StatusCode::METHOD_NOT_ALLOWED),
        ("POST", "/status", StatusCode::METHOD_NOT_ALLOWED),
        ("GET", "/status/missing", StatusCode::NOT_FOUND),
    ] {
        let response = create_router(Path::new("missing-frontend"))
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
}

#[tokio::test]
async fn serves_deep_links_and_recoverable_not_found_pages_without_path_escape()
{
    let directory = std::env::temp_dir().join(format!(
        "silkboard-assets-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("index.html"), "<h1>Silkboard</h1>").unwrap();
    std::fs::write(directory.join("app.js"), "console.log('status')").unwrap();
    for (path, expected_status, expected_body) in [
        ("/", StatusCode::OK, "<h1>Silkboard</h1>"),
        ("/status", StatusCode::OK, "<h1>Silkboard</h1>"),
        ("/todos", StatusCode::OK, "<h1>Silkboard</h1>"),
        ("/plans", StatusCode::OK, "<h1>Silkboard</h1>"),
        (
            "/events?project=alpha&hostname=dev&tool=codex&session_id=native&subagent_id=",
            StatusCode::OK,
            "<h1>Silkboard</h1>",
        ),
        (
            "/plans/silkboard/11111111-1111-4111-8111-111111111111?sequence=1",
            StatusCode::OK,
            "<h1>Silkboard</h1>",
        ),
        (
            "/plans/missing",
            StatusCode::NOT_FOUND,
            "<h1>Silkboard</h1>",
        ),
        ("/app.js", StatusCode::OK, "console.log('status')"),
        ("/missing.js", StatusCode::NOT_FOUND, "<h1>Silkboard</h1>"),
        (
            "/../Cargo.toml",
            StatusCode::NOT_FOUND,
            "<h1>Silkboard</h1>",
        ),
        (
            "/%2e%2e/Cargo.toml",
            StatusCode::NOT_FOUND,
            "<h1>Silkboard</h1>",
        ),
    ] {
        let response = create_router(&directory)
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected_status, "{path}");
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        assert_eq!(body.as_ref(), expected_body.as_bytes(), "{path}");
    }
    std::fs::remove_dir_all(directory).unwrap();
}
