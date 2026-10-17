//! A second policy fixture exercises reuse without adopting another application.

use serde_json::{Value, json};
use silkboard_updater::{Release, evidence, profile::Policy};

#[test]
fn an_independent_policy_binds_its_own_repository_workflow_and_job_set() {
    let policy = Policy::from_json(&serde_json::to_vec(&json!({
        "schema":1,"application":"static-fixture","repository":"example/site",
        "repository_id":42,"workflow_id":57,"workflow_path":".github/workflows/publish.yaml",
        "ci_path":".github/workflows/check.yaml","required_jobs":["Trust", "Static build"]
    })).unwrap()).unwrap();
    let release = Release::new("v1.0.0", &"a".repeat(40)).unwrap();
    let run = json!({"id":2,"workflow_id":57,"path":".github/workflows/publish.yaml",
        "event":"push","head_branch":"v1.0.0","head_sha":release.commit(),
        "status":"completed","conclusion":"success","run_attempt":1,
        "repository":{"id":42,"full_name":"example/site"},
        "head_repository":{"id":42,"full_name":"example/site"},
        "referenced_workflows":[{"ref":"refs/tags/v1.0.0","sha":release.commit(),
            "path":format!("example/site/.github/workflows/check.yaml@{}",release.commit())}]});
    assert!(evidence::valid_workflow_for(
        &json!({"id":57,"path":".github/workflows/publish.yaml","state":"active"}),
        &policy
    ));
    assert_eq!(
        evidence::passing_run_for(
            &json!({"workflow_runs":[run.clone()]}),
            &release,
            &policy
        ),
        Some(2)
    );
    assert!(!evidence::valid_run(&run, &release));
    let jobs = json!({"total_count":2,"jobs":[{"name":"Trust","status":"completed","conclusion":"success"},
        {"name":"Static build","status":"completed","conclusion":"success"}]});
    assert!(evidence::passing_jobs_for(&jobs, &policy));
    assert!(!evidence::passing_jobs(&jobs));
    for field in [
        "workflow_id",
        "path",
        "head_sha",
        "head_branch",
        "repository",
        "head_repository",
        "referenced_workflows",
    ] {
        let mut wrong = run.clone();
        wrong[field] = Value::Null;
        assert!(!evidence::valid_run_for(&wrong, &release, &policy));
    }
    for malformed in [
        json!({}),
        json!({"total_count":3,"jobs":jobs["jobs"]}),
        json!({"total_count":2,"jobs":[jobs["jobs"][0].clone(),jobs["jobs"][0].clone()]}),
    ] {
        assert!(!evidence::passing_jobs_for(&malformed, &policy));
    }
}

#[test]
fn installed_example_preserves_the_existing_silkboard_gates() {
    assert_eq!(
        Policy::from_json(include_bytes!("../../../deploy/vm/deployment.json"))
            .unwrap(),
        Policy::silkboard()
    );
}
