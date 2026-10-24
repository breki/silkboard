//! Installed, best-effort progress and task-plan publishing without an outgoing queue.

mod codex;
mod model;
mod plans;
mod state;
mod transport;

use std::{ffi::OsString, io::Read, path::Path};

/// Fixed, secret-free diagnostics for the installed client.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Unsupported command or argument shape.
    Usage,
    /// Invalid, unknown, oversized or inconsistent report/configuration fields.
    Input,
    /// A private file/directory was inaccessible, unsafe or malformed.
    PrivateFile,
    /// Working-state persistence failed or incompatible state was found.
    State,
    /// Another process is changing this session or task state.
    Busy,
    /// Delivery did not complete within the network budget.
    Network,
    /// The publisher credential was not accepted.
    Unauthorized,
    /// Local writer binding or server ordering/identity conflicted.
    Conflict,
    /// The server returned another unsuccessful HTTP status.
    Rejected,
    /// The server did not return the exact bounded acknowledgement contract.
    Acknowledgement,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Usage => "usage: silkboard-publish {publish|codex-hook|codex-plan start|codex-plan publish|plan start|plan publish|plan handoff} --config PATH (JSON on stdin)",
            Self::Input => "invalid publisher input or configuration",
            Self::PrivateFile => "publisher requires readable private files and directories",
            Self::State => "publisher working state unavailable or incompatible",
            Self::Busy => "publisher session busy; report was not sent",
            Self::Network => "report delivery failed or timed out; no automatic retry",
            Self::Unauthorized => "publisher credential not accepted",
            Self::Conflict => "report conflict; check session or task state",
            Self::Rejected => "publisher report rejected by server",
            Self::Acknowledgement => "publisher acknowledgement invalid; delivery unconfirmed",
        })
    }
}
impl std::error::Error for Error {}

/// Run the CLI with explicit arguments and bounded stdin.
///
/// # Errors
/// Returns a fixed diagnostic on invalid input or unsuccessful delivery.
pub fn run(args: &[OsString], input: impl Read) -> Result<&'static str, Error> {
    match args {
        [licenses] if licenses == "--licenses" => {
            return Ok(include_str!("../THIRD_PARTY_NOTICES.txt"));
        }
        [help] if help == "--help" => {
            return Ok(
                "silkboard-publish publish --config PATH\nsilkboard-publish codex-hook --config PATH\nsilkboard-publish codex-plan {start|publish} --config PATH\nsilkboard-publish plan {start|publish|handoff} --config PATH\nRead one JSON progress/plan report, handoff or native Codex hook payload from stdin. codex-plan derives identity from operator Codex configuration and CODEX_THREAD_ID. One bounded attempt; no queue or service. codex-hook always returns {} with exit 0.",
            );
        }
        [version] if version == "--version" => {
            return Ok(concat!(
                "silkboard-publish ",
                env!("CARGO_PKG_VERSION")
            ));
        }
        [command, flag, path] if command == "publish" && flag == "--config" => {
            let config = configuration(Path::new(path))?;
            let report: model::Report =
                serde_json::from_slice(&model::bounded(input, 8192)?)
                    .map_err(|_| Error::Input)?;
            publish(&config, report)?;
        }
        [command, flag, path]
            if command == "codex-hook" && flag == "--config" =>
        {
            // Native async hooks must never turn delivery failure into a
            // continuation, context injection or permission decision.
            let _ = codex::run(Path::new(path), input);
            return Ok("{}");
        }
        [plan, command, flag, path] if plan == "plan" && flag == "--config" => {
            return plans::run(command, Path::new(path), input);
        }
        [plan, command, flag, path]
            if plan == "codex-plan" && flag == "--config" =>
        {
            return plans::run_codex(command, Path::new(path), input);
        }
        _ => return Err(Error::Usage),
    }
    Ok("Progress acknowledged")
}

fn configuration(path: &Path) -> Result<model::Config, Error> {
    let config: model::Config =
        serde_json::from_slice(&state::read_private(path, 16384)?)
            .map_err(|_| Error::Input)?;
    config.validate()?;
    Ok(config)
}

fn publish(config: &model::Config, report: model::Report) -> Result<(), Error> {
    report.validate()?;
    let token = token(config)?;
    let event = state::allocate(config, report)?;
    transport::send(config, &event, &token)
}

fn token(config: &model::Config) -> Result<String, Error> {
    let token_bytes = state::read_private(&config.token_file, 66)?;
    let token =
        std::str::from_utf8(&token_bytes).map_err(|_| Error::PrivateFile)?;
    let token = token.strip_suffix('\n').unwrap_or(token);
    if token.len() != 64
        || !token
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(Error::PrivateFile);
    }
    Ok(token.to_owned())
}
