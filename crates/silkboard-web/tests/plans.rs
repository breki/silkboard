//! Plan reporting preserves publisher/owner boundaries and received history.

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use silkboard_web::{
    create_owner_router, owner::set_password_from_reader,
    progress::ProgressStore,
};
use std::{fmt::Write as _, path::Path};
use tower::ServiceExt;

const ORIGIN: &str = "https://silkboard.example";
const TASK: &str = "11111111-1111-4111-8111-111111111111";

fn snapshot(sequence: &str) -> Value {
    json!({"schema_version":1,"task_id":TASK,"sequence":sequence,"plan_revision":"1","writer_generation":"1",
    "reporter":{"hostname":"dev","tool":"codex","session_id":"native-a"},
    "title":"Task-plan API","steps":[
        {"id":"contract","title":"Agree contract","depends_on":[],"state":"done"},
        {"id":"api","title":"Implement API","depends_on":["contract"],"state":"active"}
    ]})
}

async fn fixture() -> (ProgressStore, Router, String, String) {
    let store = ProgressStore::open_memory().unwrap();
    set_password_from_reader(&store, b"private plan test password".as_slice())
        .unwrap();
    let token = store.issue_token("silkboard", "plan-agent").unwrap();
    let app = create_owner_router(Path::new("missing"), store.clone(), ORIGIN)
        .unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header("origin", ORIGIN)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"password":"private plan test password"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    (store, app, token, cookie)
}
async fn request(
    app: &Router,
    method: &str,
    path: &str,
    token: &str,
    value: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if token.starts_with("__Host-") {
        req = req.header("cookie", token);
    } else {
        req = req.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(
            req.body(
                value.map_or_else(Body::empty, |v| Body::from(v.to_string())),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn plan_snapshot_is_publishable_and_only_the_owner_can_read_it() {
    let (_store, app, token, cookie) = fixture().await;
    let (status, ack) = request(
        &app,
        "POST",
        "/api/projects/silkboard/plans",
        &token,
        Some(snapshot("1")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ack["task_id"], TASK);
    assert_eq!(ack["sequence"], "1");
    let path = format!("/api/owner/projects/silkboard/plans/{TASK}");
    assert_eq!(
        request(&app, "GET", &path, &token, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, detail) = request(&app, "GET", &path, &cookie, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["snapshot"]["steps"][1]["state"], "active");
}

#[tokio::test]
async fn replay_order_handoffs_and_history_are_independent() {
    let (_store, app, token, cookie) = fixture().await;
    let publish = "/api/projects/silkboard/plans";
    let detail = format!("/api/owner/projects/silkboard/plans/{TASK}");
    let first = snapshot("1");
    let (_, ack) =
        request(&app, "POST", publish, &token, Some(first.clone())).await;
    let mut next = snapshot("3");
    next["steps"][1]["state"] = json!("done");
    assert_eq!(
        request(&app, "POST", publish, &token, Some(next.clone()))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "POST", publish, &token, Some(first.clone()))
            .await
            .1,
        ack
    );
    assert_eq!(
        request(&app, "POST", publish, &token, Some(snapshot("2")))
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(&app, "POST", publish, &token, Some(snapshot("3")))
            .await
            .0,
        StatusCode::CONFLICT
    );
    next["sequence"] = json!("4");
    next["reporter"]["session_id"] = json!("native-b");
    assert_eq!(
        request(&app, "POST", publish, &token, Some(next.clone()))
            .await
            .0,
        StatusCode::CONFLICT
    );
    next["writer_generation"] = json!("3");
    next["steps"][1]["state"] = json!("active");
    assert_eq!(
        request(&app, "POST", publish, &token, Some(next.clone()))
            .await
            .0,
        StatusCode::OK
    );
    let (_, current) = request(&app, "GET", &detail, &cookie, None).await;
    assert_eq!(current["summary"]["history_incomplete"], true);
    assert_eq!(current["prior_completions"][0]["step_id"], "api");
    assert_eq!(current["prior_completions"][0]["sequence"], "3");
    assert_eq!(
        request(&app, "POST", publish, &token, Some(snapshot("100")))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let (_, historical) =
        request(&app, "GET", &format!("{detail}?sequence=3"), &cookie, None)
            .await;
    assert_eq!(historical["snapshot"]["steps"][1]["state"], "done");
    assert_eq!(historical["prior_completions"], json!([]));
    let mut changed = next.clone();
    changed["sequence"] = json!("5");
    changed["title"] = json!("Revised title");
    assert_eq!(
        request(&app, "POST", publish, &token, Some(changed.clone()))
            .await
            .0,
        StatusCode::CONFLICT
    );
    changed["plan_revision"] = json!("2");
    assert_eq!(
        request(&app, "POST", publish, &token, Some(changed.clone()))
            .await
            .0,
        StatusCode::OK
    );
    changed["sequence"] = json!("6");
    changed["plan_revision"] = json!("1");
    assert_eq!(
        request(&app, "POST", publish, &token, Some(changed))
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_revision_history(&app, &cookie, &detail).await;
}

async fn assert_revision_history(app: &Router, cookie: &str, detail: &str) {
    let (_, revision) =
        request(app, "GET", &format!("{detail}?revision=1"), cookie, None)
            .await;
    assert_eq!(revision["snapshot"]["sequence"], "4");
    let (_, history) =
        request(app, "GET", &format!("{detail}/history"), cookie, None).await;
    assert_eq!(history["items"].as_array().unwrap().len(), 4);
    let (_, revisions) =
        request(app, "GET", &format!("{detail}/revisions"), cookie, None).await;
    assert_eq!(revisions["items"].as_array().unwrap().len(), 2);
    assert_eq!(
        request(app, "GET", &format!("{detail}?revision=9"), cookie, None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(app, "GET", &format!("{detail}?sequence=2"), cookie, None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            app,
            "GET",
            &format!("{detail}?sequence=1&revision=1"),
            cookie,
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn authorization_bounds_and_existing_progress_remain_independent() {
    let (store, app, token, cookie) = fixture().await;
    let path = "/api/projects/silkboard/plans";
    let other = store.issue_token("other", "agent").unwrap();
    for credential in ["", other.as_str(), cookie.as_str()] {
        assert_eq!(
            request(&app, "POST", path, credential, Some(snapshot("1")))
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
    for query in ["?before=bad", "?unexpected=1"] {
        assert_eq!(
            request(
                &app,
                "GET",
                &format!("/api/owner/plans{query}"),
                &cookie,
                None
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    let response = app
        .clone()
        .oneshot(
            Request::post(path)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let response = app
        .clone()
        .oneshot(
            Request::post(path)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(" ".repeat(262_145)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let mut bad = snapshot("1");
    bad["steps"][1]["depends_on"] = json!(["missing"]);
    assert_eq!(
        request(&app, "POST", path, &token, Some(bad)).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(&app, "POST", path, &token, Some(snapshot("1")))
            .await
            .0,
        StatusCode::OK
    );
    let (_, page) =
        request(&app, "GET", "/api/owner/plans", &cookie, None).await;
    assert_eq!(page["items"][0]["task_id"], TASK);
    assert!(page["items"][0].get("steps").is_none());
    assert_eq!(
        request(
            &app,
            "GET",
            &format!("/api/owner/projects/other/plans/{TASK}"),
            &cookie,
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let ordinary = json!({"event_id":"after-plan","hostname":"dev","tool":"codex","session_id":"native","sequence":1,"message":"Ordinary progress still works"});
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/projects/silkboard/progress",
            &token,
            Some(ordinary)
        )
        .await
        .0,
        StatusCode::OK
    );
    store.revoke_token("silkboard", "plan-agent").unwrap();
    assert_eq!(
        request(&app, "POST", path, &token, Some(snapshot("1")))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn publishing_requires_owner_initialization() {
    let path = "/api/projects/silkboard/plans";
    let uninitialized = ProgressStore::open_memory().unwrap();
    let t = uninitialized.issue_token("silkboard", "new").unwrap();
    let a = create_owner_router(Path::new("missing"), uninitialized, ORIGIN)
        .unwrap();
    assert_eq!(
        request(&a, "POST", path, &t, Some(snapshot("1"))).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn pages_and_revision_summaries_are_bounded_and_exact() {
    let (_store, app, token, cookie) = fixture().await;
    let base = format!("/api/owner/projects/silkboard/plans/{TASK}");
    for i in 1..=23 {
        let mut v = snapshot(&i.to_string());
        v["plan_revision"] = json!(i.to_string());
        v["title"] = json!(format!("Revision {i}"));
        assert_eq!(
            request(
                &app,
                "POST",
                "/api/projects/silkboard/plans",
                &token,
                Some(v)
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    for suffix in ["history", "revisions"] {
        let (_, page) =
            request(&app, "GET", &format!("{base}/{suffix}"), &cookie, None)
                .await;
        assert_eq!(page["items"].as_array().unwrap().len(), 20);
        assert_eq!(page["next_cursor"], "4");
        let (_, next) = request(
            &app,
            "GET",
            &format!("{base}/{suffix}?before=4"),
            &cookie,
            None,
        )
        .await;
        assert_eq!(next["items"].as_array().unwrap().len(), 3);
        assert!(next["next_cursor"].is_null());
    }
    for i in 2..=22 {
        let mut v = snapshot("1");
        v["task_id"] = json!(format!("{i:08x}-1111-4111-8111-111111111111"));
        if i == 2 {
            v["reporter"]["hostname"] = json!("older-filter-match");
        }
        assert_eq!(
            request(
                &app,
                "POST",
                "/api/projects/silkboard/plans",
                &token,
                Some(v)
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let (status, filtered) = request(&app, "GET", "/api/owner/plans?project=silkboard&agent=older-filter-match&state=active", &cookie, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(filtered["items"].as_array().unwrap().len(), 1);
    assert!(filtered["next_cursor"].is_null());
    let (_, page) =
        request(&app, "GET", "/api/owner/plans", &cookie, None).await;
    assert_eq!(page["items"].as_array().unwrap().len(), 20);
    let cursor = page["next_cursor"].as_str().unwrap();
    let mut encoded = String::new();
    for byte in cursor.bytes() {
        write!(encoded, "%{byte:02X}").unwrap();
    }
    let (_, next) = request(
        &app,
        "GET",
        &format!("/api/owner/plans?before={encoded}"),
        &cookie,
        None,
    )
    .await;
    assert_eq!(next["items"].as_array().unwrap().len(), 2);
    let mut huge = snapshot("9007199254740993");
    huge["task_id"] = json!("ffffffff-ffff-4fff-8fff-ffffffffffff");
    huge["writer_generation"] = json!("9223372036854775807");
    huge["plan_revision"] = json!("9223372036854775807");
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/projects/silkboard/plans",
            &token,
            Some(huge)
        )
        .await
        .1["sequence"],
        "9007199254740993"
    );
}

#[tokio::test]
async fn maximum_counter_is_visible_in_history_without_a_cursor() {
    let (_store, app, token, cookie) = fixture().await;
    let mut value = snapshot("9223372036854775807");
    value["plan_revision"] = json!("9223372036854775807");
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/projects/silkboard/plans",
            &token,
            Some(value)
        )
        .await
        .0,
        StatusCode::OK
    );
    for kind in ["history", "revisions"] {
        let (_, page) = request(
            &app,
            "GET",
            &format!("/api/owner/projects/silkboard/plans/{TASK}/{kind}"),
            &cookie,
            None,
        )
        .await;
        assert_eq!(page["items"].as_array().unwrap().len(), 1);
        assert_eq!(page["items"][0]["sequence"], "9223372036854775807");
    }
}

#[tokio::test]
async fn invalid_requests_are_atomic_and_large_plans_do_not_expand_progress() {
    let (_store, app, token, cookie) = fixture().await;
    let publish = "/api/projects/silkboard/plans";
    let mut large = snapshot("1");
    large["steps"] = json!((0..12).map(|i|json!({"id":format!("s{i}"),"title":"Parallel work","description":"x".repeat(1000),"depends_on":[],"state":"active"})).collect::<Vec<_>>());
    assert!(large.to_string().len() > 8192);
    assert_eq!(
        request(&app, "POST", publish, &token, Some(large.clone()))
            .await
            .0,
        StatusCode::OK
    );
    let base = format!("/api/owner/projects/silkboard/plans/{TASK}");
    let before = request(&app, "GET", &base, &cookie, None).await.1;
    let mut bad = large;
    bad["sequence"] = json!("2");
    bad["steps"][0]["depends_on"] = json!(["missing"]);
    assert_eq!(
        request(&app, "POST", publish, &token, Some(bad)).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(request(&app, "GET", &base, &cookie, None).await.1, before);
    let (_, history) =
        request(&app, "GET", &format!("{base}/history"), &cookie, None).await;
    assert_eq!(history["items"].as_array().unwrap().len(), 1);
    let ordinary = json!({"event_id":"large","hostname":"dev","tool":"codex","session_id":"native","sequence":1,"message":"x".repeat(9000)});
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/projects/silkboard/progress",
            &token,
            Some(ordinary)
        )
        .await
        .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    for path in [
        format!("{base}?sequence=01"),
        format!("{base}/history?before=0"),
        format!("{base}/revisions?extra=1"),
        "/api/owner/projects/silkboard/plans/bad".into(),
    ] {
        assert_eq!(
            request(&app, "GET", &path, &cookie, None).await.0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn authentication_precedes_body_parsing_and_errors_do_not_echo_input() {
    let (_store, app, token, _) = fixture().await;
    for (credential, body, expected) in [
        (
            "x".repeat(64),
            "private invalid body".as_bytes().to_vec(),
            StatusCode::UNAUTHORIZED,
        ),
        (token.clone(), vec![0xff, 0xfe], StatusCode::BAD_REQUEST),
        (
            token,
            b"{\"schema_version\":1,\"schema_version\":1}".to_vec(),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/projects/silkboard/plans")
                    .header("authorization", format!("Bearer {credential}"))
                    .header("content-type", "application/json; charset=utf-8")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(!text.contains("private invalid body"));
        assert!(!text.contains(&credential));
    }
}

#[tokio::test]
async fn removed_and_reintroduced_steps_retain_only_received_completions() {
    let (_store, app, token, cookie) = fixture().await;
    let path = "/api/projects/silkboard/plans";
    let base = format!("/api/owner/projects/silkboard/plans/{TASK}");
    let mut s = snapshot("1");
    assert_eq!(
        request(&app, "POST", path, &token, Some(s.clone())).await.0,
        StatusCode::OK
    );
    s["sequence"] = json!("2");
    s["steps"][1]["state"] = json!("done");
    assert_eq!(
        request(&app, "POST", path, &token, Some(s.clone())).await.0,
        StatusCode::OK
    );
    s["sequence"] = json!("3");
    s["plan_revision"] = json!("2");
    s["steps"].as_array_mut().unwrap().pop();
    assert_eq!(
        request(&app, "POST", path, &token, Some(s)).await.0,
        StatusCode::OK
    );
    let mut restored = snapshot("4");
    restored["plan_revision"] = json!("3");
    restored["steps"][1]["state"] = json!("blocked");
    restored["steps"][1]["blocked_reason"] = json!("Waiting for owner");
    assert_eq!(
        request(&app, "POST", path, &token, Some(restored)).await.0,
        StatusCode::OK
    );
    let (_, detail) = request(&app, "GET", &base, &cookie, None).await;
    assert_eq!(detail["summary"]["state"], "blocked");
    assert_eq!(detail["summary"]["blocked_step_ids"], json!(["api"]));
    assert_eq!(detail["summary"]["history_incomplete"], false);
    assert_eq!(detail["prior_completions"][0]["sequence"], "2");
    let (_, initial) =
        request(&app, "GET", &format!("{base}?sequence=1"), &cookie, None)
            .await;
    assert_eq!(initial["prior_completions"], json!([]));
    let (_, removed) =
        request(&app, "GET", &format!("{base}?revision=2"), &cookie, None)
            .await;
    assert_eq!(removed["snapshot"]["steps"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn workflow_filters_use_current_report_and_validate() {
    let (_store, app, token, cookie) = fixture().await;
    request(
        &app,
        "POST",
        "/api/projects/silkboard/plans",
        &token,
        Some(snapshot("1")),
    )
    .await;
    let (status, result) = request(
        &app,
        "GET",
        "/api/owner/plans?project=silkboard&agent=CODEX&state=active",
        &cookie,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    for query in ["state=completed", "project=other", "agent=%25"] {
        let (status, result) = request(
            &app,
            "GET",
            &format!("/api/owner/plans?{query}"),
            &cookie,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(result["items"], json!([]));
    }
    let mut completed = snapshot("2");
    completed["steps"][1]["state"] = json!("done");
    request(
        &app,
        "POST",
        "/api/projects/silkboard/plans",
        &token,
        Some(completed),
    )
    .await;
    let (_, active) =
        request(&app, "GET", "/api/owner/plans?state=active", &cookie, None)
            .await;
    assert_eq!(active["items"], json!([]));
    let (_, done) = request(
        &app,
        "GET",
        "/api/owner/plans?state=completed&agent=dev",
        &cookie,
        None,
    )
    .await;
    assert_eq!(done["items"].as_array().unwrap().len(), 1);
    for query in ["state=done", "project=BAD", "agent=%0A", "from=0"] {
        assert_eq!(
            request(
                &app,
                "GET",
                &format!("/api/owner/plans?{query}"),
                &cookie,
                None
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
}

fn associated_event(id: &str, sequence: i64, task: Option<&str>) -> Value {
    json!({"event_id":id,"hostname":"dev","tool":"codex","session_id":"native-a","sequence":sequence,"message":"Workflow progress","task_id":task})
}

async fn linked_event_fixture() -> (ProgressStore, Router, String, String) {
    let (store, app, token, cookie) = fixture().await;
    for (id, seq, task) in [("legacy", 1, None), ("linked", 2, Some(TASK))] {
        assert_eq!(
            request(
                &app,
                "POST",
                "/api/projects/silkboard/progress",
                &token,
                Some(associated_event(id, seq, task))
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let (_, feed) =
        request(&app, "GET", "/api/owner/events", &cookie, None).await;
    assert!(feed["items"][0]["workflow_task_id"].is_null());
    request(
        &app,
        "POST",
        "/api/projects/silkboard/plans",
        &token,
        Some(snapshot("1")),
    )
    .await;
    let other = "22222222-2222-4222-8222-222222222222";
    let mut second = snapshot("1");
    second["task_id"] = json!(other);
    request(
        &app,
        "POST",
        "/api/projects/silkboard/plans",
        &token,
        Some(second),
    )
    .await;
    (store, app, token, cookie)
}

#[tokio::test]
async fn event_workflow_links_are_explicit_project_scoped_and_survive_handoff()
{
    let (store, app, token, cookie) = linked_event_fixture().await;
    let other_token = store.issue_token("other", "agent").unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/projects/other/progress",
            &other_token,
            Some(associated_event("cross", 1, Some(TASK)))
        )
        .await
        .0,
        StatusCode::OK
    );
    let mut handoff = snapshot("2");
    handoff["writer_generation"] = json!("2");
    handoff["reporter"]["session_id"] = json!("native-b");
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/projects/silkboard/plans",
            &token,
            Some(handoff)
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, feed) =
        request(&app, "GET", "/api/owner/events", &cookie, None).await;
    for item in feed["items"].as_array().unwrap() {
        if item["event_id"] == "linked" {
            assert_eq!(item["workflow_task_id"], TASK);
        } else {
            assert!(item["workflow_task_id"].is_null());
        }
    }
    let (_, sessions) = request(
        &app,
        "GET",
        "/api/owner/projects/silkboard/sessions",
        &cookie,
        None,
    )
    .await;
    assert_eq!(sessions["items"][0]["latest"]["workflow_task_id"], TASK);
    let session = sessions["items"][0]["id"].as_str().unwrap();
    let (_, history) = request(
        &app,
        "GET",
        &format!("/api/owner/projects/silkboard/sessions/{session}/events"),
        &cookie,
        None,
    )
    .await;
    assert_eq!(history["items"][0]["workflow_task_id"], TASK);
    assert!(history["items"][1]["workflow_task_id"].is_null());
    let (_, older) = request(
        &app,
        "GET",
        &format!(
            "/api/owner/projects/silkboard/sessions/{session}/events?before=2"
        ),
        &cookie,
        None,
    )
    .await;
    assert!(older["items"][0]["workflow_task_id"].is_null());
}

#[tokio::test]
async fn event_workflow_associations_reject_invalid_ids_and_rewrites() {
    let (_store, app, token, _cookie) = linked_event_fixture().await;
    let other = "22222222-2222-4222-8222-222222222222";
    assert_eq!(
        request(&app, "GET", "/api/owner/events", &token, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/projects/silkboard/progress",
            &token,
            Some(associated_event("linked", 2, Some(other)))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    for invalid in ["", "../bad", "11111111-1111-1111-1111-111111111111"] {
        assert_eq!(
            request(
                &app,
                "POST",
                "/api/projects/silkboard/progress",
                &token,
                Some(associated_event("invalid", 3, Some(invalid)))
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn event_workflow_links_follow_each_event_across_global_pages() {
    let (_store, app, token, cookie) = fixture().await;
    let second = "22222222-2222-4222-8222-222222222222";
    for task in [TASK, second] {
        let mut plan = snapshot("1");
        plan["task_id"] = json!(task);
        assert_eq!(
            request(
                &app,
                "POST",
                "/api/projects/silkboard/plans",
                &token,
                Some(plan)
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    for sequence in 1..=24 {
        let task = match sequence {
            1 => Some(TASK),
            2 => Some(second),
            _ => None,
        };
        let event = json!({"event_id":format!("event-{sequence}"),"hostname":"dev","tool":"codex","session_id":"one-session","sequence":sequence,"message":"Report","task_id":task});
        assert_eq!(
            request(
                &app,
                "POST",
                "/api/projects/silkboard/progress",
                &token,
                Some(event)
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let (_, page) =
        request(&app, "GET", "/api/owner/events", &cookie, None).await;
    assert_eq!(page["items"].as_array().unwrap().len(), 20);
    let mut encoded = String::new();
    for byte in page["next_cursor"].as_str().unwrap().bytes() {
        write!(encoded, "%{byte:02X}").unwrap();
    }
    let (_, older) = request(
        &app,
        "GET",
        &format!("/api/owner/events?before={encoded}"),
        &cookie,
        None,
    )
    .await;
    let events = older["items"].as_array().unwrap();
    assert_eq!(events.len(), 4);
    for item in events {
        match item["event_id"].as_str().unwrap() {
            "event-1" => assert_eq!(item["workflow_task_id"], TASK),
            "event-2" => assert_eq!(item["workflow_task_id"], second),
            _ => assert!(item["workflow_task_id"].is_null()),
        }
    }
}
