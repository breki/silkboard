//! Silkboard's read-only status application.

use std::path::Path;

use axum::{Json, Router, routing::get};
use serde::Serialize;
use tower_http::services::ServeDir;

/// Construct the status application's HTTP routes and static file service.
///
/// Unknown routes return 404; the application exposes no mutation API.
pub fn create_router(frontend_path: &Path) -> Router {
    Router::new()
        .route("/health", get(|| async { Json(Health { status: "ok" }) }))
        .route("/api/status", get(status))
        .fallback_service(ServeDir::new(frontend_path))
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

#[derive(Serialize)]
struct Status {
    health: &'static str,
    version: &'static str,
    release_tag: &'static str,
    commit: &'static str,
}

async fn status() -> Json<Status> {
    Json(Status {
        health: "ok",
        version: env!("CARGO_PKG_VERSION"),
        release_tag: env!("SILKBOARD_BUILD_TAG"),
        commit: env!("SILKBOARD_BUILD_COMMIT"),
    })
}
