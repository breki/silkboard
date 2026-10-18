//! Browser session HTTP boundary and default protection for owner-only APIs.

use super::{AuthError, OwnerAuth, setup, storage};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use subtle::ConstantTimeEq;

const COOKIE: &str = "__Host-silkboard-owner";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Login {
    password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    credential: String,
    password: String,
}

pub(crate) fn routes(auth: OwnerAuth) -> Router {
    // Future feed/session reads MUST be registered inside this protected router.
    let protected = Router::new()
        .merge(super::feed::routes())
        .route("/session", get(session))
        .route("/logout", post(logout))
        .fallback(|| async { StatusCode::NOT_FOUND })
        .layer(middleware::from_fn_with_state(auth.clone(), guard));
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/setup", get(setup_status).post(register))
        .nest("/api/owner", protected)
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::map_response(|mut response: Response| async {
            response
                .headers_mut()
                .insert("cache-control", HeaderValue::from_static("no-store"));
            response.headers_mut().insert(
                "x-content-type-options",
                HeaderValue::from_static("nosniff"),
            );
            response
        }))
        .with_state(auth)
}

fn same_origin(auth: &OwnerAuth, headers: &HeaderMap) -> Result<(), AuthError> {
    let origins: Vec<_> = headers.get_all("origin").iter().collect();
    if origins.len() != 1 || origins[0].as_bytes() != auth.0.origin.as_bytes() {
        return Err(AuthError::Forbidden);
    }
    Ok(())
}

fn cookie(headers: &HeaderMap) -> Result<String, AuthError> {
    let mut found = None;
    for header in headers.get_all("cookie") {
        for part in header
            .to_str()
            .map_err(|_| AuthError::Unauthorized)?
            .split(';')
        {
            if let Some(value) = part.trim().strip_prefix(&format!("{COOKIE}="))
            {
                if found.is_some() {
                    return Err(AuthError::Unauthorized);
                }
                found = Some(value.to_owned());
            }
        }
    }
    found.ok_or(AuthError::Unauthorized)
}

async fn login(
    State(auth): State<OwnerAuth>,
    headers: HeaderMap,
    input: Result<Json<Login>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AuthError> {
    same_origin(&auth, &headers)?;
    let Json(input) = input.map_err(|_| AuthError::Invalid)?;
    auth.attempt()?;
    let permit = auth
        .0
        .login_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| AuthError::Limited)?;
    let token = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        storage::login(&auth.0.store, &input.password)
    })
    .await
    .map_err(|_| AuthError::Unavailable)??;
    Ok((StatusCode::NO_CONTENT, [("set-cookie", format!("{COOKIE}={token}; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age={}", storage::LIFETIME))]).into_response())
}

async fn guard(
    State(auth): State<OwnerAuth>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let token = cookie(request.headers())?;
    if request.headers().contains_key("origin") {
        same_origin(&auth, request.headers())?;
    }
    if request
        .headers()
        .get("sec-fetch-site")
        .is_some_and(|h| h != "same-origin" && h != "none")
    {
        return Err(AuthError::Forbidden);
    }
    if !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) {
        same_origin(&auth, request.headers())?;
        let expected = storage::csrf(&token);
        let csrf = request
            .headers()
            .get("x-silkboard-csrf")
            .ok_or(AuthError::Forbidden)?;
        if !bool::from(expected.as_bytes().ct_eq(csrf.as_bytes())) {
            return Err(AuthError::Forbidden);
        }
    }
    let permit = auth
        .0
        .store_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| AuthError::Unavailable)?;
    let state = auth.clone();
    let value = token.clone();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        storage::authenticate(&state.0.store, &value)
    })
    .await
    .map_err(|_| AuthError::Unavailable)??;
    request.extensions_mut().insert(OwnerSession(token));
    Ok(next.run(request).await)
}

#[derive(Clone)]
struct OwnerSession(String);

async fn session(
    axum::Extension(session): axum::Extension<OwnerSession>,
) -> Json<serde_json::Value> {
    Json(
        serde_json::json!({"authenticated":true,"csrf_token":storage::csrf(&session.0)}),
    )
}

async fn logout(
    State(auth): State<OwnerAuth>,
    axum::Extension(session): axum::Extension<OwnerSession>,
) -> Result<Response, AuthError> {
    let permit = auth
        .0
        .store_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| AuthError::Unavailable)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        storage::logout(&auth.0.store, &session.0)
    })
    .await
    .map_err(|_| AuthError::Unavailable)??;
    Ok((StatusCode::NO_CONTENT, [("set-cookie", format!("{COOKIE}=; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=0"))]).into_response())
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Invalid => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Limited => StatusCode::TOO_MANY_REQUESTS,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::SetupClosed => StatusCode::GONE,
        };
        let mut response =
            (status, Json(serde_json::json!({"error":self.to_string()})))
                .into_response();
        if matches!(self, Self::Limited) {
            response
                .headers_mut()
                .insert("retry-after", HeaderValue::from_static("60"));
        }
        response
    }
}

async fn setup_status(
    State(auth): State<OwnerAuth>,
) -> Result<Json<serde_json::Value>, AuthError> {
    let state = auth.query(setup::status).await?;
    Ok(Json(serde_json::json!({"state":state})))
}

async fn register(
    State(auth): State<OwnerAuth>,
    headers: HeaderMap,
    input: Result<Json<Registration>, axum::extract::rejection::JsonRejection>,
) -> Result<StatusCode, AuthError> {
    same_origin(&auth, &headers)?;
    if headers.get("x-silkboard-setup").is_none_or(|h| h != "1")
        || headers
            .get("sec-fetch-site")
            .is_some_and(|h| h != "same-origin" && h != "none")
    {
        return Err(AuthError::Forbidden);
    }
    let Json(input) = input.map_err(|_| AuthError::Invalid)?;
    auth.attempt()?;
    let permit = auth
        .0
        .login_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| AuthError::Limited)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        setup::register(&auth.0.store, &input.credential, &input.password)
    })
    .await
    .map_err(|_| AuthError::Unavailable)??;
    Ok(StatusCode::NO_CONTENT)
}
