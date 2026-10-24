//! Bounded indexed projections for authenticated owner handlers.
use super::{PlanSummary, Snapshot, decode};
use crate::{
    owner::AuthError,
    progress::{ProgressStore, validate_project},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use silkboard_plan::{Counter, StepState, valid_task_id};

pub(crate) type Summary = PlanSummary;
#[derive(Serialize)]
pub(crate) struct Page<T> {
    items: Vec<T>,
    next_cursor: Option<String>,
}
fn page(
    mut items: Vec<Summary>,
    cursor: impl FnOnce(&Summary) -> String,
) -> Page<Summary> {
    let more = items.len() > 20;
    items.truncate(20);
    let next_cursor = items.last().filter(|_| more).map(cursor);
    Page { items, next_cursor }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Cursor {
    received_at: i64,
    project: String,
    task_id: String,
}
impl Cursor {
    pub(crate) fn parse(value: &str) -> Result<Self, AuthError> {
        if value.len() > 2048 {
            return Err(AuthError::Invalid);
        }
        let c: Self =
            serde_json::from_str(value).map_err(|_| AuthError::Invalid)?;
        if !(0..=8_640_000_000_000_000).contains(&c.received_at)
            || !valid_task_id(&c.task_id)
        {
            return Err(AuthError::Invalid);
        }
        validate_project(&c.project).map_err(|_| AuthError::Invalid)?;
        Ok(c)
    }
}
fn summary(value: &str) -> Result<Summary, AuthError> {
    if value.len() > 16_384 {
        return Err(AuthError::Unavailable);
    }
    let s: Summary =
        serde_json::from_str(value).map_err(|_| AuthError::Unavailable)?;
    if !valid_task_id(&s.task_id)
        || s.counts.total == 0
        || s.counts.total > 128
        || [s.counts.done, s.counts.active, s.counts.blocked]
            .iter()
            .any(|n| *n > s.counts.total)
        || s.counts.done + s.counts.active + s.counts.blocked > s.counts.total
        || s.active_step_ids.len() != s.counts.active
        || s.blocked_step_ids.len() != s.counts.blocked
    {
        return Err(AuthError::Unavailable);
    }
    Ok(s)
}
fn collect(
    statement: &mut rusqlite::Statement<'_>,
    parameters: impl rusqlite::Params,
) -> Result<Vec<Summary>, AuthError> {
    let strings = statement
        .query_map(parameters, |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    strings.iter().map(|v| summary(v)).collect()
}
pub(crate) fn list(
    store: &ProgressStore,
    before: Option<&Cursor>,
) -> Result<Page<Summary>, AuthError> {
    let conn = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    let items=collect(&mut conn.prepare("SELECT summary FROM task_plans WHERE (received_at,project,task_id)<(?1,?2,?3) ORDER BY received_at DESC,project DESC,task_id DESC LIMIT 21")?,params![before.map_or(i64::MAX,|c|c.received_at),before.map_or("",|c|c.project.as_str()),before.map_or("",|c|c.task_id.as_str())])?;
    Ok(page(items, |s| {
        serde_json::json!({"received_at":s.received_at,"project":s.project,"task_id":s.task_id}).to_string()
    }))
}
fn task(conn: &Connection, project: &str, id: &str) -> Result<(), AuthError> {
    validate_project(project).map_err(|_| AuthError::Invalid)?;
    if !valid_task_id(id) {
        return Err(AuthError::Invalid);
    }
    if !conn.query_row("SELECT EXISTS(SELECT 1 FROM task_plans WHERE project=?1 AND task_id=?2)",params![project,id],|r|r.get::<_,bool>(0))?{return Err(AuthError::NotFound);}
    Ok(())
}
#[derive(Serialize)]
pub(crate) struct Completion {
    step_id: String,
    sequence: Counter,
    received_at: i64,
}
#[derive(Serialize)]
pub(crate) struct Detail {
    snapshot: Snapshot,
    summary: Summary,
    prior_completions: Vec<Completion>,
}
pub(crate) fn current(
    store: &ProgressStore,
    project: &str,
    id: &str,
    sequence: Option<Counter>,
    revision: Option<Counter>,
) -> Result<Detail, AuthError> {
    let conn = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    task(&conn, project, id)?;
    if sequence.is_some() && revision.is_some() {
        return Err(AuthError::Invalid);
    }
    let sequence = if let Some(s) = sequence {
        s.get()
    } else if let Some(r) = revision {
        conn.query_row("SELECT latest_sequence FROM plan_revisions WHERE project=?1 AND task_id=?2 AND revision=?3",params![project,id,r.get()],|r|r.get(0)).optional()?.ok_or(AuthError::NotFound)?
    } else {
        conn.query_row("SELECT latest_sequence FROM task_plans WHERE project=?1 AND task_id=?2",params![project,id],|r|r.get(0))?
    };
    let row:Option<(String,String)>=conn.query_row("SELECT payload,summary FROM plan_snapshots WHERE project=?1 AND task_id=?2 AND sequence=?3",params![project,id,sequence],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    let (body, stored) = row.ok_or(AuthError::NotFound)?;
    let snapshot = decode(&body).map_err(|_| AuthError::Unavailable)?;
    let summary = summary(&stored)?;
    let mut prior_completions = Vec::new();
    let mut stmt=conn.prepare("SELECT h.sequence,p.received_at FROM plan_step_history h JOIN plan_snapshots p ON p.project=h.project AND p.task_id=h.task_id AND p.sequence=h.sequence WHERE h.project=?1 AND h.task_id=?2 AND h.step_id=?3 AND h.state='done' AND h.sequence<=?4 ORDER BY h.sequence DESC LIMIT 1")?;
    for s in snapshot.steps.iter().filter(|s| s.state != StepState::Done) {
        let old: Option<(i64, i64)> = stmt
            .query_row(params![project, id, s.id, sequence], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?;
        if let Some((seq, received_at)) = old {
            prior_completions.push(Completion {
                step_id: s.id.clone(),
                sequence: Counter::new(seq)
                    .map_err(|_| AuthError::Unavailable)?,
                received_at,
            });
        }
    }
    Ok(Detail {
        snapshot,
        summary,
        prior_completions,
    })
}
pub(crate) fn history(
    store: &ProgressStore,
    project: &str,
    id: &str,
    before: Option<Counter>,
    revisions: bool,
) -> Result<Page<Summary>, AuthError> {
    let conn = store.0.lock().map_err(|_| AuthError::Unavailable)?;
    task(&conn, project, id)?;
    let sql = if revisions {
        "SELECT summary FROM plan_revisions WHERE project=?1 AND task_id=?2 AND revision<=?3 ORDER BY revision DESC LIMIT 21"
    } else {
        "SELECT summary FROM plan_snapshots WHERE project=?1 AND task_id=?2 AND sequence<=?3 ORDER BY sequence DESC LIMIT 21"
    };
    let items = collect(
        &mut conn.prepare(sql)?,
        params![project, id, before.map_or(i64::MAX, |n| n.get() - 1)],
    )?;
    Ok(page(items, |s| {
        if revisions {
            s.plan_revision.to_string()
        } else {
            s.sequence.to_string()
        }
    }))
}
