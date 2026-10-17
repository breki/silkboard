//! GitHub release-suite identity and exact-revision evidence.

use crate::Release;
use serde_json::Value;

/// Return the newest matching successful tag-push run, rejecting foreign evidence.
#[must_use]
pub fn passing_run(response: &Value, release: &Release) -> Option<u64> {
    let runs = response["workflow_runs"].as_array()?;
    let run = runs.iter().max_by_key(|r| r["id"].as_u64().unwrap_or(0))?;
    valid_run(run, release)
        .then(|| run["id"].as_u64())
        .flatten()
}

/// Verify the fixed registered workflow, independent of repository content.
#[must_use]
pub fn valid_workflow(workflow: &Value) -> bool {
    workflow["id"] == 387_356_305_u64
        && workflow["path"] == ".github/workflows/release.yml"
        && workflow["state"] == "active"
}

/// Require successful completion of the exact repository, tag and workflow.
#[must_use]
pub fn valid_run(run: &Value, release: &Release) -> bool {
    run["id"].as_u64().is_some_and(|id| id > 0)
        && run["workflow_id"] == 387_356_305_u64
        && run["path"] == ".github/workflows/release.yml"
        && run["event"] == "push"
        && run["head_branch"] == release.tag()
        && run["head_sha"] == release.commit()
        && run["status"] == "completed"
        && run["conclusion"] == "success"
        && run["run_attempt"].as_u64().is_some_and(|n| n > 0)
        && ["repository", "head_repository"].iter().all(|key| {
            run[key]["id"] == 1_417_054_470_u64
                && run[key]["full_name"] == "breki/silkboard"
        })
        && run["referenced_workflows"].as_array().is_some_and(|refs| {
            refs.len() == 1
                && refs[0]["ref"] == format!("refs/tags/{}", release.tag())
                && refs[0]["sha"] == release.commit()
                && refs[0]["path"]
                    == format!(
                        "breki/silkboard/.github/workflows/ci.yml@{}",
                        release.commit()
                    )
        })
}

/// Require all three gates, with no missing, skipped, or additional jobs.
#[must_use]
pub fn passing_jobs(response: &Value) -> bool {
    let Some(jobs) = response["jobs"].as_array() else {
        return false;
    };
    let required = [
        "Signed stable tag and exact revision",
        "gates / Full validation (Linux)",
        "Build the checked release revision",
    ];
    response["total_count"] == 3
        && jobs.len() == 3
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
