//! Task-plan reads remain inside the existing owner-only guard.
use super::{AuthError, OwnerAuth};
use crate::plans::{self, Cursor, Detail, Page, Summary};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::Deserialize;
use silkboard_plan::Counter;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Before {
    before: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Select {
    sequence: Option<Counter>,
    revision: Option<Counter>,
}
pub(super) fn routes() -> Router<OwnerAuth> {
    Router::new()
        .route("/plans", get(list))
        .route("/projects/{project}/plans/{task}", get(detail))
        .route("/projects/{project}/plans/{task}/history", get(history))
        .route("/projects/{project}/plans/{task}/revisions", get(revisions))
}
async fn list(
    State(auth): State<OwnerAuth>,
    query: Result<
        Query<plans::ListQuery>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Result<Json<Page<Summary>>, AuthError> {
    let Query(q) = query.map_err(|_| AuthError::Invalid)?;
    q.validate()?;
    let cursor = q.before.as_deref().map(Cursor::parse).transpose()?;
    Ok(Json(
        auth.query(move |store| plans::list(store, cursor.as_ref(), &q))
            .await?,
    ))
}
async fn detail(
    State(auth): State<OwnerAuth>,
    Path((project, task)): Path<(String, String)>,
    query: Result<Query<Select>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Detail>, AuthError> {
    let Query(q) = query.map_err(|_| AuthError::Invalid)?;
    Ok(Json(
        auth.query(move |store| {
            plans::current(store, &project, &task, q.sequence, q.revision)
        })
        .await?,
    ))
}
async fn read_history(
    auth: OwnerAuth,
    project: String,
    task: String,
    query: Result<Query<Before>, axum::extract::rejection::QueryRejection>,
    revisions: bool,
) -> Result<Json<Page<Summary>>, AuthError> {
    let Query(q) = query.map_err(|_| AuthError::Invalid)?;
    let before = q
        .before
        .as_deref()
        .map(str::parse::<Counter>)
        .transpose()
        .map_err(|_| AuthError::Invalid)?;
    Ok(Json(
        auth.query(move |store| {
            plans::history(store, &project, &task, before, revisions)
        })
        .await?,
    ))
}
async fn history(
    State(auth): State<OwnerAuth>,
    Path((project, task)): Path<(String, String)>,
    query: Result<Query<Before>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Page<Summary>>, AuthError> {
    read_history(auth, project, task, query, false).await
}
async fn revisions(
    State(auth): State<OwnerAuth>,
    Path((project, task)): Path<(String, String)>,
    query: Result<Query<Before>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Page<Summary>>, AuthError> {
    read_history(auth, project, task, query, true).await
}
