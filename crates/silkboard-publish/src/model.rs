//! Bounded report/configuration inputs matching the ingestion contract.

use crate::Error;
use serde::{Deserialize, Serialize};
use std::{io::Read, path::PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Config {
    pub endpoint: String,
    pub project: String,
    pub token_file: PathBuf,
    pub state_dir: PathBuf,
}

impl Config {
    pub fn validate(&self) -> Result<(), Error> {
        let uri: ureq::http::Uri =
            self.endpoint.parse().map_err(|_| Error::Input)?;
        let authority = uri.authority().ok_or(Error::Input)?;
        let host = uri.host().ok_or(Error::Input)?;
        let scheme = uri.scheme_str().ok_or(Error::Input)?;
        if authority.as_str().contains('@')
            || host != host.to_ascii_lowercase()
            || host.is_empty()
            || uri.port_u16() == Some(0)
            || self.endpoint != format!("{scheme}://{authority}")
            || !(scheme == "https"
                || (scheme == "http" && matches!(host, "127.0.0.1" | "[::1]")))
            || self.project.is_empty()
            || self.project.len() > 64
            || !self.project.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'
            })
            || !self.token_file.is_absolute()
            || !self.state_dir.is_absolute()
        {
            return Err(Error::Input);
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Report {
    pub hostname: String,
    pub tool: String,
    pub session_id: String,
    pub subagent_id: Option<String>,
    pub parent_session_id: Option<String>,
    pub session_name: Option<String>,
    pub message: String,
}

impl Report {
    pub fn validate(&self) -> Result<(), Error> {
        for s in [&self.hostname, &self.tool, &self.session_id]
            .into_iter()
            .chain(self.subagent_id.as_ref())
            .chain(self.parent_session_id.as_ref())
        {
            if s.is_empty()
                || s.len() > 256
                || !s.bytes().all(|b| b.is_ascii_graphic())
            {
                return Err(Error::Input);
            }
        }
        if self.parent_session_id.is_some() && self.subagent_id.is_none() {
            return Err(Error::Input);
        }
        for (s, limit) in std::iter::once((&self.message, 2000))
            .chain(self.session_name.as_ref().map(|s| (s, 120)))
        {
            if s.trim().is_empty()
                || s.len() > limit
                || s.chars().any(char::is_control)
            {
                return Err(Error::Input);
            }
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub(super) struct ProgressEvent {
    pub event_id: String,
    pub sequence: i64,
    #[serde(flatten)]
    pub report: Report,
}

pub(super) fn bounded(input: impl Read, limit: u64) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    input
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Input)?;
    if bytes.len() as u64 > limit {
        return Err(Error::Input);
    }
    Ok(bytes)
}
