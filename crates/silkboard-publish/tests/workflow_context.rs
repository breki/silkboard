//! Explicit local workflow selection supplies native hook associations.
#![cfg(unix)]
mod support;
#[path = "support/workflow.rs"]
mod workflow;
use serde_json::json;
use support::server_at;
use workflow::*;
#[test]
fn delayed_summary_keeps_capture_time_workflow() {
    let mut f = fixture();
    let (origin, server) = server_at(3, "plans|progress", |v| ack(&v));
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    let payload = json!({"hook_event_name":"Stop","turn_id":"turn-a","session_id":"native-parent","cwd":f.root});
    let mut args = f.args();
    args[0] = "codex-hook-context".into();
    assert_eq!(
        silkboard_publish::run(&args, payload.to_string().as_bytes()),
        Ok("{}")
    );
    success(&f, "start", &draft(B));
    args[0] = "codex-hook".into();
    assert_eq!(
        silkboard_publish::run(&args, payload.to_string().as_bytes()),
        Ok("{}")
    );
    linked(&server.join().unwrap(), 2, Some(A));
}
#[test]
fn acknowledged_plan_start_links_automatic_summary_without_trusting_hook_task_id()
 {
    let mut f = fixture();
    let (origin, server) = server_at(2, "plans|progress", |value| ack(&value));
    f.config["endpoint"] = json!(origin);
    assert_eq!(plan(&f, "start", &draft(A)), Ok("Task plan acknowledged"));
    assert_eq!(
        plan(&f, "start", &draft(A)),
        Err(silkboard_publish::Error::Conflict)
    );
    hook(&f, "native-parent", None);
    let reports = server.join().unwrap();
    assert_eq!(reports[1]["task_id"], A);
    assert_eq!(reports[1]["message"], "Work completed.");
}

#[test]
fn completion_clear_select_and_task_switch_are_explicit() {
    let mut f = fixture();
    let (origin, server) = server_at(13, "plans|progress", |value| ack(&value));
    f.config["endpoint"] = json!(origin);
    let mut a = draft(A);
    success(&f, "start", &a);
    hook(&f, "native-parent", None);
    a["steps"][0]["state"] = json!("done");
    success(&f, "publish", &a);
    hook(&f, "native-parent", None);
    success(&f, "clear", &choice(None, reporter("native-parent")));
    hook(&f, "native-parent", None);
    success(&f, "publish", &a);
    hook(&f, "native-parent", None);
    success(&f, "select", &choice(Some(A), reporter("native-parent")));
    success(&f, "publish", &a);
    hook(&f, "native-parent", None);
    success(&f, "start", &draft(B));
    hook(&f, "native-parent", None);
    success(&f, "publish", &a);
    hook(&f, "native-parent", None);
    let reports = server.join().unwrap();
    for (index, id) in [
        (1, Some(A)),
        (3, Some(A)),
        (4, None),
        (6, None),
        (8, Some(A)),
        (10, Some(B)),
        (12, Some(B)),
    ] {
        linked(&reports, index, id);
    }
}
#[test]
fn failed_new_task_submission_cannot_reuse_previous_context() {
    let mut f = fixture();
    let (origin, server) = server_at(6, "plans|progress", |v| {
        if v.get("event_id").is_none()
            && v["task_id"] == B
            && v["sequence"] == "1"
        {
            (503, "{}".into())
        } else {
            ack(&v)
        }
    });
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    hook(&f, "native-parent", None);
    assert_eq!(
        plan(&f, "start", &draft(B)),
        Err(silkboard_publish::Error::Rejected)
    );
    hook(&f, "native-parent", None);
    success(&f, "publish", &draft(B));
    hook(&f, "native-parent", None);
    let reports = server.join().unwrap();
    linked(&reports, 1, Some(A));
    linked(&reports, 3, None);
    linked(&reports, 5, Some(B));
}
#[test]
fn handoff_requires_explicit_selection_and_never_inherits_parent_context() {
    let mut f = fixture();
    let (origin, server) = server_at(14, "plans|progress", |value| ack(&value));
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    hook(&f, "native-parent", None);
    transfer(&f, reporter("native-parent"), reporter("replacement"));
    hook(&f, "native-parent", None);
    success(&f, "select", &choice(Some(A), reporter("replacement")));
    hook(&f, "replacement", None);
    let mut next = draft(A);
    next["reporter"] = reporter("replacement");
    success(&f, "publish", &next);
    hook(&f, "replacement", None);
    hook(&f, "native-parent", Some("child"));
    let mut child = reporter("native-parent");
    child["subagent_id"] = json!("child");
    child["parent_session_id"] = json!("native-parent");
    transfer(&f, reporter("replacement"), child.clone());
    success(&f, "select", &choice(Some(A), child.clone()));
    next["reporter"] = child.clone();
    success(&f, "publish", &next);
    hook(&f, "native-parent", Some("child"));
    hook(&f, "replacement", None);
    transfer(&f, child, reporter("native-parent"));
    success(&f, "publish", &draft(A));
    hook(&f, "native-parent", None);
    success(&f, "select", &choice(Some(A), reporter("native-parent")));
    success(&f, "publish", &draft(A));
    hook(&f, "native-parent", None);
    let reports = server.join().unwrap();
    for (index, id) in [
        (1, Some(A)),
        (2, None),
        (3, None),
        (5, Some(A)),
        (6, None),
        (8, Some(A)),
        (9, None),
        (11, None),
        (13, Some(A)),
    ] {
        linked(&reports, index, id);
    }
}
#[test]
fn disabled_reporting_and_other_native_identities_do_not_use_context() {
    let mut f = fixture();
    let (origin, server) = server_at(8, "plans|progress", |value| ack(&value));
    f.config["endpoint"] = json!(origin);
    success(&f, "start", &draft(A));
    hook(&f, "unrelated", None);
    hook(&f, "native-parent", Some("child"));
    f.config["codex"]["hostname"] = json!("another-host");
    hook(&f, "native-parent", None);
    f.config["codex"]["hostname"] = json!("dev");
    f.config["codex"]["plan_reporting"] = json!(false);
    hook(&f, "native-parent", None);
    f.config["codex"]["plan_reporting"] = json!(true);
    hook(&f, "native-parent", None);
    f.config["codex"]["plan_reporting"] = json!(false);
    success(&f, "start", &draft(B));
    f.config["codex"]["plan_reporting"] = json!(true);
    hook(&f, "native-parent", None);
    let reports = server.join().unwrap();
    for index in 1..5 {
        linked(&reports, index, None);
    }
    linked(&reports, 5, Some(A));
    linked(&reports, 7, None);
}
