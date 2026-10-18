//! Installed, best-effort progress publishing without an outgoing queue.

mod model;
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
    /// Counter persistence failed or incompatible state was found.
    State,
    /// Another process is allocating this session's sequence.
    Busy,
    /// Delivery did not complete within the network budget.
    Network,
    /// The publisher credential was not accepted.
    Unauthorized,
    /// The server rejected a conflicting event/session identity.
    Conflict,
    /// The server returned another unsuccessful HTTP status.
    Rejected,
    /// The server did not return the exact bounded acknowledgement contract.
    Acknowledgement,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Usage => "usage: silkboard-publish publish --config PATH (JSON report on stdin)",
            Self::Input => "invalid publisher input or configuration",
            Self::PrivateFile => "publisher requires readable private files and directories",
            Self::State => "publisher counter state unavailable or incompatible",
            Self::Busy => "publisher session busy; report was not sent",
            Self::Network => "progress delivery failed or timed out; no automatic retry",
            Self::Unauthorized => "publisher credential not accepted",
            Self::Conflict => "progress identity conflict; check session configuration",
            Self::Rejected => "progress report rejected by server",
            Self::Acknowledgement => "progress acknowledgement invalid; delivery unconfirmed",
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
                "silkboard-publish publish --config PATH\nRead one JSON progress report from stdin. One bounded attempt; no queue or service.",
            );
        }
        [version] if version == "--version" => {
            return Ok(concat!(
                "silkboard-publish ",
                env!("CARGO_PKG_VERSION")
            ));
        }
        [command, flag, path] if command == "publish" && flag == "--config" => {
            let config: model::Config = serde_json::from_slice(
                &state::read_private(Path::new(path), 16384)?,
            )
            .map_err(|_| Error::Input)?;
            config.validate()?;
            let report: model::Report =
                serde_json::from_slice(&model::bounded(input, 8192)?)
                    .map_err(|_| Error::Input)?;
            report.validate()?;
            let token_bytes = state::read_private(&config.token_file, 66)?;
            let token = std::str::from_utf8(&token_bytes)
                .map_err(|_| Error::PrivateFile)?;
            let token = token.strip_suffix('\n').unwrap_or(token);
            if token.len() != 64
                || !token
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                return Err(Error::PrivateFile);
            }
            let event = state::allocate(&config, report)?;
            transport::send(&config, &event, token)?;
        }
        _ => return Err(Error::Usage),
    }
    Ok("Progress acknowledged")
}
