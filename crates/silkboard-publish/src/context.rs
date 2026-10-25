//! Explicit current-workflow selection, never discovery from stored plans.
use crate::{Error, model::Config, state};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use silkboard_plan::{Counter, Reporter, Snapshot, valid_task_id};
use std::{fmt::Write, fs::File, path::PathBuf};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    task_id: String,
    writer_generation: Counter,
    acknowledged: bool,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    schema_version: u8,
    selection: Option<Selection>,
}

pub(super) struct Context {
    _lock: File,
    path: PathBuf,
    selection: Option<Selection>,
}
impl Context {
    // Lock ordering is native context before task state. Handoff takes only
    // the task lock; readers revalidate its current writer and generation.
    pub fn lock(config: &Config, reporter: &Reporter) -> Result<Self, Error> {
        let (lock, key) = native_lock(config, reporter)?;
        let path = config.state_dir.join(format!("{key}.json"));
        let selection = read_selection(&path)?;
        Ok(Self {
            _lock: lock,
            path,
            selection,
        })
    }
    pub fn clear(&mut self) -> Result<(), Error> {
        self.save(None)
    }
    pub fn select(
        &mut self,
        snapshot: &Snapshot,
        acknowledged: bool,
    ) -> Result<(), Error> {
        self.save(Some(Selection {
            task_id: snapshot.task_id.clone(),
            writer_generation: snapshot.writer_generation,
            acknowledged,
        }))
    }
    fn save(&mut self, selection: Option<Selection>) -> Result<(), Error> {
        let stored = Stored {
            schema_version: 1,
            selection: selection.clone(),
        };
        state::replace(
            &self.path,
            &serde_json::to_vec(&stored).map_err(|_| Error::State)?,
        )?;
        self.selection = selection;
        Ok(())
    }
    pub fn matches(&self, snapshot: &Snapshot) -> bool {
        self.selection.as_ref().is_some_and(|s| {
            s.task_id == snapshot.task_id
                && s.writer_generation == snapshot.writer_generation
        })
    }
    pub fn task_id(
        &self,
        config: &Config,
        reporter: &Reporter,
    ) -> Option<String> {
        let selected = self.selection.as_ref().filter(|s| s.acknowledged)?;
        let snapshot =
            crate::plans::owned(config, &selected.task_id, reporter).ok()?;
        (snapshot.writer_generation == selected.writer_generation)
            .then(|| selected.task_id.clone())
    }
}

pub(super) fn native_lock(
    config: &Config,
    reporter: &Reporter,
) -> Result<(File, String), Error> {
    let key = key(config, reporter)?;
    let lock = state::lock(&config.state_dir, &key)?;
    Ok((lock, key))
}

fn read_selection(path: &std::path::Path) -> Result<Option<Selection>, Error> {
    let selection = match path.symlink_metadata() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err(Error::State),
        Ok(_) => {
            let stored: Stored =
                serde_json::from_slice(&state::read_private(path, 2048)?)
                    .map_err(|_| Error::State)?;
            if stored.schema_version != 1
                || stored
                    .selection
                    .as_ref()
                    .is_some_and(|s| !valid_task_id(&s.task_id))
            {
                return Err(Error::State);
            }
            stored.selection
        }
    };
    Ok(selection)
}

pub(super) fn key(
    config: &Config,
    reporter: &Reporter,
) -> Result<String, Error> {
    let report = crate::model::Report {
        hostname: reporter.hostname.clone(),
        tool: reporter.tool.clone(),
        session_id: reporter.session_id.clone(),
        subagent_id: reporter.subagent_id.clone(),
        parent_session_id: reporter.parent_session_id.clone(),
        session_name: reporter.session_name.clone(),
        message: "Workflow context".into(),
        description: None,
        task_id: None,
    };
    report.validate()?;
    let identity = serde_json::to_vec(&(
        &config.endpoint,
        &config.project,
        &reporter.hostname,
        &reporter.tool,
        &reporter.session_id,
        &reporter.subagent_id,
    ))
    .map_err(|_| Error::Input)?;
    let mut key = String::from("context-");
    for byte in Sha256::digest(identity) {
        write!(key, "{byte:02x}").map_err(|_| Error::State)?;
    }
    Ok(key)
}
