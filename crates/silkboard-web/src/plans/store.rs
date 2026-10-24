//! Authenticated, transactional ordering and immutable received history.
use super::{Acknowledgement, PlanSummary, Snapshot, decode, json};
use crate::{
    progress::{ProgressError, ProgressStore, validate_project},
    secrets::token_hash,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use silkboard_plan::StepState;

pub(crate) fn authenticate(
    conn: &Connection,
    project: &str,
    token: &str,
) -> Result<(), ProgressError> {
    validate_project(project)?;
    if !conn.query_row("SELECT EXISTS(SELECT 1 FROM owner)", [], |r| {
        r.get::<_, bool>(0)
    })? {
        return Err(ProgressError::Unavailable);
    }
    if token.len()!=64 || !conn.query_row("SELECT EXISTS(SELECT 1 FROM publishers WHERE project=?1 AND token_hash=?2 AND revoked=0)",params![project,token_hash(token)],|r|r.get::<_,bool>(0))?{return Err(ProgressError::Unauthorized);}
    Ok(())
}

/// Authenticate and atomically accept a full plan or acknowledge an exact retry.
///
/// # Errors
/// Rejects unauthorized publishers, invalid plans, stale/conflicting reports
/// and unavailable storage. No response contains the current plan.
pub fn publish(
    store: &ProgressStore,
    project: &str,
    token: &str,
    input: Snapshot,
) -> Result<Acknowledgement, ProgressError> {
    let mut conn = store.0.lock().map_err(|_| ProgressError::Unavailable)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    authenticate(&tx, project, token)?;
    let s = input.normalized().map_err(|_| ProgressError::Invalid)?;
    let payload = json(&s)?;
    let old:Option<(String,i64)>=tx.query_row("SELECT payload,received_at FROM plan_snapshots WHERE project=?1 AND task_id=?2 AND sequence=?3",params![project,s.task_id,s.sequence.get()],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    if let Some((old, received_at)) = old {
        if old != payload {
            return Err(ProgressError::Conflict);
        }
        return Ok(Acknowledgement {
            task_id: s.task_id,
            sequence: s.sequence,
            received_at,
        });
    }
    let previous:Option<(String,String)>=tx.query_row("SELECT p.payload,p.summary FROM plan_snapshots p JOIN task_plans t ON p.project=t.project AND p.task_id=t.task_id AND p.sequence=t.latest_sequence WHERE t.project=?1 AND t.task_id=?2",params![project,s.task_id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    let mut incomplete = s.sequence.get() > 1
        || s.plan_revision.get() > 1
        || s.writer_generation.get() > 1;
    let previous = if let Some((body, summary)) = previous {
        let old = decode(&body)?;
        if s.sequence <= old.sequence
            || s.writer_generation < old.writer_generation
            || s.plan_revision < old.plan_revision
            || s.writer_generation == old.writer_generation
                && !s.reporter.same_identity(&old.reporter)
            || s.plan_revision == old.plan_revision && !s.same_definition(&old)
        {
            return Err(ProgressError::Conflict);
        }
        let summary: PlanSummary = serde_json::from_str(&summary)
            .map_err(|_| ProgressError::Unavailable)?;
        incomplete = summary.history_incomplete
            || s.sequence.get() > old.sequence.get().saturating_add(1)
            || s.plan_revision.get()
                > old.plan_revision.get().saturating_add(1)
            || s.writer_generation.get()
                > old.writer_generation.get().saturating_add(1);
        Some(old)
    } else {
        None
    };
    let received_at: i64 = tx.query_row(
        "SELECT CAST(unixepoch('subsec')*1000 AS INTEGER)",
        [],
        |r| r.get(0),
    )?;
    let summary =
        json(&PlanSummary::new(project, &s, received_at, incomplete))?;
    tx.execute("INSERT INTO task_plans(project,task_id,latest_sequence,received_at,summary) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(project,task_id) DO UPDATE SET latest_sequence=excluded.latest_sequence,received_at=excluded.received_at,summary=excluded.summary",params![project,s.task_id,s.sequence.get(),received_at,summary])?;
    tx.execute("INSERT INTO plan_snapshots(project,task_id,sequence,plan_revision,writer_generation,payload,summary,received_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",params![project,s.task_id,s.sequence.get(),s.plan_revision.get(),s.writer_generation.get(),payload,summary,received_at])?;
    tx.execute("INSERT INTO plan_revisions(project,task_id,revision,latest_sequence,summary) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(project,task_id,revision) DO UPDATE SET latest_sequence=excluded.latest_sequence,summary=excluded.summary",params![project,s.task_id,s.plan_revision.get(),s.sequence.get(),summary])?;
    changes(&tx, project, &s, previous.as_ref())?;
    tx.commit()?;
    Ok(Acknowledgement {
        task_id: s.task_id,
        sequence: s.sequence,
        received_at,
    })
}
fn state(s: StepState) -> &'static str {
    match s {
        StepState::Pending => "pending",
        StepState::Active => "active",
        StepState::Done => "done",
        StepState::Blocked => "blocked",
    }
}
fn changes(
    conn: &Connection,
    project: &str,
    s: &Snapshot,
    old: Option<&Snapshot>,
) -> Result<(), ProgressError> {
    for step in &s.steps {
        let before = old.and_then(|p| p.steps.iter().find(|x| x.id == step.id));
        if before.is_none_or(|p| p.state != step.state) {
            conn.execute("INSERT INTO plan_step_history(project,task_id,step_id,sequence,state) VALUES(?1,?2,?3,?4,?5)",params![project,s.task_id,step.id,s.sequence.get(),state(step.state)])?;
        }
    }
    if let Some(old) = old {
        for step in &old.steps {
            if !s.steps.iter().any(|x| x.id == step.id) {
                conn.execute("INSERT INTO plan_step_history(project,task_id,step_id,sequence,state) VALUES(?1,?2,?3,?4,NULL)",params![project,s.task_id,step.id,s.sequence.get()])?;
            }
        }
    }
    Ok(())
}
