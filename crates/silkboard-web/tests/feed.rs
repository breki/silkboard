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
