//! Complete reports repair controlled transport gaps without a resend queue.
#![cfg(unix)]
mod support;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    process::{Command, Output, Stdio},
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use support::Fixture;
const TASK: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const ORIGIN: &str = "https://silkboard.example.test";
const PASSWORD: &str = "private recovery fixture password";
#[derive(Default)]
struct Fault {
    // 0 passes; 1 rejects before write; 2 rejects only the acknowledgement;
    // 3 stays silent beyond the publisher's network deadline, without writing.
    mode: AtomicU8,
    writes: AtomicUsize,
}
async fn transport_fault(
    State(fault): State<Arc<Fault>>,
    request: Request,
    next: Next,
) -> Response {
    if request.method() != "POST"
        || request.uri().path() != "/api/projects/silkboard/plans"
    {
        return next.run(request).await;
    }
    fault.writes.fetch_add(1, Ordering::SeqCst);
    let mode = fault.mode.load(Ordering::SeqCst);
    if mode == 1 {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    if mode == 3 {
        tokio::task::spawn_blocking(|| {
            std::thread::sleep(Duration::from_millis(3400));
        })
        .await
        .unwrap();
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let response = next.run(request).await;
    if mode == 2 {
        assert_eq!(response.status(), StatusCode::OK);
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    response
}
fn draft() -> Value {
    json!({"schema_version":1,"task_id":TASK,"title":"Recover a complete plan",
      "reporter":{"hostname":"dev","tool":"codex","session_id":"native-a"},
      "steps":[{"id":"contract","title":"Agree contract","state":"done","depends_on":[]},
       {"id":"code","title":"Build feature","state":"active","depends_on":["contract"]},
       {"id":"verify","title":"Verify feature","state":"pending","depends_on":["code"]}]})
}
fn cli(f: &Fixture, command: &str, value: &Value) -> Output {
    let mut args = f.args();
    args[0] = command.into();
    args.insert(0, OsString::from("plan"));
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
fn working(f: &Fixture) -> Value {
    serde_json::from_slice::<Value>(
        &std::fs::read(&f.state_files("json")[0]).unwrap(),
    )
    .unwrap()["snapshot"]
        .clone()
}
fn successful(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"Task plan acknowledged\n");
}
fn rejected(f: &Fixture, command: &str, value: &Value, fault: &Fault) {
    let count = fault.writes.load(Ordering::SeqCst);
    let started = Instant::now();
    let output = cli(f, command, value);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(fault.writes.load(Ordering::SeqCst), count + 1);
}
fn read(origin: &str, cookie: &str, suffix: &str) -> Value {
    let bytes = ureq::get(format!(
        "{origin}/api/owner/projects/silkboard/plans/{TASK}{suffix}"
    ))
    .header("Cookie", cookie)
    .call()
    .unwrap()
    .body_mut()
    .read_to_vec()
    .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
fn raw(origin: &str, token: &str, value: &Value) -> (u16, Value) {
    let mut response = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .new_agent()
        .post(format!("{origin}/api/projects/silkboard/plans"))
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .send(value.to_string())
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.body_mut().read_to_vec().unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_reports_repair_gaps_without_fabricating_history() {
    let store = silkboard_web::progress::ProgressStore::open_memory().unwrap();
    silkboard_web::owner::set_password_from_reader(&store, PASSWORD.as_bytes())
        .unwrap();
    let token = store.issue_token("silkboard", "recovery-fixture").unwrap();
    let fault = Arc::new(Fault::default());
    let app = silkboard_web::create_owner_router(
        std::path::Path::new("missing"),
        store,
        ORIGIN,
    )
    .unwrap()
    .layer(middleware::from_fn_with_state(
        Arc::clone(&fault),
        transport_fault,
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::task::spawn_blocking(move || {
        let login = ureq::post(format!("{origin}/api/auth/login"))
            .header("Origin", ORIGIN)
            .header("Content-Type", "application/json")
            .send(json!({"password":PASSWORD}).to_string())
            .unwrap();
        let cookie = login
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let mut f = Fixture::new();
        f.config["endpoint"] = json!(origin);
        support::private(&f.root.join("token"), token.as_bytes());
        let c = Recovery {
            f,
            origin,
            token,
            cookie,
            fault,
        };
        let current = lost_updates(&c);
        handoffs(&c, current);
        assert_eq!(c.f.state_files("json").len(), 1);
    })
    .await
    .unwrap();
    server.abort();
}

struct Recovery {
    f: Fixture,
    origin: String,
    token: String,
    cookie: String,
    fault: Arc<Fault>,
}
impl Recovery {
    fn mode(&self, value: u8) {
        self.fault.mode.store(value, Ordering::SeqCst);
    }
    fn read(&self, suffix: &str) -> Value {
        read(&self.origin, &self.cookie, suffix)
    }
    fn raw(&self, value: &Value) -> (u16, Value) {
        raw(&self.origin, &self.token, value)
    }
    fn cli(&self, command: &str, value: &Value) -> Output {
        cli(&self.f, command, value)
    }
    fn reject(&self, value: &Value) {
        rejected(&self.f, "publish", value, &self.fault);
    }
    fn working(&self) -> Value {
        working(&self.f)
    }
}
fn lost_updates(c: &Recovery) -> Value {
    let mut current = draft();
    successful(&c.cli("start", &current));
    let initial = c.working();
    let initial_owner = c.read("");
    // Losing status reports and two definition revisions still preserves
    // local high-water marks; the server retains only its last receipt.
    c.mode(1);
    current["steps"][1]["state"] = json!("done");
    c.reject(&current);
    let delayed = c.working();
    assert_eq!(delayed["sequence"], "2");
    current["steps"][2]["depends_on"] = json!(["contract"]);
    c.reject(&current);
    current["steps"].as_array_mut().unwrap().push(json!({"id":"review","title":"Review feature","state":"active","depends_on":["contract"]}));
    c.reject(&current);
    assert_eq!(c.working()["sequence"], "4");
    assert_eq!(c.working()["plan_revision"], "3");
    assert_eq!(c.read(""), initial_owner);
    // A fresh full report repairs current state, never fabricating reports
    // for sequences or revisions that the server did not receive.
    c.mode(0);
    successful(&c.cli("publish", &current));
    let repaired = c.working();
    let owner = c.read("");
    assert_eq!(owner["snapshot"], repaired);
    assert_eq!(owner["summary"]["sequence"], "5");
    assert_eq!(owner["summary"]["plan_revision"], "3");
    assert_eq!(owner["summary"]["counts"]["done"], 2);
    assert_eq!(owner["summary"]["history_incomplete"], true);
    let history = c.read("/history");
    assert_eq!(
        history["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["sequence"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["5", "1"]
    );
    let revisions = c.read("/revisions");
    assert_eq!(
        revisions["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["plan_revision"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["3", "1"]
    );
    assert_eq!(c.read("?sequence=1")["snapshot"], initial);
    assert_ordering(c, &repaired, &delayed, &owner);
    acknowledgement_gap(c, &mut current, &initial, &initial_owner);
    current
}
fn assert_ordering(
    c: &Recovery,
    repaired: &Value,
    delayed: &Value,
    owner: &Value,
) {
    // Exact replays keep their original receipt and never move current.
    let ack = c.raw(repaired);
    assert_eq!(ack.0, 200);
    assert_eq!(ack.1["received_at"], owner["summary"]["received_at"]);
    assert_eq!(c.raw(delayed).0, 409);
    let mut conflict = repaired.clone();
    conflict["title"] = json!("Conflicting same sequence");
    assert_eq!(c.raw(&conflict).0, 409);
    conflict["sequence"] = json!("6");
    assert_eq!(c.raw(&conflict).0, 409);
    conflict = repaired.clone();
    conflict["sequence"] = json!("6");
    conflict["plan_revision"] = json!("2");
    assert_eq!(c.raw(&conflict).0, 409);
    let mut orphan = repaired.clone();
    orphan["sequence"] = json!("6");
    orphan["plan_revision"] = json!("4");
    orphan["steps"].as_array_mut().unwrap().remove(0);
    assert_eq!(c.raw(&orphan).0, 400);
    assert_eq!(c.read(""), *owner);
}
fn acknowledgement_gap(
    c: &Recovery,
    current: &mut Value,
    initial: &Value,
    initial_owner: &Value,
) {
    // A missing acknowledgement is ambiguous: the write may have landed.
    c.mode(2);
    current["steps"][3]["state"] = json!("done");
    c.reject(current);
    let accepted_unacknowledged = c.working();
    assert_eq!(c.read("")["snapshot"], accepted_unacknowledged);
    c.mode(0);
    current["steps"][2]["state"] = json!("blocked");
    current["steps"][2]["blocked_reason"] = json!("Waiting for owner review");
    successful(&c.cli("publish", current));
    let last = c.read("");
    assert_eq!(last["summary"]["sequence"], "7");
    assert_eq!(last["summary"]["state"], "blocked");
    assert_eq!(last["summary"]["counts"]["done"], 3);
    assert_eq!(c.raw(&accepted_unacknowledged).0, 200);
    assert_eq!(
        c.raw(initial).1["received_at"],
        initial_owner["summary"]["received_at"]
    );
    assert_eq!(c.read(""), last);
}
fn handoffs(c: &Recovery, mut current: Value) {
    // Removing completed work changes only the current definition; its
    // received completion stays readable in the older snapshot.
    current["steps"].as_array_mut().unwrap().remove(1);
    successful(&c.cli("publish", &current));
    assert_eq!(c.read("")["summary"]["plan_revision"], "4");
    assert_eq!(
        c.read("?sequence=7")["snapshot"]["steps"][1]["state"],
        "done"
    );
    // Several handoffs can occur during a gap; the next binding fences
    // the previous writer without inventing unseen session history.
    let reporter = current["reporter"].clone();
    let binding = |from: &str, to: &str| {
        let mut a = reporter.clone();
        a["session_id"] = json!(from);
        let mut b = a.clone();
        b["session_id"] = json!(to);
        json!({"schema_version":1,"task_id":TASK,"from":a,"to":b})
    };
    assert!(
        c.cli("handoff", &binding("native-a", "native-b"))
            .status
            .success()
    );
    current["reporter"]["session_id"] = json!("native-b");
    c.mode(1);
    c.reject(&current);
    assert!(
        c.cli("handoff", &binding("native-b", "native-c"))
            .status
            .success()
    );
    current["reporter"]["session_id"] = json!("native-c");
    c.mode(0);
    successful(&c.cli("publish", &current));
    let resumed = c.read("");
    assert_eq!(resumed["summary"]["sequence"], "10");
    assert_eq!(resumed["summary"]["writer_generation"], "3");
    let mut old_writer = c.working();
    old_writer["sequence"] = json!("11");
    old_writer["writer_generation"] = json!("1");
    old_writer["reporter"]["session_id"] = json!("native-a");
    assert_eq!(c.raw(&old_writer).0, 409);
    assert_eq!(c.read(""), resumed);

    timeout(c, &current);
}
fn timeout(c: &Recovery, current: &Value) {
    let resumed = c.read("");
    // A silent peer exhausts the same bounded transport budget, does
    // not write, and does not queue anything to replay after recovery.
    c.mode(3);
    let count = c.fault.writes.load(Ordering::SeqCst);
    let started = Instant::now();
    let output = c.cli("publish", current);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("report delivery failed or timed out")
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(c.fault.writes.load(Ordering::SeqCst), count + 1);
    assert_eq!(c.working()["sequence"], "11");
    assert_eq!(c.read(""), resumed);
    c.mode(0);
    successful(&c.cli("publish", current));
    assert_eq!(c.fault.writes.load(Ordering::SeqCst), count + 2);
    assert_eq!(c.read("")["summary"]["sequence"], "12");
    let history = c.read("/history");
    assert_eq!(
        history["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["sequence"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["12", "10", "8", "7", "6", "5", "1"]
    );
}
