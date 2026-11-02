//! Client publication uses the updater's complete release admission evidence.
use serde_json::{Value, json};
use std::path::Path;

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn run() -> Value {
    json!({"id":42,"run_attempt":2,"workflow_id":387_356_305,
        "path":".github/workflows/release.yml","event":"push",
        "head_branch":"v0.12.0","head_sha":SHA,"status":"completed",
        "conclusion":"success","repository":{"id":1_417_054_470,"full_name":"breki/silkboard"},
        "head_repository":{"id":1_417_054_470,"full_name":"breki/silkboard"},
        "referenced_workflows":[{"ref":"refs/tags/v0.12.0","sha":SHA,
            "path":format!("breki/silkboard/.github/workflows/ci.yml@{SHA}")}]})
}

fn jobs() -> Value {
    let jobs = [
        "Signed stable tag and exact revision",
        "gates / Full validation (Linux)",
        "Build the checked release revision",
    ]
    .map(
        |name| json!({"name":name,"status":"completed","conclusion":"success"}),
    );
    json!({"total_count":3,"jobs":jobs})
}

fn check(
    run: &Value,
    jobs: &Value,
    id: &str,
    attempt: &str,
) -> Result<(), String> {
    xtask::dispatch(
        &["client-release-check", "v0.12.0", SHA, id, attempt]
            .map(String::from),
        Path::new("."),
        &mut |program, args, _| {
            assert_eq!(program, "gh");
            assert_eq!(args[0], "api");
            match args[1] {
                "repos/breki/silkboard/actions/runs/42" => Ok(run.to_string()),
                "repos/breki/silkboard/actions/runs/42/attempts/2/jobs?per_page=100" => {
                    Ok(jobs.to_string())
                }
                endpoint if endpoint.contains("/workflows/") => {
                    Ok(json!({"workflow_runs":[run]}).to_string())
                }
                other => panic!("unexpected endpoint {other}"),
            }
        },
    )
}

#[test]
fn requires_exact_successful_run_attempt_and_all_three_jobs() {
    assert!(check(&run(), &jobs(), "42", "2").is_ok());
    for field in [
        "workflow_id",
        "repository",
        "referenced_workflows",
        "head_sha",
        "head_branch",
        "status",
        "conclusion",
        "id",
        "run_attempt",
    ] {
        let mut invalid = run();
        invalid[field] = Value::Null;
        assert!(check(&invalid, &jobs(), "42", "2").is_err(), "{field}");
    }
    for outcome in ["failure", "skipped", "cancelled"] {
        let mut invalid = jobs();
        invalid["jobs"][1]["conclusion"] = json!(outcome);
        assert!(check(&run(), &invalid, "42", "2").is_err());
    }
    for invalid in [json!({}), json!({"total_count":3,"jobs":[]})] {
        assert!(check(&run(), &invalid, "42", "2").is_err());
    }
    for id in ["0", "-1", "42/path", "not-a-run"] {
        assert!(check(&run(), &jobs(), id, "2").is_err());
        assert!(check(&run(), &jobs(), "42", id).is_err());
    }
}

#[test]
fn unavailable_or_malformed_evidence_and_invalid_releases_fail_closed() {
    let args =
        ["client-release-check", "v0.12.0", SHA, "42", "2"].map(String::from);
    for fail_jobs in [false, true] {
        for response in [Ok("not JSON".into()), Err("API unavailable".into())] {
            assert!(
                xtask::dispatch(&args, Path::new("."), &mut |_, argv, _| {
                    if fail_jobs && !argv[1].contains("/jobs?") {
                        Ok(run().to_string())
                    } else {
                        response.clone()
                    }
                })
                .is_err()
            );
        }
    }
    for (tag, sha) in [("v0.12.0-rc.1", SHA), ("v0.12.0", "bad-sha")] {
        assert!(
            xtask::dispatch(
                &["client-release-check", tag, sha, "42", "2"]
                    .map(String::from),
                Path::new("."),
                &mut |_, _, _| panic!("invalid release must not call GitHub"),
            )
            .is_err()
        );
    }
}

#[test]
fn a_newer_run_or_changed_attempt_revokes_admission() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    for change_attempt in [false, true] {
        let reads = AtomicUsize::new(0);
        let mut newer = run();
        newer["id"] = json!(43);
        let result = xtask::dispatch(
            &["client-release-check", "v0.12.0", SHA, "42", "2"]
                .map(String::from),
            Path::new("."),
            &mut |_, args, _| {
                if args[1].contains("/jobs?") {
                    Ok(jobs().to_string())
                } else if args[1].contains("/workflows/") {
                    Ok(json!({"workflow_runs":[if change_attempt {run()} else {newer.clone()}]}).to_string())
                } else {
                    let mut value = run();
                    if reads.fetch_add(1, Ordering::SeqCst) > 0 {
                        value["run_attempt"] = json!(3);
                        value["status"] = json!("in_progress");
                        value["conclusion"] = Value::Null;
                    }
                    Ok(value.to_string())
                }
            },
        );
        assert!(result.is_err());
    }
}
