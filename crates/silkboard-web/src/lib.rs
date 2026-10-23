//! Silkboard's status application and optional publish-only progress ingestion.

use std::path::Path;

use axum::{
    Json, Router, http::HeaderValue, middleware, response::Response,
    routing::get,
};
use serde::Serialize;
use tower_http::services::{ServeDir, ServeFile};

pub mod owner;
pub mod progress;
mod secrets;

/// Construct the progress application with the owner's authentication API.
///
/// # Errors
/// Returns an error for an invalid or non-HTTPS public origin.
pub fn create_owner_router(
    frontend_path: &Path,
    store: progress::ProgressStore,
    origin: &str,
) -> Result<Router, owner::AuthError> {
    let auth = owner::OwnerAuth::new(store.clone(), origin)?;
    let setup = Router::new().route_service(
        "/setup",
        ServeFile::new(frontend_path.join("index.html")),
    );
    Ok(create_progress_router(frontend_path, store)
        .merge(owner::routes(auth))
        .merge(setup)
        .layer(middleware::map_response(|mut response: Response| async {
            for (key, value) in [
                ("cache-control", "no-store"),
                ("referrer-policy", "no-referrer"),
                ("x-content-type-options", "nosniff"),
                ("x-frame-options", "DENY"),
                ("content-security-policy", "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'"),
            ] { response.headers_mut().insert(key, HeaderValue::from_static(value)); }
            response
        })))
}

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
        .route_service(
            "/status",
            ServeFile::new(frontend_path.join("index.html")),
        )
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
