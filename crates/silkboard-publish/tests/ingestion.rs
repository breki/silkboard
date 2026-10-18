//! The compiled client publishes to the actual Axum ingestion application.
#![cfg(unix)]
mod support;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn executable_survives_restarts_and_server_revocation() {
    let store = silkboard_web::progress::ProgressStore::open_memory().unwrap();
    let token = store.issue_token("silkboard", "client-test").unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let app = silkboard_web::create_progress_router(
        std::path::Path::new("missing"),
        store.clone(),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let mut fixture = support::Fixture::new();
    fixture.config["endpoint"] = serde_json::json!(origin);
    support::private(&fixture.root.join("token"), token.as_bytes());
    let args = fixture.args();
    let report = serde_json::to_vec(&support::report()).unwrap();
    for attempt in 0..3 {
        if attempt == 2 {
            store.revoke_token("silkboard", "client-test").unwrap();
        }
        let args = args.clone();
        let report = report.clone();
        let output = tokio::task::spawn_blocking(move || {
            use std::io::Write;
            let mut child = std::process::Command::new(env!(
                "CARGO_BIN_EXE_silkboard-publish"
            ))
            .env("PATH", "/nonexistent")
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
            child.stdin.take().unwrap().write_all(&report).unwrap();
            child.wait_with_output().unwrap()
        })
        .await
        .unwrap();
        if attempt < 2 {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8(output.stdout).unwrap().trim(),
                "Progress acknowledged"
            );
        } else {
            assert!(!output.status.success());
            assert_eq!(
                String::from_utf8(output.stderr).unwrap().trim(),
                "publisher credential not accepted"
            );
        }
    }
    let counter: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&fixture.state_files("json")[0]).unwrap(),
    )
    .unwrap();
    assert_eq!(counter["sequence"], 3);
    server.abort();
}
