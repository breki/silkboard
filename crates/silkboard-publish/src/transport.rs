//! One synchronous authenticated POST; no redirects, proxies or response actions.

use crate::{
    Error,
    model::{Config, ProgressEvent},
};
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Acknowledgement {
    event_id: String,
    received_at: i64,
}

pub(super) fn send(
    config: &Config,
    event: &ProgressEvent,
    token: &str,
) -> Result<(), Error> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(3)))
        .max_redirects(0)
        .max_response_header_size(8192)
        .proxy(None)
        .http_status_as_error(false)
        .build()
        .new_agent();
    let body = serde_json::to_vec(event).map_err(|_| Error::Input)?;
    let mut response = agent
        .post(format!(
            "{}/api/projects/{}/progress",
            config.endpoint, config.project
        ))
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .send(body)
        .map_err(|_| Error::Network)?;
    match response.status().as_u16() {
        200 => {}
        401 => return Err(Error::Unauthorized),
        409 => return Err(Error::Conflict),
        _ => return Err(Error::Rejected),
    }
    let bytes = response
        .body_mut()
        .with_config()
        .limit(1024)
        .read_to_vec()
        .map_err(|_| Error::Acknowledgement)?;
    let ack: Acknowledgement =
        serde_json::from_slice(&bytes).map_err(|_| Error::Acknowledgement)?;
    if ack.event_id != event.event_id || ack.received_at < 0 {
        return Err(Error::Acknowledgement);
    }
    Ok(())
}
