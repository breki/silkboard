//! Native Codex payload boundaries and best-effort publishing.
#![cfg(unix)]
mod support;
use serde_json::{Value, json};
use support::{Fixture, ack, server};

fn hook(
    f: &Fixture,
    payload: &Value,
) -> Result<&'static str, silkboard_publish::Error> {
    let mut args = f.args();
    args[0] = "codex-hook".into();
    silkboard_publish::run(
        &args,
        serde_json::to_vec(payload).unwrap().as_slice(),
    )
}

fn payload(f: &Fixture, event: &str) -> Value {
    json!({"hook_event_name":event,"session_id":"native-parent","cwd":f.root,
        "last_assistant_message":"Implemented progress publishing. Extra private detail.",
        "transcript_path":"/must/not/read","prompt":"DO-NOT-UPLOAD","tool_input":"DO-NOT-UPLOAD"})
}

#[test]
fn disabled_and_invalid_hooks_never_control_codex() {
    let mut f = Fixture::new();
    assert_eq!(hook(&f, &payload(&f, "Stop")), Ok("{}"));
    f.config["codex"] = json!({"workspace":f.root,"hostname":"dev"});
    for change in [
        json!({"hook_event_name":"PostToolUse"}),
        json!({"session_id":""}),
        json!({"cwd":"/other/workspace"}),
        json!({"cwd":f.root.join("../elsewhere")}),
        json!({"stop_hook_active":true}),
        json!({"agent_id":"child"}),
    ] {
        let mut value = payload(&f, "Stop");
        value
            .as_object_mut()
            .unwrap()
            .extend(change.as_object().unwrap().clone());
        assert_eq!(hook(&f, &value), Ok("{}"));
    }
    assert!(f.state_files("json").is_empty());
    let mut args = f.args();
    args[0] = "codex-hook".into();
    for input in [b"not JSON".to_vec(), vec![b'x'; 65537]] {
        assert_eq!(silkboard_publish::run(&args, input.as_slice()), Ok("{}"));
    }
}

#[test]
fn root_resumes_and_children_publish_only_bounded_observations() {
    let mut f = Fixture::new();
    let (origin, h) = server(4, |event| ack(&event));
    f.config["endpoint"] = json!(origin);
    f.config["codex"] = json!({"workspace":f.root,"hostname":"dev","session_name":"Progress adapter"});
    let root = payload(&f, "Stop");
    assert_eq!(hook(&f, &root), Ok("{}"));
    for event in ["SubagentStart", "SubagentStop"] {
        let mut child = payload(&f, event);
        child["agent_id"] = json!("native-child");
        child["agent_type"] = json!("reviewer");
        assert_eq!(hook(&f, &child), Ok("{}"));
    }
    assert_eq!(hook(&f, &root), Ok("{}"));
    let events = h.join().unwrap();
    assert_eq!(events[0]["message"], "Implemented progress publishing.");
    assert_eq!(events[0]["session_name"], "Progress adapter");
    assert_eq!(events[0]["session_id"], "native-parent");
    assert_eq!(events[1]["subagent_id"], "native-child");
    assert_eq!(events[1]["parent_session_id"], "native-parent");
    assert_eq!(events[1]["session_name"], "reviewer");
    assert_eq!(events[1]["message"], "Subagent started.");
    assert_eq!(events[3]["sequence"], 2);
    assert_eq!(events[2]["sequence"], 2);
    let wire = serde_json::to_string(&events).unwrap();
    assert!(!wire.contains("DO-NOT-UPLOAD"));
    assert!(!wire.contains("must/not/read"));
    assert!(!wire.contains("Extra private detail"));
}

#[test]
fn summaries_are_utf8_bounded_single_line_and_use_lifecycle_fallbacks() {
    let mut f = Fixture::new();
    let long = "é".repeat(121);
    let examples = [
        ("**Ready!** Do not upload this.", "Ready!".to_owned()),
        ("\n**Ready!**\nDo not upload this.", "Ready!".to_owned()),
        ("```sh\nsecret command\n```", "Turn completed.".to_owned()),
        ("~~~\nsecret\n~~~", "Turn completed.".to_owned()),
        ("\n \n", "Turn completed.".to_owned()),
        ("**", "Turn completed.".to_owned()),
        (
            "Checked\u{202e}\t successfully? Hidden.",
            "Checked successfully?".to_owned(),
        ),
        (long.as_str(), "é".repeat(120)),
    ];
    let (origin, h) = server(examples.len(), |event| ack(&event));
    f.config["endpoint"] = json!(origin);
    f.config["codex"] = json!({"workspace":f.root,"hostname":"dev"});
    for (text, _) in &examples {
        let mut p = payload(&f, "Stop");
        p["last_assistant_message"] = json!(text);
        assert_eq!(hook(&f, &p), Ok("{}"));
    }
    for (event, (_, expected)) in h.join().unwrap().iter().zip(examples) {
        assert_eq!(event["message"], expected);
    }
}

#[test]
fn missing_fields_invalid_config_and_delivery_errors_are_quiet() {
    let mut f = Fixture::new();
    f.config["codex"] = json!({"workspace":f.root,"hostname":"dev"});
    for p in [
        json!({}),
        json!({"hook_event_name":"SubagentStop","session_id":"p","cwd":f.root}),
        json!({"hook_event_name":"Stop","session_id":"p","cwd":"relative"}),
    ] {
        assert_eq!(hook(&f, &p), Ok("{}"));
    }
    f.config["codex"]["workspace"] = json!("relative");
    assert_eq!(hook(&f, &payload(&f, "Stop")), Ok("{}"));
    f.config["codex"]["workspace"] = json!(f.root);
    let mut p = payload(&f, "SubagentStop");
    p["agent_id"] = json!("c");
    p.as_object_mut().unwrap().remove("last_assistant_message");
    let (origin, h) = server(1, |_| (401, "DO-NOT-ECHO".to_owned()));
    f.config["endpoint"] = json!(origin);
    assert_eq!(hook(&f, &p), Ok("{}"));
    assert_eq!(h.join().unwrap()[0]["message"], "Subagent completed.");
    assert_eq!(hook(&f, &p), Ok("{}")); // closed listener: no retry or failure output
    let mut args = f.args();
    args[0] = "codex-hook".into();
    support::private(&f.root.join("config.json"), b"invalid");
    assert_eq!(silkboard_publish::run(&args, &b""[..]), Ok("{}"));
}

#[test]
fn wrapper_missing_installation_and_failed_executable_cannot_block() {
    use std::process::Command;
    let f = Fixture::new();
    let wrapper = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.codex/hooks/publish-progress.sh");
    for binary in ["/nonexistent", "/bin/false"] {
        let output = Command::new("sh")
            .arg(&wrapper)
            .env("SILKBOARD_PUBLISH_BIN", binary)
            .env("SILKBOARD_PUBLISH_CONFIG", f.args()[2].clone())
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"{}\n");
        assert!(output.stderr.is_empty());
    }
}
