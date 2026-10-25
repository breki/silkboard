//! HTTP ingestion boundary; no feed-read capability.

use super::{ProgressError, ProgressEvent, ProgressStore};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone)]
struct Ingestion {
    store: ProgressStore,
    slots: Arc<Semaphore>,
}

pub(crate) fn routes(store: ProgressStore) -> Router {
    Router::new()
        .route("/api/projects/{project}/progress", post(publish))
        .layer(DefaultBodyLimit::max(65_536))
        .with_state(Ingestion {
            store,
            slots: Arc::new(Semaphore::new(8)),
        })
}

async fn publish(
    State(state): State<Ingestion>,
    Path(project): Path<String>,
    headers: HeaderMap,
    Json(event): Json<ProgressEvent>,
) -> Result<Response, ProgressError> {
    let token = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(ProgressError::Unauthorized)?
        .to_owned();
    if token.len() != 64 {
        return Err(ProgressError::Unauthorized);
    }
    let permit = state
        .slots
        .try_acquire_owned()
        .map_err(|_| ProgressError::Unavailable)?;
    let ack = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        state.store.publish(&project, &token, &event)
    })
    .await
    .map_err(|_| ProgressError::Unavailable)??;
    Ok(([("cache-control", "no-store")], Json(ack)).into_response())
}

impl IntoResponse for ProgressError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Invalid => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Unavailable | Self::NewerSchema => {
                StatusCode::SERVICE_UNAVAILABLE
            }
        };
        (
            status,
            [("cache-control", "no-store")],
            Json(serde_json::json!({"error": self.to_string()})),
        )
            .into_response()
    }
}
