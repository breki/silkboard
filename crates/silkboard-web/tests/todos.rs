//! Persistent owner-managed checklist boundaries.
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
    owner_for(ProgressStore::open_memory().unwrap()).await
}
async fn owner_for(
    store: ProgressStore,
) -> (ProgressStore, Router, String, String) {
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
async fn checklist_mutations_promote_children_and_keep_completion_independent()
{
    let (_, app, cookie, csrf) = owner().await;
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    let path = "/api/owner/todos";
    let result = call(
        app.clone(),
        "POST",
        path,
        &headers,
        json!({"revision":"0","parent_id":null,"text":"Parent"}),
    )
    .await;
    assert_eq!(result.0, StatusCode::CREATED);
    let parent = result.2["items"][0]["id"].as_str().unwrap();
    let result = call(
        app.clone(),
        "POST",
        path,
        &headers,
        json!({"revision":"1","parent_id":parent,"text":"Child"}),
    )
    .await;
    let child = result.2["items"][1]["id"].as_str().unwrap();
    let result = call(
        app.clone(),
        "PATCH",
        &format!("{path}/{parent}"),
        &headers,
        json!({"revision":"2","text":"Edited parent","completed":true}),
    )
    .await;
    assert_eq!(result.0, StatusCode::OK);
    assert_eq!(result.2["items"][0]["completed"], true);
    assert_eq!(result.2["items"][1]["completed"], false);
    let result = call(
        app.clone(),
        "DELETE",
        &format!("{path}/{parent}"),
        &headers,
        json!({"revision":"3"}),
    )
    .await;
    assert_eq!(result.0, StatusCode::OK);
    assert_eq!(
        result.2,
        json!({"revision":"4","items":[{"id":child,"parent_id":null,"text":"Child","completed":false}]})
    );
    assert_eq!(
        call(
            app,
            "POST",
            path,
            &headers,
            json!({"revision":"2","parent_id":null,"text":"Stale"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn owner_only_integrity_invalid_inputs_and_depth_limit() {
    let (store, app, cookie, csrf) = owner().await;
    let path = "/api/owner/todos";
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    let token = store.issue_token("alpha", "test").unwrap();
    for method in ["GET", "POST", "PATCH", "DELETE"] {
        let target = if method == "PATCH" || method == "DELETE" {
            format!("{path}/1")
        } else {
            path.to_owned()
        };
        assert_eq!(
            call(
                app.clone(),
                method,
                &target,
                &[("authorization", &format!("Bearer {token}"))],
                json!({})
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
    }
    for bad in [
        vec![("cookie", cookie.as_str()), ("origin", ORIGIN)],
        vec![
            ("cookie", cookie.as_str()),
            ("origin", "https://evil.example"),
            ("x-silkboard-csrf", csrf.as_str()),
        ],
    ] {
        assert_eq!(
            call(
                app.clone(),
                "POST",
                path,
                &bad,
                json!({"revision":"0","text":"No"})
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    for value in [
        json!({"revision":"00","text":"No"}),
        json!({"revision":"-1","text":"No"}),
        json!({"revision":"bad","text":"No"}),
        json!({"revision":"0","text":" "}),
        json!({"revision":"0","text":"x\ny"}),
        json!({"revision":"0","text":"é".repeat(501)}),
        json!({"revision":"0","text":"x","parent_id":"01"}),
        json!({"revision":"0","text":"x","extra":true}),
        json!({"revision":0,"text":"x"}),
    ] {
        assert_eq!(
            call(app.clone(), "POST", path, &headers, value).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        call(
            app.clone(),
            "POST",
            path,
            &headers,
            json!({"revision":"0","text":"Missing parent","parent_id":"1"})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    set_password_from_reader(&store, PASSWORD.as_bytes()).unwrap();
    assert_eq!(
        call(app, "GET", path, &headers, Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn depth_limit_and_missing_targets_preserve_the_list() {
    let (store, app, cookie, csrf) = owner().await;
    let path = "/api/owner/todos";
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    for revision in 0..5 {
        let parent = if revision == 0 {
            None
        } else {
            Some(revision.to_string())
        };
        assert_eq!(call(app.clone(),"POST",path,&headers,json!({"revision":revision.to_string(),"parent_id":parent,"text":format!("Level {revision}")})).await.0,StatusCode::CREATED);
    }
    assert_eq!(
        call(
            app.clone(),
            "POST",
            path,
            &headers,
            json!({"revision":"5","parent_id":"5","text":"Too deep"})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    for method in ["PATCH", "DELETE"] {
        let value = if method == "PATCH" {
            json!({"revision":"5","text":"Missing","completed":false})
        } else {
            json!({"revision":"5"})
        };
        assert_eq!(
            call(app.clone(), method, &format!("{path}/999"), &headers, value)
                .await
                .0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            call(
                app.clone(),
                method,
                &format!("{path}/bad"),
                &headers,
                json!({"revision":"5","text":"x","completed":false})
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    let listed = call(app.clone(), "GET", path, &headers, Value::Null).await;
    assert_eq!(listed.1["cache-control"], "no-store");
    assert_eq!(listed.2["revision"], "5");
    assert_eq!(listed.2["items"].as_array().unwrap().len(), 5);
    set_password_from_reader(&store, PASSWORD.as_bytes()).unwrap();
    assert_eq!(
        call(app, "GET", path, &headers, Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn simultaneous_edits_cannot_overwrite_and_nested_deletion_promotes_only_direct_children()
 {
    let (_, app, cookie, csrf) = owner().await;
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    let path = "/api/owner/todos";
    for (revision, parent, text) in [
        ("0", None, "Root"),
        ("1", Some("1"), "Middle"),
        ("2", Some("2"), "Leaf"),
        ("3", None, "Other root"),
    ] {
        assert_eq!(
            call(
                app.clone(),
                "POST",
                path,
                &headers,
                json!({"revision":revision,"parent_id":parent,"text":text})
            )
            .await
            .0,
            StatusCode::CREATED
        );
    }
    let body = json!({"revision":"4","text":"Changed","completed":true});
    let target = format!("{path}/1");
    let (a, b) = tokio::join!(
        call(app.clone(), "PATCH", &target, &headers, body.clone()),
        call(app.clone(), "PATCH", &target, &headers, body)
    );
    assert!(
        (a.0 == StatusCode::OK && b.0 == StatusCode::CONFLICT)
            || (b.0 == StatusCode::OK && a.0 == StatusCode::CONFLICT)
    );
    let result = call(
        app,
        "DELETE",
        &format!("{path}/2"),
        &headers,
        json!({"revision":"5"}),
    )
    .await;
    assert_eq!(result.0, StatusCode::OK);
    assert_eq!(
        result.2["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["1", "3", "4"]
    );
    assert_eq!(result.2["items"][1]["parent_id"], "1");
    assert_eq!(result.2["items"][1]["completed"], false);
}

struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "silkboard-plan-data-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&p).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                &p,
                std::fs::Permissions::from_mode(0o700),
            )
            .unwrap();
        }
        Self(p)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[tokio::test]
async fn checklist_survives_restart_and_private_backup_restore() {
    use silkboard_web::progress::database_command;
    let dir = Directory::new();
    let live = dir.0.join("live.sqlite");
    let backup = dir.0.join("backup.sqlite");
    let restored = dir.0.join("restored.sqlite");
    let (store, app, cookie, csrf) =
        owner_for(ProgressStore::open(&live).unwrap()).await;
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    let path = "/api/owner/todos";
    call(
        app.clone(),
        "POST",
        path,
        &headers,
        json!({"revision":"0","text":"Retain parent"}),
    )
    .await;
    call(
        app.clone(),
        "POST",
        path,
        &headers,
        json!({"revision":"1","parent_id":"1","text":"Retain child"}),
    )
    .await;
    let expected = call(
        app.clone(),
        "PATCH",
        &format!("{path}/2"),
        &headers,
        json!({"revision":"2","text":"Retain child","completed":true}),
    )
    .await
    .2;
    drop(app);
    drop(store);
    let restarted = ProgressStore::open(&live).unwrap();
    let app =
        create_owner_router(Path::new("missing"), restarted.clone(), ORIGIN)
            .unwrap();
    assert_eq!(
        call(app, "GET", path, &headers, Value::Null).await.2,
        expected
    );
    drop(restarted);
    database_command(
        &live,
        &["backup".into(), backup.to_str().unwrap().into()],
    )
    .unwrap();
    database_command(
        &restored,
        &["restore".into(), backup.to_str().unwrap().into()],
    )
    .unwrap();
    let (_, app, cookie, csrf) =
        owner_for(ProgressStore::open(&restored).unwrap()).await;
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    assert_eq!(
        call(app, "GET", path, &headers, Value::Null).await.2,
        expected
    );
    let conn = rusqlite::Connection::open(&restored).unwrap();
    conn.execute("UPDATE todo_schema SET version=2", [])
        .unwrap();
    drop(conn);
    assert!(ProgressStore::open(&restored).is_err());
    assert!(
        database_command(
            &restored,
            &[
                "backup".into(),
                dir.0.join("future.sqlite").to_str().unwrap().into()
            ]
        )
        .is_err()
    );
}

#[test]
fn private_backups_remain_valid_during_concurrent_checklist_edits() {
    use silkboard_web::progress::database_command;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let dir = Directory::new();
    let live = dir.0.join("live.sqlite");
    drop(ProgressStore::open(&live).unwrap());
    let conn = rusqlite::Connection::open(&live).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    drop(conn);
    let running = Arc::new(AtomicBool::new(true));
    let active = running.clone();
    let path = live.clone();
    let writer = std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.busy_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        let mut count = 0;
        while active.load(Ordering::Acquire) {
            conn.execute(
                "INSERT INTO todos(text) VALUES('Concurrent task')",
                [],
            )
            .unwrap();
            conn.execute("DELETE FROM todos", []).unwrap();
            count += 1;
        }
        count
    });
    let results: Vec<_> = (0..30)
        .map(|n| {
            database_command(
                &live,
                &[
                    "backup".into(),
                    dir.0
                        .join(format!("backup-{n}.sqlite"))
                        .to_str()
                        .unwrap()
                        .into(),
                ],
            )
        })
        .collect();
    running.store(false, Ordering::Release);
    assert!(writer.join().unwrap() > 0);
    assert!(results.into_iter().all(|result| result.is_ok()));
}
