#![allow(dead_code)] // Each integration suite uses only its relevant fixture actions.
use crate::support::{self, Fixture};
use serde_json::{Value, json};
pub const A: &str = "11111111-1111-4111-8111-111111111111";
pub const B: &str = "22222222-2222-4222-8222-222222222222";
pub fn fixture() -> Fixture {
    let mut f = Fixture::new();
    f.config["codex"] =
        json!({"workspace":f.root,"hostname":"dev","plan_reporting":true});
    f
}
pub fn reporter(native: &str) -> Value {
    json!({"hostname":"dev","tool":"codex","session_id":native})
}
pub fn draft(id: &str) -> Value {
    json!({"schema_version":1,"task_id":id,"reporter":reporter("native-parent"),"title":"Workflow context","steps":[{"id":"work","title":"Work","depends_on":[],"state":"active"}]})
}
pub fn plan(
    f: &Fixture,
    command: &str,
    value: &Value,
) -> Result<&'static str, silkboard_publish::Error> {
    let mut args = f.args();
    args[0] = command.into();
    args.insert(0, "plan".into());
    silkboard_publish::run(&args, serde_json::to_vec(value).unwrap().as_slice())
}
static NEXT_TURN: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
pub fn hook(f: &Fixture, native: &str, child: Option<&str>) {
    let mut args = f.args();
    args[0] = "codex-hook".into();
    let payload = json!({"turn_id":format!("turn-{}", NEXT_TURN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)),"hook_event_name":if child.is_some(){"SubagentStop"}else{"Stop"},"session_id":native,"agent_id":child,"cwd":f.root,"last_assistant_message":"Work completed. Private extra text.","task_id":B,"transcript_path":"/must/not/read"});
    for command in ["codex-hook-context", "codex-hook"] {
        args[0] = command.into();
        assert_eq!(
            silkboard_publish::run(&args, payload.to_string().as_bytes()),
            Ok("{}")
        );
    }
}
pub fn ack(value: &Value) -> (u16, String) {
    if value.get("event_id").is_some() {
        support::ack(value)
    } else {
        (200,json!({"task_id":value["task_id"],"sequence":value["sequence"],"received_at":123}).to_string())
    }
}
pub fn success(f: &Fixture, command: &str, value: &Value) {
    assert!(plan(f, command, value).is_ok(), "{command}");
}
pub fn choice(id: Option<&str>, writer: Value) -> Value {
    let mut input = json!({"schema_version":1,"task_id":id});
    input["reporter"] = writer;
    input
}
pub fn transfer(f: &Fixture, from: Value, to: Value) {
    let mut input = json!({"schema_version":1,"task_id":A});
    input["from"] = from;
    input["to"] = to;
    success(f, "handoff", &input);
}
pub fn linked(reports: &[Value], index: usize, id: Option<&str>) {
    assert_eq!(
        reports[index].get("task_id").and_then(Value::as_str),
        id,
        "report {index}"
    );
}

pub fn native(
    f: &Fixture,
    command: &str,
    input: &Value,
    thread: Option<&str>,
) -> std::process::Output {
    use std::process::{Command, Stdio};
    let args = f.args();
    let mut process = Command::new(env!("CARGO_BIN_EXE_silkboard-publish"));
    process
        .args(["codex-plan", command, "--config"])
        .arg(&args[2])
        .current_dir(&f.root)
        .env("PATH", "/nonexistent")
        .env_remove("CODEX_THREAD_ID")
        .stdin(if command == "clear" {
            Stdio::null()
        } else {
            Stdio::piped()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(thread) = thread {
        process.env("CODEX_THREAD_ID", thread);
    }
    let child = process.spawn().unwrap();
    if command == "clear" {
        child.wait_with_output().unwrap()
    } else {
        support::finish_cli(child, &serde_json::to_vec(input).unwrap())
    }
}
pub fn state_path(
    f: &Fixture,
    prefix: &str,
    extension: &str,
) -> std::path::PathBuf {
    f.state_files(extension)
        .into_iter()
        .find(|p| p.file_name().unwrap().to_string_lossy().starts_with(prefix))
        .unwrap()
}
