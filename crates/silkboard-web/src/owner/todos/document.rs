//! One revision-checked document transaction; client keys never select server IDs.
use super::{
    AuthError, Checklist, OwnerAuth, Todo, mutate, number, snapshot, valid_text,
};
use crate::owner::http::OwnerSession;
use axum::{
    Extension, Json,
    extract::{DefaultBodyLimit, State},
    routing::{MethodRouter, put},
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    revision: String,
    items: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    key: String,
    id: Option<String>,
    parent_key: Option<String>,
    text: String,
    completed: bool,
}
#[derive(Serialize)]
struct Saved {
    #[serde(flatten)]
    checklist: Checklist,
    keys: HashMap<String, String>,
}

pub(super) fn route() -> MethodRouter<OwnerAuth> {
    // 1,000 tasks, each with 1,000 UTF-8 bytes, plus JSON escaping/metadata.
    // This bound applies only here; other owner mutations retain 4 KiB.
    put(save).layer(DefaultBodyLimit::max(3_000_000))
}
async fn save(
    State(auth): State<OwnerAuth>,
    Extension(session): Extension<OwnerSession>,
    input: Result<Json<Document>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Saved>, AuthError> {
    let Json(input) = input.map_err(|_| AuthError::Invalid)?;
    Ok(Json(
        auth.query(move |store| {
            let mut keys = HashMap::new();
            let checklist = mutate(store, &session.0, &input.revision, |tx| {
                keys = replace(tx, &input.items)?;
                Ok(())
            })?;
            Ok(Saved { checklist, keys })
        })
        .await?,
    ))
}
fn key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
fn validate(items: &[Entry], current: &[Todo]) -> Result<(), AuthError> {
    if items.len() > 1000 {
        return Err(AuthError::Invalid);
    }
    let known: HashSet<_> =
        current.iter().map(|item| item.id.as_str()).collect();
    let mut keys = HashSet::new();
    let mut ids = HashSet::new();
    let mut ancestors: Vec<&str> = Vec::new();
    for item in items {
        if !key(&item.key)
            || !keys.insert(item.key.as_str())
            || !valid_text(&item.text)
        {
            return Err(AuthError::Invalid);
        }
        if let Some(id) = &item.id {
            number(id, false)?;
            if !ids.insert(id.as_str()) {
                return Err(AuthError::Invalid);
            }
            if !known.contains(id.as_str()) {
                return Err(AuthError::NotFound);
            }
        }
        if let Some(parent) = &item.parent_key {
            let index = ancestors
                .iter()
                .position(|key| *key == parent)
                .ok_or(AuthError::Invalid)?;
            ancestors.truncate(index + 1);
        } else {
            ancestors.clear();
        }
        ancestors.push(&item.key);
        if ancestors.len() > 5 {
            return Err(AuthError::Invalid);
        }
    }
    Ok(())
}
fn replace(
    conn: &Connection,
    items: &[Entry],
) -> Result<HashMap<String, String>, AuthError> {
    let current = snapshot(conn)?.items;
    validate(items, &current)?;
    let keep: HashSet<_> =
        items.iter().filter_map(|item| item.id.as_deref()).collect();
    // Clear references first so any removed parent can be deleted atomically.
    conn.execute("UPDATE todos SET parent_id=NULL", [])?;
    for item in &current {
        if !keep.contains(item.id.as_str()) {
            conn.execute(
                "DELETE FROM todos WHERE id=?1",
                [number(&item.id, false)?],
            )?;
        }
    }
    let mut keys = HashMap::new();
    for item in items {
        let id = if let Some(id) = &item.id {
            number(id, false)?
        } else {
            conn.execute(
                "INSERT INTO todos(text,completed) VALUES(?1,?2)",
                params![item.text, item.completed],
            )?;
            conn.last_insert_rowid()
        };
        keys.insert(item.key.clone(), id.to_string());
    }
    let mut placements = Vec::new();
    for item in items {
        let parent = item
            .parent_key
            .as_ref()
            .map(|parent| number(&keys[parent], false))
            .transpose()?;
        let id = number(&keys[&item.key], false)?;
        conn.execute(
            "UPDATE todos SET text=?1,completed=?2 WHERE id=?3",
            params![item.text, item.completed, id],
        )?;
        placements.push(super::layout::Placement { id, parent });
    }
    super::layout::write(conn, &placements)?;
    super::layout::project_legacy(conn)?;
    Ok(keys)
}
