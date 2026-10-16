//! Behavioral tests for the Stop hook, using Cargo or standalone rustc.

#![forbid(unsafe_code)]
#![deny(warnings)]

#[cfg(not(unix))]
compile_error!("Stop hook tests require a Unix host with Bash, Git, and jq");

use std::env;
use std::fs;
use std::io::{ErrorKind, Write};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const HOOK: &str = include_str!("../stop-check.sh");
const CONFIG: &str = include_str!("../../hooks.json");
const STAGES: [&str; 4] = [
    "fmt --all -- --check",
    "xtask clippy",
    "xtask doc",
    "xtask test",
];
const MOCK_CARGO: &str = r#"#!/bin/sh
printf '%s|%s\n' "$PWD" "$*" >> "$HOOK_TEST_LOG"
if [ "$*" = "$HOOK_TEST_FAIL" ]; then
  echo 'mock gate unavailable or failed' >&2
  exit 1
fi
"#;

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must follow the Unix epoch")
            .as_nanos();
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "silkboard hook tests {} {timestamp} {sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create a unique test directory");
        Self(path)
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("Could not clean up {}: {error}", self.0.display());
        }
    }
}

fn tool(name: &str) -> PathBuf {
    env::split_paths(&env::var_os("PATH").expect("PATH must be set"))
        .map(|directory| directory.join(name))
        .find(|path| {
            path.is_file()
                && path.metadata().is_ok_and(|metadata| {
                    metadata.permissions().mode() & 0o111 != 0
                })
        })
        .unwrap_or_else(|| panic!("required test tool is unavailable: {name}"))
}

fn output_with_input(mut command: Command, input: &str) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start test subprocess");
    let written = child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(input.as_bytes());
    if let Err(error) = written {
        // A prerequisite failure may exit without consuming the payload.
        // Still collect and check the child's status and diagnostics below.
        assert_eq!(error.kind(), ErrorKind::BrokenPipe, "{error}");
    }
    child.wait_with_output().expect("wait for test subprocess")
}

#[test]
fn subprocess_can_close_stdin_before_reading_its_payload() {
    // Exceed the pipe capacity so a writer cannot finish before stdin closes.
    let payload = "x".repeat(1024 * 1024);
    for code in [0, 7] {
        let mut command = Command::new(tool("bash"));
        command.env_clear().args([
            "-c",
            "exec 0<&-; printf warning; printf diagnostic >&2; exit \"$1\"",
            "early-exit-test",
            &code.to_string(),
        ]);
        let output = output_with_input(command, &payload);
        assert_eq!(output.status.code(), Some(code));
        assert_eq!(output.stdout, b"warning");
        assert_eq!(output.stderr, b"diagnostic");
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "subprocess failed: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

struct Fixture {
    directory: TempDirectory,
    repo: PathBuf,
    bin: PathBuf,
    log: PathBuf,
    fail_stage: String,
}

impl Fixture {
    fn new() -> Self {
        let directory = TempDirectory::new();
        let repo = directory.0.join("checkout with spaces");
        let bin = directory.0.join("bin");
        let log = directory.0.join("calls");
        fs::create_dir(&repo).expect("create fixture worktree");
        fs::create_dir(&bin).expect("create isolated tool directory");
        for name in ["git", "jq", "cat", "bash"] {
            symlink(tool(name), bin.join(name)).expect("link fixture tool");
        }
        let cargo = bin.join("cargo");
        fs::write(&cargo, MOCK_CARGO).expect("write mock Cargo");
        fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755))
            .expect("make mock Cargo executable");
        fs::write(directory.0.join("stop-check.sh"), HOOK)
            .expect("write the embedded hook");
        let fixture = Self {
            directory,
            repo,
            bin,
            log,
            fail_stage: String::new(),
        };
        fixture.git(&["init", "-q"]);
        fixture.git(&["config", "user.name", "Hook test"]);
        fixture.git(&["config", "user.email", "hook@example.invalid"]);
        fs::write(fixture.repo.join("main.rs"), "fn main() {}\n")
            .expect("write initial Rust source");
        fs::write(fixture.repo.join("Cargo.toml"), "# mocked workspace\n")
            .expect("write fixture manifest");
        fixture.git(&["add", "."]);
        fixture.git(&["commit", "-qm", "initial"]);
        fixture
    }

    fn command(&self, executable: &Path) -> Command {
        let mut command = Command::new(executable);
        command
            .current_dir(&self.directory.0)
            .env_clear()
            .env("PATH", &self.bin)
            .env("HOME", &self.directory.0)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .env("HOOK_TEST_LOG", &self.log)
            .env("HOOK_TEST_FAIL", &self.fail_stage);
        command
    }

    fn git(&self, args: &[&str]) {
        let output = self
            .command(&tool("git"))
            .current_dir(&self.repo)
            .args(args)
            .output()
            .expect("run Git in the fixture");
        assert_success(&output);
    }

    fn payload(&self, cwd: &Path, active: &str) -> String {
        let mut command = self.command(&tool("jq"));
        command
            .args(["-cn", "--arg", "cwd"])
            .arg(cwd)
            .args(["--argjson", "active", active])
            .arg(
                "if $active == null then {cwd:$cwd} \
                 else {cwd:$cwd,stop_hook_active:$active} end",
            );
        let output = command.output().expect("construct hook JSON payload");
        assert_success(&output);
        String::from_utf8(output.stdout).expect("UTF-8 payload")
    }

    fn run(&self, payload: &str) -> Output {
        let mut command = self.command(&tool("bash"));
        command.arg(self.directory.0.join("stop-check.sh"));
        output_with_input(command, payload)
    }

    fn run_default(&self) -> Output {
        self.run(&self.payload(&self.repo, "null"))
    }

    fn calls(&self) -> Vec<String> {
        match fs::read_to_string(&self.log) {
            Ok(contents) => contents.lines().map(str::to_owned).collect(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Vec::new()
            }
            Err(error) => panic!("read mock Cargo log: {error}"),
        }
    }

    fn change_rust(&self) {
        fs::write(self.repo.join("main.rs"), "fn main() { panic!(); }\n")
            .expect("modify fixture source");
    }

    fn assert_pass(&self, output: &Output, stages: &[&str]) {
        assert_success(output);
        assert!(output.stdout.is_empty(), "unexpected stdout: {output:?}");
        assert!(output.stderr.is_empty(), "unexpected stderr: {output:?}");
        let expected: Vec<_> = stages
            .iter()
            .map(|stage| format!("{}|{stage}", self.repo.display()))
            .collect();
        assert_eq!(self.calls(), expected);
    }

    fn assert_unavailable(&self, output: &Output, reason: &str) {
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("unavailable"), "{stderr}");
        assert!(stderr.contains(reason), "{stderr}");
        assert!(self.calls().is_empty());
    }

    fn assert_parser_warning(&self, output: &Output) {
        assert_success(output);
        assert!(output.stderr.is_empty());
        let mut command = self.command(&tool("jq"));
        command.args([
            "-e",
            ".systemMessage | contains(\"validation unavailable\") \
             and contains(\"jq is required\") and contains(\"Checks did not run\")",
        ]);
        let parsed = output_with_input(
            command,
            std::str::from_utf8(&output.stdout).expect("UTF-8 warning JSON"),
        );
        assert_success(&parsed);
        assert!(self.calls().is_empty());
    }
}

#[test]
fn no_rust_changes() {
    let fixture = Fixture::new();
    fs::write(fixture.repo.join("notes.md"), "documentation change\n")
        .expect("write fixture documentation");
    fixture.assert_pass(&fixture.run_default(), &[]);
}

#[test]
fn unstaged_rust() {
    let fixture = Fixture::new();
    fixture.change_rust();
    fixture.assert_pass(&fixture.run_default(), &STAGES);
}

#[test]
fn staged_rust() {
    let fixture = Fixture::new();
    fixture.change_rust();
    fixture.git(&["add", "main.rs"]);
    fixture.assert_pass(&fixture.run_default(), &STAGES);
}

#[test]
fn untracked_rust() {
    let fixture = Fixture::new();
    fs::write(fixture.repo.join("new file.rs"), "// new\n")
        .expect("write untracked source");
    fixture.assert_pass(&fixture.run_default(), &STAGES);
}

#[test]
fn unstaged_rust_deletion() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.repo.join("main.rs")).expect("delete source");
    fixture.assert_pass(&fixture.run_default(), &STAGES);
}

#[test]
fn staged_rust_deletion() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.repo.join("main.rs")).expect("delete source");
    fixture.git(&["add", "-u"]);
    fixture.assert_pass(&fixture.run_default(), &STAGES);
}

#[test]
fn payload_subdirectory_and_false_flag() {
    let fixture = Fixture::new();
    fixture.change_rust();
    let subdir = fixture.repo.join("subdir");
    fs::create_dir(&subdir).expect("create payload subdirectory");
    let output = fixture.run(&fixture.payload(&subdir, "false"));
    fixture.assert_pass(&output, &STAGES);
}

#[test]
fn true_flag_prevents_recursion() {
    let fixture = Fixture::new();
    fixture.change_rust();
    let output = fixture.run(&fixture.payload(&fixture.repo, "true"));
    fixture.assert_pass(&output, &[]);
}

#[test]
fn string_true_does_not_skip() {
    let fixture = Fixture::new();
    fixture.change_rust();
    let output = fixture.run(&fixture.payload(&fixture.repo, r#""true""#));
    fixture.assert_pass(&output, &STAGES);
}

#[test]
fn each_failed_stage_stops_the_pipeline() {
    let mut fixture = Fixture::new();
    fixture.change_rust();
    for (index, stage) in STAGES.iter().enumerate() {
        if fixture.log.exists() {
            fs::remove_file(&fixture.log).expect("clear mock Cargo calls");
        }
        fixture.fail_stage = (*stage).to_owned();
        let output = fixture.run_default();
        assert_eq!(output.status.code(), Some(2), "stage: {stage}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("failed or unavailable"), "{stderr}");
        assert!(
            stderr.contains("mock gate unavailable or failed"),
            "{stderr}"
        );
        let expected: Vec<_> = STAGES[..=index]
            .iter()
            .map(|stage| format!("{}|{stage}", fixture.repo.display()))
            .collect();
        assert_eq!(fixture.calls(), expected);
    }
}

#[test]
fn missing_workspace() {
    let fixture = Fixture::new();
    fixture.change_rust();
    fs::remove_file(fixture.repo.join("Cargo.toml")).expect("remove manifest");
    fixture.assert_unavailable(&fixture.run_default(), "Cargo.toml is missing");
}

#[test]
fn missing_cargo() {
    let fixture = Fixture::new();
    fixture.change_rust();
    fs::remove_file(fixture.bin.join("cargo")).expect("remove mock Cargo");
    fixture.assert_unavailable(&fixture.run_default(), "Cargo is required");
}

#[test]
fn missing_jq() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.bin.join("jq")).expect("remove fixture jq");
    fixture.assert_parser_warning(&fixture.run_default());
}

#[test]
fn missing_jq_does_not_block_continuation() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.bin.join("jq")).expect("remove fixture jq");
    let output = fixture.run(&fixture.payload(&fixture.repo, "true"));
    fixture.assert_parser_warning(&output);
}

#[test]
fn invalid_payloads() {
    let fixture = Fixture::new();
    for payload in ["invalid", "[]", "{}", r#"{"cwd":1}"#] {
        fixture.assert_unavailable(&fixture.run(payload), "unavailable");
    }
}

#[test]
fn non_repository_payload() {
    let fixture = Fixture::new();
    let output = fixture.run(&fixture.payload(&fixture.directory.0, "null"));
    fixture.assert_unavailable(&output, "Git worktree");
}

#[test]
fn registered_command_from_subdirectory() {
    let fixture = Fixture::new();
    let mut query = fixture.command(&tool("jq"));
    query.args([
        "-er",
        ".hooks.Stop[].hooks[] | \
         select(.command | contains(\"/.codex/hooks/stop-check.sh\")) | .command",
    ]);
    let configured = output_with_input(query, CONFIG);
    assert_success(&configured);
    let command = String::from_utf8(configured.stdout).expect("UTF-8 command");
    let destination = fixture.repo.join(".codex/hooks");
    fs::create_dir_all(&destination).expect("create fixture hook directory");
    fs::write(destination.join("stop-check.sh"), HOOK).expect("write hook");
    let subdir = fixture.repo.join("subdir");
    fs::create_dir(&subdir).expect("create invocation subdirectory");
    fixture.change_rust();
    let mut shell = fixture.command(&tool("bash"));
    shell.current_dir(subdir).arg("-c").arg(command.trim());
    let output =
        output_with_input(shell, &fixture.payload(&fixture.repo, "null"));
    fixture.assert_pass(&output, &STAGES);
}
