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
    if !matches!(
        command.to_str(),
        Some("start" | "publish" | "select" | "clear")
    ) {
        return Err(Error::Usage);
    }
    let config = configuration(path)?;
    let reporter = if command == "clear" {
        crate::codex::context_reporter(&config)?
    } else {
        crate::codex::plan_reporter(&config)?
    };
    if command == "clear" {
        crate::context::Context::lock(&config, &reporter)?.clear()?;
        return Ok("Workflow context cleared locally");
    }
    if command == "select" {
        let choice: CodexSelection =
            serde_json::from_slice(&bounded(input, 8192)?)
                .map_err(|_| Error::Input)?;
        return select(
            &config,
            choice.schema_version,
            &choice.task_id,
            &reporter,
        );
    }
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
    if !matches!(
        command.to_str(),
        Some("start" | "publish" | "handoff" | "select" | "clear")
    ) {
        return Err(Error::Usage);
    }
    let config = configuration(path)?;
    let bytes = bounded(input, silkboard_plan::MAX_SNAPSHOT_BYTES as u64)?;
    if command == "select" || command == "clear" {
        let choice: Selection =
            serde_json::from_slice(&bytes).map_err(|_| Error::Input)?;
        if command == "select" {
            return select(
                &config,
                choice.schema_version,
                choice.task_id.as_deref().ok_or(Error::Input)?,
                &choice.reporter,
            );
        }
        if choice.schema_version != 1
            || choice.task_id.is_some()
            || !crate::codex::context_scope(&config, &choice.reporter)
        {
            return Err(Error::Input);
        }
        crate::context::Context::lock(&config, &choice.reporter)?.clear()?;
        return Ok("Workflow context cleared locally");
    }
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
    let mut context = if crate::codex::context_scope(config, &snapshot.reporter)
    {
        Some(crate::context::Context::lock(config, &snapshot.reporter)?)
    } else {
        None
    };
    let mut selected = false;
    let snapshot = state::allocate(config, snapshot, start, |snapshot| {
        selected = crate::codex::context_enabled(config, &snapshot.reporter)
            && context
                .as_ref()
                .is_some_and(|c| start || c.matches(snapshot));
        if let Some(context) = &mut context {
            if selected {
                context.select(snapshot, false)?;
            } else if start {
                context.clear()?;
            }
        }
        Ok(())
    })?;
    transport::send_plan(config, &snapshot, &token)?;
    if selected && let Some(context) = &mut context {
        context.select(&snapshot, true)?;
    }
    Ok("Task plan acknowledged")
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexSelection {
    schema_version: u8,
    task_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    schema_version: u8,
    task_id: Option<String>,
    reporter: Reporter,
}

fn select(
    config: &crate::model::Config,
    schema: u8,
    id: &str,
    reporter: &Reporter,
) -> Result<&'static str, Error> {
    if schema != 1 || !crate::codex::context_enabled(config, reporter) {
        return Err(Error::Input);
    }
    let mut context = crate::context::Context::lock(config, reporter)?;
    let snapshot = owned(config, id, reporter)?;
    context.select(&snapshot, false)?;
    Ok("Workflow context selected locally; publish the plan to confirm")
}
pub(super) fn owned(
    config: &crate::model::Config,
    id: &str,
    reporter: &Reporter,
) -> Result<Snapshot, Error> {
    state::owned(config, id, reporter)
}
