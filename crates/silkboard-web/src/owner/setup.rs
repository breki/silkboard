//! Expiring hash-only authorization and atomic permanent first-owner closure.
use super::{AuthError, OwnerAuth, storage};
use crate::{
    progress::ProgressStore,
    secrets::{encode, token_hash},
};
use rusqlite::{Connection, TransactionBehavior, params};

/// Prepare/replace a one-hour first-owner setup link for private delivery.
/// Only its verification hash is persisted. The caller must transport the
/// returned secret through a private pipe, never logs, chat or a server file.
/// Existing initialized owners and permanently closed setup cannot be reopened.
///
/// # Errors
/// Returns invalid HTTPS origin, closed setup, entropy or storage errors.
pub fn prepare_setup(
    store: &ProgressStore,
    origin: &str,
) -> Result<String, AuthError> {
    let _ = OwnerAuth::new(store.clone(), origin)?;
    let credential = encode(&storage::random::<32>()?);
    let mut connection = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    let tx =
        connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    ensure_open(&tx)?;
    tx.execute("UPDATE owner_setup SET token_hash = ?1, expires_at = unixepoch() + 3600 WHERE id = 1", [token_hash(&credential)])?;
    tx.commit()?;
    Ok(format!("{origin}/setup#setup={credential}"))
}

fn ensure_open(connection: &Connection) -> Result<(), AuthError> {
    let closed: bool = connection.query_row("SELECT closed OR EXISTS(SELECT 1 FROM owner) FROM owner_setup WHERE id = 1", [], |r| r.get(0))?;
    if closed {
        return Err(AuthError::SetupClosed);
    }
    Ok(())
}

fn authorize(
    connection: &Connection,
    credential: &str,
) -> Result<(), AuthError> {
    ensure_open(connection)?;
    if credential.len() != 64
        || !credential.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err(AuthError::Unauthorized);
    }
    let valid: bool = connection.query_row("SELECT token_hash = ?1 AND expires_at > unixepoch() FROM owner_setup WHERE id = 1", [token_hash(credential)], |r| Ok(r.get::<_, Option<bool>>(0)?.unwrap_or(false)))?;
    if !valid {
        return Err(AuthError::Unauthorized);
    }
    Ok(())
}

pub(super) fn status(store: &ProgressStore) -> Result<&'static str, AuthError> {
    let connection = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    match ensure_open(&connection) {
        Err(AuthError::SetupClosed) => Ok("closed"),
        Err(error) => Err(error),
        Ok(()) => {
            let pending: bool = connection.query_row("SELECT token_hash IS NOT NULL AND expires_at > unixepoch() FROM owner_setup WHERE id = 1", [], |r| r.get(0))?;
            Ok(if pending { "available" } else { "waiting" })
        }
    }
}

pub(super) fn register(
    store: &ProgressStore,
    credential: &str,
    password: &str,
) -> Result<(), AuthError> {
    {
        let connection = store.0.lock().map_err(|_| AuthError::Unavailable)?;
        authorize(&connection, credential)?;
    }
    // Hash outside the database lock; the HTTP layer bounds hashing slots.
    let hash = storage::password_hash(password)?;
    let mut connection = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    let tx =
        connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    authorize(&tx, credential)?;
    tx.execute(
        "INSERT INTO owner (id, password_hash, generation) VALUES (1, ?1, 1)",
        [hash],
    )?;
    tx.execute("UPDATE owner_setup SET closed = 1, token_hash = NULL, expires_at = 0 WHERE id = 1", params![])?;
    tx.commit()?;
    Ok(())
}
