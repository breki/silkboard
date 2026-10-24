//! Boundary-owned validation with bounded, iterative graph traversal.
use crate::{InvalidPlan, MAX_EDGES, MAX_STEPS, Snapshot, StepState};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) fn uuid(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 36
        && b[14] == b'4'
        && matches!(b[19], b'8' | b'9' | b'a' | b'b')
        && b.iter().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c == b'-'
            } else {
                c.is_ascii_digit() || (b'a'..=b'f').contains(c)
            }
        })
}
pub(super) fn step_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value.bytes().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || b"_-".contains(&c)
        })
}
fn text(value: &str, limit: usize, multiline: bool) -> bool {
    !value.trim().is_empty()
        && value.len() <= limit
        && !value.chars().any(|c| {
            c.is_control() && !(multiline && matches!(c, '\n' | '\r' | '\t'))
        })
}
fn optional(value: Option<&str>, limit: usize, multiline: bool) -> bool {
    value.is_none_or(|v| text(v, limit, multiline))
}
fn identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|b| b.is_ascii_graphic())
}
pub(super) fn snapshot(s: &Snapshot) -> Result<(), InvalidPlan> {
    let r = &s.reporter;
    if s.schema_version != 1
        || !uuid(&s.task_id)
        || !text(&s.title, 160, false)
        || !optional(s.description.as_deref(), 2000, true)
        || s.steps.is_empty()
        || s.steps.len() > MAX_STEPS
        || ![&r.hostname, &r.tool, &r.session_id]
            .into_iter()
            .chain(r.subagent_id.as_ref())
            .chain(r.parent_session_id.as_ref())
            .all(|v| identity(v))
        || !optional(r.session_name.as_deref(), 120, false)
        || (r.parent_session_id.is_some() && r.subagent_id.is_none())
    {
        return Err(InvalidPlan);
    }
    if let Some(issue) = &s.github_issue {
        let parts: Vec<_> = issue.repository.split('/').collect();
        if issue.repository.len() > 100
            || parts.len() != 2
            || parts.iter().any(|p| {
                p.is_empty()
                    || matches!(*p, "." | "..")
                    || !p.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || b"._-".contains(&b)
                    })
            })
        {
            return Err(InvalidPlan);
        }
    }
    let mut ids = BTreeMap::new();
    for (i, step) in s.steps.iter().enumerate() {
        if !step_id(&step.id)
            || !text(&step.title, 160, false)
            || !optional(step.description.as_deref(), 2000, true)
            || !optional(step.report.as_deref(), 2000, true)
            || step.depends_on.len() > 32
            || ids.insert(step.id.as_str(), i).is_some()
        {
            return Err(InvalidPlan);
        }
        if step.state == StepState::Blocked {
            if !step
                .blocked_reason
                .as_ref()
                .is_some_and(|s| text(s, 1000, true))
            {
                return Err(InvalidPlan);
            }
        } else if step.blocked_reason.is_some() {
            return Err(InvalidPlan);
        }
    }
    let mut outgoing = vec![Vec::new(); s.steps.len()];
    let mut degrees = vec![0; s.steps.len()];
    let mut edges = 0;
    for (i, step) in s.steps.iter().enumerate() {
        let mut seen = BTreeSet::new();
        for id in &step.depends_on {
            let parent = *ids.get(id.as_str()).ok_or(InvalidPlan)?;
            if parent == i
                || !seen.insert(id)
                || matches!(step.state, StepState::Active | StepState::Done)
                    && s.steps[parent].state != StepState::Done
            {
                return Err(InvalidPlan);
            }
            outgoing[parent].push(i);
            degrees[i] += 1;
            edges += 1;
        }
    }
    if edges > MAX_EDGES {
        return Err(InvalidPlan);
    }
    let mut queue: VecDeque<_> = degrees
        .iter()
        .enumerate()
        .filter_map(|(i, n)| (*n == 0).then_some(i))
        .collect();
    let mut visited = 0;
    while let Some(i) = queue.pop_front() {
        visited += 1;
        for &child in &outgoing[i] {
            degrees[child] -= 1;
            if degrees[child] == 0 {
                queue.push_back(child);
            }
        }
    }
    if visited != s.steps.len() {
        return Err(InvalidPlan);
    }
    Ok(())
}
