//! Versioned, policy-bound persistent state; no filesystem side effects.

use crate::{Release, State, Version, profile::Policy};
use serde::{Deserialize, Serialize};

/// Current installed application adapter/layout contract.
pub const SILKBOARD_ADAPTER: &str = "silkboard-linux-v1";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    adapter: String,
    policy: Policy,
    state: State,
}

/// Encode state with the exact policy and installed adapter identity.
///
/// # Errors
/// Returns serialization errors; durable persistence belongs to the backend.
pub fn encode(
    state: &State,
    policy: &Policy,
    adapter: &str,
) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&Document {
        schema: 1,
        adapter: adapter.into(),
        policy: policy.clone(),
        state: state.clone(),
    })
    .map_err(|e| e.to_string())
}

/// Decode without changing bytes or discarding incompatible transactions.
/// Legacy state is accepted only for the canonical original Silkboard profile.
///
/// # Errors
/// Rejects unknown schemas/fields, context mismatch and invalid transactions.
pub fn decode(
    bytes: &[u8],
    policy: &Policy,
    adapter: &str,
) -> Result<State, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let state = if value.get("schema").is_some() {
        let document: Document =
            serde_json::from_value(value).map_err(|e| e.to_string())?;
        if document.schema != 1
            || document.policy != *policy
            || document.adapter != adapter
        {
            return Err("state schema, policy or adapter differs; operator migration required".into());
        }
        document.state
    } else {
        if *policy != Policy::silkboard() || adapter != SILKBOARD_ADAPTER {
            return Err("legacy state requires the canonical Silkboard policy and adapter".into());
        }
        serde_json::from_value(value).map_err(|e| e.to_string())?
    };
    for tag in &state.blocked {
        if Version::parse(tag).is_none() {
            return Err("invalid blocked tag".into());
        }
    }
    if let Some(pending) = &state.pending {
        for release in [&pending.previous, &pending.candidate] {
            Release::new(release.tag(), release.commit())?;
        }
        if !state.blocked.contains(pending.candidate.tag())
            || Version::parse(pending.previous.tag())
                >= Version::parse(pending.candidate.tag())
        {
            return Err("invalid recovery transaction".into());
        }
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn pending() -> State {
        State {
            blocked: ["v0.2.0".into()].into(),
            pending: Some(crate::Transaction {
                previous: Release::new("v0.1.1", &"a".repeat(40)).unwrap(),
                candidate: Release::new("v0.2.0", &"b".repeat(40)).unwrap(),
            }),
        }
    }

    #[test]
    fn legacy_and_versioned_transactions_preserve_recovery_identity() {
        let policy = Policy::silkboard();
        let state = pending();
        for bytes in [
            serde_json::to_vec(&state).unwrap(),
            encode(&state, &policy, SILKBOARD_ADAPTER).unwrap(),
        ] {
            assert_eq!(
                decode(&bytes, &policy, SILKBOARD_ADAPTER).unwrap(),
                state
            );
            assert!(decode(&bytes, &policy, "other-adapter").is_err());
            let mut changed = serde_json::to_value(&policy).unwrap();
            changed["workflow_id"] = json!(123);
            let changed =
                Policy::from_json(&serde_json::to_vec(&changed).unwrap())
                    .unwrap();
            assert!(decode(&bytes, &changed, SILKBOARD_ADAPTER).is_err());
        }
    }

    #[test]
    fn future_unknown_and_malformed_state_is_not_silently_reset() {
        let policy = Policy::silkboard();
        let original: Value = serde_json::from_slice(
            &encode(&pending(), &policy, SILKBOARD_ADAPTER).unwrap(),
        )
        .unwrap();
        for (key, value) in [("schema", json!(2)), ("extra", json!(true))] {
            let mut bad = original.clone();
            bad[key] = value;
            assert!(
                decode(
                    &serde_json::to_vec(&bad).unwrap(),
                    &policy,
                    SILKBOARD_ADAPTER
                )
                .is_err()
            );
        }
        for value in [
            json!({"blocked":["../bad"], "pending":null}),
            json!({"blocked":[], "pending":{"previous":{"tag":"v0.1.1","commit":"a".repeat(40)},"candidate":{"tag":"v0.2.0","commit":"b".repeat(40)}}}),
            json!({"blocked":["v0.2.0"], "pending":{"previous":{"tag":"v0.3.0","commit":"a".repeat(40)},"candidate":{"tag":"v0.2.0","commit":"b".repeat(40)}}}),
            json!({"blocked":["v0.2.0"], "pending":{"previous":{"tag":"v0.1.1","commit":"bad"},"candidate":{"tag":"v0.2.0","commit":"b".repeat(40)}}}),
            json!({"blocked":[],"pending":null,"extra":true}),
        ] {
            assert!(
                decode(
                    &serde_json::to_vec(&value).unwrap(),
                    &policy,
                    SILKBOARD_ADAPTER
                )
                .is_err()
            );
        }
        assert!(decode(b"bad-json", &policy, SILKBOARD_ADAPTER).is_err());
    }
}
