//! Frozen native turn associations never follow a later task selection.
#![cfg(unix)]
mod support;
#[path = "support/workflow.rs"]
mod workflow;
use serde_json::{Value, json};
use support::{Fixture, private, server_at};
use workflow::*;
fn payload(f: &Fixture, turn: &str) -> Value {
    json!({"hook_event_name":"Stop","session_id":"native-parent","turn_id":turn,"cwd":f.root})
}
fn call(f: &Fixture, command: &str, payload: &Value) {
    let mut args = f.args();
    args[0] = command.into();
    assert_eq!(
        silkboard_publish::run(&args, payload.to_string().as_bytes()),
        Ok("{}")
    );
}
#[test]
fn missing_and_invalid_turns_or_capture_never_use_current_selection() {
    let mut f = fixture();
    let (origin, server) = server_at(6, "plans|progress", |v| ack(&v));
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    for turn in [
        json!(null),
        json!(""),
        json!("bad turn"),
        json!("x".repeat(257)),
        json!("not-captured"),
    ] {
        let mut p = payload(&f, "");
        p["turn_id"] = turn;
        call(&f, "codex-hook", &p);
    }
    let reports = server.join().unwrap();
    for i in 1..6 {
        linked(&reports, i, None);
    }
}
#[test]
fn capture_is_bounded_and_conflicting_duplicates_stay_unlinked() {
    let mut f = fixture();
    let (origin, server) = server_at(6, "plans|progress", |v| ack(&v));
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    let old = payload(&f, "old");
    call(&f, "codex-hook-context", &old);
    for i in 0..32 {
        call(&f, "codex-hook-context", &payload(&f, &format!("turn-{i}")));
    }
    call(&f, "codex-hook", &old); // evicted, never fall back to A
    let last = payload(&f, "turn-31");
    call(&f, "codex-hook", &last);
    success(&f, "start", &draft(B));
    call(&f, "codex-hook-context", &last); // ambiguous A/B
    call(&f, "codex-hook-context", &last); // cannot revive it
    call(&f, "codex-hook", &last);
    call(&f, "codex-hook-context", &payload(&f, "new"));
    call(&f, "codex-hook", &payload(&f, "new"));
    let reports = server.join().unwrap();
    for (i, id) in [(1, None), (2, Some(A)), (4, None), (5, Some(B))] {
        linked(&reports, i, id);
    }
    let path = state_path(&f, "hook-context-", "json");
    let stored: Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(stored["entries"].as_array().unwrap().len(), 32);
}
#[test]
fn historical_capture_survives_handoff_but_new_old_writer_capture_does_not() {
    let mut f = fixture();
    let (origin, server) = server_at(3, "plans|progress", |v| ack(&v));
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    call(&f, "codex-hook-context", &payload(&f, "before"));
    transfer(&f, reporter("native-parent"), reporter("replacement"));
    call(&f, "codex-hook", &payload(&f, "before"));
    call(&f, "codex-hook-context", &payload(&f, "after"));
    call(&f, "codex-hook", &payload(&f, "after"));
    let reports = server.join().unwrap();
    linked(&reports, 1, Some(A));
    linked(&reports, 2, None);
}
#[test]
fn corrupt_capture_and_wrong_identity_cannot_supply_workflow_ids() {
    let mut f = fixture();
    let (origin, server) = server_at(7, "plans|progress", |v| ack(&v));
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    let p = payload(&f, "same");
    call(&f, "codex-hook-context", &p);
    let mut other = p.clone();
    other["session_id"] = json!("other");
    call(&f, "codex-hook", &other);
    other = p.clone();
    other["hook_event_name"] = json!("SubagentStop");
    other["agent_id"] = json!("child");
    call(&f, "codex-hook", &other);
    let path = state_path(&f, "hook-context-", "json");
    for data in [b"{".to_vec(),json!({"schema_version":2,"entries":[]}).to_string().into_bytes(),json!({"schema_version":1,"entries":[{"event":"Stop","turn":"same","task_id":"invalid"}]}).to_string().into_bytes(),vec![b'x';16385]] {
        private(&path,&data);call(&f,"codex-hook",&p);
    }
    let reports = server.join().unwrap();
    for i in 1..7 {
        linked(&reports, i, None);
    }
}
#[test]
fn publisher_waits_for_an_atomically_installed_capture() {
    let mut f = fixture();
    let (origin, server) = server_at(2, "plans|progress", |v| ack(&v));
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    let p = payload(&f, "concurrent");
    call(&f, "codex-hook-context", &p);
    let capture = state_path(&f, "hook-context-", "json");
    let pending = capture.with_extension("pending");
    std::fs::rename(&capture, &pending).unwrap();
    let args = f.args();
    let mut upload = args.clone();
    upload[0] = "codex-hook".into();
    let input = p.to_string();
    let worker = std::thread::spawn(move || {
        silkboard_publish::run(&upload, input.as_bytes())
    });
    std::thread::sleep(std::time::Duration::from_millis(40));
    // Install real capture metadata atomically. Invoking another hook here
    // would race its nonblocking lock and rewrite the fixture configuration.
    std::fs::rename(&pending, &capture).unwrap();
    assert_eq!(worker.join().unwrap(), Ok("{}"));
    linked(&server.join().unwrap(), 1, Some(A));
}

#[test]
fn escaped_max_length_turn_ids_cannot_poison_later_captures() {
    let mut f = fixture();
    let (origin, server) = server_at(3, "plans|progress", |v| ack(&v));
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    let mut last = String::new();
    for index in 0..32 {
        last = format!("{index:02}{}", "\"\\".repeat(127));
        call(&f, "codex-hook-context", &payload(&f, &last));
    }
    let path = state_path(&f, "hook-context-", "json");
    assert!(std::fs::metadata(path).unwrap().len() <= 16384);
    call(&f, "codex-hook", &payload(&f, &last));
    call(
        &f,
        "codex-hook-context",
        &payload(&f, "normal-after-escaped"),
    );
    call(&f, "codex-hook", &payload(&f, "normal-after-escaped"));
    let reports = server.join().unwrap();
    linked(&reports, 1, Some(A));
    linked(&reports, 2, Some(A));
}
