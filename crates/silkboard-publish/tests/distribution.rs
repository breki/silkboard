//! The copied release client runs without a checkout or runtime tools.
mod support;

use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};
use support::{Fixture, ack, report, server_projects};

fn executable(fixture: &Fixture) -> PathBuf {
    let source = std::env::var_os("SILKBOARD_TEST_PUBLISHER")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_silkboard-publish").into());
    let path = fixture.root.join(if cfg!(windows) {
        "silkboard-publish.exe"
    } else {
        "silkboard-publish"
    });
    fs::copy(source, &path).unwrap();
    path
}

fn cli(
    f: &Fixture,
    binary: &PathBuf,
    args: &[std::ffi::OsString],
    input: &[u8],
) -> std::process::Output {
    let child = Command::new(binary)
        .args(args)
        .current_dir(&f.root)
        .env("PATH", "")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    support::finish_cli(child, input)
}

#[test]
fn copied_client_publishes_events_and_resumes_workflow_plans_without_tools() {
    let mut f = Fixture::new();
    let binary = executable(&f);
    let version = cli(&f, &binary, &["--version".into()], b"");
    assert!(version.status.success());
    if let Ok(expected) = std::env::var("SILKBOARD_TEST_EXPECTED_VERSION") {
        assert_eq!(String::from_utf8(version.stdout).unwrap().trim(), expected);
    }
    let licenses = cli(&f, &binary, &["--licenses".into()], b"");
    assert!(licenses.status.success());
    assert!(String::from_utf8(licenses.stdout).unwrap().contains("ring"));
    let (origin, handle) = server_projects(
        4,
        "silkboard",
        "progress|plans",
        |value| {
            if value.get("event_id").is_some() {
                ack(&value)
            } else {
                (200, json!({"task_id":value["task_id"],"sequence":value["sequence"],"received_at":123}).to_string())
            }
        },
    );
    f.config["endpoint"] = json!(origin);
    let args = f.args();
    let plan = json!({"schema_version":1,"task_id":"11111111-1111-4111-8111-111111111111","reporter":{"hostname":"dev","tool":"codex","session_id":"native-1"},"title":"Packaged client smoke check","steps":[{"id":"smoke","title":"Publish from copied client","depends_on":[],"state":"active"}]});
    for (command, input, expected) in [
        (
            vec!["publish".into(), args[1].clone(), args[2].clone()],
            report(),
            "Progress acknowledged",
        ),
        (
            vec!["publish".into(), args[1].clone(), args[2].clone()],
            report(),
            "Progress acknowledged",
        ),
        (
            vec![
                "plan".into(),
                "start".into(),
                args[1].clone(),
                args[2].clone(),
            ],
            plan.clone(),
            "Task plan acknowledged",
        ),
        (
            vec![
                "plan".into(),
                "publish".into(),
                args[1].clone(),
                args[2].clone(),
            ],
            plan,
            "Task plan acknowledged",
        ),
    ] {
        let output =
            cli(&f, &binary, &command, &serde_json::to_vec(&input).unwrap());
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), expected);
    }
    let requests = handle.join().unwrap();
    assert_eq!(requests[0]["sequence"], 1);
    assert_eq!(requests[1]["sequence"], 2);
    assert_eq!(requests[2]["sequence"], "1");
    assert_eq!(requests[3]["sequence"], "2");
}
