//! Silkboard's status application and optional publish-only progress ingestion.

use std::path::Path;

use axum::{Json, Router, routing::get};
use serde::Serialize;
use tower_http::services::ServeDir;

pub mod progress;

/// Construct the application with persistent, project-scoped ingestion.
pub fn create_progress_router(
    frontend_path: &Path,
    store: progress::ProgressStore,
) -> Router {
    create_router(frontend_path).merge(progress::routes(store))
}

/// Construct the status application's HTTP routes and static file service.
///
/// Without a configured progress store, no mutation API is exposed.
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
