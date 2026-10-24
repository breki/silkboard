//! Observable validation and normalization rules for the shared wire contract.
use serde_json::{Value, json};
use silkboard_plan::{
    Counter, InvalidPlan, MAX_SNAPSHOT_BYTES, Snapshot, TaskState,
    valid_step_id, valid_task_id,
};
fn fixture() -> Value {
    json!({"schema_version":1,"task_id":"11111111-1111-4111-8111-111111111111","sequence":"1","plan_revision":"1","writer_generation":"1","reporter":{"hostname":"dev","tool":"codex","session_id":"native"},"title":"Plan","steps":[{"id":"a","title":"Prerequisite","depends_on":[],"state":"done"},{"id":"b","title":"Work","depends_on":["a"],"state":"active"}]})
}
fn normalize(v: Value) -> Result<Snapshot, InvalidPlan> {
    serde_json::from_value::<Snapshot>(v)
        .map_err(|_| InvalidPlan)?
        .normalized()
}
#[test]
fn counters_are_exact_canonical_strings() {
    for v in [
        json!(0),
        json!(1),
        json!(null),
        json!(""),
        json!("0"),
        json!("01"),
        json!("+1"),
        json!("-1"),
        json!("1.0"),
        json!("1 "),
        json!("9223372036854775808"),
        json!("12345678901234567890"),
    ] {
        assert!(serde_json::from_value::<Counter>(v).is_err());
    }
    for n in [1, 9_007_199_254_740_993, i64::MAX] {
        let c = Counter::new(n).unwrap();
        assert_eq!(c.get(), n);
        assert_eq!(serde_json::to_value(c).unwrap(), json!(n.to_string()));
        assert_eq!(
            serde_json::from_value::<Counter>(json!(n.to_string())).unwrap(),
            c
        );
    }
    assert!(Counter::new(-1).is_err());
    assert!(Counter::new(0).is_err());
    assert_eq!(InvalidPlan.to_string(), "invalid task plan");
}
#[test]
fn reported_states_and_definition_are_separate() {
    let base = normalize(fixture()).unwrap();
    assert_eq!(base.overall_state(), TaskState::Active);
    assert_eq!(base.counts().done, 1);
    let mut v = fixture();
    v["steps"][1]["state"] = json!("done");
    let done = normalize(v.clone()).unwrap();
    assert_eq!(done.overall_state(), TaskState::Completed);
    assert!(base.same_definition(&done));
    v["steps"][1]["state"] = json!("blocked");
    v["steps"][1]["blocked_reason"] = json!("Waiting\nfor feedback");
    assert_eq!(normalize(v).unwrap().overall_state(), TaskState::Blocked);
    let mut v = fixture();
    v["steps"][0]["state"] = json!("pending");
    v["steps"][1]["state"] = json!("pending");
    assert_eq!(normalize(v).unwrap().overall_state(), TaskState::Pending);
    let mut v = fixture();
    v["steps"].as_array_mut().unwrap().push(json!({"id":"c","title":"Parallel","depends_on":["a"],"state":"active"}));
    let multi = normalize(v).unwrap();
    assert_eq!(multi.counts().active, 2);
    assert!(!multi.same_definition(&base));
    let mut named = base.clone();
    named.reporter.session_name = Some("Renamed".into());
    assert!(named.reporter.same_identity(&base.reporter));
    named.reporter.session_id = "different".into();
    assert!(!named.reporter.same_identity(&base.reporter));
    for field in ["title", "description"] {
        let mut changed = fixture();
        changed[field] = json!("Changed");
        assert!(!normalize(changed).unwrap().same_definition(&base));
    }
}
#[test]
fn ids_text_and_optionals_preserve_boundaries() {
    for id in ["", "A", "_a", "a b", "é", "a/b"] {
        assert!(!valid_step_id(id));
    }
    assert!(valid_step_id("0-a_b"));
    assert!(!valid_step_id(&"a".repeat(65)));
    for id in [
        "",
        "11111111-1111-1111-8111-111111111111",
        "11111111-1111-4111-7111-111111111111",
        "11111111_1111-4111-8111-111111111111",
        "aaaaaaaa-aaaa-4aaa-Aaaa-aaaaaaaaaaaa",
    ] {
        assert!(!valid_task_id(id));
    }
    for (field, value) in [
        ("schema_version", json!(2)),
        ("task_id", json!("bad")),
        ("title", json!(" \t")),
        ("title", json!("a\nb")),
        ("title", json!("x".repeat(161))),
        ("description", json!("x".repeat(2001))),
        ("description", json!("bad\u{7f}")),
    ] {
        let mut v = fixture();
        v[field] = value;
        assert!(normalize(v).is_err(), "{field}");
    }
    for field in [
        "hostname",
        "tool",
        "session_id",
        "subagent_id",
        "parent_session_id",
        "session_name",
    ] {
        let mut v = fixture();
        v["reporter"][field] = json!("bad\nvalue");
        assert!(normalize(v).is_err(), "{field}");
    }
    let mut v = fixture();
    v["reporter"]["parent_session_id"] = json!("parent");
    assert!(normalize(v.clone()).is_err());
    v["reporter"]["subagent_id"] = json!("child");
    v["description"] = json!("Markdown\r\n\ttext");
    v["steps"][1]["report"] = json!("Line\n2");
    assert!(normalize(v).is_ok());
    for (field, value) in [
        ("id", json!("BAD")),
        ("title", json!("")),
        ("description", json!(" ")),
        ("report", json!("x".repeat(2001))),
        ("blocked_reason", json!("Not blocked")),
    ] {
        let mut v = fixture();
        v["steps"][1][field] = value;
        assert!(normalize(v).is_err(), "{field}");
    }
    let mut blocked = fixture();
    blocked["steps"][1]["state"] = json!("blocked");
    assert!(normalize(blocked.clone()).is_err());
    blocked["steps"][1]["blocked_reason"] = json!("x".repeat(1001));
    assert!(normalize(blocked).is_err());
    let mut a = fixture();
    let mut b = fixture();
    a["description"] = Value::Null;
    b["reporter"]["session_name"] = Value::Null;
    assert_eq!(normalize(a).unwrap(), normalize(b).unwrap());
}
#[test]
fn graph_validation_rejects_cycles_duplicates_and_invalid_states() {
    let mut empty = fixture();
    empty["steps"] = json!([]);
    assert!(normalize(empty).is_err());
    for deps in [
        json!(["missing"]),
        json!(["b"]),
        json!(["a", "a"]),
        json!(vec!["a"; 33]),
    ] {
        let mut v = fixture();
        v["steps"][1]["depends_on"] = deps;
        assert!(normalize(v).is_err());
    }
    let mut v = fixture();
    v["steps"][1]["id"] = json!("a");
    assert!(normalize(v).is_err());
    let mut v = fixture();
    v["steps"][0]["depends_on"] = json!(["b"]);
    v["steps"][0]["state"] = json!("pending");
    v["steps"][1]["state"] = json!("pending");
    assert!(normalize(v).is_err());
    for state in ["active", "done"] {
        let mut v = fixture();
        v["steps"][0]["state"] = json!("pending");
        v["steps"][1]["state"] = json!(state);
        assert!(normalize(v).is_err());
    }
    let mut v = fixture();
    v["steps"].as_array_mut().unwrap().push(
        json!({"id":"c","title":"Other root","depends_on":[],"state":"done"}),
    );
    v["steps"][1]["depends_on"] = json!(["c", "a"]);
    let a = normalize(v.clone()).unwrap();
    v["steps"][1]["depends_on"] = json!(["a", "c"]);
    let b = normalize(v).unwrap();
    assert_eq!(a, b);
    assert!(a.same_definition(&b));
    for count in [128, 129] {
        let mut v = fixture();
        v["steps"]=json!((0..count).map(|i|json!({"id":format!("s{i}"),"title":"Root","depends_on":[],"state":"pending"})).collect::<Vec<_>>());
        assert_eq!(normalize(v).is_ok(), count == 128);
    }
    let mut v = fixture();
    v["steps"]=json!((0..50).map(|i|json!({"id":format!("s{i}"),"title":"Step","depends_on":(0..i.min(16)).map(|j|format!("s{j}")).collect::<Vec<_>>(),"state":"pending"})).collect::<Vec<_>>());
    assert!(normalize(v).is_err());
}
#[test]
fn metadata_and_encoding_remain_bounded_and_strict() {
    for repo in ["", "a", "a/b/c", "../x", "a/..", "a/%2f", "a/é", "a//b"] {
        let mut v = fixture();
        v["github_issue"] = json!({"repository":repo,"number":"1"});
        assert!(normalize(v).is_err());
    }
    let mut v = fixture();
    v["github_issue"] = json!({"repository":"breki/silkboard","number":"73"});
    let with_issue = normalize(v.clone()).unwrap();
    assert!(!with_issue.same_definition(&normalize(fixture()).unwrap()));
    v["github_issue"]["repository"] = json!(format!("{}/a", "x".repeat(100)));
    assert!(normalize(v).is_err());
    for place in ["root", "reporter", "step", "issue"] {
        let mut v = fixture();
        match place {
            "root" => v["unknown"] = json!(1),
            "reporter" => v["reporter"]["unknown"] = json!(1),
            "step" => v["steps"][0]["unknown"] = json!(1),
            _ => {
                v["github_issue"] =
                    json!({"repository":"a/b","number":"1","unknown":1});
            }
        }
        assert!(normalize(v).is_err());
    }
    let duplicate = serde_json::to_string(&fixture()).unwrap().replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    assert!(serde_json::from_str::<Snapshot>(&duplicate).is_err());
    let mut v = fixture();
    v["steps"]=json!((0..128).map(|i|json!({"id":format!("s{i}"),"title":"Step","description":"x".repeat(2000),"report":"y".repeat(2000),"depends_on":[],"state":"pending"})).collect::<Vec<_>>());
    assert!(serde_json::to_vec(&v).unwrap().len() > MAX_SNAPSHOT_BYTES);
    assert!(normalize(v).is_err());
}
