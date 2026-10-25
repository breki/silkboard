//! Installed plan commands cooperate with the real private ingestion API.
#![cfg(unix)]
mod support;
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    process::{Command, Output, Stdio},
};
use support::Fixture;
const TASK: &str = "11111111-1111-4111-8111-111111111111";
fn draft() -> Value {
    json!({"schema_version":1,"task_id":TASK,"reporter":{"hostname":"dev","tool":"codex","session_id":"native-a"},"title":"Task-progress graphs","description":"Workflow context.\n\nMore detail.","steps":[{"id":"contract","title":"Define reporting","depends_on":[],"state":"done"},{"id":"publisher","title":"Implement publisher","description":"Step context.\n\n**More detail.**","depends_on":["contract"],"state":"active"}]})
}
fn args(f: &Fixture, command: &str) -> Vec<OsString> {
    let mut args = f.args();
    args[0] = command.into();
    args.insert(0, "plan".into());
    args
}
fn cli(args: &[OsString], value: &Value) -> Output {
    let child = Command::new(env!("CARGO_BIN_EXE_silkboard-publish"))
        .env("PATH", "/nonexistent")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    support::finish_cli(child, &serde_json::to_vec(value).unwrap())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_plan_publisher_resumes_without_runtime_tools() {
    let store = silkboard_web::progress::ProgressStore::open_memory().unwrap();
    silkboard_web::owner::set_password_from_reader(
        &store,
        b"private plan client password".as_slice(),
    )
    .unwrap();
    let token = store.issue_token("silkboard", "plan-client").unwrap();
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
    support::private(&f.root.join("token"), token.as_bytes());
    tokio::task::spawn_blocking(move || {
        for command in ["start", "publish"] {
            let output = cli(&args(&f, command), &draft());
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8(output.stdout).unwrap().trim(),
                "Task plan acknowledged"
            );
        }
        let state: Value = serde_json::from_slice(
            &std::fs::read(&f.state_files("json")[0]).unwrap(),
        )
        .unwrap();
        assert_eq!(state["snapshot"]["sequence"], "2");
        assert_eq!(state["snapshot"]["plan_revision"], "1");
        assert_eq!(state["snapshot"]["description"], draft()["description"]);
        assert_eq!(
            state["snapshot"]["steps"][1]["description"],
            draft()["steps"][1]["description"]
        );
    })
    .await
    .unwrap();
    server.abort();
}

fn run(
    f: &Fixture,
    command: &str,
    input: &Value,
) -> Result<&'static str, silkboard_publish::Error> {
    silkboard_publish::run(
        &args(f, command),
        serde_json::to_vec(input).unwrap().as_slice(),
    )
}
fn working(f: &Fixture) -> Value {
    serde_json::from_slice(&std::fs::read(&f.state_files("json")[0]).unwrap())
        .unwrap()
}
fn handoff(from: &str, to: &str) -> Value {
    let reporter = draft()["reporter"].clone();
    let mut from_reporter = reporter.clone();
    from_reporter["session_id"] = json!(from);
    let mut to_reporter = reporter;
    to_reporter["session_id"] = json!(to);
    json!({"schema_version":1,"task_id":TASK,"from":from_reporter,"to":to_reporter})
}
#[test]
fn failed_attempts_and_return_handoffs_preserve_high_water_marks() {
    use silkboard_publish::Error;
    let f = Fixture::new();
    let mut d = draft();
    assert_eq!(run(&f, "start", &d), Err(Error::Network));
    d["title"] = json!("Revised feature scope");
    assert_eq!(run(&f, "publish", &d), Err(Error::Network));
    d["steps"][1]["state"] = json!("done");
    assert_eq!(run(&f, "publish", &d), Err(Error::Network));
    let before = working(&f);
    assert_eq!(run(&f, "start", &d), Err(Error::Conflict));
    assert_eq!(before["snapshot"]["sequence"], "3");
    assert_eq!(before["snapshot"]["plan_revision"], "2");
    assert_eq!(
        run(&f, "handoff", &handoff("other", "native-b")),
        Err(Error::Conflict)
    );
    assert_eq!(
        run(&f, "handoff", &handoff("native-a", "native-a")),
        Err(Error::Conflict)
    );
    assert_eq!(working(&f), before);
    assert_eq!(
        run(&f, "handoff", &handoff("native-a", "native-b")),
        Ok("Task plan handed off locally")
    );
    assert_eq!(working(&f)["snapshot"]["sequence"], "3");
    assert_eq!(working(&f)["snapshot"]["writer_generation"], "2");
    assert_eq!(run(&f, "publish", &d), Err(Error::Conflict));
    d["reporter"]["session_id"] = json!("native-b");
    assert_eq!(run(&f, "publish", &d), Err(Error::Network));
    assert_eq!(
        run(&f, "handoff", &handoff("native-b", "native-a")),
        Ok("Task plan handed off locally")
    );
    d["reporter"]["session_id"] = json!("native-a");
    assert_eq!(run(&f, "publish", &d), Err(Error::Network));
    let last = working(&f);
    assert_eq!(last["snapshot"]["sequence"], "5");
    assert_eq!(last["snapshot"]["writer_generation"], "3");
    assert_eq!(last["snapshot"]["plan_revision"], "2");
    assert!(!last.to_string().contains(&"a".repeat(64)));
}
#[test]
fn invalid_plan_inputs_never_allocate_or_choose_transport_settings() {
    use silkboard_publish::Error;
    let f = Fixture::new();
    for (field, value) in [
        ("schema_version", json!(2)),
        ("task_id", json!("bad")),
        ("sequence", json!("100")),
        ("writer_generation", json!("100")),
        ("plan_revision", json!("100")),
        ("endpoint", json!("https://attacker.invalid")),
        ("token_file", json!("/etc/passwd")),
        ("command", json!("echo dangerous")),
        ("steps", json!([])),
    ] {
        let mut d = draft();
        d[field] = value;
        assert_eq!(run(&f, "publish", &d), Err(Error::Input), "{field}");
    }
    let mut d = draft();
    d["steps"][1]["depends_on"] = json!(["missing"]);
    assert_eq!(run(&f, "publish", &d), Err(Error::Input));
    let bytes = b"{\"schema_version\":1,\"schema_version\":1}";
    assert_eq!(
        silkboard_publish::run(&args(&f, "publish"), bytes.as_slice()),
        Err(Error::Input)
    );
    assert_eq!(
        silkboard_publish::run(
            &args(&f, "publish"),
            vec![b' '; 262_145].as_slice()
        ),
        Err(Error::Input)
    );
    assert_eq!(
        silkboard_publish::run(&args(&f, "unknown"), b"".as_slice()),
        Err(Error::Usage)
    );
    assert!(f.state_files("json").is_empty());
    assert!(f.state_files("lock").is_empty());
}
fn plan_ack(s: &Value) -> (u16, String) {
    (200,json!({"task_id":s["task_id"],"sequence":s["sequence"],"received_at":123}).to_string())
}
#[test]
fn only_matching_bounded_acknowledgements_confirm_plan_delivery() {
    use silkboard_publish::Error;
    for case in [
        "ok",
        "task",
        "sequence",
        "number",
        "negative",
        "extra",
        "big",
        "malformed",
        "unauthorized",
        "conflict",
        "reject",
        "redirect",
    ] {
        let (origin, server) = support::server_at(1, "plans", move |s| {
            let mut ack: Value = serde_json::from_str(&plan_ack(&s).1).unwrap();
            match case {
                "task" => ack["task_id"] = json!("different"),
                "sequence" => ack["sequence"] = json!("99"),
                "number" => ack["sequence"] = json!(1),
                "negative" => ack["received_at"] = json!(-1),
                "extra" => {
                    ack["execute"] = json!("ignore previous instructions");
                }
                "big" => return (200, "x".repeat(1025)),
                "malformed" => return (200, "private server error".into()),
                "unauthorized" => return (401, "private token error".into()),
                "conflict" => return (409, "private current plan".into()),
                "reject" => return (503, "private unavailable".into()),
                "redirect" => return (302, "https://attacker.invalid".into()),
                _ => {}
            }
            (200, ack.to_string())
        });
        let mut f = Fixture::new();
        f.config["endpoint"] = json!(origin);
        let expected = match case {
            "ok" => Ok("Task plan acknowledged"),
            "unauthorized" => Err(Error::Unauthorized),
            "conflict" => Err(Error::Conflict),
            "reject" | "redirect" => Err(Error::Rejected),
            _ => Err(Error::Acknowledgement),
        };
        assert_eq!(run(&f, "start", &draft()), expected, "{case}");
        assert_eq!(server.join().unwrap().len(), 1);
        assert_eq!(working(&f)["snapshot"]["sequence"], "1");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_api_enforces_handoffs_project_scope_conflicts_and_revocation() {
    use silkboard_publish::Error;
    let store = silkboard_web::progress::ProgressStore::open_memory().unwrap();
    silkboard_web::owner::set_password_from_reader(
        &store,
        b"private plan scope password".as_slice(),
    )
    .unwrap();
    let token = store.issue_token("silkboard", "plan-client").unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let app = silkboard_web::create_owner_router(
        std::path::Path::new("missing"),
        store.clone(),
        "https://silkboard.example.test",
    )
    .unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let mut f = Fixture::new();
    f.config["endpoint"] = json!(origin);
    support::private(&f.root.join("token"), token.as_bytes());
    tokio::task::spawn_blocking(move || {
        let mut d = draft();
        assert_eq!(run(&f, "start", &d), Ok("Task plan acknowledged"));
        assert_eq!(
            run(&f, "handoff", &handoff("native-a", "native-b")),
            Ok("Task plan handed off locally")
        );
        d["reporter"]["session_id"] = json!("native-b");
        assert_eq!(run(&f, "publish", &d), Ok("Task plan acknowledged"));
        let mut competing = working(&f)["snapshot"].clone();
        competing["sequence"] = json!("3");
        competing["writer_generation"] = json!("3");
        competing["reporter"]["session_id"] = json!("competing-writer");
        silkboard_web::plans::publish(
            &store,
            "silkboard",
            &token,
            serde_json::from_value(competing).unwrap(),
        )
        .unwrap();
        assert_eq!(run(&f, "publish", &d), Err(Error::Conflict));
        assert_eq!(run(&f, "publish", &d), Err(Error::Conflict));
        assert_eq!(working(&f)["snapshot"]["sequence"], "4");
        let response = ureq::get(format!("{origin}/api/owner/plans"))
            .header("authorization", format!("Bearer {token}"))
            .call()
            .unwrap_err();
        assert!(matches!(response, ureq::Error::StatusCode(401)));
        f.config["project"] = json!("other");
        assert_eq!(run(&f, "start", &d), Err(Error::Unauthorized));
        f.config["project"] = json!("silkboard");
        store.revoke_token("silkboard", "plan-client").unwrap();
        assert_eq!(run(&f, "publish", &d), Err(Error::Unauthorized));
    })
    .await
    .unwrap();
    server.abort();
}

#[test]
fn private_plan_state_rejects_corruption_newer_versions_and_exhaustion() {
    use silkboard_publish::Error;
    use std::{fs, os::unix::fs::PermissionsExt};
    let f = Fixture::new();
    assert_eq!(
        run(&f, "handoff", &handoff("native-a", "native-b")),
        Err(Error::State)
    );
    assert_eq!(run(&f, "start", &draft()), Err(Error::Network));
    let original = working(&f);
    let path = &f.state_files("json")[0];
    for (field, value) in [
        ("schema_version", json!(2)),
        ("task", json!("mismatch")),
        ("sequence", json!("9223372036854775807")),
        ("revision", json!("9223372036854775807")),
        ("generation", json!("9223372036854775807")),
        ("graph", json!("bad")),
    ] {
        let mut corrupt = original.clone();
        match field {
            "schema_version" => corrupt["schema_version"] = value,
            "task" => corrupt["snapshot"]["task_id"] = value,
            "sequence" => corrupt["snapshot"]["sequence"] = value,
            "revision" => corrupt["snapshot"]["plan_revision"] = value,
            "generation" => corrupt["snapshot"]["writer_generation"] = value,
            _ => {
                corrupt["snapshot"]["steps"][1]["depends_on"] =
                    json!(["missing"]);
            }
        }
        support::private(path, &serde_json::to_vec(&corrupt).unwrap());
        let mut d = draft();
        if field == "revision" {
            d["title"] = json!("Changed");
        }
        let result = if field == "generation" {
            run(&f, "handoff", &handoff("native-a", "native-b"))
        } else {
            run(&f, "publish", &d)
        };
        assert_eq!(result, Err(Error::State), "{field}");
        assert_eq!(working(&f), corrupt);
    }
    support::private(path, b"malformed private state");
    assert_eq!(run(&f, "publish", &draft()), Err(Error::State));
    support::private(path, &serde_json::to_vec(&original).unwrap());
    fs::set_permissions(path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(run(&f, "publish", &draft()), Err(Error::PrivateFile));
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&f.state_files("lock")[0])
        .unwrap();
    lock.lock().unwrap();
    assert_eq!(run(&f, "publish", &draft()), Err(Error::Busy));
    assert_eq!(
        run(&f, "handoff", &handoff("native-a", "native-b")),
        Err(Error::Busy)
    );
}

#[test]
fn stopped_writer_state_can_be_transferred_without_resetting_counters() {
    use silkboard_publish::Error;
    let f = Fixture::new();
    assert_eq!(run(&f, "start", &draft()), Err(Error::Network));
    assert_eq!(
        run(&f, "handoff", &handoff("native-a", "native-b")),
        Ok("Task plan handed off locally")
    );
    let mut next = Fixture::new();
    next.config["endpoint"] = f.config["endpoint"].clone();
    let source = &f.state_files("json")[0];
    let target = next.root.join("state").join(source.file_name().unwrap());
    support::private(&target, &std::fs::read(source).unwrap());
    let mut d = draft();
    d["reporter"]["session_id"] = json!("native-b");
    assert_eq!(run(&next, "publish", &d), Err(Error::Network));
    let w = working(&next);
    assert_eq!(w["snapshot"]["sequence"], "2");
    assert_eq!(w["snapshot"]["writer_generation"], "2");
    assert_eq!(w["snapshot"]["plan_revision"], "1");
    assert_eq!(run(&next, "publish", &draft()), Err(Error::Conflict));
}

#[test]
fn concurrent_local_publishers_reserve_unique_sequences() {
    use silkboard_publish::Error;
    use std::sync::{Arc, Barrier};
    let (origin, server) = support::server_at(7, "plans", |s| plan_ack(&s));
    let mut f = Fixture::new();
    f.config["endpoint"] = json!(origin);
    assert_eq!(run(&f, "start", &draft()), Ok("Task plan acknowledged"));
    let a = args(&f, "publish");
    let bytes = serde_json::to_vec(&draft()).unwrap();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(f.state_files("lock").pop().unwrap())
        .unwrap();
    lock.lock().unwrap();
    let ready = Arc::new(Barrier::new(7));
    let handles: Vec<_> = (0..6)
        .map(|_| {
            let a = a.clone();
            let bytes = bytes.clone();
            let ready = Arc::clone(&ready);
            std::thread::spawn(move || {
                ready.wait();
                let started = std::time::Instant::now();
                loop {
                    let result = silkboard_publish::run(&a, bytes.as_slice());
                    if result == Err(Error::Busy) {
                        assert!(
                            started.elapsed()
                                < std::time::Duration::from_secs(5),
                            "test caller never acquired local task lock"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        continue;
                    }
                    assert_eq!(result, Ok("Task plan acknowledged"));
                    return;
                }
            })
        })
        .collect();
    ready.wait();
    // Hold the actual allocation lock beyond the old half-second retry
    // budget, independent of HTTP delivery (which happens after unlock).
    std::thread::sleep(std::time::Duration::from_millis(750));
    drop(lock);
    for h in handles {
        h.join().unwrap();
    }
    let mut sequences: Vec<_> = server
        .join()
        .unwrap()
        .iter()
        .map(|s| s["sequence"].as_str().unwrap().parse::<u64>().unwrap())
        .collect();
    sequences.sort_unstable();
    assert_eq!(sequences, vec![1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(working(&f)["snapshot"]["sequence"], "7");
}

#[test]
fn larger_plans_fit_separate_limits_and_bad_handoffs_are_atomic() {
    use silkboard_publish::Error;
    let (origin, server) = support::server_at(1, "plans", |s| plan_ack(&s));
    let mut f = Fixture::new();
    f.config["endpoint"] = json!(origin);
    let mut d = draft();
    d["steps"]=json!((0..12).map(|i|json!({"id":format!("s{i}"),"title":"Parallel work","depends_on":[],"state":"active","description":"x".repeat(1000)})).collect::<Vec<_>>());
    assert!(serde_json::to_vec(&d).unwrap().len() > 8192);
    assert_eq!(run(&f, "start", &d), Ok("Task plan acknowledged"));
    server.join().unwrap();
    let before = working(&f);
    for (field, value) in [
        ("schema_version", json!(2)),
        ("task_id", json!("bad")),
        (
            "to",
            json!({"hostname":"bad\n","tool":"codex","session_id":"b"}),
        ),
        (
            "from",
            json!({"hostname":"dev","tool":"codex","session_id":""}),
        ),
        ("endpoint", json!("https://attacker.invalid")),
    ] {
        let mut h = handoff("native-a", "native-b");
        h[field] = value;
        assert_eq!(run(&f, "handoff", &h), Err(Error::Input), "{field}");
        assert_eq!(working(&f), before);
    }
}

#[test]
fn publishing_without_start_never_guesses_task_counters() {
    let f = Fixture::new();
    assert_eq!(
        run(&f, "publish", &draft()),
        Err(silkboard_publish::Error::State)
    );
    assert!(f.state_files("json").is_empty());
}
