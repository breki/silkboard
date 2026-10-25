//! Bounded owner-only choices from the complete current view, not one page.
use super::{AuthError, OwnerAuth};
use crate::progress::{ProgressStore, validate_identity, validate_project};
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum View {
    Events,
    Workflows,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Project,
    Agent,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Options {
    view: View,
    kind: Kind,
    q: Option<String>,
    after: Option<String>,
}
impl Options {
    fn validate(&self) -> Result<(), AuthError> {
        if self
            .q
            .as_ref()
            .is_some_and(|q| q.len() > 513 || q.chars().any(char::is_control))
        {
            return Err(AuthError::Invalid);
        }
        if let Some(after) = &self.after {
            match self.kind {
                Kind::Project => {
                    validate_project(after).map_err(|_| AuthError::Invalid)?;
                }
                Kind::Agent => {
                    let (tool, host) =
                        after.split_once(' ').ok_or(AuthError::Invalid)?;
                    validate_identity(tool).map_err(|_| AuthError::Invalid)?;
                    validate_identity(host).map_err(|_| AuthError::Invalid)?;
                }
            }
        }
        Ok(())
    }
}
#[derive(Serialize)]
struct Choices {
    items: Vec<String>,
    next_cursor: Option<String>,
}
pub(super) fn routes() -> Router<OwnerAuth> {
    Router::new().route("/filter-options", get(options))
}
async fn options(
    State(auth): State<OwnerAuth>,
    query: Result<Query<Options>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Choices>, AuthError> {
    let Query(query) = query.map_err(|_| AuthError::Invalid)?;
    query.validate()?;
    Ok(Json(auth.query(move |store| choices(store, &query)).await?))
}
fn choices(
    store: &ProgressStore,
    query: &Options,
) -> Result<Choices, AuthError> {
    let source = match (&query.view, &query.kind) {
        (View::Events, Kind::Project) => "SELECT project AS value FROM events",
        (View::Events, Kind::Agent) => {
            "SELECT s.tool || ' ' || s.hostname AS value FROM sessions s JOIN events e ON e.session=s.id"
        }
        (View::Workflows, Kind::Project) => {
            "SELECT project AS value FROM task_plans"
        }
        (View::Workflows, Kind::Agent) => {
            "SELECT json_extract(summary,'$.reporter.tool') || ' ' || json_extract(summary,'$.reporter.hostname') AS value FROM task_plans"
        }
    };
    let sql = format!(
        "SELECT DISTINCT value FROM ({source}) WHERE value>coalesce(?1,'') AND instr(lower(value),lower(coalesce(?2,'')))>0 ORDER BY value LIMIT 21"
    );
    let conn = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    let mut items = conn
        .prepare(&sql)?
        .query_map(params![query.after, query.q], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let more = items.len() > 20;
    items.truncate(20);
    let next_cursor = items.last().filter(|_| more).cloned();
    Ok(Choices { items, next_cursor })
}
