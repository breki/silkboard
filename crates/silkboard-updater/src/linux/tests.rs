//! Real filesystem transactions and simulated external process contracts.

use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "silkboard-update-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("releases")).unwrap();
        Self(root)
    }
    fn backend<R>(&self, runner: R) -> Linux<R> {
        Linux {
            runner,
            state: self.0.join("state.json"),
            releases: self.0.join("releases"),
            current: self.0.join("current"),
        }
    }
    fn stage(&self, tag: &str) -> Release {
        let release = Release::new(tag, &"a".repeat(40)).unwrap();
        let path = self.0.join("releases").join(format!(
            "{}-{}",
            release.tag(),
            release.commit()
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("identity"),
            format!("{}\n{}\n", release.tag(), release.commit()),
        )
        .unwrap();
        release
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn run_record() -> Value {
    json!({"id":1,"workflow_id":387_356_305,"path":".github/workflows/release.yml",
        "event":"push","head_branch":"v0.2.0","head_sha":"a".repeat(40),"status":"completed",
        "conclusion":"success","run_attempt":1,"referenced_workflows":[{"ref":"refs/tags/v0.2.0","sha":"a".repeat(40),"path":format!("breki/silkboard/.github/workflows/ci.yml@{}","a".repeat(40))}],"repository":{"id":1_417_054_470,"full_name":"breki/silkboard"},
        "head_repository":{"id":1_417_054_470,"full_name":"breki/silkboard"}})
}

fn jobs() -> Value {
    json!({"total_count":3,"jobs":(["Signed stable tag and exact revision",
        "gates / Full validation (Linux)","Build the checked release revision"]
        .map(|name| json!({"name":name,"status":"completed","conclusion":"success"})))})
}

fn tag_object() -> String {
    format!(
        "object {}\ntype commit\ntag v0.2.0\ntagger Test\n\nrelease\n-----BEGIN SSH SIGNATURE-----\nsignature\n-----END SSH SIGNATURE-----\n",
        "a".repeat(40)
    )
}

#[allow(clippy::unnecessary_wraps)] // Implements the fallible command-runner contract.
fn response(program: &str, args: &[&str]) -> Result<String, String> {
    if program == "timeout" {
        assert!(args.contains(&"silkfetch"));
        assert!(args.contains(&"GIT_TERMINAL_PROMPT=0"));
        if args.contains(&"cat-file") {
            return Ok(if args.contains(&"-t") {
                "commit".into()
            } else {
                tag_object()
            });
        }
        return Ok(if args.contains(&"for-each-ref") {
            "v0.2.0\nmain\n".into()
        } else {
            String::new()
        });
    }
    let path = args.last().copied().unwrap_or("");
    if path.starts_with("actions/") {
        assert!(args.contains(&"silkfetch"));
        let value = if path.contains("/jobs?") {
            jobs()
        } else if path.contains("/runs?") {
            json!({"workflow_runs":[run_record()]})
        } else if path == "actions/workflows/387356305" {
            json!({"id":387_356_305,"path":".github/workflows/release.yml","state":"active"})
        } else {
            run_record()
        };
        return Ok(value.to_string());
    }
    if program == "curl" {
        return Ok(if path.ends_with("/health") { json!({"status":"ok"}) }
            else { json!({"health":"ok","version":"0.2.0","release_tag":"v0.2.0","commit":"a".repeat(40)}) }.to_string());
    }
    Ok(String::new())
}

#[test]
fn root_state_is_atomic_private_validated_and_survives_reopen() {
    let fixture = Fixture::new();
    let mut backend = fixture.backend(response);
    assert_eq!(backend.load().unwrap(), State::default());
    let mut state = State {
        blocked: ["v0.2.0".into()].into(),
        pending: Some(crate::Transaction {
            previous: Release::new("v0.1.1", &"b".repeat(40)).unwrap(),
            candidate: fixture.stage("v0.2.0"),
        }),
    };
    backend.save(&state).unwrap();
    assert_eq!(fixture.backend(response).load().unwrap(), state);
    assert_eq!(
        fs::metadata(&backend.state).unwrap().permissions().mode() & 0o777,
        0o600
    );
    state.blocked.clear();
    backend.save(&state).unwrap();
    assert!(backend.load().is_err());
    state.pending = None;
    state.blocked.insert("../bad".into());
    backend.save(&state).unwrap();
    assert!(backend.load().is_err());
    fs::write(&backend.state, b"malformed").unwrap();
    assert!(backend.load().is_err());
    fs::remove_file(&backend.state).unwrap();
    fs::create_dir(&backend.state).unwrap();
    assert!(backend.load().is_err());
    backend.state = fixture.0.join("missing/parent/state");
    assert!(backend.save(&State::default()).is_err());
}

#[test]
fn actual_link_swap_persists_identity_even_if_service_restart_fails() {
    let fixture = Fixture::new();
    let release = fixture.stage("v0.2.0");
    let mut backend = fixture.backend(response);
    assert!(backend.current().is_err());
    symlink("stale", backend.current.with_extension("new")).unwrap();
    backend.activate(&release).unwrap();
    assert_eq!(backend.current().unwrap(), release);
    backend.healthy(&release).unwrap();
    let mut failing = fixture.backend(|p: &str, _: &[&str]| {
        if p == "systemctl" {
            Err("restart".into())
        } else {
            Ok(String::new())
        }
    });
    assert!(failing.activate(&release).is_err());
    assert_eq!(failing.current().unwrap(), release);
    fs::write(
        backend.directory(&release).join("identity"),
        "v0.2.0\nwrong\n",
    )
    .unwrap();
    assert!(backend.current().is_err());
    assert!(backend.activate(&release).is_err());
    fs::write(backend.directory(&release).join("identity"), "one\n").unwrap();
    assert!(backend.current().is_err());
    fs::write(
        backend.directory(&release).join("identity"),
        format!("v0.2.1\n{}\n", "a".repeat(40)),
    )
    .unwrap();
    assert!(backend.current().is_err());
}

#[test]
fn fetch_and_admission_run_as_downloader_and_fail_closed() {
    let fixture = Fixture::new();
    let mut backend = fixture.backend(response);
    assert_eq!(backend.refresh().unwrap(), ["v0.2.0", "main"]);
    assert_eq!(backend.admit("v0.2.0").unwrap().unwrap().tag(), "v0.2.0");
    assert!(backend.admit("--bad").is_err());
    for failure in [
        "fetch",
        "object",
        "name",
        "signature",
        "commit",
        "workflow",
        "runs",
        "jobs",
        "rerun",
        "json",
        "api",
    ] {
        let mut reads = 0;
        let mut backend = fixture.backend(|p: &str, a: &[&str]| {
            let path = a.last().copied().unwrap_or("");
            if (failure == "fetch" && a.contains(&"fetch"))
                || (failure == "object" && a.contains(&"cat-file"))
                || (failure == "signature" && a.contains(&"verify-tag"))
                || (failure == "api" && path.starts_with("actions/"))
            {
                return Err("edge failed".into());
            }
            if failure == "name" && a.contains(&"cat-file") {
                return Ok(tag_object().replace("tag v0.2.0", "tag alias"));
            }
            if failure == "commit" && a.contains(&"-t") {
                return Ok("tag".into());
            }
            if path.starts_with("actions/") {
                if failure == "json" {
                    return Ok("malformed".into());
                }
                if failure == "workflow"
                    && path == "actions/workflows/387356305"
                {
                    return Ok("{}".into());
                }
                if failure == "runs" && path.contains("/runs?") {
                    return Ok("{\"workflow_runs\":[]}".into());
                }
                if failure == "jobs" && path.contains("/jobs?") {
                    return Ok("{}".into());
                }
                if failure == "rerun" && path == "actions/runs/1" {
                    reads += 1;
                    if reads == 2 {
                        let mut r = run_record();
                        r["run_attempt"] = json!(2);
                        return Ok(r.to_string());
                    }
                }
            }
            response(p, a)
        });
        if failure == "fetch" {
            assert!(backend.refresh().is_err());
        } else {
            assert!(
                !matches!(backend.admit("v0.2.0"), Ok(Some(_))),
                "{failure}"
            );
        }
    }
}

#[test]
fn build_runs_unprivileged_and_only_stages_regular_installed_identity() {
    let fixture = Fixture::new();
    let current = fixture.stage("v0.1.1");
    let mut backend = fixture.backend(response);
    backend.activate(&current).unwrap();
    let candidate = fixture.stage("v0.2.0");
    let mut events = vec![];
    let mut backend = fixture.backend(|p: &str, a: &[&str]| {
        events.push((
            p.to_owned(),
            a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        ));
        if p.ends_with("/stage") {
            fixture.stage("v0.2.0");
        }
        response(p, a)
    });
    backend.build(&candidate).unwrap();
    assert!(backend.build(&current).is_err());
    assert!(events.iter().any(|(p, a)| p == "runuser"
        && a.contains(&"silkbuild".to_owned())
        && a.contains(&format!("{TOOLS}/build"))));
    assert!(
        events
            .iter()
            .any(|(p, a)| p.ends_with("/snapshot-bundle") && a == &["v0.2.0"])
    );
}

#[test]
fn unhealthy_or_wrong_identity_exhausts_bounded_retries() {
    let fixture = Fixture::new();
    let release = fixture.stage("v0.2.0");
    for value in ["not-json", "{}", "{\"health\":\"bad\"}"] {
        let mut calls = 0;
        let mut backend = fixture.backend(|p: &str, _: &[&str]| {
            if p == "curl" {
                calls += 1;
                Ok(value.into())
            } else {
                Ok(String::new())
            }
        });
        assert!(backend.healthy(&release).is_err());
        assert_eq!(calls, 30);
    }
    let mut backend = fixture.backend(|p: &str, a: &[&str]| {
        if p == "curl" && a.last().unwrap().ends_with("/health") {
            Err("secondary failed".into())
        } else {
            response(p, a)
        }
    });
    assert!(backend.healthy(&release).is_err());
}
