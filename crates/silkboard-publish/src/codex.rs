//! Narrow native hook adapter: no transcript reads or Codex control output.
use crate::{
    Error,
    model::{Report, bounded},
};
use serde::Deserialize;
use std::{
    io::Read,
    path::{Component, Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Settings {
    workspace: PathBuf,
    hostname: String,
    session_name: Option<String>,
    #[serde(default)]
    plan_reporting: bool,
}

// Deliberately ignore unknown native fields. Only these fields may enter a
// report; paths to transcripts and tool/prompt content are never inspected.
#[derive(Deserialize)]
struct Payload {
    hook_event_name: String,
    turn_id: Option<String>,
    session_id: String,
    cwd: PathBuf,
    agent_id: Option<String>,
    agent_type: Option<String>,
    #[serde(default)]
    stop_hook_active: bool,
    last_assistant_message: Option<String>,
}

pub(super) fn context_enabled(
    config: &crate::model::Config,
    reporter: &silkboard_plan::Reporter,
) -> bool {
    context_scope(config, reporter)
        && config.codex.as_ref().is_some_and(|s| s.plan_reporting)
}

pub(super) fn context_scope(
    config: &crate::model::Config,
    reporter: &silkboard_plan::Reporter,
) -> bool {
    config.codex.as_ref().is_some_and(|s| {
        reporter.tool == "codex" && reporter.hostname == s.hostname
    })
}

pub(super) fn plan_reporter(
    config: &crate::model::Config,
) -> Result<silkboard_plan::Reporter, Error> {
    if !config.codex.as_ref().is_some_and(|s| s.plan_reporting) {
        return Err(Error::Input);
    }
    context_reporter(config)
}
pub(super) fn context_reporter(
    config: &crate::model::Config,
) -> Result<silkboard_plan::Reporter, Error> {
    let settings = config.codex.as_ref().ok_or(Error::Input)?;
    let cwd = std::env::current_dir().map_err(|_| Error::Input)?;
    if !clean_absolute(&settings.workspace)
        || !clean_absolute(&cwd)
        || !cwd.starts_with(&settings.workspace)
    {
        return Err(Error::Input);
    }
    Ok(silkboard_plan::Reporter {
        hostname: settings.hostname.clone(),
        tool: "codex".into(),
        session_id: std::env::var("CODEX_THREAD_ID")
            .map_err(|_| Error::Input)?,
        subagent_id: None,
        parent_session_id: None,
        session_name: settings.session_name.clone(),
    })
}

pub(super) fn run(
    path: &Path,
    input: impl Read,
    capture: bool,
) -> Result<(), Error> {
    let config = crate::configuration(path)?;
    let settings = config.codex.as_ref().ok_or(Error::Input)?;
    let payload: Payload = serde_json::from_slice(&bounded(input, 65536)?)
        .map_err(|_| Error::Input)?;
    if !clean_absolute(&settings.workspace)
        || !clean_absolute(&payload.cwd)
        || !payload.cwd.starts_with(&settings.workspace)
        || payload.stop_hook_active
    {
        return Err(Error::Input);
    }
    let (child, fallback) = match payload.hook_event_name.as_str() {
        "Stop" if payload.agent_id.is_none() => (false, "Turn completed."),
        "SubagentStart" => (true, "Subagent started."),
        "SubagentStop" => (true, "Subagent completed."),
        _ => return Err(Error::Input),
    };
    if child && payload.agent_id.is_none() {
        return Err(Error::Input);
    }
    let message = if payload.hook_event_name == "SubagentStart" {
        fallback.to_owned()
    } else {
        payload
            .last_assistant_message
            .as_deref()
            .and_then(first_sentence)
            .unwrap_or_else(|| fallback.to_owned())
    };
    let mut report = Report {
        task_id: None,
        description: None,
        hostname: settings.hostname.clone(),
        tool: "codex".to_owned(),
        parent_session_id: child.then(|| payload.session_id.clone()),
        session_id: payload.session_id,
        subagent_id: payload.agent_id,
        session_name: if child {
            payload.agent_type
        } else {
            settings.session_name.clone()
        },
        message,
    };
    report.validate()?;
    let reporter = report.reporter();
    if capture {
        if context_enabled(&config, &reporter) {
            crate::hook_context::capture(
                &config,
                &reporter,
                &payload.hook_event_name,
                payload.turn_id.as_deref().ok_or(Error::Input)?,
            )?;
        }
        return Ok(());
    }
    if context_enabled(&config, &reporter) {
        report.task_id = payload.turn_id.as_deref().and_then(|turn| {
            crate::hook_context::resolve(
                &config,
                &reporter,
                &payload.hook_event_name,
                turn,
            )
        });
    }
    let (event, token) = crate::prepare(&config, report)?;
    crate::transport::send(&config, &event, &token)
}

fn clean_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|c| matches!(c, Component::RootDir | Component::Normal(_)))
}

fn first_sentence(text: &str) -> Option<String> {
    let line = text.lines().find(|line| !line.trim().is_empty())?.trim();
    // A code block is not a progress summary. Do not mine later lines for one.
    if line.starts_with("```") || line.starts_with("~~~") {
        return None;
    }
    let line = line.trim_start_matches(['*', '#', '`', '_']).trim();
    let mut result = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_control()
            || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        {
            continue;
        }
        if result.len() + c.len_utf8() > 240 {
            break;
        }
        result.push(c);
        if matches!(c, '.' | '!' | '?')
            && chars
                .clone()
                .find(|next| !matches!(next, '*' | '_' | '`'))
                .is_none_or(char::is_whitespace)
        {
            break;
        }
    }
    let result = result.trim().trim_matches(['*', '#', '`', '_']).trim();
    (!result.is_empty()).then(|| result.to_owned())
}
