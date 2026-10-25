//! Explicit Codex graph reports use native identity and operator configuration.
#![cfg(unix)]
mod support;
use serde_json::{Value, json};
use std::process::{Command, Output, Stdio};
use support::{Fixture, server_at};

fn draft() -> Value {
    json!({"schema_version":1,"task_id":"77777777-7777-4777-8777-777777777777","title":"Codex evolving plan reporting","steps":[{"id":"surface","title":"Verify native integration","depends_on":[],"state":"done"},{"id":"adapter","title":"Implement explicit reporting","depends_on":["surface"],"state":"active"}]})
}
fn cli(
    f: &Fixture,
    command: &str,
    input: &Value,
    native: Option<&str>,
) -> Output {
    let args = f.args();
    let mut process = Command::new(env!("CARGO_BIN_EXE_silkboard-publish"));
    process
        .args(["codex-plan", command, "--config"])
        .arg(&args[2])
        .current_dir(&f.root)
        .env("PATH", "/nonexistent")
        .env_remove("CODEX_THREAD_ID")
        .env("CODEX_SESSION_ID", "must-not-replace-native-thread")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(native) = native {
        process.env("CODEX_THREAD_ID", native);
    }
    support::finish_cli(
        process.spawn().unwrap(),
        &serde_json::to_vec(input).unwrap(),
    )
}

#[test]
fn rejected_cli_can_exit_before_the_test_writes_stdin() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_silkboard-publish"))
        .arg("unsupported-fixture-command")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Keep the writer open while forcing the receiver to exit. Child::wait
    // otherwise closes its owned stdin handle before waiting.
    let stdin = child.stdin.take().unwrap();
    assert!(!child.wait().unwrap().success());
    child.stdin = Some(stdin);
    let output =
        support::finish_cli(child, &serde_json::to_vec(&draft()).unwrap());
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage:"));
}
#[test]
fn codex_reports_initial_plan_and_transitions_with_operator_native_identity() {
    let mut f = Fixture::new();
    let (origin, server) = server_at(2, "plans", |snapshot| {
        (200,json!({"task_id":snapshot["task_id"],"sequence":snapshot["sequence"],"received_at":123}).to_string())
    });
    f.config["endpoint"] = json!(origin);
    f.config["codex"] = json!({"workspace":f.root,"hostname":"configured-host","plan_reporting":true,"session_name":"Codex graph work"});
    let first = cli(&f, "start", &draft(), Some("native-root"));
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let mut next = draft();
    next["steps"][1]["state"] = json!("done");
    let second = cli(&f, "publish", &next, Some("native-root"));
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let snapshots = server.join().unwrap();
    assert_eq!(snapshots[0]["reporter"]["hostname"], "configured-host");
    assert_eq!(snapshots[0]["reporter"]["tool"], "codex");
    assert_eq!(snapshots[0]["reporter"]["session_id"], "native-root");
    assert_eq!(snapshots[0]["reporter"]["session_name"], "Codex graph work");
    assert_eq!(snapshots[1]["sequence"], "2");
    assert_eq!(snapshots[1]["plan_revision"], "1");
    assert_eq!(snapshots[1]["steps"][1]["state"], "done");
}

#[test]
fn explicit_reports_reject_missing_identity_disabled_scope_and_input_overrides()
{
    let mut f = Fixture::new();
    assert!(
        !cli(&f, "start", &draft(), Some("native-root"))
            .status
            .success()
    );
    f.config["codex"] = json!({"workspace":f.root,"hostname":"configured-host","plan_reporting":true});
    for native in [None, Some(""), Some("bad\nidentity")] {
        assert!(!cli(&f, "start", &draft(), native).status.success());
        assert!(f.state_files("json").is_empty());
    }
    for field in [
        "reporter",
        "endpoint",
        "token_file",
        "state_dir",
        "sequence",
        "writer_generation",
        "plan_revision",
        "command",
    ] {
        let mut invalid = draft();
        invalid[field] = json!("DO-NOT-PRINT");
        let result = cli(&f, "start", &invalid, Some("native-root"));
        assert!(!result.status.success(), "{field}");
        assert!(
            !String::from_utf8_lossy(&result.stderr).contains("DO-NOT-PRINT")
        );
        assert!(f.state_files("json").is_empty());
    }
    f.config["codex"]["workspace"] = json!(f.root.join("another-worktree"));
    assert!(
        !cli(&f, "start", &draft(), Some("native-root"))
            .status
            .success()
    );
    assert!(f.state_files("json").is_empty());
    f.config["codex"]["workspace"] = json!("relative");
    assert!(
        !cli(&f, "start", &draft(), Some("native-root"))
            .status
            .success()
    );
    assert!(
        !cli(&f, "handoff", &draft(), Some("native-root"))
            .status
            .success()
    );
}

#[test]
fn failed_delivery_keeps_native_writer_binding_and_does_not_restart_task() {
    let mut f = Fixture::new();
    f.config["codex"] = json!({"workspace":f.root,"hostname":"configured-host","plan_reporting":true});
    let first = cli(&f, "start", &draft(), Some("native-root"));
    assert!(!first.status.success());
    let paths = f.state_files("json");
    let path = paths
        .iter()
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("plan-")
        })
        .unwrap();
    let read = || {
        serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap()
    };
    assert_eq!(read()["snapshot"]["sequence"], "1");
    assert!(
        !cli(&f, "start", &draft(), Some("native-root"))
            .status
            .success()
    );
    assert_eq!(read()["snapshot"]["sequence"], "1");
    assert!(
        !cli(&f, "publish", &draft(), Some("different-native-session"))
            .status
            .success()
    );
    assert_eq!(read()["snapshot"]["sequence"], "1");
    assert!(
        !cli(&f, "publish", &draft(), Some("native-root"))
            .status
            .success()
    );
    assert_eq!(read()["snapshot"]["sequence"], "2");
}

#[test]
fn existing_summary_configuration_does_not_enable_plan_reporting() {
    let mut f = Fixture::new();
    f.config["codex"] =
        json!({"workspace":f.root,"hostname":"configured-host"});
    assert!(
        !cli(&f, "start", &draft(), Some("native-root"))
            .status
            .success()
    );
    assert!(f.state_files("json").is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actual_api_accepts_evolving_graphs_and_explicit_native_handoff() {
    let store = silkboard_web::progress::ProgressStore::open_memory().unwrap();
    silkboard_web::owner::set_password_from_reader(
        &store,
        b"owned codex plan fixture password".as_slice(),
    )
    .unwrap();
    let token = store
        .issue_token("silkboard", "codex-plan-fixture")
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let app = silkboard_web::create_progress_router(
        std::path::Path::new("missing"),
        store,
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let mut f = Fixture::new();
    f.config["endpoint"] = json!(origin);
    f.config["codex"] = json!({"workspace":f.root,"hostname":"configured-host","plan_reporting":true});
    support::private(&f.root.join("token"), token.as_bytes());
    tokio::task::spawn_blocking(move || exercise_evolving_workflow(&f))
        .await
        .unwrap();
    server.abort();
}

fn exercise_evolving_workflow(f: &Fixture) {
    let mut graph = draft();
    for transition in 0..5 {
        match transition {
            1 => {
                graph["steps"][1]["state"] = json!("blocked");
                graph["steps"][1]["blocked_reason"] =
                    json!("Owned test dependency unavailable");
            }
            2 => {
                graph["steps"][1]["state"] = json!("done");
                graph["steps"][1]
                    .as_object_mut()
                    .unwrap()
                    .remove("blocked_reason");
            }
            3 => graph["steps"][1]["state"] = json!("active"),
            4 => {
                graph["steps"].as_array_mut().unwrap().push(json!({"id":"workflow","title":"Integrate workflow","depends_on":["surface"],"state":"active"}));
            }
            _ => {}
        }
        let command = if transition == 0 { "start" } else { "publish" };
        let result = cli(f, command, &graph, Some("native-root"));
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let path = f
        .state_files("json")
        .into_iter()
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("plan-")
        })
        .unwrap();
    let snapshot = || {
        serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()
            ["snapshot"]
            .clone()
    };
    assert_eq!(snapshot()["sequence"], "5");
    assert_eq!(snapshot()["plan_revision"], "2");
    assert_eq!(snapshot()["steps"][1]["id"], "adapter");
    assert_eq!(snapshot()["steps"][1]["state"], "active");
    let before_stop = snapshot();
    let mut args = f.args();
    args[0] = "codex-hook".into();
    let stop = json!({"hook_event_name":"Stop","session_id":"native-root","cwd":f.root,"last_assistant_message":"Turn ended. Task remains in progress."});
    assert_eq!(
        silkboard_publish::run(
            &args,
            serde_json::to_vec(&stop).unwrap().as_slice()
        ),
        Ok("{}")
    );
    assert_eq!(
        snapshot(),
        before_stop,
        "a Stop must not complete or publish a plan"
    );
    let identity = |session| json!({"hostname":"configured-host","tool":"codex","session_id":session});
    let handoff = json!({"schema_version":1,"task_id":graph["task_id"],"from":identity("native-root"),"to":identity("native-next")});
    let mut args = f.args();
    args[0] = "handoff".into();
    args.insert(0, "plan".into());
    silkboard_publish::run(
        &args,
        serde_json::to_vec(&handoff).unwrap().as_slice(),
    )
    .unwrap();
    assert!(
        !cli(f, "publish", &graph, Some("native-root"))
            .status
            .success()
    );
    let result = cli(f, "publish", &graph, Some("native-next"));
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(snapshot()["sequence"], "6");
    assert_eq!(snapshot()["plan_revision"], "2");
    assert_eq!(snapshot()["writer_generation"], "2");
    assert_eq!(snapshot()["reporter"]["session_id"], "native-next");
}

#[test]
#[should_panic(expected = "successful CLI did not consume input")]
fn broken_pipe_does_not_hide_an_unexpected_successful_exit() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_silkboard-publish"))
        .arg("--help")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
    assert!(child.wait().unwrap().success());
    child.stdin = Some(stdin);
    support::finish_cli(child, b"unread fixture input");
}
