//! Persistent publish-only progress storage and project credentials.

mod http;
mod model;
mod operator;

pub(crate) use http::routes;
pub(crate) use model::project as validate_project;
pub use model::{Acknowledgement, ProgressEvent};
pub use operator::publisher_command;

use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::secrets::{encode, token_hash};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, TransactionBehavior, params,
};

/// A storage failure. Display text never includes SQL, payloads or credentials.
#[derive(Debug)]
pub enum ProgressError {
    /// A malformed identity, payload or operator argument.
    Invalid,
    /// Missing, revoked or wrong-project publisher credential.
    Unauthorized,
    /// An event ID, sequence or immutable session metadata was reused differently.
    Conflict,
    /// Storage, entropy or filesystem failure; no successful delivery is implied.
    Unavailable,
    /// The database belongs to a newer application schema.
    NewerSchema,
}

impl std::fmt::Display for ProgressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Invalid => "invalid progress input",
            Self::Unauthorized => "publisher not authorized",
            Self::Conflict => "conflicting progress identity",
            Self::Unavailable => "progress storage unavailable",
            Self::NewerSchema => {
                "database schema is newer than this application"
            }
        })
    }
}

impl std::error::Error for ProgressError {}

impl From<rusqlite::Error> for ProgressError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Unavailable
    }
}

/// Shared synchronous `SQLite` store. HTTP callers use a bounded blocking pool.
#[derive(Clone)]
pub struct ProgressStore(pub(crate) Arc<Mutex<Connection>>);

impl ProgressStore {
    /// Open and migrate a database in an existing private directory.
    ///
    /// Reject symlinks and group/world-accessible directories/files on Unix.
    /// Keep this path outside release trees and the static asset directory.
    ///
    /// # Errors
    /// Returns invalid-path/permission, storage or incompatible-schema errors.
    pub fn open(path: &Path) -> Result<Self, ProgressError> {
        operator::prepare_database(path)?;
        Self::initialize(Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?)
    }

    /// Create a private in-memory store, primarily for embedded use and tests.
    ///
    /// # Errors
    /// Returns a storage error if initialization fails.
    pub fn open_memory() -> Result<Self, ProgressError> {
        Self::initialize(Connection::open_in_memory()?)
    }

    fn initialize(mut connection: Connection) -> Result<Self, ProgressError> {
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        // Immediate transaction serializes first-start migration with operators.
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 =
            tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version == 0 {
            tx.execute_batch(include_str!("progress/schema.sql"))?;
        }
        match version {
            0 | 1 => tx.execute_batch(include_str!("owner/schema.sql"))?,
            2 | 3 => {}
            _ => return Err(ProgressError::NewerSchema),
        }
        if version <= 2 {
            tx.execute_batch(include_str!("owner/setup.sql"))?;
        }
        tx.commit()?;
        Ok(Self(Arc::new(Mutex::new(connection))))
    }

    /// Issue a random 256-bit publisher token; only its SHA-256 hash is stored.
    ///
    /// The caller must transport/store the returned secret privately. Labels are
    /// unique within a project, including revoked labels; use a new label to rotate.
    ///
    /// # Errors
    /// Returns invalid input, duplicate-label, entropy or storage errors.
    pub fn issue_token(
        &self,
        project: &str,
        label: &str,
    ) -> Result<String, ProgressError> {
        model::project(project)?;
        model::identity(label)?;
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| ProgressError::Unavailable)?;
        let token = encode(&bytes);
        let mut conn = self.0.lock().map_err(|_| ProgressError::Unavailable)?;
        let tx =
            conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT OR IGNORE INTO projects (id) VALUES (?1)",
            [project],
        )?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO publishers (project, label, token_hash) VALUES (?1, ?2, ?3)",
            params![project, label, token_hash(&token)],
        )?;
        if inserted == 0 {
            return Err(ProgressError::Conflict);
        }
        tx.commit()?;
        Ok(token)
    }

    /// Revoke a project/label credential, idempotently. History is retained.
    ///
    /// # Errors
    /// Returns invalid-input or storage errors.
    pub fn revoke_token(
        &self,
        project: &str,
        label: &str,
    ) -> Result<(), ProgressError> {
        model::project(project)?;
        model::identity(label)?;
        self.0.lock().map_err(|_| ProgressError::Unavailable)?.execute(
            "UPDATE publishers SET revoked = 1 WHERE project = ?1 AND label = ?2",
            (project, label),
        )?;
        Ok(())
    }

    /// Atomically authenticate and store one event, or acknowledge an exact retry.
    /// No method in the ingestion API returns session history.
    ///
    /// # Errors
    /// Returns authorization, validation, identity-conflict or storage errors.
    pub fn publish(
        &self,
        project: &str,
        token: &str,
        event: &ProgressEvent,
    ) -> Result<Acknowledgement, ProgressError> {
        model::project(project)?;
        let mut conn = self.0.lock().map_err(|_| ProgressError::Unavailable)?;
        let tx =
            conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !tx.query_row("SELECT EXISTS(SELECT 1 FROM owner)", [], |r| {
            r.get::<_, bool>(0)
        })? {
            return Err(ProgressError::Unavailable);
        }
        let authorized = token.len() == 64 && tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM publishers WHERE project = ?1 AND token_hash = ?2 AND revoked = 0)",
            params![project, token_hash(token)], |r| r.get::<_, bool>(0),
        )?;
        if !authorized {
            return Err(ProgressError::Unauthorized);
        }
        event.validate()?;
        let payload =
            serde_json::to_string(event).map_err(|_| ProgressError::Invalid)?;
        let existing: Option<(String, i64)> = tx.query_row(
            "SELECT payload, received_at FROM events WHERE project = ?1 AND event_id = ?2",
            (project, &event.event_id), |r| Ok((r.get(0)?, r.get(1)?)),
        ).optional()?;
        if let Some((stored, received_at)) = existing {
            if stored != payload {
                return Err(ProgressError::Conflict);
            }
            return Ok(Acknowledgement {
                event_id: event.event_id.clone(),
                received_at,
            });
        }
        let child = event.subagent_id.as_deref().unwrap_or("");
        tx.execute(
            "INSERT OR IGNORE INTO sessions (project, hostname, tool, native_id, child_id, parent_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![project, event.hostname, event.tool, event.session_id, child, event.parent_session_id],
        )?;
        let (session, parent): (i64, Option<String>) = tx.query_row(
            "SELECT id, parent_id FROM sessions WHERE project = ?1 AND hostname = ?2 AND tool = ?3 AND native_id = ?4 AND child_id = ?5",
            params![project, event.hostname, event.tool, event.session_id, child],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if parent != event.parent_session_id {
            return Err(ProgressError::Conflict);
        }
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO events (project, event_id, session, sequence, message, session_name, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![project, event.event_id, session, event.sequence, event.message, event.session_name, payload],
        )?;
        if inserted == 0 {
            return Err(ProgressError::Conflict);
        }
        let received_at = tx.query_row(
            "SELECT received_at FROM events WHERE project = ?1 AND event_id = ?2",
            (project, &event.event_id), |r| r.get(0),
        )?;
        tx.commit()?;
        Ok(Acknowledgement {
            event_id: event.event_id.clone(),
            received_at,
        })
    }
}
