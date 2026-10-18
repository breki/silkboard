//! Password hashing and durable owner sessions, never publisher tokens.

use super::AuthError;
use crate::progress::ProgressStore;
use crate::secrets::{encode, token_hash as digest};
use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier,
    Version, password_hash::SaltString,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};

pub(super) const LIFETIME: i64 = 12 * 60 * 60;

pub(super) fn random<const N: usize>() -> Result<[u8; N], AuthError> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(|_| AuthError::Unavailable)?;
    Ok(bytes)
}

fn password_valid(password: &str) -> Result<(), AuthError> {
    if !(12..=1024).contains(&password.len())
        || password.chars().any(char::is_control)
    {
        return Err(AuthError::Invalid);
    }
    Ok(())
}

fn algorithm() -> Argon2<'static> {
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(65536, 3, 1, Some(32))
            .expect("fixed Argon2 parameters are valid"),
    )
}

pub(super) fn set_password(
    store: &ProgressStore,
    password: &str,
) -> Result<(), AuthError> {
    let hash = password_hash(password)?;
    let mut conn = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute("INSERT INTO owner (id, password_hash, generation) VALUES (1, ?1, 1) ON CONFLICT(id) DO UPDATE SET password_hash = excluded.password_hash, generation = generation + 1", [hash])?;
    tx.execute("UPDATE owner_setup SET closed = 1, token_hash = NULL, expires_at = 0 WHERE id = 1", [])?;
    tx.execute("DELETE FROM owner_sessions", [])?;
    tx.commit()?;
    Ok(())
}

pub(super) fn password_hash(password: &str) -> Result<String, AuthError> {
    password_valid(password)?;
    let salt = SaltString::encode_b64(&random::<16>()?)
        .map_err(|_| AuthError::Unavailable)?;
    Ok(algorithm()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AuthError::Unavailable)?
        .to_string())
}

pub(super) fn login(
    store: &ProgressStore,
    password: &str,
) -> Result<String, AuthError> {
    // Invalid passwords never reach the expensive verifier.
    password_valid(password).map_err(|_| AuthError::Unauthorized)?;
    let (hash, generation): (String, i64) = store
        .0
        .lock()
        .map_err(|_| AuthError::Unavailable)?
        .query_row(
            "SELECT password_hash, generation FROM owner WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or(AuthError::Unauthorized)?;
    let parsed =
        PasswordHash::new(&hash).map_err(|_| AuthError::Unavailable)?;
    // Reject corrupted/unexpected PHC work factors rather than allocate from them.
    if parsed.algorithm.as_str() != "argon2id"
        || parsed.version != Some(19)
        || parsed.params.get_decimal("m") != Some(65536)
        || parsed.params.get_decimal("t") != Some(3)
        || parsed.params.get_decimal("p") != Some(1)
        || parsed.hash.is_none_or(|h| h.len() != 32)
    {
        return Err(AuthError::Unavailable);
    }
    algorithm()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AuthError::Unauthorized)?;
    let token = encode(&random::<32>()?);
    let mut conn = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute(
        "DELETE FROM owner_sessions WHERE expires_at <= unixepoch()",
        [],
    )?;
    let inserted = tx.execute(
        "INSERT INTO owner_sessions (token_hash, expires_at) SELECT ?1, unixepoch() + ?2 WHERE EXISTS (SELECT 1 FROM owner WHERE id = 1 AND generation = ?3)",
        params![digest(&token), LIFETIME, generation],
    )?;
    if inserted != 1 {
        return Err(AuthError::Unauthorized);
    }
    tx.commit()?;
    Ok(token)
}

pub(super) fn authenticate(
    store: &ProgressStore,
    token: &str,
) -> Result<(), AuthError> {
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(AuthError::Unauthorized);
    }
    let authorized = store.0.lock().map_err(|_| AuthError::Unavailable)?.query_row(
        "SELECT EXISTS(SELECT 1 FROM owner_sessions WHERE token_hash = ?1 AND expires_at > unixepoch())", [digest(token)], |r| r.get::<_, bool>(0),
    )?;
    if !authorized {
        return Err(AuthError::Unauthorized);
    }
    Ok(())
}

pub(super) fn logout(
    store: &ProgressStore,
    token: &str,
) -> Result<(), AuthError> {
    store
        .0
        .lock()
        .map_err(|_| AuthError::Unavailable)?
        .execute(
            "DELETE FROM owner_sessions WHERE token_hash = ?1",
            [digest(token)],
        )?;
    Ok(())
}

pub(super) fn csrf(token: &str) -> String {
    encode(&Sha256::digest(
        format!("silkboard-owner-csrf:{token}").as_bytes(),
    ))
}
