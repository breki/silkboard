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
    let bytes = to_bytes(response.into_body(), 3_000_000).await.unwrap();
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
            conn.execute_batch("BEGIN IMMEDIATE;INSERT INTO todos(text) VALUES('Concurrent task');UPDATE todo_schema SET revision=revision+1;COMMIT;").unwrap();
            conn.execute_batch("BEGIN IMMEDIATE;DELETE FROM todos;UPDATE todo_schema SET revision=revision+1;COMMIT;").unwrap();
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
    for (index, result) in results.into_iter().enumerate() {
        result.unwrap_or_else(|error| {
            panic!("concurrent backup {index} failed: {error:?}")
        });
    }
}

#[tokio::test]
async fn document_save_reorders_reparents_and_commits_one_revision() {
    let (_, app, cookie, csrf) = owner().await;
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    for (revision, parent, text) in [
        ("0", None, "Parent"),
        ("1", Some("1"), "Child"),
        ("2", None, "Other"),
    ] {
        assert_eq!(
            call(
                app.clone(),
                "POST",
                "/api/owner/todos",
                &headers,
                json!({"revision":revision,"parent_id":parent,"text":text})
            )
            .await
            .0,
            StatusCode::CREATED
        );
    }
    let body = json!({"revision":"3","items":[
        {"key":"other","id":"3","parent_key":null,"text":" Other ","completed":false},
        {"key":"new","id":null,"parent_key":"other","text":"New parent","completed":true},
        {"key":"parent","id":"1","parent_key":"new","text":"Parent","completed":false},
        {"key":"child","id":"2","parent_key":"parent","text":"Child","completed":true}
    ]});
    let result = call(
        app.clone(),
        "PUT",
        "/api/owner/todos/document",
        &headers,
        body.clone(),
    )
    .await;
    assert_eq!(result.0, StatusCode::OK);
    assert_eq!(result.2["revision"], "4");
    assert_eq!(
        result.2["keys"],
        json!({"other":"3","new":"4","parent":"1","child":"2"})
    );
    assert_eq!(
        result.2["items"],
        json!([
            {"id":"3","parent_id":null,"text":" Other ","completed":false},
            {"id":"4","parent_id":"3","text":"New parent","completed":true},
            {"id":"1","parent_id":"4","text":"Parent","completed":false},
            {"id":"2","parent_id":"1","text":"Child","completed":true}
        ])
    );
    assert_eq!(
        call(
            app.clone(),
            "PUT",
            "/api/owner/todos/document",
            &headers,
            body
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let loaded =
        call(app, "GET", "/api/owner/todos", &headers, Value::Null).await;
    assert_eq!(loaded.2["revision"], "4");
    assert_eq!(loaded.2["items"], result.2["items"]);
}

#[tokio::test]
async fn document_save_rejects_invalid_trees_and_requires_owner_integrity() {
    let (store, app, cookie, csrf) = owner().await;
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    let row = json!({"key":"a","id":null,"parent_key":null,"text":"Task","completed":false});
    for items in [
        json!([row.clone(), row.clone()]),
        json!([{ "key":"a","id":null,"parent_key":"a","text":"Cycle","completed":false}]),
        json!([{ "key":"a","id":null,"parent_key":"missing","text":"Orphan","completed":false}]),
        json!([{ "key":"a","id":"01","parent_key":null,"text":"Invalid id","completed":false}]),
        json!([{ "key":"a","id":null,"parent_key":null,"text":" ","completed":false}]),
        json!([{ "key":"a","id":null,"parent_key":null,"text":"x\ny","completed":false}]),
        json!([{ "key":"a","id":null,"parent_key":null,"text":"Task","completed":false,"extra":true}]),
        json!([
            {"key":"a","id":null,"parent_key":null,"text":"A","completed":false},
            {"key":"b","id":null,"parent_key":null,"text":"B","completed":false},
            {"key":"c","id":null,"parent_key":"a","text":"Closed subtree","completed":false}
        ]),
    ] {
        assert_eq!(
            call(
                app.clone(),
                "PUT",
                "/api/owner/todos/document",
                &headers,
                json!({"revision":"0","items":items})
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            call(
                app.clone(),
                "GET",
                "/api/owner/todos",
                &headers,
                Value::Null
            )
            .await
            .2,
            json!({"revision":"0","items":[]})
        );
    }
    let valid = json!({"revision":"0","items":[row]});
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
                "PUT",
                "/api/owner/todos/document",
                &bad,
                valid.clone()
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    let token = store.issue_token("alpha", "test").unwrap();
    assert_eq!(
        call(
            app,
            "PUT",
            "/api/owner/todos/document",
            &[("authorization", &format!("Bearer {token}"))],
            valid
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn document_limits_and_missing_ids_never_change_the_saved_revision() {
    let (_, app, cookie, csrf) = owner().await;
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    let entry = |key: String, parent: Option<String>| json!({"key":key,"id":null,"parent_key":parent,"text":"Task","completed":false});
    let mut invalid = vec![
        (
            json!({"revision":"0","items":[{"key":"missing","id":"1","parent_key":null,"text":"Task","completed":false}]}),
            StatusCode::NOT_FOUND,
        ),
        (
            json!({"revision":"0","items":[entry(String::new(),None)]}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"revision":"0","items":[entry("x".repeat(65),None)]}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"revision":"0","items":[entry("unicode-é".into(),None)]}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"revision":"0","items":(0..1001).map(|n|entry(format!("n{n}"),None)).collect::<Vec<_>>()}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"revision":"0","items":(0..6).map(|n|entry(format!("n{n}"),if n==0 {None}else{Some(format!("n{}",n-1))})).collect::<Vec<_>>()}),
            StatusCode::BAD_REQUEST,
        ),
    ];
    for text in ["é".repeat(501), "x\u{2028}y".into(), "x\u{feff}y".into()] {
        invalid.push((json!({"revision":"0","items":[{"key":"a","id":null,"parent_key":null,"text":text,"completed":false}]}),StatusCode::BAD_REQUEST));
    }
    for (body, status) in invalid {
        assert_eq!(
            call(
                app.clone(),
                "PUT",
                "/api/owner/todos/document",
                &headers,
                body
            )
            .await
            .0,
            status
        );
    }
    assert_eq!(
        call(app, "GET", "/api/owner/todos", &headers, Value::Null)
            .await
            .2,
        json!({"revision":"0","items":[]})
    );
}

fn legacy_text_edit(path: &Path, id: i64, text: &str) {
    let mut conn = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        conn.query_row("SELECT version FROM todo_schema", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1,
        "an older app must still admit the checklist after an upgrade"
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM todos WHERE parent_id>=id",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let tx = conn.transaction().unwrap();
    tx.execute(
        "UPDATE todos SET text=?1 WHERE id=?2",
        rusqlite::params![text, id],
    )
    .unwrap();
    tx.execute("UPDATE todo_schema SET revision=revision+1", [])
        .unwrap();
    tx.commit().unwrap();
}

#[tokio::test]
async fn document_reordering_survives_restart_backup_and_restore() {
    use silkboard_web::progress::database_command;
    let dir = Directory::new();
    let live = dir.0.join("document.sqlite");
    let backup = dir.0.join("backup.sqlite");
    let restored = dir.0.join("restored.sqlite");
    let (store, app, cookie, csrf) =
        owner_for(ProgressStore::open(&live).unwrap()).await;
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    for (revision, text) in [("0", "First"), ("1", "Second")] {
        assert_eq!(
            call(
                app.clone(),
                "POST",
                "/api/owner/todos",
                &headers,
                json!({"revision":revision,"text":text})
            )
            .await
            .0,
            StatusCode::CREATED
        );
    }
    let mut expected =call(app.clone(),"PUT","/api/owner/todos/document",&headers,json!({"revision":"2","items":[
        {"key":"second","id":"2","parent_key":null,"text":"Second","completed":false},
        {"key":"first","id":"1","parent_key":"second","text":"First","completed":true}
    ]})).await;
    assert_eq!(expected.0, StatusCode::OK);
    legacy_text_edit(&live, 1, "Legacy edit");
    expected = call(
        app.clone(),
        "GET",
        "/api/owner/todos",
        &headers,
        Value::Null,
    )
    .await;
    assert_eq!(expected.2["items"][0]["id"], "2");
    assert_eq!(expected.2["items"][1]["parent_id"], "2");
    assert_eq!(expected.2["items"][1]["text"], "Legacy edit");
    drop(app);
    drop(store);
    let restarted = ProgressStore::open(&live).unwrap();
    let app =
        create_owner_router(Path::new("missing"), restarted.clone(), ORIGIN)
            .unwrap();
    assert_eq!(
        call(app, "GET", "/api/owner/todos", &headers, Value::Null)
            .await
            .2["items"],
        expected.2["items"]
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
    assert_eq!(
        call(
            app,
            "GET",
            "/api/owner/todos",
            &[
                ("cookie", &cookie),
                ("origin", ORIGIN),
                ("x-silkboard-csrf", &csrf)
            ],
            Value::Null
        )
        .await
        .2["items"],
        expected.2["items"]
    );
}

#[tokio::test]
async fn document_allocation_failure_rolls_back_partial_reordering_and_deletion()
 {
    let dir = Directory::new();
    let live = dir.0.join("failure.sqlite");
    let (_, app, cookie, csrf) =
        owner_for(ProgressStore::open(&live).unwrap()).await;
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    for (revision, parent, text) in
        [("0", None, "Parent"), ("1", Some("1"), "Child")]
    {
        call(
            app.clone(),
            "POST",
            "/api/owner/todos",
            &headers,
            json!({"revision":revision,"parent_id":parent,"text":text}),
        )
        .await;
    }
    let before = call(
        app.clone(),
        "GET",
        "/api/owner/todos",
        &headers,
        Value::Null,
    )
    .await
    .2;
    let conn = rusqlite::Connection::open(&live).unwrap();
    conn.execute(
        "UPDATE sqlite_sequence SET seq=9223372036854775807 WHERE name='todos'",
        [],
    )
    .unwrap();
    let result=call(app.clone(),"PUT","/api/owner/todos/document",&headers,json!({"revision":"2","items":[
        {"key":"child","id":"2","parent_key":null,"text":"Changed","completed":true},
        {"key":"new","id":null,"parent_key":null,"text":"Cannot allocate","completed":false}
    ]})).await;
    assert_eq!(result.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        call(app, "GET", "/api/owner/todos", &headers, Value::Null)
            .await
            .2,
        before
    );
}

#[tokio::test]
async fn complete_bounded_document_can_exceed_the_small_owner_form_limit() {
    let (_, app, cookie, csrf) = owner().await;
    let headers = [
        ("cookie", cookie.as_str()),
        ("origin", ORIGIN),
        ("x-silkboard-csrf", csrf.as_str()),
    ];
    let text = "\"".repeat(1000);
    let items:Vec<_>=(0..1000).map(|n|json!({"key":format!("task-{n}"),"id":null,"parent_key":null,"text":text,"completed":false})).collect();
    let body = json!({"revision":"0","items":items});
    assert!(body.to_string().len() > 2_000_000);
    let saved = call(
        app.clone(),
        "PUT",
        "/api/owner/todos/document",
        &headers,
        body,
    )
    .await;
    assert_eq!(saved.0, StatusCode::OK);
    assert_eq!(saved.2["items"].as_array().unwrap().len(), 1000);
    assert_eq!(saved.2["items"][999]["text"], text);
    let request = Request::builder()
        .method("PUT")
        .uri("/api/owner/todos/document")
        .header("cookie", &cookie)
        .header("origin", ORIGIN)
        .header("x-silkboard-csrf", &csrf)
        .header("content-type", "application/json")
        .body(Body::from(vec![b' '; 3_000_001]))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            app,
            "POST",
            "/api/auth/login",
            &[("origin", ORIGIN)],
            json!({"password":"x".repeat(5000)})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}
