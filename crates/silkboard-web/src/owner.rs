//! Single-owner browser authentication, independent of publisher credentials.

mod feed;
mod http;
mod plans;
mod publishers;
mod setup;
mod storage;

use crate::progress::ProgressStore;
use axum::http::Uri;
use std::{
    io::Read,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;

pub(crate) use http::routes;
pub use setup::prepare_setup;

/// Owner authentication failures contain no credentials or database details.
#[derive(Debug)]
pub enum AuthError {
    /// Malformed input or configuration.
    Invalid,
    /// Missing, invalid, expired or revoked credentials.
    Unauthorized,
    /// Origin or request-forgery check failed.
    Forbidden,
    /// Login budget or hashing concurrency exhausted.
    Limited,
    /// Storage, password-hash or entropy operation failed.
    Unavailable,
    /// An authenticated read addressed a nonexistent project or session.
    NotFound,
    /// First-owner registration has permanently closed.
    SetupClosed,
    /// An installation label is already used, including revoked credentials.
    Conflict,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Invalid => "invalid owner input",
            Self::Unauthorized => "owner authentication required",
            Self::Forbidden => "owner request rejected",
            Self::Limited => "owner login temporarily limited",
            Self::Unavailable => "owner authentication unavailable",
            Self::NotFound => "progress resource not found",
            Self::SetupClosed => "owner setup is already complete",
            Self::Conflict => "publisher installation label already exists",
        })
    }
}
impl std::error::Error for AuthError {}
impl From<rusqlite::Error> for AuthError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Unavailable
    }
}

#[derive(Clone)]
pub(crate) struct OwnerAuth(Arc<Inner>);
struct Inner {
    store: ProgressStore,
    origin: String,
    login_slots: Arc<Semaphore>,
    store_slots: Arc<Semaphore>,
    attempts: Mutex<Attempts>,
}
struct Attempts {
    since: Instant,
    count: u8,
}

impl OwnerAuth {
    pub(crate) fn new(
        store: ProgressStore,
        origin: &str,
    ) -> Result<Self, AuthError> {
        let uri: Uri = origin.parse().map_err(|_| AuthError::Invalid)?;
        // An exact serialized HTTPS origin, without credentials/path/query.
        let authority = uri.authority().ok_or(AuthError::Invalid)?;
        let host = authority.host();
        let suffix = authority
            .as_str()
            .strip_prefix(host)
            .ok_or(AuthError::Invalid)?;
        if !suffix.is_empty() {
            let port = suffix
                .strip_prefix(':')
                .ok_or(AuthError::Invalid)?
                .parse::<u16>()
                .map_err(|_| AuthError::Invalid)?;
            if port == 0 || port == 443 || suffix != format!(":{port}") {
                return Err(AuthError::Invalid);
            }
        }
        if uri.scheme_str() != Some("https")
            || authority.as_str().contains('@')
            || authority.host().is_empty()
            || host != host.to_ascii_lowercase()
            || uri.path_and_query().is_some_and(|p| p.as_str() != "/")
            || origin != format!("https://{authority}")
        {
            return Err(AuthError::Invalid);
        }
        Ok(Self(Arc::new(Inner {
            store,
            origin: origin.into(),
            login_slots: Arc::new(Semaphore::new(2)),
            store_slots: Arc::new(Semaphore::new(8)),
            attempts: Mutex::new(Attempts {
                since: Instant::now(),
                count: 0,
            }),
        })))
    }

    fn attempt(&self) -> Result<(), AuthError> {
        let mut attempts =
            self.0.attempts.lock().map_err(|_| AuthError::Unavailable)?;
        if attempts.since.elapsed() >= Duration::from_secs(60) {
            attempts.since = Instant::now();
            attempts.count = 0;
        }
        if attempts.count >= 5 {
            return Err(AuthError::Limited);
        }
        attempts.count += 1;
        Ok(())
    }

    async fn query<T: Send + 'static>(
        &self,
        query: impl FnOnce(&ProgressStore) -> Result<T, AuthError> + Send + 'static,
    ) -> Result<T, AuthError> {
        let permit = self
            .0
            .store_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| AuthError::Unavailable)?;
        let store = self.0.store.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            query(&store)
        })
        .await
        .map_err(|_| AuthError::Unavailable)?
    }
}

/// Set/reset the sole owner password from a private pipe, revoking all sessions.
/// Reads at most 1,027 bytes; strips one final LF (and optional preceding CR).
/// No password is accepted in command arguments or returned to the caller.
///
/// # Errors
/// Returns invalid-password/input, entropy, hashing or database errors.
pub fn set_password_from_reader(
    store: &ProgressStore,
    reader: impl Read,
) -> Result<(), AuthError> {
    let mut input = String::new();
    reader
        .take(1027)
        .read_to_string(&mut input)
        .map_err(|_| AuthError::Invalid)?;
    if input.len() > 1026 {
        return Err(AuthError::Invalid);
    }
    if input.ends_with('\n') {
        input.pop();
        if input.ends_with('\r') {
            input.pop();
        }
    }
    storage::set_password(store, &input)
}
