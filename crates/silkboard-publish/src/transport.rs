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
    let body = serde_json::to_vec(event).map_err(|_| Error::Input)?;
    let bytes = post(config, "progress", body, token)?;
    let ack: Acknowledgement =
        serde_json::from_slice(&bytes).map_err(|_| Error::Acknowledgement)?;
    if ack.event_id != event.event_id || ack.received_at < 0 {
        return Err(Error::Acknowledgement);
    }
    Ok(())
}

fn post(
    config: &Config,
    route: &str,
    body: Vec<u8>,
    token: &str,
) -> Result<Vec<u8>, Error> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(3)))
        .max_redirects(0)
        .max_response_header_size(8192)
        .proxy(None)
        .http_status_as_error(false)
        .build()
        .new_agent();
    let mut response = agent
        .post(format!(
            "{}/api/projects/{}/{route}",
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
    response
        .body_mut()
        .with_config()
        .limit(1024)
        .read_to_vec()
        .map_err(|_| Error::Acknowledgement)
}

pub(super) fn send_plan(
    config: &Config,
    snapshot: &silkboard_plan::Snapshot,
    token: &str,
) -> Result<(), Error> {
    let body = serde_json::to_vec(snapshot).map_err(|_| Error::Input)?;
    let bytes = post(config, "plans", body, token)?;
    let ack: silkboard_plan::Acknowledgement =
        serde_json::from_slice(&bytes).map_err(|_| Error::Acknowledgement)?;
    if ack.task_id != snapshot.task_id
        || ack.sequence != snapshot.sequence
        || ack.received_at < 0
    {
        return Err(Error::Acknowledgement);
    }
    Ok(())
}
