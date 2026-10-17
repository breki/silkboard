//! Release workflow admission contracts.
use serde_json::{Value, json};
use silkboard_updater::{Release, evidence::passing_run};

fn run(id: u64) -> Value {
    json!({"id": id, "workflow_id":387_356_305,
        "path":".github/workflows/release.yml", "event":"push",
        "head_branch":"v0.2.0", "head_sha":"a".repeat(40),
        "status":"completed", "conclusion":"success", "run_attempt":1,"referenced_workflows":[{"ref":"refs/tags/v0.2.0","sha":"a".repeat(40),"path":format!("breki/silkboard/.github/workflows/ci.yml@{}","a".repeat(40))}],
        "repository":{"id":1_417_054_470,"full_name":"breki/silkboard"},
        "head_repository":{"id":1_417_054_470,"full_name":"breki/silkboard"}})
}

#[test]
fn accepts_only_exact_trusted_workflow_repository_tag_and_revision() {
    let release = Release::new("v0.2.0", &"a".repeat(40)).unwrap();
    assert_eq!(
        passing_run(&json!({"workflow_runs":[run(1)]}), &release),
        Some(1)
    );
    for field in [
        "workflow_id",
        "path",
        "event",
        "head_branch",
        "head_sha",
        "status",
        "conclusion",
        "repository",
        "head_repository",
        "run_attempt",
    ] {
        let mut bad = run(1);
        bad[field] = Value::Null;
        assert_eq!(
            passing_run(&json!({"workflow_runs":[bad]}), &release),
            None,
            "{field}"
        );
    }
}

#[test]
fn a_later_failed_or_pending_rerun_revokes_an_old_success() {
    let release = Release::new("v0.2.0", &"a".repeat(40)).unwrap();
    for (status, conclusion) in [("completed", "failure"), ("in_progress", "")]
    {
        let mut newer = run(2);
        newer["status"] = json!(status);
        newer["conclusion"] = json!(conclusion);
        assert_eq!(
            passing_run(&json!({"workflow_runs":[run(1),newer]}), &release),
            None
        );
    }
    assert_eq!(passing_run(&json!({}), &release), None);
}

#[test]
fn branch_push_with_same_name_and_commit_is_not_tag_push_evidence() {
    let release = Release::new("v0.2.0", &"a".repeat(40)).unwrap();
    let mut branch_run = run(1);
    branch_run["referenced_workflows"] = json!([{
        "path":format!("breki/silkboard/.github/workflows/ci.yml@{}", "a".repeat(40)),
        "ref":"refs/heads/v0.2.0", "sha":"a".repeat(40)}]);
    assert_eq!(
        passing_run(&json!({"workflow_runs":[branch_run]}), &release),
        None
    );
}
