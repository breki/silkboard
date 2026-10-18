//! Owner-only metadata and transactionally authorized credential mutations.
use super::{AuthError, OwnerAuth, http::OwnerSession, storage};
use crate::progress::{
    ProgressError, ProgressStore, validate_identity, validate_project,
};
use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use rusqlite::{TransactionBehavior, params};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Label {
    label: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    after: Option<String>,
}
#[derive(Serialize)]
struct Publisher {
    label: String,
    revoked: bool,
}
#[derive(Serialize)]
struct Page {
    items: Vec<Publisher>,
    next_cursor: Option<String>,
}

pub(super) fn routes() -> Router<OwnerAuth> {
    Router::new()
        .route("/projects/{project}/publishers", get(list).post(issue))
        .route("/projects/{project}/publishers/revoke", post(revoke))
}

impl From<ProgressError> for AuthError {
    fn from(error: ProgressError) -> Self {
        match error {
            ProgressError::Invalid => AuthError::Invalid,
            ProgressError::Conflict => AuthError::Conflict,
            ProgressError::Unauthorized => AuthError::Unauthorized,
            ProgressError::Unavailable | ProgressError::NewerSchema => {
                AuthError::Unavailable
            }
        }
    }
}

async fn list(
    State(auth): State<OwnerAuth>,
    Path(project): Path<String>,
    Query(cursor): Query<Cursor>,
) -> Result<Json<Page>, AuthError> {
    validate_project(&project).map_err(AuthError::from)?;
    if let Some(after) = &cursor.after {
        validate_identity(after).map_err(AuthError::from)?;
    }
    let result=auth.query(move|store|{
        let connection=store.0.lock().map_err(|_|AuthError::Unavailable)?;
        let mut statement=connection.prepare("SELECT label,revoked FROM publishers WHERE project=?1 AND label>coalesce(?2,'') ORDER BY label LIMIT 21")?;
        let mut items=statement.query_map(params![project,cursor.after],|r|Ok(Publisher{label:r.get(0)?,revoked:r.get(1)?}))?.collect::<Result<Vec<_>,_>>()?;
        let more=items.len()>20;items.truncate(20);
        let next_cursor=items.last().filter(|_|more).map(|p|p.label.clone());
        Ok(Page{items,next_cursor})
    }).await?;
    Ok(Json(result))
}

async fn issue(
    State(auth): State<OwnerAuth>,
    Extension(session): Extension<OwnerSession>,
    Path(project): Path<String>,
    input: Result<Json<Label>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<serde_json::Value>), AuthError> {
    let Json(input) = input.map_err(|_| AuthError::Invalid)?;
    let token = auth
        .query(move |store| {
            let mut connection =
                store.0.lock().map_err(|_| AuthError::Unavailable)?;
            let tx = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            storage::authenticate_on(&tx, &session.0)?;
            let token = ProgressStore::issue_in_transaction(
                &tx,
                &project,
                &input.label,
            )
            .map_err(AuthError::from)?;
            tx.commit()?;
            Ok(token)
        })
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"token":token})),
    ))
}

async fn revoke(
    State(auth): State<OwnerAuth>,
    Extension(session): Extension<OwnerSession>,
    Path(project): Path<String>,
    input: Result<Json<Label>, axum::extract::rejection::JsonRejection>,
) -> Result<StatusCode, AuthError> {
    let Json(input) = input.map_err(|_| AuthError::Invalid)?;
    auth.query(move |store| {
        let mut connection =
            store.0.lock().map_err(|_| AuthError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        storage::authenticate_on(&tx, &session.0)?;
        ProgressStore::revoke_in_transaction(&tx, &project, &input.label)
            .map_err(AuthError::from)?;
        tx.commit()?;
        Ok(())
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn revoked_sessions_cannot_mutate_after_an_earlier_guard_would_have_passed()
     {
        let store = ProgressStore::open_memory().unwrap();
        storage::set_password(&store, "private transaction test password")
            .unwrap();
        let token = storage::login(&store, "private transaction test password")
            .unwrap();
        storage::authenticate(&store, &token).unwrap();
        let publisher = store.issue_token("silkboard", "retained").unwrap();
        storage::logout(&store, &token).unwrap();
        let auth =
            OwnerAuth::new(store.clone(), "https://silkboard.example").unwrap();
        let result = issue(
            State(auth.clone()),
            Extension(OwnerSession(token.clone())),
            Path("silkboard".into()),
            Ok(Json(Label {
                label: "forbidden".into(),
            })),
        )
        .await;
        assert!(matches!(result, Err(AuthError::Unauthorized)));
        let result = revoke(
            State(auth),
            Extension(OwnerSession(token)),
            Path("silkboard".into()),
            Ok(Json(Label {
                label: "retained".into(),
            })),
        )
        .await;
        assert!(matches!(result, Err(AuthError::Unauthorized)));
        let event=serde_json::from_value(serde_json::json!({"event_id":"still-valid","hostname":"dev","tool":"codex","session_id":"native","sequence":1,"message":"Revoked owner session did not revoke publisher"})).unwrap();
        store.publish("silkboard", &publisher, &event).unwrap();
        assert_eq!(
            store
                .0
                .lock()
                .unwrap()
                .query_row("SELECT count(*) FROM publishers", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
