//! Operator command parsing and reports, reusable without process I/O.
//!
//! Callers must serialize operations using the deployment lock. Installed
//! entry points use the existing root-owned credential/lock wrapper.

use crate::{
    Backend, State, Version,
    outcome::{Admission, Error, Outcome, Stage},
    profile::Policy,
};
use serde::Serialize;

/// Compatibility and machine-report command syntax.
pub const USAGE: &str = "usage: silkboard-update [--json] [--profile silkboard] poll | status | retry TAG | inspect TAG";

/// A parsed operation; tags are validated before any backend call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Poll once or recover interrupted work.
    Poll,
    /// Inspect local state without contacting GitHub.
    Status,
    /// Inspect the named tag without a build or activation.
    Inspect(String),
    /// Explicitly clear a failed-tag block.
    Retry(String),
    /// Display syntax without loading configuration.
    Help,
}

/// A parsed request for the registered adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Emit machine-readable reports instead of legacy terminal text.
    pub json: bool,
    /// Selected operation.
    pub operation: Operation,
}

/// Parse only registered profiles; arbitrary configuration paths are forbidden.
///
/// # Errors
/// Rejects unknown profiles, malformed tags and unsupported command syntax.
pub fn parse(arguments: &[String]) -> Result<Request, Error> {
    let mut arguments = arguments.iter().map(String::as_str).peekable();
    let json = if arguments.peek() == Some(&"--json") {
        arguments.next();
        true
    } else {
        false
    };
    if arguments.peek() == Some(&"--profile") {
        arguments.next();
        if arguments.next() != Some("silkboard") {
            return Err(Error::new(
                Stage::Configuration,
                "application profile is not registered",
            ));
        }
    }
    let arguments: Vec<_> = arguments.collect();
    let operation = match arguments.as_slice() {
        ["poll"] => Operation::Poll,
        ["status"] => Operation::Status,
        ["--help" | "-h"] => Operation::Help,
        ["inspect", tag] if Version::parse(tag).is_some() => {
            Operation::Inspect((*tag).into())
        }
        ["retry", tag] if Version::parse(tag).is_some() => {
            Operation::Retry((*tag).into())
        }
        _ => return Err(Error::new(Stage::Usage, USAGE)),
    };
    Ok(Request { json, operation })
}

/// A structured, versioned command result. No credentials are included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Report {
    /// Completed one poll/recovery operation.
    Polled {
        /// State-machine result.
        result: Outcome,
    },
    /// Read local deployment state successfully.
    Status {
        /// Installed application/layout contract.
        adapter: String,
        /// Independently installed admission policy.
        policy: Policy,
        /// Current pending transaction and failed-tag blocks.
        state: State,
    },
    /// Inspected eligibility without building or activating.
    Inspected {
        /// Canonical requested tag.
        tag: String,
        /// Typed eligibility, including waiting/rejection.
        admission: Admission,
    },
    /// Cleared the named block; deployment remains subject to admission.
    Retried {
        /// Canonical retried tag.
        tag: String,
    },
    /// Syntax only; no configuration or backend access.
    Help {
        /// Compatibility command syntax.
        usage: String,
    },
}

/// Execute a parsed command against a trusted, serialized backend.
///
/// # Errors
/// Reports boundary failures without treating unavailable evidence as success.
pub fn execute(
    operation: &Operation,
    backend: &mut impl Backend,
    policy: &Policy,
    adapter: &str,
) -> Result<Report, Error> {
    if let Operation::Inspect(tag) | Operation::Retry(tag) = operation
        && Version::parse(tag).is_none()
    {
        return Err(Error::new(Stage::Usage, "invalid stable tag"));
    }
    let result = match operation {
        Operation::Poll => Report::Polled {
            result: crate::poll(backend)?,
        },
        Operation::Status => Report::Status {
            adapter: adapter.into(),
            policy: policy.clone(),
            state: backend.load().map_err(|e| Error::new(Stage::State, e))?,
        },
        Operation::Inspect(tag) => {
            backend.refresh().map_err(|e| Error::new(Stage::Fetch, e))?;
            Report::Inspected {
                tag: tag.clone(),
                admission: backend
                    .inspect(tag)
                    .map_err(|e| Error::new(Stage::Admission, e))?,
            }
        }
        Operation::Retry(tag) => {
            crate::retry(backend, tag)?;
            Report::Retried { tag: tag.clone() }
        }
        Operation::Help => Report::Help {
            usage: USAGE.into(),
        },
    };
    Ok(result)
}

/// Render the existing human interface; ineligible inspection remains a failure.
///
/// # Errors
/// Rejects ineligible inspection and reports serialization failure.
pub fn human(report: &Report) -> Result<String, Error> {
    Ok(match report {
        Report::Polled { result } => match result {
            Outcome::Unchanged { current, .. } => {
                format!("No newer eligible release; current {}", current.tag())
            }
            Outcome::Activated { current, .. } => {
                format!("Activated {} ({})", current.tag(), current.commit())
            }
            Outcome::Recovered { current, .. } => {
                format!("Restored {} after incomplete upgrade", current.tag())
            }
        },
        Report::Status { state, .. } => serde_json::to_string_pretty(state)
            .map_err(|e| Error::new(Stage::State, e.to_string()))?,
        Report::Inspected {
            admission: Admission::Eligible { release },
            ..
        } => format!("Eligible {} ({})", release.tag(), release.commit()),
        Report::Inspected { .. } => {
            return Err(Error::new(
                Stage::Admission,
                "release is not eligible",
            ));
        }
        Report::Retried { tag } => {
            format!("Retry enabled for {tag}; admission remains required")
        }
        Report::Help { usage } => usage.clone(),
    })
}

/// Serialize a versioned machine report, reserving stdout for this document.
///
/// # Errors
/// Returns report serialization errors.
pub fn machine(report: &Report) -> Result<String, Error> {
    #[derive(Serialize)]
    struct Envelope<'a> {
        schema: u32,
        #[serde(flatten)]
        report: &'a Report,
    }
    serde_json::to_string(&Envelope { schema: 1, report })
        .map_err(|e| Error::new(Stage::State, e.to_string()))
}
