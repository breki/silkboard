//! HTTP contracts for the status application.

use std::path::Path;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use silkboard_web::{
    create_owner_router, create_router, progress::ProgressStore,
};
use tower::ServiceExt;

const GZIP: &[u8] = &[
    31, 139, 8, 0, 0, 0, 0, 0, 0, 3, 75, 206, 207, 43, 206, 207, 73, 213, 203,
    201, 79, 215, 48, 212, 4, 0, 104, 254, 1, 67, 14, 0, 0, 0,
];

fn asset_fixture() -> (axum::Router, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "silkboard-asset-policy-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(directory.join("assets")).unwrap();
    std::fs::write(directory.join("index.html"), "shell").unwrap();
    let asset = directory.join("assets/app-12345678.js");
    std::fs::write(&asset, "console.log(1)").unwrap();
    std::fs::write(directory.join("assets/theme-ab-cd_12.css"), "body{}")
        .unwrap();
    for name in [
        "plain.js",
        "app-123.js",
        "app-1234567!.js",
        "app-12345678.txt",
        ".js",
    ] {
        std::fs::write(directory.join("assets").join(name), "unversioned")
            .unwrap();
    }
    std::fs::write(directory.join("assets/app-12345678.js.gz"), GZIP).unwrap();
    let app = create_owner_router(
        &directory,
        ProgressStore::open_memory().unwrap(),
        "https://silkboard.example",
    )
    .unwrap();
    (app, directory)
}

#[tokio::test]
async fn public_fingerprinted_assets_are_cacheable_and_negotiate_gzip() {
    let (app, directory) = asset_fixture();
    for (encoding, expected_encoding, body) in [
        ("gzip", Some("gzip"), GZIP),
        ("identity", None, b"console.log(1)".as_slice()),
        ("gzip;q=0", None, b"console.log(1)".as_slice()),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get("/assets/app-12345678.js")
                    .header("accept-encoding", encoding)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["cache-control"],
            "public, max-age=31536000, immutable"
        );
        assert_eq!(
            response
                .headers()
                .get("content-encoding")
                .and_then(|v| v.to_str().ok()),
            expected_encoding
        );
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        assert_eq!(response.headers()["vary"], "Accept-Encoding");
        let modified = response.headers()["last-modified"].clone();
        assert_eq!(
            to_bytes(response.into_body(), 4096).await.unwrap().as_ref(),
            body
        );
        let revalidated = app
            .clone()
            .oneshot(
                Request::get("/assets/app-12345678.js")
                    .header("if-modified-since", modified)
                    .header("accept-encoding", encoding)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(revalidated.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(
            revalidated.headers()["cache-control"],
            "public, max-age=31536000, immutable"
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn private_errors_and_unversioned_files_keep_no_store_policy() {
    let (app, directory) = asset_fixture();
    for (path, status) in [
        ("/", StatusCode::OK),
        ("/events", StatusCode::OK),
        ("/plans", StatusCode::OK),
        ("/assets/plain.js", StatusCode::OK),
        ("/assets/app-123.js", StatusCode::OK),
        ("/assets/app-1234567!.js", StatusCode::OK),
        ("/assets/app-12345678.txt", StatusCode::OK),
        ("/assets/.js", StatusCode::OK),
        ("/api/owner/events", StatusCode::UNAUTHORIZED),
        ("/api/owner/plans", StatusCode::UNAUTHORIZED),
        ("/assets/missing.js", StatusCode::NOT_FOUND),
        ("/assets/../index.html", StatusCode::NOT_FOUND),
        ("/assets/%2e%2e/index.html", StatusCode::NOT_FOUND),
    ] {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status, "{path}");
        assert_eq!(response.headers()["cache-control"], "no-store", "{path}");
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn public_assets_support_head_ranges_and_uncompressed_fallback() {
    let (app, directory) = asset_fixture();
    for (method, path, range, status, body) in [
        ("HEAD", "/assets/app-12345678.js", None, StatusCode::OK, ""),
        (
            "GET",
            "/assets/app-12345678.js",
            Some("bytes=0-3"),
            StatusCode::PARTIAL_CONTENT,
            "cons",
        ),
        (
            "GET",
            "/assets/theme-ab-cd_12.css",
            None,
            StatusCode::OK,
            "body{}",
        ),
    ] {
        let mut request = Request::builder().method(method).uri(path).header(
            "accept-encoding",
            if range.is_some() { "identity" } else { "gzip" },
        );
        if let Some(range) = range {
            request = request.header("range", range);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(
            response.headers()["cache-control"],
            "public, max-age=31536000, immutable"
        );
        assert_eq!(response.headers()["vary"], "Accept-Encoding");
        assert_eq!(
            to_bytes(response.into_body(), 4096).await.unwrap().as_ref(),
            body.as_bytes()
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
}

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
