#![allow(dead_code)] // Each integration test imports only the fixture helpers it needs.
use serde_json::{Value, json};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    ffi::OsString,
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
pub fn finish_cli(
    mut child: std::process::Child,
    input: &[u8],
) -> std::process::Output {
    let written = child.stdin.take().unwrap().write_all(input);
    let output = child.wait_with_output().unwrap();
    if let Err(error) = written {
        // Invalid invocations may reject before reading stdin. Keep their
        // real exit/output available for assertions; never hide other I/O
        // errors or a successful child that failed to consume its input.
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        assert!(
            !output.status.success(),
            "successful CLI did not consume input"
        );
        assert_eq!(
            output.status.code(),
            Some(1),
            "CLI did not reject normally"
        );
    }
    output
}

pub struct Fixture {
    pub root: PathBuf,
    pub config: Value,
}
impl Fixture {
    pub fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "silkboard-publisher-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        private_directory(&root);
        let state = root.join("state");
        fs::create_dir(&state).unwrap();
        private_directory(&state);
        let token = root.join("token");
        private(&token, "a".repeat(64).as_bytes());
        let config = json!({"endpoint":"http://127.0.0.1:9","project":"silkboard","token_file":token,"state_dir":state});
        Self { root, config }
    }
    pub fn args(&self) -> Vec<OsString> {
        let path = self.root.join("config.json");
        private(&path, &serde_json::to_vec(&self.config).unwrap());
        vec!["publish".into(), "--config".into(), path.into()]
    }
    pub fn run(
        &self,
        report: &Value,
    ) -> Result<&'static str, silkboard_publish::Error> {
        silkboard_publish::run(
            &self.args(),
            serde_json::to_vec(report).unwrap().as_slice(),
        )
    }
    pub fn state_files(&self, extension: &str) -> Vec<PathBuf> {
        fs::read_dir(self.root.join("state"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == extension))
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
pub fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn private_directory(path: &Path) {
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    #[cfg(windows)]
    {
        use windows_permissions::{
            LocalBox, SecurityDescriptor,
            constants::{SeObjectType, SecurityInformation},
            utilities::current_process_sid,
            wrappers::SetNamedSecurityInfo,
        };
        let user = current_process_sid().unwrap();
        let sd: LocalBox<SecurityDescriptor> =
            format!("D:P(A;OICI;FA;;;{user})").parse().unwrap();
        SetNamedSecurityInfo(
            path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
            None,
            None,
            sd.dacl(),
            None,
        )
        .unwrap();
    }
}
pub fn report() -> Value {
    json!({"hostname":"dev","tool":"codex","session_id":"native-1","message":"Running behavioral tests"})
}

pub fn server(
    count: usize,
    respond: impl FnMut(Value) -> (u16, String) + Send + 'static,
) -> (String, thread::JoinHandle<Vec<Value>>) {
    server_at(count, "progress", respond)
}

pub fn server_at(
    count: usize,
    route: &'static str,
    respond: impl FnMut(Value) -> (u16, String) + Send + 'static,
) -> (String, thread::JoinHandle<Vec<Value>>) {
    server_projects(count, "silkboard", route, respond)
}

pub fn server_projects(
    count: usize,
    projects: &'static str,
    route: &'static str,
    mut respond: impl FnMut(Value) -> (u16, String) + Send + 'static,
) -> (String, thread::JoinHandle<Vec<Value>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let mut events = Vec::new();
        for _ in 0..count {
            let start = Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            start.elapsed() < Duration::from_secs(6),
                            "Client never connected"
                        );
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => panic!("accept: {e}"),
                }
            };
            // Windows inherits the listener's nonblocking mode on accept.
            // Request reads use the existing bounded blocking timeout.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(4)))
                .unwrap();
            let mut bytes = Vec::new();
            let split = loop {
                let mut chunk = [0; 1024];
                let n = stream.read(&mut chunk).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
                assert!(bytes.len() < 16384);
                if let Some(i) = bytes.windows(4).position(|v| v == b"\r\n\r\n")
                {
                    break i + 4;
                }
            };
            let headers = std::str::from_utf8(&bytes[..split])
                .unwrap()
                .to_ascii_lowercase();
            assert!(projects.split('|').any(|project| route.split('|').any(
                |route| headers.starts_with(&format!(
                    "post /api/projects/{project}/{route} http/1.1\r\n"
                ))
            )));
            assert!(headers.contains(&format!(
                "authorization: bearer {}\r\n",
                "a".repeat(64)
            )));
            assert!(headers.contains("content-type: application/json\r\n"));
            let length = headers
                .lines()
                .find_map(|l| l.strip_prefix("content-length: "))
                .unwrap()
                .parse::<usize>()
                .unwrap();
            while bytes.len() < split + length {
                let mut b = [0; 1024];
                let n = stream.read(&mut b).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&b[..n]);
            }
            let event: Value =
                serde_json::from_slice(&bytes[split..split + length]).unwrap();
            let (status, body) = respond(event.clone());
            let wire = format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(wire.as_bytes());
            events.push(event);
        }
        events
    });
    (origin, handle)
}
pub fn ack(event: &Value) -> (u16, String) {
    (
        200,
        json!({"event_id":event["event_id"],"received_at":123}).to_string(),
    )
}
