//! Client publication reuses the updater's exact release evidence policy.
use crate::Runner;
use serde_json::Value;
use silkboard_updater::{Release, evidence};
use std::path::Path;

pub(super) fn check(
    root: &Path,
    runner: &Runner<'_>,
    tag: &str,
    sha: &str,
    run: &str,
    attempt: &str,
) -> Result<(), String> {
    let release = Release::new(tag, sha)?;
    let run = positive(run)?;
    let attempt = positive(attempt)?;
    let endpoint = format!("repos/breki/silkboard/actions/runs/{run}");
    let metadata = fetch(root, runner, &endpoint)?;
    let exact = |metadata: &Value| {
        evidence::valid_run(metadata, &release)
            && metadata["id"] == run
            && metadata["run_attempt"] == attempt
    };
    if !exact(&metadata) {
        return Err("client publication requires the exact successful release run attempt".into());
    }
    let jobs = fetch(
        root,
        runner,
        &format!("{endpoint}/attempts/{attempt}/jobs?per_page=100"),
    )?;
    if !evidence::passing_jobs(&jobs) {
        return Err(
            "client publication requires all three successful release jobs"
                .into(),
        );
    }
    let latest = fetch(
        root,
        runner,
        &format!(
            "repos/breki/silkboard/actions/workflows/387356305/runs?event=push&branch={tag}&head_sha={sha}&per_page=100"
        ),
    )?;
    if evidence::passing_run(&latest, &release) != Some(run)
        || !exact(&fetch(root, runner, &endpoint)?)
    {
        return Err(
            "upstream release evidence changed during verification".into()
        );
    }
    println!("Exact release run attempt and complete gate set verified.");
    Ok(())
}

fn positive(value: &str) -> Result<u64, String> {
    value.parse::<u64>().ok().filter(|n| *n > 0).ok_or_else(|| {
        "release run and attempt must be positive integers".into()
    })
}

fn fetch(
    root: &Path,
    runner: &Runner<'_>,
    endpoint: &str,
) -> Result<Value, String> {
    serde_json::from_str(&runner("gh", &["api", endpoint], root)?)
        .map_err(|error| format!("release evidence JSON: {error}"))
}
