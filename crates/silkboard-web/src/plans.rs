//! Project-scoped full plan snapshots and private owner projections.
mod http;
mod read;
pub(crate) mod schema;
mod store;

pub(crate) use http::routes;
pub(crate) use read::{Cursor, Detail, Page, Summary, current, history, list};
pub use silkboard_plan::{Acknowledgement, Snapshot};
pub use store::publish;

use crate::progress::ProgressError;
use serde::{Deserialize, Serialize};
use silkboard_plan::{Counter, Counts, Reporter, TaskState};

/// Private owner graph data; this is not exposed through publisher responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PlanSummary {
    project: String,
    task_id: String,
    sequence: Counter,
    plan_revision: Counter,
    writer_generation: Counter,
    title: String,
    reporter: Reporter,
    state: TaskState,
    counts: Counts,
    active_step_ids: Vec<String>,
    blocked_step_ids: Vec<String>,
    received_at: i64,
    history_incomplete: bool,
}
impl PlanSummary {
    fn new(
        project: &str,
        s: &Snapshot,
        received_at: i64,
        history_incomplete: bool,
    ) -> Self {
        Self {
            project: project.into(),
            task_id: s.task_id.clone(),
            sequence: s.sequence,
            plan_revision: s.plan_revision,
            writer_generation: s.writer_generation,
            title: s.title.clone(),
            reporter: s.reporter.clone(),
            state: s.overall_state(),
            counts: s.counts(),
            active_step_ids: s
                .steps
                .iter()
                .filter(|s| s.state == silkboard_plan::StepState::Active)
                .map(|s| s.id.clone())
                .collect(),
            blocked_step_ids: s
                .steps
                .iter()
                .filter(|s| s.state == silkboard_plan::StepState::Blocked)
                .map(|s| s.id.clone())
                .collect(),
            received_at,
            history_incomplete,
        }
    }
}
fn decode(payload: &str) -> Result<Snapshot, ProgressError> {
    if payload.len() > silkboard_plan::MAX_SNAPSHOT_BYTES {
        return Err(ProgressError::Unavailable);
    }
    serde_json::from_str::<Snapshot>(payload)
        .map_err(|_| ProgressError::Unavailable)?
        .normalized()
        .map_err(|_| ProgressError::Unavailable)
}
fn json<T: Serialize>(value: &T) -> Result<String, ProgressError> {
    serde_json::to_string(value).map_err(|_| ProgressError::Unavailable)
}
