//! Unsafe or ambiguous local context cannot control an automatic association.
#![cfg(unix)]
mod support;
#[path = "support/workflow.rs"]
mod workflow;
use serde_json::json;
use silkboard_publish::Error;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
use support::{private, server_at};
use workflow::*;

#[test]
fn native_select_and_clear_use_current_identity_and_require_confirmation() {
    let mut f = fixture();
    let (origin, server) = server_at(7, "plans|progress", |value| ack(&value));
    f.config["endpoint"] = json!(origin);
    let mut initial = draft(A);
    initial.as_object_mut().unwrap().remove("reporter");
    assert!(
        native(&f, "start", &initial, Some("native-parent"))
            .status
            .success()
    );
    hook(&f, "native-parent", None);
    f.config["codex"]["plan_reporting"] = json!(false);
    assert!(
        native(&f, "clear", &json!(null), Some("native-parent"))
            .status
            .success()
    );
    f.config["codex"]["plan_reporting"] = json!(true);
    hook(&f, "native-parent", None);
    assert!(
        native(
            &f,
            "select",
            &json!({"schema_version":1,"task_id":A}),
            Some("native-parent")
        )
        .status
        .success()
    );
    hook(&f, "native-parent", None);
    assert!(
        native(&f, "publish", &initial, Some("native-parent"))
            .status
            .success()
    );
    hook(&f, "native-parent", None);
    assert!(
        !native(
            &f,
            "select",
            &json!({"schema_version":2,"task_id":A}),
            Some("native-parent")
        )
        .status
        .success()
    );
    assert!(!native(&f,"select",&json!({"schema_version":1,"task_id":A,"reporter":reporter("forged")}),Some("native-parent")).status.success());
    assert!(!native(&f, "clear", &json!(null), None).status.success());
    assert!(
        !native(&f, "clear", &json!(null), Some("bad native identity"))
            .status
            .success()
    );
    hook(&f, "native-parent", None);
    let reports = server.join().unwrap();
    for (index, id) in [
        (1, Some(A)),
        (2, None),
        (3, None),
        (5, Some(A)),
        (6, Some(A)),
    ] {
        linked(&reports, index, id);
    }
}
#[test]
fn invalid_operator_context_commands_do_not_replace_selection() {
    let mut f = fixture();
    let (origin, server) = server_at(2, "plans|progress", |value| ack(&value));
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    for (command, input) in [
        ("select", choice(None, reporter("native-parent"))),
        ("select", choice(Some(B), reporter("native-parent"))),
        (
            "select",
            choice(Some("../../outside"), reporter("native-parent")),
        ),
        ("select", choice(Some(A), reporter("other-native"))),
        ("clear", choice(Some(A), reporter("native-parent"))),
        (
            "clear",
            json!({"schema_version":2,"reporter":reporter("native-parent")}),
        ),
        (
            "clear",
            json!({"schema_version":1,"reporter":reporter("bad native")}),
        ),
        (
            "select",
            choice(
                Some(A),
                json!({"hostname":"dev","tool":"claude","session_id":"native-parent"}),
            ),
        ),
        (
            "select",
            json!({"schema_version":1,"task_id":A,"reporter":reporter("native-parent"),"path":"/must/not/read"}),
        ),
    ] {
        assert!(plan(&f, command, &input).is_err());
    }
    hook(&f, "native-parent", None);
    linked(&server.join().unwrap(), 1, Some(A));
}
#[test]
fn corrupt_private_or_stale_context_falls_back_to_unlinked_summary() {
    let mut f = fixture();
    let (origin, server) = server_at(9, "plans|progress", |value| ack(&value));
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    let path = state_path(&f, "context-", "json");
    let original = fs::read(&path).unwrap();
    for bytes in [b"not JSON".to_vec(),serde_json::to_vec(&json!({"schema_version":2,"selection":null})).unwrap(),serde_json::to_vec(&json!({"schema_version":1,"selection":{"task_id":"../../bad","writer_generation":"1","acknowledged":true}})).unwrap(),vec![b'x';2049]] {
        private(&path,&bytes);hook(&f,"native-parent",None);
    }
    private(&path, &original);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    hook(&f, "native-parent", None);
    fs::remove_file(&path).unwrap();
    let target = f.root.join("untrusted-context");
    private(&target, &original);
    symlink(&target, &path).unwrap();
    hook(&f, "native-parent", None);
    fs::remove_file(&path).unwrap();
    fs::hard_link(&target, &path).unwrap();
    hook(&f, "native-parent", None);
    fs::remove_file(&path).unwrap();
    private(&path, &original);
    fs::remove_file(state_path(&f, "plan-", "json")).unwrap();
    hook(&f, "native-parent", None);
    let reports = server.join().unwrap();
    for index in 1..9 {
        linked(&reports, index, None);
    }
    assert_eq!(fs::read(target).unwrap(), original);
}
#[test]
fn busy_context_is_unlinked_without_blocking_hook_or_overwriting_selection() {
    let mut f = fixture();
    let (origin, server) = server_at(3, "plans|progress", |value| ack(&value));
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(state_path(&f, "context-", "lock"))
        .unwrap();
    lock.try_lock().unwrap();
    assert_eq!(
        plan(&f, "clear", &choice(None, reporter("native-parent"))),
        Err(Error::Busy)
    );
    hook(&f, "native-parent", None);
    drop(lock);
    hook(&f, "native-parent", None);
    let reports = server.join().unwrap();
    linked(&reports, 1, None);
    linked(&reports, 2, Some(A));
}
#[test]
fn in_flight_plan_submission_cannot_link_a_premature_hook() {
    let mut f = fixture();
    let (tx, rx) = std::sync::mpsc::channel();
    let (origin, server) = server_at(3, "plans|progress", move |v| {
        if v.get("event_id").is_none() {
            tx.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
        ack(&v)
    });
    f.config["endpoint"] = json!(origin);
    let mut args = f.args();
    args[0] = "start".into();
    args.insert(0, "plan".into());
    let worker = std::thread::spawn(move || {
        silkboard_publish::run(
            &args,
            serde_json::to_vec(&draft(A)).unwrap().as_slice(),
        )
    });
    rx.recv_timeout(std::time::Duration::from_secs(3)).unwrap();
    assert_eq!(
        plan(&f, "clear", &choice(None, reporter("native-parent"))),
        Err(Error::Busy)
    );
    hook(&f, "native-parent", None);
    assert!(worker.join().unwrap().is_ok());
    hook(&f, "native-parent", None);
    let reports = server.join().unwrap();
    linked(&reports, 1, None);
    linked(&reports, 2, Some(A));
}

#[test]
fn project_endpoint_and_tool_scope_never_reuse_another_selection() {
    let mut f = fixture();
    let (origin, server) = support::server_projects(
        4,
        "silkboard|other",
        "plans|progress",
        |value| ack(&value),
    );
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    f.config["project"] = json!("other");
    hook(&f, "native-parent", None);
    f.config["project"] = json!("silkboard");
    let mut other = draft(B);
    other["reporter"]["tool"] = json!("claude");
    success(&f, "start", &other);
    hook(&f, "native-parent", None);
    let reports = server.join().unwrap();
    linked(&reports, 1, None);
    linked(&reports, 3, Some(A));
    let (endpoint, another) = server_at(1, "progress", |value| ack(&value));
    f.config["endpoint"] = json!(endpoint);
    hook(&f, "native-parent", None);
    linked(&another.join().unwrap(), 0, None);
}

#[test]
fn slow_hook_transport_does_not_block_the_next_workflow_start() {
    let mut f = fixture();
    let (tx, rx) = std::sync::mpsc::channel();
    let mut delayed = false;
    let (origin, server) = server_at(4, "plans|progress", move |value| {
        if value.get("event_id").is_some() && !delayed {
            delayed = true;
            tx.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        ack(&value)
    });
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    let mut args = f.args();
    args[0] = "codex-hook".into();
    let input=serde_json::to_vec(&json!({"hook_event_name":"Stop","session_id":"native-parent","cwd":f.root,"turn_id":"turn-slow","last_assistant_message":"Slow summary."})).unwrap();
    args[0] = "codex-hook-context".into();
    assert_eq!(silkboard_publish::run(&args, input.as_slice()), Ok("{}"));
    args[0] = "codex-hook".into();
    let worker = std::thread::spawn(move || {
        silkboard_publish::run(&args, input.as_slice())
    });
    rx.recv_timeout(std::time::Duration::from_secs(3)).unwrap();
    let start = plan(&f, "start", &draft(B));
    assert_eq!(worker.join().unwrap(), Ok("{}"));
    // Complete the HTTP fixture even if the regression fires; retain and
    // assert the original result rather than hiding it with this cleanup.
    if start.is_err() {
        success(&f, "start", &draft(B));
    }
    hook(&f, "native-parent", None);
    let reports = server.join().unwrap();
    assert_eq!(start, Ok("Task plan acknowledged"));
    linked(&reports, 1, Some(A));
    linked(&reports, 3, Some(B));
}
