//! Actual HTTP requests, process restarts and private local state behavior.
#![cfg(unix)]
mod support;
use serde_json::json;
use silkboard_publish::Error;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    time::{Duration, Instant},
};
use support::{Fixture, ack, private, report, server};

#[test]
fn publishes_independent_native_identities_and_resumes_without_a_queue() {
    let mut f = Fixture::new();
    let (origin, handle) = server(5, |event| ack(&event));
    f.config["endpoint"] = json!(origin);
    for item in [
        report(),
        report(),
        json!({"tool":"codex","hostname":"dev","session_id":"native-1","subagent_id":"child","parent_session_id":"native-1","session_name":"Review","message":"Reviewing"}),
        json!({"tool":"claude","hostname":"dev","session_id":"native-1","message":"Separate tool"}),
        report(),
    ] {
        assert_eq!(f.run(&item), Ok("Progress acknowledged"));
    }
    let events = handle.join().unwrap();
    assert_eq!(
        events
            .iter()
            .map(|e| e["sequence"].as_i64().unwrap())
            .collect::<Vec<_>>(),
        [1, 2, 1, 1, 3]
    );
    assert_ne!(events[0]["event_id"], events[1]["event_id"]);
    assert_eq!(events[2]["parent_session_id"], "native-1");
    for file in f.state_files("json") {
        let text = fs::read_to_string(file).unwrap();
        assert!(
            !text.contains("message")
                && !text.contains("Running")
                && !text.contains(&"a".repeat(64))
        );
    }
}

#[test]
fn statuses_and_malformed_acknowledgements_never_become_success_or_instructions()
 {
    for (status, body, expected) in [
        (401, "secret error", Error::Unauthorized),
        (409, "secret error", Error::Conflict),
        (503, "secret error", Error::Rejected),
        (302, "redirect", Error::Rejected),
        (204, "", Error::Rejected),
        (200, "not json", Error::Acknowledgement),
        (
            200,
            "{\"event_id\":\"wrong\",\"received_at\":1}",
            Error::Acknowledgement,
        ),
        (
            200,
            "{\"event_id\":\"wrong\",\"received_at\":-1}",
            Error::Acknowledgement,
        ),
    ] {
        let mut f = Fixture::new();
        let (origin, h) = server(1, move |_| (status, body.to_owned()));
        f.config["endpoint"] = json!(origin);
        assert_eq!(f.run(&report()), Err(expected));
        h.join().unwrap();
    }
    for kind in ["negative", "extra", "large"] {
        let mut f = Fixture::new();
        let (origin, h) = server(1, move |event| {
            let mut value =
                json!({"event_id":event["event_id"],"received_at":1});
            match kind {
                "negative" => value["received_at"] = json!(-1),
                "extra" => value["execute"] = json!("do something"),
                _ => return (200, "x".repeat(2000)),
            }
            (200, value.to_string())
        });
        f.config["endpoint"] = json!(origin);
        assert_eq!(f.run(&report()), Err(Error::Acknowledgement));
        h.join().unwrap();
    }
}

#[test]
fn delivery_timeout_is_bounded_and_failed_reports_are_not_queued() {
    let mut f = Fixture::new();
    let (origin, h) = server(1, |event| {
        std::thread::sleep(Duration::from_millis(3400));
        ack(&event)
    });
    f.config["endpoint"] = json!(origin);
    let start = Instant::now();
    assert_eq!(f.run(&report()), Err(Error::Network));
    assert!(start.elapsed() < Duration::from_millis(3300));
    h.join().unwrap();
    assert_eq!(f.run(&report()), Err(Error::Network));
    let counter: serde_json::Value =
        serde_json::from_slice(&fs::read(&f.state_files("json")[0]).unwrap())
            .unwrap();
    assert_eq!(counter["sequence"], 2);
    assert_eq!(counter.as_object().unwrap().len(), 3);
}

#[test]
fn unsafe_files_and_inconsistent_counter_state_fail_closed() {
    let f = Fixture::new();
    let token = f.root.join("token");
    fs::set_permissions(&token, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(f.run(&report()), Err(Error::PrivateFile));
    fs::set_permissions(&token, fs::Permissions::from_mode(0o600)).unwrap();
    let link = f.root.join("linked-token");
    symlink(&token, &link).unwrap();
    let mut f = f;
    f.config["token_file"] = json!(link);
    assert_eq!(f.run(&report()), Err(Error::PrivateFile));
    f.config["token_file"] = json!(token);
    for bad in ["short".to_owned(), "F".repeat(64), "a".repeat(67)] {
        private(&token, bad.as_bytes());
        assert_eq!(f.run(&report()), Err(Error::PrivateFile));
    }
    private(&token, "a".repeat(64).as_bytes());
    assert_eq!(f.run(&report()), Err(Error::Network));
    let counter = f.state_files("json").pop().unwrap();
    for body in [
        "garbage",
        r#"{"schema":2,"sequence":1,"parent":null}"#,
        r#"{"schema":1,"sequence":-1,"parent":null}"#,
        r#"{"schema":1,"sequence":9223372036854775807,"parent":null}"#,
    ] {
        private(&counter, body.as_bytes());
        assert_eq!(f.run(&report()), Err(Error::State));
    }
}

#[test]
fn invalid_inputs_never_create_counter_state() {
    let mut f = Fixture::new();
    for endpoint in [
        "http://remote.example",
        "https://user:secret@host",
        "https://host/",
        "https://host/path",
        "https://host?q=1",
        "https://host#secret",
        "https://HOST",
        "https://host:0",
        "invalid",
    ] {
        f.config["endpoint"] = json!(endpoint);
        assert_eq!(f.run(&report()), Err(Error::Input), "{endpoint}");
    }
    f.config["endpoint"] = json!("http://127.0.0.1:9");
    for (key, value) in [
        ("hostname", ""),
        ("tool", "bad tool"),
        ("session_id", "é"),
        ("message", "\n"),
        ("message", ""),
        ("session_name", "name\nline"),
        ("session_name", "name\tpart"),
        ("session_id", "native\nchild"),
    ] {
        let mut value_report = report();
        value_report[key] = json!(value);
        assert_eq!(f.run(&value_report), Err(Error::Input));
    }
    for (key, value) in [
        ("session_name", json!("x".repeat(121))),
        ("message", json!("é".repeat(1001))),
        ("parent_session_id", json!("parent")),
        ("unknown", json!("value")),
    ] {
        let mut value_report = report();
        value_report[key] = value;
        assert_eq!(f.run(&value_report), Err(Error::Input));
    }
    for control in (0..=159)
        .filter_map(char::from_u32)
        .filter(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        let mut value_report = report();
        value_report["message"] = json!(format!("before{control}after"));
        assert_eq!(f.run(&value_report), Err(Error::Input));
    }
    assert!(f.state_files("json").is_empty());
}

#[test]
fn fifo_configuration_is_rejected_instead_of_blocking_agent_work() {
    let f = Fixture::new();
    let fifo = f.root.join("config-pipe");
    assert!(
        std::process::Command::new("mkfifo")
            .arg("-m")
            .arg("600")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let mut child =
        std::process::Command::new(env!("CARGO_BIN_EXE_silkboard-publish"))
            .args(["publish", "--config"])
            .arg(fifo)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
    std::thread::sleep(Duration::from_millis(200));
    let status = child.try_wait().unwrap();
    if status.is_none() {
        child.kill().unwrap();
        child.wait().unwrap();
    }
    assert!(
        status.is_some_and(|s| !s.success()),
        "FIFO stalled the publishing command"
    );
}

#[test]
fn concurrent_callers_allocate_unique_sequences() {
    let mut f = Fixture::new();
    let (origin, handle) = server(8, |event| ack(&event));
    f.config["endpoint"] = json!(origin);
    let args = f.args();
    let bytes = serde_json::to_vec(&report()).unwrap();
    let round = std::sync::Barrier::new(8);
    std::thread::scope(|scope| {
        let mut children = Vec::new();
        for _ in 0..8 {
            let args = &args;
            let bytes = &bytes;
            let round = &round;
            children.push(scope.spawn(move || {
                let mut result = None;
                // All outstanding callers contend in each round. Wait for every
                // attempt to finish before retrying Busy: at least one writer
                // completes per round, regardless of disk or scheduler speed.
                for _ in 0..8 {
                    round.wait();
                    if result.is_none() {
                        let attempt =
                            silkboard_publish::run(args, bytes.as_slice());
                        if attempt != Err(Error::Busy) {
                            result = Some(attempt);
                        }
                    }
                    round.wait();
                }
                result
            }));
        }
        for child in children {
            assert_eq!(
                child.join().unwrap(),
                Some(Ok("Progress acknowledged"))
            );
        }
    });
    let mut sequences = handle
        .join()
        .unwrap()
        .iter()
        .map(|e| e["sequence"].as_i64().unwrap())
        .collect::<Vec<_>>();
    sequences.sort_unstable();
    assert_eq!(sequences, (1..=8).collect::<Vec<_>>());
}

#[test]
fn busy_call_returns_before_the_allocation_lock_is_released() {
    let f = Fixture::new();
    assert_eq!(f.run(&report()), Err(Error::Network));
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&f.state_files("lock")[0])
        .unwrap();
    lock.lock().unwrap();
    let args = f.args();
    let bytes = serde_json::to_vec(&report()).unwrap();
    std::thread::scope(|scope| {
        let (send, receive) = std::sync::mpsc::channel();
        let caller = scope.spawn(move || {
            let _ = send.send(silkboard_publish::run(&args, bytes.as_slice()));
        });
        // The lock stays held until the caller responds. Release it even if
        // this check fails, so a regression to blocking acquisition can unwind.
        let result = receive.recv_timeout(Duration::from_secs(1));
        drop(lock);
        caller.join().unwrap();
        assert_eq!(result.unwrap(), Err(Error::Busy));
    });
    assert_eq!(
        fs::read_to_string(&f.state_files("json")[0])
            .unwrap()
            .parse::<serde_json::Value>()
            .unwrap()["sequence"],
        1
    );
}

#[test]
fn allocation_lock_is_released_before_http_acknowledgement() {
    let mut f = Fixture::new();
    let (entered, waiting) = std::sync::mpsc::channel();
    let (release, held) = std::sync::mpsc::channel();
    let (origin, server) = server(1, move |event| {
        entered.send(()).unwrap();
        held.recv_timeout(Duration::from_secs(3)).unwrap();
        ack(&event)
    });
    f.config["endpoint"] = json!(origin);
    let args = f.args();
    let bytes = serde_json::to_vec(&report()).unwrap();
    std::thread::scope(|scope| {
        let caller =
            scope.spawn(|| silkboard_publish::run(&args, bytes.as_slice()));
        waiting.recv_timeout(Duration::from_secs(3)).unwrap();
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&f.state_files("lock")[0])
            .unwrap();
        let acquired = lock.try_lock();
        release.send(()).unwrap();
        assert!(
            acquired.is_ok(),
            "HTTP delivery retained the allocation lock"
        );
        assert_eq!(caller.join().unwrap(), Ok("Progress acknowledged"));
    });
    server.join().unwrap();
}

#[test]
fn parent_metadata_and_private_counter_permissions_cannot_change() {
    let f = Fixture::new();
    let mut child = report();
    child["subagent_id"] = json!("child");
    child["parent_session_id"] = json!("parent");
    assert_eq!(f.run(&child), Err(Error::Network));
    child["parent_session_id"] = json!("different-parent");
    assert_eq!(f.run(&child), Err(Error::State));
    child["parent_session_id"] = json!("parent");
    let path = &f.state_files("json")[0];
    fs::set_permissions(path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(f.run(&child), Err(Error::PrivateFile));
}

#[test]
fn command_validation_and_secret_free_diagnostics_cover_local_failures() {
    let f = Fixture::new();
    assert_eq!(
        silkboard_publish::run(&["--version".into()], &b""[..]),
        Ok(concat!(
            "silkboard-publish ",
            env!("CARGO_PKG_VERSION"),
            " (",
            env!("SILKBOARD_PUBLISH_IDENTITY"),
            ")"
        ))
    );
    for args in [
        vec![],
        vec!["publish".into()],
        vec!["--token=DO-NOT-ECHO".into()],
    ] {
        let error = silkboard_publish::run(&args, &b""[..]).unwrap_err();
        assert_eq!(error, Error::Usage);
        assert!(!error.to_string().contains("DO-NOT-ECHO"));
    }
    for bytes in [
        b"invalid DO-NOT-ECHO".to_vec(),
        vec![b'x'; 8193],
        vec![0xff],
    ] {
        let e =
            silkboard_publish::run(&f.args(), bytes.as_slice()).unwrap_err();
        assert_eq!(e, Error::Input);
        assert!(!e.to_string().contains("DO-NOT-ECHO"));
    }
    private(&f.root.join("config.json"), b"DO-NOT-ECHO malformed config");
    let args = vec![
        "publish".into(),
        "--config".into(),
        f.root.join("config.json").into(),
    ];
    assert_eq!(silkboard_publish::run(&args, &b""[..]), Err(Error::Input));
    for error in [
        Error::PrivateFile,
        Error::State,
        Error::Busy,
        Error::Network,
        Error::Unauthorized,
        Error::Conflict,
        Error::Rejected,
        Error::Acknowledgement,
    ] {
        let message = error.to_string();
        assert!(!message.contains("DO-NOT-ECHO"));
        assert!(!message.contains(&"a".repeat(64)));
        assert!(!message.contains("Running behavioral tests"));
    }
}

#[test]
fn explicit_workflow_ids_are_per_event_and_unknown_context_stays_absent() {
    let mut f = Fixture::new();
    let first = "11111111-1111-4111-8111-111111111111";
    let second = "22222222-2222-4222-8222-222222222222";
    let (origin, handle) = server(3, |event| ack(&event));
    f.config["endpoint"] = json!(origin);
    for task in [Some(first), Some(second), None] {
        let mut value = report();
        value["task_id"] = json!(task);
        assert_eq!(f.run(&value), Ok("Progress acknowledged"));
    }
    let events = handle.join().unwrap();
    assert_eq!(events[0]["task_id"], first);
    assert_eq!(events[1]["task_id"], second);
    assert!(events[2].get("task_id").is_none());
    assert_eq!(events[2]["sequence"], 3);
    for task in ["", "../bad", "11111111-1111-1111-1111-111111111111"] {
        let mut value = report();
        value["task_id"] = json!(task);
        assert_eq!(f.run(&value), Err(Error::Input));
    }
}

#[test]
fn descriptions_preserve_multiline_details_and_reject_invalid_input() {
    let mut f = Fixture::new();
    let (origin, handle) = server(3, |event| ack(&event));
    f.config["endpoint"] = json!(origin);
    let mut detailed = report();
    // Maximum detail made of JSON-escaped characters exceeds the old input bound.
    detailed["description"] = json!(format!("{}\n\nend", "\t".repeat(7995)));
    assert!(serde_json::to_vec(&detailed).unwrap().len() > 8192);
    assert_eq!(f.run(&detailed), Ok("Progress acknowledged"));
    assert_eq!(f.run(&report()), Ok("Progress acknowledged"));
    let mut null = report();
    null["description"] = serde_json::Value::Null;
    assert_eq!(f.run(&null), Ok("Progress acknowledged"));
    let events = handle.join().unwrap();
    assert_eq!(events[0]["description"], detailed["description"]);
    assert!(events[1].get("description").is_none());
    assert!(events[2].get("description").is_none());
    for description in [
        json!(" "),
        json!("é".repeat(4001)),
        json!("bad\u{0001}"),
        json!(12),
    ] {
        detailed["description"] = description;
        assert_eq!(f.run(&detailed), Err(Error::Input));
    }
}
