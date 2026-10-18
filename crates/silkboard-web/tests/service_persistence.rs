//! Process/release-directory replacement with disposable private storage.
use silkboard_web::{owner::set_password_from_reader, progress::ProgressStore};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Server(Child, SocketAddr);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn start(release: &Path, database: &Path) -> Server {
    let child = Command::new(release.join("silkboard-web"))
        .current_dir(release)
        .env("SILKBOARD_DATABASE", database)
        .env("SILKBOARD_OWNER_ORIGIN", "https://silkboard.example")
        .env("SILKBOARD_FRONTEND", release.join("frontend"))
        .env("SILKBOARD_LISTEN", "127.0.0.1:0")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut server = Server(child, SocketAddr::from(([127, 0, 0, 1], 0)));
    let mut line = String::new();
    BufReader::new(server.0.stderr.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    server.1 = line
        .trim()
        .strip_prefix("Silkboard listening on http://")
        .unwrap()
        .parse()
        .unwrap();
    server
}
fn request(server: &Server, path: &str, token: Option<&str>) -> String {
    let mut socket =
        TcpStream::connect_timeout(&server.1, Duration::from_secs(2)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let body=serde_json::json!({"event_id":path,"hostname":"dev","tool":"codex","session_id":"persist","sequence":if path=="first"{1}else{2},"message":"Durable observation"}).to_string();
    let request = if let Some(token) = token {
        format!(
            "POST /api/projects/silkboard/progress HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    } else {
        format!(
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
    };
    socket.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).unwrap();
    response
}
#[test]
fn data_survives_process_restart_and_replacement_of_release_directory() {
    let root = Directory(std::env::temp_dir().join(format!(
            "silkboard-service-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    fs::create_dir(&root.0).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root.0, fs::Permissions::from_mode(0o700))
            .unwrap();
    }
    let database = root.0.join("progress.sqlite");
    let store = ProgressStore::open(&database).unwrap();
    set_password_from_reader(
        &store,
        b"private service fixture password".as_slice(),
    )
    .unwrap();
    let token = store.issue_token("silkboard", "persistent").unwrap();
    for (release, event) in [
        ("release-a", "first"),
        ("release-a", "second"),
        ("release-b", "second"),
    ] {
        let path = root.0.join(release);
        if !path.exists() {
            fs::create_dir_all(path.join("frontend")).unwrap();
            fs::copy(
                env!("CARGO_BIN_EXE_silkboard-web"),
                path.join("silkboard-web"),
            )
            .unwrap();
            fs::write(path.join("frontend/index.html"), release).unwrap();
        }
        let server = start(&path, &database);
        assert!(request(&server, "/", None).contains(release));
        assert!(
            request(&server, "/api/owner/projects", None)
                .starts_with("HTTP/1.1 401")
        );
        assert!(
            request(&server, event, Some(&token)).starts_with("HTTP/1.1 200")
        );
        drop(server);
    }
    let connection = rusqlite::Connection::open(&database).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM events", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM publishers WHERE revoked=0",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM owner", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}
