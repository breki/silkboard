//! Owner-only credential administration, never a publisher capability.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use silkboard_web::{
    create_owner_router, owner::set_password_from_reader,
    progress::ProgressStore,
};
use std::path::Path;
use tower::ServiceExt;

const ORIGIN: &str = "https://silkboard.example";
const PASSWORD: &str = "private owner credential test password";
async fn call(
    app: Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Value,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 16384).await.unwrap();
    (
        status,
        headers,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        },
    )
}
async fn owner() -> (ProgressStore, Router, String, String) {
    let store = ProgressStore::open_memory().unwrap();
    set_password_from_reader(&store, PASSWORD.as_bytes()).unwrap();
    let app = create_owner_router(Path::new("missing"), store.clone(), ORIGIN)
        .unwrap();
    let response = call(
        app.clone(),
        "POST",
        "/api/auth/login",
        &[("origin", ORIGIN)],
        json!({"password":PASSWORD}),
    )
    .await;
    assert_eq!(response.0, StatusCode::NO_CONTENT);
    let cookie = response.1["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let response = call(
        app.clone(),
        "GET",
        "/api/owner/session",
        &[("cookie", &cookie)],
        Value::Null,
    )
    .await;
    (
        store,
        app,
        cookie,
        response.2["csrf_token"].as_str().unwrap().to_owned(),
    )
}
#[tokio::test]
async fn owner_can_issue_list_and_revoke_without_retrieving_existing_secrets() {
    let (store, app, cookie, csrf) = owner().await;
    let path = "/api/owner/projects/silkboard/publishers";
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    let (status, reply_headers, issued) = call(
        app.clone(),
        "POST",
        path,
        &headers,
        json!({"label":"dev-vm"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(reply_headers["cache-control"], "no-store");
    let token = issued["token"].as_str().unwrap();
    assert_eq!(token.len(), 64);
    assert_eq!(issued.as_object().unwrap().len(), 1);
    let listed = call(app.clone(), "GET", path, &headers, Value::Null).await;
    assert_eq!(
        listed.2,
        json!({"items":[{"label":"dev-vm","revoked":false}],"next_cursor":null})
    );
    assert!(!listed.2.to_string().contains(token));
    assert_eq!(
        call(
            app.clone(),
            "POST",
            path,
            &headers,
            json!({"label":"dev-vm"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let event=serde_json::from_value(json!({"event_id":"before-revoke","hostname":"dev","tool":"codex","session_id":"native","sequence":1,"message":"Retain history"})).unwrap();
    store.publish("silkboard", token, &event).unwrap();
    assert_eq!(
        call(
            app.clone(),
            "POST",
            &format!("{path}/revoke"),
            &headers,
            json!({"label":"dev-vm"})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert!(matches!(
        store.publish("silkboard", token, &event),
        Err(silkboard_web::progress::ProgressError::Unauthorized)
    ));
    let listed = call(app.clone(), "GET", path, &headers, Value::Null).await;
    assert_eq!(listed.2["items"][0]["revoked"], true);
    assert_eq!(
        call(app, "POST", path, &headers, json!({"label":"dev-vm"}))
            .await
            .0,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn publisher_tokens_and_forged_owner_mutations_cannot_manage_credentials()
{
    let (store, app, cookie, csrf) = owner().await;
    let publisher = store.issue_token("silkboard", "agent").unwrap();
    let path = "/api/owner/projects/silkboard/publishers";
    for headers in [
        vec![],
        vec![("authorization", format!("Bearer {publisher}"))],
        vec![("cookie", format!("__Host-silkboard-owner={publisher}"))],
    ] {
        let mut borrowed = headers
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect::<Vec<_>>();
        // Even a CSRF value derived from the known publisher token must not
        // turn that token into a valid owner session.
        let forged_csrf = format!(
            "{:x}",
            Sha256::digest(
                format!("silkboard-owner-csrf:{publisher}").as_bytes()
            )
        );
        borrowed.extend([
            ("origin", ORIGIN),
            ("x-silkboard-csrf", forged_csrf.as_str()),
        ]);
        for (method, endpoint) in [
            ("GET", path.to_owned()),
            ("POST", path.to_owned()),
            ("POST", format!("{path}/revoke")),
        ] {
            assert_eq!(
                call(
                    app.clone(),
                    method,
                    &endpoint,
                    &borrowed,
                    json!({"label":"forged"})
                )
                .await
                .0,
                StatusCode::UNAUTHORIZED
            );
        }
    }
    for extra in [
        vec![],
        vec![("origin", ORIGIN)],
        vec![
            ("origin", "https://evil.example"),
            ("x-silkboard-csrf", csrf.as_str()),
        ],
        vec![("origin", ORIGIN), ("x-silkboard-csrf", "wrong")],
        vec![
            ("origin", ORIGIN),
            ("origin", ORIGIN),
            ("x-silkboard-csrf", csrf.as_str()),
        ],
        vec![
            ("origin", ORIGIN),
            ("sec-fetch-site", "cross-site"),
            ("x-silkboard-csrf", csrf.as_str()),
        ],
    ] {
        let mut headers = vec![("cookie", cookie.as_str())];
        headers.extend(extra);
        for endpoint in [path.to_owned(), format!("{path}/revoke")] {
            assert_eq!(
                call(
                    app.clone(),
                    "POST",
                    &endpoint,
                    &headers,
                    json!({"label":"forged"})
                )
                .await
                .0,
                StatusCode::FORBIDDEN
            );
        }
    }
    let valid = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    assert_eq!(
        call(
            app.clone(),
            "POST",
            "/api/owner/logout",
            &valid,
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(app, "POST", path, &valid, json!({"label":"after-logout"}))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn metadata_is_bounded_scoped_and_keeps_hostile_labels_as_text() {
    let (store, app, cookie, _) = owner().await;
    for index in 0..25 {
        store
            .issue_token("silkboard", &format!("dev-{index:02}"))
            .unwrap();
    }
    store.issue_token("other", "not-visible").unwrap();
    let headers = [("cookie", cookie.as_str())];
    let path = "/api/owner/projects/silkboard/publishers";
    let first = call(app.clone(), "GET", path, &headers, Value::Null).await;
    assert_eq!(first.0, StatusCode::OK);
    assert_eq!(first.2["items"].as_array().unwrap().len(), 20);
    assert_eq!(first.2["next_cursor"], "dev-19");
    let second = call(
        app.clone(),
        "GET",
        &format!("{path}?after=dev-19"),
        &headers,
        Value::Null,
    )
    .await;
    assert_eq!(second.2["items"].as_array().unwrap().len(), 5);
    assert_eq!(second.2["next_cursor"], Value::Null);
    assert!(!first.2.to_string().contains("not-visible"));
    store
        .issue_token("hostile", "<script>alert(1)</script>")
        .unwrap();
    let hostile = call(
        app.clone(),
        "GET",
        "/api/owner/projects/hostile/publishers",
        &headers,
        Value::Null,
    )
    .await;
    assert_eq!(
        hostile.2["items"][0],
        json!({"label":"<script>alert(1)</script>","revoked":false})
    );
    for endpoint in [
        "/api/owner/projects/INVALID/publishers",
        &format!("{path}?after=has%20spaces"),
        &format!("{path}?extra=invalid"),
    ] {
        assert_eq!(
            call(app.clone(), "GET", endpoint, &headers, Value::Null)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn invalid_inputs_and_concurrent_duplicate_issuance_do_not_replace_a_token()
 {
    let (store, app, cookie, csrf) = owner().await;
    let path = "/api/owner/projects/new-project/publishers";
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    for input in [
        json!({"label":""}),
        json!({"label":"bad label"}),
        json!({"label":"é"}),
        json!({"label":"x".repeat(257)}),
        json!({"label":"dev","token":"must-not-be-accepted"}),
        json!({"label":"x".repeat(5000)}),
    ] {
        let result = call(app.clone(), "POST", path, &headers, input).await;
        assert_eq!(result.0, StatusCode::BAD_REQUEST);
        assert_eq!(result.1["cache-control"], "no-store");
        assert!(!result.2.to_string().contains("must-not-be-accepted"));
    }
    let (a, b) = tokio::join!(
        call(app.clone(), "POST", path, &headers, json!({"label":"same"})),
        call(app.clone(), "POST", path, &headers, json!({"label":"same"}))
    );
    assert!(matches!(
        (a.0, b.0),
        (StatusCode::CREATED, StatusCode::CONFLICT)
            | (StatusCode::CONFLICT, StatusCode::CREATED)
    ));
    let issued = if a.0 == StatusCode::CREATED { a.2 } else { b.2 };
    let token = issued["token"].as_str().unwrap();
    let event=serde_json::from_value(json!({"event_id":"valid","hostname":"dev","tool":"codex","session_id":"native","sequence":1,"message":"Unique credential remains valid"})).unwrap();
    store.publish("new-project", token, &event).unwrap();
    for _ in 0..2 {
        assert_eq!(
            call(
                app.clone(),
                "POST",
                &format!("{path}/revoke"),
                &headers,
                json!({"label":"missing"})
            )
            .await
            .0,
            StatusCode::NO_CONTENT
        );
    }
    let history = call(
        app,
        "GET",
        "/api/owner/projects/new-project/sessions",
        &headers,
        Value::Null,
    )
    .await;
    assert_eq!(
        history.2["items"][0]["latest"]["message"],
        "Unique credential remains valid"
    );
}
