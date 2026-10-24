//! Publish-only route: authenticate before decoding bounded plan content.
use super::{Snapshot, store};
use crate::progress::{ProgressError, ProgressStore};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, FromRequest, Path, Request, State},
    http::StatusCode,
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
        .route("/api/projects/{project}/plans", post(publish))
        .layer(DefaultBodyLimit::max(silkboard_plan::MAX_SNAPSHOT_BYTES))
        .with_state(Ingestion {
            store,
            slots: Arc::new(Semaphore::new(8)),
        })
}
fn error(status: StatusCode) -> Response {
    (status,[("cache-control","no-store")],Json(serde_json::json!({"error":match status{StatusCode::UNAUTHORIZED=>"publisher not authorized",StatusCode::CONFLICT=>"conflicting task plan",StatusCode::SERVICE_UNAVAILABLE=>"plan storage unavailable",StatusCode::PAYLOAD_TOO_LARGE=>"task plan too large",StatusCode::UNSUPPORTED_MEDIA_TYPE=>"JSON content required",_=>"invalid task plan"}}))).into_response()
}
fn storage_error(e: &ProgressError) -> Response {
    error(match e {
        ProgressError::Invalid => StatusCode::BAD_REQUEST,
        ProgressError::Unauthorized => StatusCode::UNAUTHORIZED,
        ProgressError::Conflict => StatusCode::CONFLICT,
        ProgressError::Unavailable | ProgressError::NewerSchema => {
            StatusCode::SERVICE_UNAVAILABLE
        }
    })
}
async fn publish(
    State(state): State<Ingestion>,
    Path(project): Path<String>,
    request: Request,
) -> Response {
    let Some(token) = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .filter(|s| s.len() == 64)
        .map(str::to_owned)
    else {
        return error(StatusCode::UNAUTHORIZED);
    };
    let Ok(permit) = state.slots.clone().try_acquire_owned() else {
        return error(StatusCode::SERVICE_UNAVAILABLE);
    };
    let (auth_store, auth_project, auth_token) =
        (state.store.clone(), project.clone(), token.clone());
    let auth = tokio::task::spawn_blocking(move || {
        let conn = auth_store
            .0
            .lock()
            .map_err(|_| ProgressError::Unavailable)?;
        store::authenticate(&conn, &auth_project, &auth_token)
    })
    .await;
    match auth {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return storage_error(&e),
        Err(_) => return error(StatusCode::SERVICE_UNAVAILABLE),
    }
    if !request
        .headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .is_some_and(|s| {
            s.split(';').next().is_some_and(|m| {
                m.trim().eq_ignore_ascii_case("application/json")
            })
        })
    {
        return error(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    let bytes = match Bytes::from_request(request, &()).await {
        Ok(b) => b,
        Err(e) => {
            return error(if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
                StatusCode::PAYLOAD_TOO_LARGE
            } else {
                StatusCode::BAD_REQUEST
            });
        }
    };
    let Ok(snapshot) = serde_json::from_slice::<Snapshot>(&bytes) else {
        return error(StatusCode::BAD_REQUEST);
    };
    match tokio::task::spawn_blocking(move || {
        let _permit = permit;
        super::publish(&state.store, &project, &token, snapshot)
    })
    .await
    {
        Ok(Ok(ack)) => {
            ([("cache-control", "no-store")], Json(ack)).into_response()
        }
        Ok(Err(e)) => storage_error(&e),
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE),
    }
}
