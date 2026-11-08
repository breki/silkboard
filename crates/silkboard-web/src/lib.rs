//! Silkboard's status application and optional publish-only progress ingestion.

use std::path::Path;

use axum::{
    Json, Router,
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use serde::Serialize;
use tower_http::services::{ServeDir, ServeFile};

pub mod owner;
pub mod plans;
pub mod progress;
mod secrets;

async fn public_asset_cache(request: Request, next: Next) -> Response {
    // Vite's default fingerprint is eight URL-safe characters. Unversioned
    // files and all HTML/API responses keep the private no-store policy.
    let fingerprinted = request.uri().path().rsplit_once('.').is_some_and(
        |(stem, extension)| {
            matches!(extension, "js" | "css")
                && stem
                    .as_bytes()
                    .get(stem.len().saturating_sub(9)..)
                    .is_some_and(|suffix| {
                        suffix.len() == 9
                            && suffix[0] == b'-'
                            && suffix[1..].iter().all(|byte| {
                                byte.is_ascii_alphanumeric()
                                    || matches!(byte, b'_' | b'-')
                            })
                    })
        },
    );
    let mut response = next.run(request).await;
    if response.status().is_success()
        || response.status() == StatusCode::NOT_MODIFIED
    {
        response
            .headers_mut()
            .insert("vary", HeaderValue::from_static("Accept-Encoding"));
        if fingerprinted {
            response.headers_mut().insert(
                "cache-control",
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
        }
    }
    response
}

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
                ("referrer-policy", "no-referrer"),
                ("x-content-type-options", "nosniff"),
                ("x-frame-options", "DENY"),
                ("content-security-policy", "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'"),
            ] { response.headers_mut().insert(key, HeaderValue::from_static(value)); }
            // Only the public fingerprinted asset service supplies a reusable
            // policy. Owner APIs set no-store independently in their router.
            response.headers_mut().entry("cache-control").or_insert(HeaderValue::from_static("no-store"));
            response
        })))
}

/// Construct the application with persistent, project-scoped ingestion.
pub fn create_progress_router(
    frontend_path: &Path,
    store: progress::ProgressStore,
) -> Router {
    create_router(frontend_path)
        .merge(plans::routes(store.clone()))
        .merge(progress::routes(store))
}

/// Construct the status application's HTTP routes and static file service.
///
/// Without a configured progress store, no mutation API is exposed.
pub fn create_router(frontend_path: &Path) -> Router {
    Router::new()
        .nest(
            "/assets",
            Router::new()
                .fallback_service(
                    ServeDir::new(frontend_path.join("assets"))
                        .append_index_html_on_directories(false)
                        .precompressed_gzip(),
                )
                .layer(middleware::from_fn(public_asset_cache)),
        )
        .route("/health", get(|| async { Json(Health { status: "ok" }) }))
        .route("/api/status", get(status))
        .route_service(
            "/status",
            ServeFile::new(frontend_path.join("index.html")),
        )
        .route_service(
            "/todos",
            ServeFile::new(frontend_path.join("index.html")),
        )
        .route_service(
            "/events",
            ServeFile::new(frontend_path.join("index.html")),
        )
        .route_service(
            "/plans",
            ServeFile::new(frontend_path.join("index.html")),
        )
        .route_service(
            "/plans/{project}/{task}",
            ServeFile::new(frontend_path.join("index.html")),
        )
        .fallback_service(ServeDir::new(frontend_path).not_found_service(
            ServeFile::new(frontend_path.join("index.html")),
        ))
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
