//! Explicit plan reporting and local handoff, never dashboard task fetching.
mod state;
use crate::{Error, configuration, model::bounded, token, transport};
use serde::Deserialize;
use silkboard_plan::{Counter, IssueReference, Reporter, Snapshot, Step};
use std::{ffi::OsStr, io::Read, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    schema_version: u8,
    task_id: String,
    reporter: Reporter,
    title: String,
    description: Option<String>,
    github_issue: Option<IssueReference>,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexDraft {
    schema_version: u8,
    task_id: String,
    title: String,
    description: Option<String>,
    github_issue: Option<IssueReference>,
    steps: Vec<Step>,
}

pub(super) fn run_codex(
    command: &OsStr,
    path: &Path,
    input: impl Read,
) -> Result<&'static str, Error> {
    if command != "start" && command != "publish" {
        return Err(Error::Usage);
    }
    let config = configuration(path)?;
    let reporter = crate::codex::plan_reporter(&config)?;
    let draft: CodexDraft = serde_json::from_slice(&bounded(
        input,
        silkboard_plan::MAX_SNAPSHOT_BYTES as u64,
    )?)
    .map_err(|_| Error::Input)?;
    send(
        &config,
        Draft {
            schema_version: draft.schema_version,
            task_id: draft.task_id,
            reporter,
            title: draft.title,
            description: draft.description,
            github_issue: draft.github_issue,
            steps: draft.steps,
        },
        command == "start",
    )
}
impl Draft {
    fn snapshot(self) -> Result<Snapshot, Error> {
        let one = Counter::new(1).map_err(|_| Error::State)?;
        Snapshot {
            schema_version: self.schema_version,
            task_id: self.task_id,
            reporter: self.reporter,
            title: self.title,
            description: self.description,
            github_issue: self.github_issue,
            steps: self.steps,
            sequence: one,
            plan_revision: one,
            writer_generation: one,
        }
        .normalized()
        .map_err(|_| Error::Input)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Handoff {
    schema_version: u8,
    task_id: String,
    from: Reporter,
    to: Reporter,
}

pub(super) fn run(
    command: &OsStr,
    path: &Path,
    input: impl Read,
) -> Result<&'static str, Error> {
    if command != "start" && command != "publish" && command != "handoff" {
        return Err(Error::Usage);
    }
    let config = configuration(path)?;
    let bytes = bounded(input, silkboard_plan::MAX_SNAPSHOT_BYTES as u64)?;
    if command == "handoff" {
        let handoff: Handoff =
            serde_json::from_slice(&bytes).map_err(|_| Error::Input)?;
        state::handoff(&config, handoff)?;
        return Ok("Task plan handed off locally");
    }
    let draft: Draft =
        serde_json::from_slice(&bytes).map_err(|_| Error::Input)?;
    send(&config, draft, command == "start")
}

fn send(
    config: &crate::model::Config,
    draft: Draft,
    start: bool,
) -> Result<&'static str, Error> {
    let snapshot = draft.snapshot()?;
    let token = token(config)?;
    let snapshot = state::allocate(config, snapshot, start)?;
    transport::send_plan(config, &snapshot, &token)?;
    Ok("Task plan acknowledged")
}
