//! Bounded, local-only native event associations captured before turn completion.
use crate::{Error, context, model::Config, state};
use serde::{Deserialize, Serialize};
use silkboard_plan::{Reporter, valid_task_id};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const LIMIT: usize = 32;
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    event: String,
    turn: String,
    task_id: Option<String>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    schema_version: u8,
    entries: Vec<Entry>,
}
fn valid(event: &str, turn: &str) -> bool {
    matches!(event, "Stop" | "SubagentStart" | "SubagentStop")
        && !turn.is_empty()
        && turn.len() <= 256
        && turn.bytes().all(|b| b.is_ascii_graphic())
}
fn path(config: &Config, key: &str) -> PathBuf {
    config.state_dir.join(format!("hook-{key}.json"))
}
fn load(path: &std::path::Path) -> Result<Stored, Error> {
    if let Err(e) = path.symlink_metadata() {
        return if e.kind() == std::io::ErrorKind::NotFound {
            Ok(Stored {
                schema_version: 1,
                entries: vec![],
            })
        } else {
            Err(Error::State)
        };
    }
    let stored: Stored =
        serde_json::from_slice(&state::read_private(path, 16384)?)
            .map_err(|_| Error::State)?;
    if stored.schema_version != 1
        || stored.entries.len() > LIMIT
        || stored.entries.iter().enumerate().any(|(i, e)| {
            !valid(&e.event, &e.turn)
                || e.task_id.as_ref().is_some_and(|id| !valid_task_id(id))
                || stored.entries[..i]
                    .iter()
                    .any(|prior| prior.event == e.event && prior.turn == e.turn)
        })
    {
        return Err(Error::State);
    }
    Ok(stored)
}
fn save(path: &std::path::Path, stored: &mut Stored) -> Result<(), Error> {
    loop {
        let bytes = serde_json::to_vec(stored).map_err(|_| Error::State)?;
        if bytes.len() <= 16384 {
            return state::replace(path, &bytes);
        }
        // Escaped turn IDs can exceed the byte budget before the count limit.
        // Evict oldest metadata so our own next read remains valid.
        stored.entries.remove(0);
    }
}

pub(super) fn capture(
    config: &Config,
    reporter: &Reporter,
    event: &str,
    turn: &str,
) -> Result<(), Error> {
    if !valid(event, turn) {
        return Err(Error::Input);
    }
    // Current selection is read only at the synchronous native boundary.
    // The guard also serializes the bounded capture store for this identity.
    let selected = context::Context::lock(config, reporter)?;
    let task_id = selected.task_id(config, reporter);
    let key = context::key(config, reporter)?;
    let path = path(config, &key);
    let mut stored = load(&path)?;
    if let Some(prior) = stored
        .entries
        .iter_mut()
        .find(|e| e.event == event && e.turn == turn)
    {
        // Conflicting duplicate captures cannot safely choose either workflow.
        if prior.task_id != task_id {
            prior.task_id = None;
        }
    } else {
        if stored.entries.len() == LIMIT {
            stored.entries.remove(0);
        }
        stored.entries.push(Entry {
            event: event.into(),
            turn: turn.into(),
            task_id,
        });
    }
    save(&path, &mut stored)
}

pub(super) fn resolve(
    config: &Config,
    reporter: &Reporter,
    event: &str,
    turn: &str,
) -> Option<String> {
    if !valid(event, turn) {
        return None;
    }
    // Matching native command hooks can start concurrently. Wait briefly for
    // the synchronous capture, never for networking or a mutable selection.
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        match lookup(config, reporter, event, turn) {
            Ok(Some(entry)) => return entry.task_id,
            Ok(None) | Err(Error::Busy) => {}
            Err(_) => return None,
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn lookup(
    config: &Config,
    reporter: &Reporter,
    event: &str,
    turn: &str,
) -> Result<Option<Entry>, Error> {
    let (_lock, key) = context::native_lock(config, reporter)?;
    let path = path(config, &key);
    let stored = load(&path)?;
    let Some(index) = stored
        .entries
        .iter()
        .position(|e| e.event == event && e.turn == turn)
    else {
        return Ok(None);
    };
    let entry = &stored.entries[index];
    // Historical association is immutable; a later switch/handoff is irrelevant.
    Ok(Some(entry.clone()))
}
