//! Owner-only project feed reads preserve native agent identities and order.

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
use std::path::Path;
use tower::ServiceExt;

const ORIGIN: &str = "https://silkboard.example";

#[tokio::test]
async fn agent_filters_search_complete_history_and_exact_identity() {
    let store = ProgressStore::open_memory().unwrap();
    let (app, cookie) = owner(store.clone()).await;
    let token = store.issue_token("alpha", "agent").unwrap();
    for index in 0..25 {
        let mut report =
            event(&format!("e-{index}"), &format!("native-{index}"), 1);
        report.session_name = Some(format!("Task {index}"));
        store.publish("alpha", &token, &report).unwrap();
    }
    let other = store.issue_token("bravo", "agent").unwrap();
    store
        .publish("bravo", &other, &event("other", "native-0", 1))
        .unwrap();
    let child_token = store.issue_token("child-only", "agent").unwrap();
    let mut child = event("child", "child-native", 1);
    child.subagent_id = Some("review".to_owned());
    store.publish("child-only", &child_token, &child).unwrap();
    for query in [
        "project=alpha&q=TASK%200",
        "project=alpha&hostname=dev&tool=codex&session_id=native-0",
        "q=ALPHA&hostname=dev&tool=codex&session_id=native-0",
    ] {
        let (status, page) =
            read(app.clone(), &format!("/api/owner/agents?{query}"), &cookie)
                .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page["items"].as_array().unwrap().len(), 1);
        assert_eq!(page["items"][0]["session_id"], "native-0");
    }
    for query in ["project=other", "q=%25"] {
        let (status, page) =
            read(app.clone(), &format!("/api/owner/agents?{query}"), &cookie)
                .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page["items"], json!([]));
    }
    assert_eq!(
        read(app.clone(), "/api/owner/agents?q=Task", "").await.0,
        StatusCode::UNAUTHORIZED
    );
    for query in ["project=BAD", "hostname=bad%20host", "q=%0A", "session_id="]
    {
        assert_eq!(
            read(app.clone(), &format!("/api/owner/agents?{query}"), &cookie)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    let (status, choices) = read(
        app,
        "/api/owner/filter-options?view=agents&kind=project",
        &cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(choices["items"], json!(["alpha", "bravo"]));
}

async fn owner(store: ProgressStore) -> (Router, String) {
    set_password_from_reader(&store, b"private test owner password".as_slice())
        .unwrap();
    let app = create_owner_router(Path::new("missing"), store, ORIGIN).unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header("origin", ORIGIN)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"password":"private test owner password"})
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
    (app, cookie)
}

async fn read(app: Router, path: &str, cookie: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::get(path)
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 200_000).await.unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

#[tokio::test]
async fn owner_can_list_projects_without_giving_publishers_read_access() {
    let store = ProgressStore::open_memory().unwrap();
    let publisher = store.issue_token("silkboard", "agent").unwrap();
    let (app, cookie) = owner(store).await;
    let (status, page) =
        read(app.clone(), "/api/owner/projects", &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        page,
        json!({"items":[{"id":"silkboard"}],"next_cursor":null})
    );
    assert_eq!(
        read(
            app,
            "/api/owner/projects",
            &format!("__Host-silkboard-owner={publisher}")
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

fn event(
    id: &str,
    native: &str,
    sequence: i64,
) -> silkboard_web::progress::ProgressEvent {
    serde_json::from_value(json!({"event_id":id,"hostname":"dev","tool":"codex","session_id":native,"sequence":sequence,"message":"Running tests"})).unwrap()
}

#[tokio::test]
async fn native_identity_history_and_names_remain_separate() {
    let store = ProgressStore::open_memory().unwrap();
    set_password_from_reader(&store, b"private test owner password".as_slice())
        .unwrap();
    let token = store.issue_token("silkboard", "agent").unwrap();
    for sequence in (1..=25).rev() {
        let mut value =
            event(&format!("event-{sequence}"), "same-native", sequence);
        if sequence == 1 {
            value.session_name = Some("Investigating".into());
        }
        store.publish("silkboard", &token, &value).unwrap();
    }
    for (index, field) in ["hostname", "tool", "session_id", "subagent_id"]
        .iter()
        .enumerate()
    {
        let mut value = serde_json::to_value(event(
            &format!("other-{index}"),
            "same-native",
            1,
        ))
        .unwrap();
        value[*field] = json!("another");
        if *field == "subagent_id" {
            value["parent_session_id"] = json!("same-native");
        }
        store
            .publish(
                "silkboard",
                &token,
                &serde_json::from_value(value).unwrap(),
            )
            .unwrap();
    }
    let foreign = store.issue_token("other", "agent").unwrap();
    store
        .publish("other", &foreign, &event("event-25", "same-native", 25))
        .unwrap();
    let (app, cookie) = owner(store).await;
    let (status, page) = read(
        app.clone(),
        "/api/owner/projects/silkboard/sessions",
        &cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = page["items"].as_array().unwrap();
    assert_eq!(items.len(), 5);
    assert_eq!(items[0]["subagent_id"], "another");
    assert_eq!(items[0]["parent_session_id"], "same-native");
    let original = items.iter().find(|s| s["id"] == "1").unwrap();
    assert_eq!(original["latest"]["event_id"], "event-25");
    assert_eq!(original["session_name"], "Investigating");
    assert_eq!(original.as_object().unwrap().len(), 8);
    let path = "/api/owner/projects/silkboard/sessions/1/events";
    let (_, messages) = read(app.clone(), path, &cookie).await;
    assert_eq!(messages["items"].as_array().unwrap().len(), 20);
    assert_eq!(messages["items"][0]["sequence"], "25");
    assert_eq!(messages["next_cursor"], "6");
    let (_, earlier) =
        read(app.clone(), &format!("{path}?before=6"), &cookie).await;
    assert_eq!(earlier["items"].as_array().unwrap().len(), 5);
    assert_eq!(earlier["items"][4]["sequence"], "1");
    assert!(earlier["next_cursor"].is_null());
    assert_eq!(
        read(app, "/api/owner/projects/other/sessions/1/events", &cookie)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn bounded_project_and_session_pages_do_not_lose_large_sequences() {
    let store = ProgressStore::open_memory().unwrap();
    set_password_from_reader(&store, b"private test owner password".as_slice())
        .unwrap();
    for index in 0..25 {
        store
            .issue_token(&format!("p-{index:02}"), "agent")
            .unwrap();
    }
    let token = store.issue_token("p-00", "another-agent").unwrap();
    for index in 0..25 {
        store
            .publish(
                "p-00",
                &token,
                &event(
                    &format!("event-{index}"),
                    &format!("native-{index}"),
                    i64::MAX,
                ),
            )
            .unwrap();
    }
    let (app, cookie) = owner(store).await;
    let (_, first) = read(app.clone(), "/api/owner/projects", &cookie).await;
    assert_eq!(first["items"].as_array().unwrap().len(), 20);
    assert_eq!(first["next_cursor"], "p-19");
    let (_, last) =
        read(app.clone(), "/api/owner/projects?after=p-19", &cookie).await;
    assert_eq!(last["items"].as_array().unwrap().len(), 5);
    let (_, first) =
        read(app.clone(), "/api/owner/projects/p-00/sessions", &cookie).await;
    assert_eq!(first["items"].as_array().unwrap().len(), 20);
    assert_eq!(first["next_cursor"], "6");
    assert_eq!(
        first["items"][0]["latest"]["sequence"],
        i64::MAX.to_string()
    );
    let (_, last) = read(
        app.clone(),
        "/api/owner/projects/p-00/sessions?before=6",
        &cookie,
    )
    .await;
    assert_eq!(last["items"].as_array().unwrap().len(), 5);
    assert_eq!(last["items"][4]["id"], "1");
    let (_, history) =
        read(app, "/api/owner/projects/p-00/sessions/1/events", &cookie).await;
    assert_eq!(history["items"][0]["sequence"], i64::MAX.to_string());
}

#[tokio::test]
async fn empty_missing_and_malformed_reads_are_safe() {
    let store = ProgressStore::open_memory().unwrap();
    let token = store.issue_token("empty", "agent").unwrap();
    let (app, cookie) = owner(store).await;
    let (_, empty) =
        read(app.clone(), "/api/owner/projects/empty/sessions", &cookie).await;
    assert_eq!(empty, json!({"items":[],"next_cursor":null}));
    for path in [
        "/api/owner/projects/missing/sessions",
        "/api/owner/projects/empty/sessions/123/events",
    ] {
        assert_eq!(
            read(app.clone(), path, &cookie).await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            read(
                app.clone(),
                path,
                &format!("__Host-silkboard-owner={token}")
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
    }
    for path in [
        "/api/owner/projects?after=INVALID",
        "/api/owner/projects?limit=100000",
        "/api/owner/projects/INVALID/sessions",
        "/api/owner/projects/empty/sessions?before=0",
        "/api/owner/projects/empty/sessions?before=-1",
        "/api/owner/projects/empty/sessions?before=01",
        "/api/owner/projects/empty/sessions/9223372036854775808/events",
        "/api/owner/projects/empty/sessions/1/events?before=bad",
    ] {
        assert_eq!(
            read(app.clone(), path, &cookie).await.0,
            StatusCode::BAD_REQUEST,
            "{path}"
        );
    }
}

fn escaped(value: &str) -> String {
    use std::fmt::Write;
    let mut encoded = String::new();
    for byte in value.bytes() {
        write!(encoded, "%{byte:02X}").unwrap();
    }
    encoded
}

#[tokio::test]
async fn global_events_are_owner_only_bounded_and_keep_every_project_identity()
{
    let store = ProgressStore::open_memory().unwrap();
    let (app, cookie) = owner(store.clone()).await;
    let (status, empty) = read(app.clone(), "/api/owner/events", &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty, json!({"items":[],"next_cursor":null}));
    let mut credentials = Vec::new();
    for project in ["alpha", "bravo"] {
        credentials
            .push((project, store.issue_token(project, "agent").unwrap()));
    }
    for index in 1..=13 {
        for (project, token) in &credentials {
            let mut report = event(
                &format!("report-\"&{index}"),
                "same-native",
                if index == 13 { i64::MAX } else { index },
            );
            report.hostname = format!("dev-{project}");
            if index == 1 {
                report.session_name = Some(format!("Work on {project}"));
            }
            store.publish(project, token, &report).unwrap();
            store.publish(project, token, &report).unwrap();
        }
    }
    let (_, first) = read(app.clone(), "/api/owner/events", &cookie).await;
    assert_eq!(first["items"].as_array().unwrap().len(), 20);
    let cursor = first["next_cursor"].as_str().unwrap();
    let (_, last) = read(
        app.clone(),
        &format!("/api/owner/events?before={}", escaped(cursor)),
        &cookie,
    )
    .await;
    assert_eq!(last["items"].as_array().unwrap().len(), 6);
    assert!(last["next_cursor"].is_null());
    let all: Vec<_> = first["items"]
        .as_array()
        .unwrap()
        .iter()
        .chain(last["items"].as_array().unwrap())
        .collect();
    let identities: std::collections::HashSet<_> = all
        .iter()
        .map(|row| {
            (
                row["project"].as_str().unwrap(),
                row["event_id"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(identities.len(), 26);
    assert!(
        all.iter()
            .any(|row| row["sequence"].as_str() == Some("9223372036854775807"))
    );
    for row in &all {
        assert_eq!(row.as_object().unwrap().len(), 11);
        assert_eq!(row["session_id"], "same-native");
        let project = row["project"].as_str().unwrap();
        assert_eq!(row["hostname"], format!("dev-{project}"));
        assert_eq!(row["session_name"], format!("Work on {project}"));
    }
    for pair in all.windows(2) {
        let key = |row: &Value| {
            (
                row["received_at"].as_i64().unwrap(),
                row["project"].as_str().unwrap().to_owned(),
                row["event_id"].as_str().unwrap().to_owned(),
            )
        };
        assert!(key(pair[0]) > key(pair[1]));
    }
    assert_eq!(
        read(app.clone(), "/api/owner/events", "").await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        read(
            app.clone(),
            "/api/owner/events",
            &format!("__Host-silkboard-owner={}", credentials[0].1)
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn global_cursor_validation_and_private_response_headers() {
    let store = ProgressStore::open_memory().unwrap();
    let publisher = store.issue_token("alpha", "agent").unwrap();
    let (app, cookie) = owner(store).await;
    let denied = app
        .clone()
        .oneshot(
            Request::get("/api/owner/events")
                .header("authorization", format!("Bearer {publisher}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    let response = app
        .clone()
        .oneshot(
            Request::get("/api/owner/events")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    for cursor in [
        json!({"received_at":-1,"project":"alpha","event_id":"x"}),
        json!({"received_at":8_640_000_000_000_001_i64,"project":"alpha","event_id":"x"}),
        json!({"received_at":0,"project":"INVALID","event_id":"x"}),
        json!({"received_at":0,"project":"alpha","event_id":"bad id"}),
        json!({"received_at":0,"project":"alpha","event_id":"x","extra":true}),
        json!({"received_at":0,"project":"alpha","event_id":"x".repeat(2050)}),
    ] {
        let path = format!(
            "/api/owner/events?before={}",
            escaped(&cursor.to_string())
        );
        assert_eq!(
            read(app.clone(), &path, &cookie).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    for cursor in [
        "",
        "0",
        "[]",
        "[0,\"alpha\",\"\"]",
        "[-1,\"alpha\",\"x\"]",
        "[8640000000000001,\"alpha\",\"x\"]",
        "[0,\"INVALID\",\"x\"]",
        "[0,\"alpha\",\"bad id\"]",
    ] {
        assert_eq!(
            read(
                app.clone(),
                &format!("/api/owner/events?before={}", escaped(cursor)),
                &cookie
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        read(app, "/api/owner/events?limit=10000", &cookie).await.0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn event_filters_combine_and_validate() {
    let store = ProgressStore::open_memory().unwrap();
    let (app, cookie) = owner(store.clone()).await;
    let token = store.issue_token("alpha", "agent").unwrap();
    store
        .publish("alpha", &token, &event("one", "native", 1))
        .unwrap();
    let (status, result) = read(app.clone(), "/api/owner/events?project=alpha&agent=CODEX&from=0&to=8640000000000000", &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    for query in ["project=other", "agent=%25", "to=0"] {
        let (status, result) =
            read(app.clone(), &format!("/api/owner/events?{query}"), &cookie)
                .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(result["items"], json!([]));
    }
    for query in [
        "from=-1",
        "from=2&to=1",
        "to=invalid",
        "project=BAD",
        "agent=%0A",
    ] {
        assert_eq!(
            read(app.clone(), &format!("/api/owner/events?{query}"), &cookie)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn event_choices_use_all_received_events_and_deduplicate_agents() {
    let store = ProgressStore::open_memory().unwrap();
    let (app, cookie) = owner(store.clone()).await;
    let token = store.issue_token("alpha", "agent").unwrap();
    store.issue_token("empty", "agent").unwrap();
    for n in 1..=24 {
        let mut value = event(&format!("choice-{n}"), "native", n);
        value.hostname = format!("host-{:02}", n.min(23));
        store.publish("alpha", &token, &value).unwrap();
    }
    let (status, choices) = read(
        app.clone(),
        "/api/owner/filter-options?view=events&kind=agent&q=HOST-23",
        &cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(choices["items"], json!(["codex host-23"]));
    let (_, projects) = read(
        app.clone(),
        "/api/owner/filter-options?view=events&kind=project",
        &cookie,
    )
    .await;
    assert_eq!(projects["items"], json!(["alpha"]));
    for query in ["view=workflows&kind=agent", "view=events&kind=agent&q=%25"] {
        let (_, empty) = read(
            app.clone(),
            &format!("/api/owner/filter-options?{query}"),
            &cookie,
        )
        .await;
        assert_eq!(empty["items"], json!([]));
    }
    for query in [
        format!("view=events&kind=agent&q={}", "x".repeat(514)),
        "view=events&kind=agent&after=codex%20bad%20host".into(),
    ] {
        assert_eq!(
            read(
                app.clone(),
                &format!("/api/owner/filter-options?{query}"),
                &cookie
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        read(
            app,
            "/api/owner/filter-options?view=events&kind=project",
            ""
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn event_descriptions_reach_every_owner_projection() {
    let store = ProgressStore::open_memory().unwrap();
    set_password_from_reader(&store, b"private test owner password".as_slice())
        .unwrap();
    let token = store.issue_token("silkboard", "agent").unwrap();
    let mut value = event("detailed", "native", 1);
    value.description = Some("Context.\n\n**More detail.**".into());
    store.publish("silkboard", &token, &value).unwrap();
    let (app, cookie) = owner(store).await;
    for path in [
        "/api/owner/events",
        "/api/owner/projects/silkboard/sessions/1/events",
    ] {
        let (status, page) = read(app.clone(), path, &cookie).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            page["items"][0]["description"],
            value.description.as_deref().unwrap()
        );
        assert_eq!(page["items"][0]["message"], "Running tests");
        assert_eq!(
            read(app.clone(), path, "").await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    let (_, page) =
        read(app, "/api/owner/projects/silkboard/sessions", &cookie).await;
    assert_eq!(
        page["items"][0]["latest"]["description"],
        value.description.unwrap()
    );
}

#[tokio::test]
async fn exact_agent_filter_distinguishes_primary_and_children() {
    let store = ProgressStore::open_memory().unwrap();
    let (app, cookie) = owner(store.clone()).await;
    let token = store.issue_token("alpha", "agent").unwrap();
    for (id, child) in [
        ("root", None),
        ("child", Some("child-a")),
        ("other-child", Some("child-b")),
    ] {
        let mut report = event(id, "native", 1);
        report.subagent_id = child.map(str::to_owned);
        store.publish("alpha", &token, &report).unwrap();
    }
    let base = "/api/owner/events?project=alpha&hostname=dev&tool=codex&session_id=native";
    for (child, id) in [
        ("", "root"),
        ("child-a", "child"),
        ("child-b", "other-child"),
    ] {
        let (status, rows) =
            read(app.clone(), &format!("{base}&subagent_id={child}"), &cookie)
                .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(rows["items"].as_array().unwrap().len(), 1);
        assert_eq!(rows["items"][0]["event_id"], id);
    }
    for query in [
        "session_id=native",
        "project=alpha&hostname=dev&tool=codex&session_id=native",
        "hostname=dev&tool=codex&session_id=native&subagent_id=",
        "project=alpha&hostname=%0A&tool=codex&session_id=native&subagent_id=",
    ] {
        assert_eq!(
            read(app.clone(), &format!("/api/owner/events?{query}"), &cookie)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
}
