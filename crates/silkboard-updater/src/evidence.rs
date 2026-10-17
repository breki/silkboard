//! GitHub release-suite identity and exact-revision evidence.

use crate::{Release, profile::Policy};
use serde_json::Value;

/// Return the newest matching successful tag-push run, rejecting foreign evidence.
#[must_use]
pub fn passing_run_for(
    response: &Value,
    release: &Release,
    policy: &Policy,
) -> Option<u64> {
    let runs = response["workflow_runs"].as_array()?;
    let run = runs.iter().max_by_key(|r| r["id"].as_u64().unwrap_or(0))?;
    valid_run_for(run, release, policy)
        .then(|| run["id"].as_u64())
        .flatten()
}

/// Verify the fixed registered workflow, independent of repository content.
#[must_use]
pub fn valid_workflow_for(workflow: &Value, policy: &Policy) -> bool {
    workflow["id"] == policy.workflow_id()
        && workflow["path"] == policy.workflow_path()
        && workflow["state"] == "active"
}

/// Require successful completion of the exact repository, tag and workflow.
#[must_use]
pub fn valid_run_for(run: &Value, release: &Release, policy: &Policy) -> bool {
    run["id"].as_u64().is_some_and(|id| id > 0)
        && run["workflow_id"] == policy.workflow_id()
        && run["path"] == policy.workflow_path()
        && run["event"] == "push"
        && run["head_branch"] == release.tag()
        && run["head_sha"] == release.commit()
        && run["status"] == "completed"
        && run["conclusion"] == "success"
        && run["run_attempt"].as_u64().is_some_and(|n| n > 0)
        && ["repository", "head_repository"].iter().all(|key| {
            run[key]["id"] == policy.repository_id()
                && run[key]["full_name"] == policy.repository()
        })
        && run["referenced_workflows"].as_array().is_some_and(|refs| {
            refs.len() == 1
                && refs[0]["ref"] == format!("refs/tags/{}", release.tag())
                && refs[0]["sha"] == release.commit()
                && refs[0]["path"]
                    == format!(
                        "{}/{}@{}",
                        policy.repository(),
                        policy.ci_path(),
                        release.commit()
                    )
        })
}

/// Require the complete configured gate set, rejecting skipped/duplicate/extra jobs.
#[must_use]
pub fn passing_jobs_for(response: &Value, policy: &Policy) -> bool {
    let Some(jobs) = response["jobs"].as_array() else {
        return false;
    };
    let required = policy.required_jobs();
    response["total_count"] == required.len()
        && jobs.len() == required.len()
        && required.iter().all(|name| {
            jobs.iter()
                .filter(|job| {
                    job["name"] == *name
                        && job["status"] == "completed"
                        && job["conclusion"] == "success"
                })
                .count()
                == 1
        })
}

/// Apply the canonical Silkboard policy (compatibility API).
#[must_use]
pub fn passing_run(response: &Value, release: &Release) -> Option<u64> {
    passing_run_for(response, release, &Policy::silkboard())
}

/// Check Silkboard’s registered release workflow.
#[must_use]
pub fn valid_workflow(workflow: &Value) -> bool {
    valid_workflow_for(workflow, &Policy::silkboard())
}

/// Check Silkboard’s exact tag-triggered successful run.
#[must_use]
pub fn valid_run(run: &Value, release: &Release) -> bool {
    valid_run_for(run, release, &Policy::silkboard())
}

/// Check Silkboard’s complete successful job set.
#[must_use]
pub fn passing_jobs(response: &Value) -> bool {
    passing_jobs_for(response, &Policy::silkboard())
}
