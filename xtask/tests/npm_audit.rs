//! The approved advisory exception cannot hide unrelated audit failures.
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn finding() -> Value {
    serde_json::from_str(include_str!("fixtures/npm-audit-deepmerge.json"))
        .unwrap()
}

fn lock() -> Value {
    json!({"lockfileVersion":3,"packages":{
        "":{},
        "node_modules/deepmerge":{"version":"4.3.1"},
        "node_modules/@sveltejs/vite-plugin-svelte":{"version":"7.2.0","dependencies":{"deepmerge":"^4.3.1"}}
    }})
}

struct Fixture(PathBuf);
impl Fixture {
    fn new(lock: &Value) -> Self {
        let path = std::env::temp_dir().join(format!(
            "silkboard-audit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("frontend")).unwrap();
        fs::write(path.join("frontend/package-lock.json"), lock.to_string())
            .unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn audit(
    report: &Value,
    code: i32,
    lock: &Value,
    stderr: &str,
) -> Result<(), String> {
    let f = Fixture::new(lock);
    let calls = AtomicUsize::new(0);
    let result = xtask::dispatch(
        &["audit".into()],
        &f.0,
        &mut |program, args, cwd| {
            calls.fetch_add(1, Ordering::Relaxed);
            match program {
                "cargo" => {
                    assert_eq!(args, ["audit", "--deny", "warnings"]);
                    Ok(String::new())
                }
                "npm" => {
                    assert_eq!(args, ["audit", "--json", "--audit-level=low"]);
                    assert_eq!(cwd, f.0.join("frontend"));
                    Ok(json!({"exit_code":code,"stdout":report.to_string(),"stderr":stderr}).to_string())
                }
                _ => panic!("unexpected audit command"),
            }
        },
    );
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    result
}

#[test]
fn accepts_only_the_exact_approved_finding_and_propagation() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_eq!(
        audit(&finding(), 1, &lock(), "").is_ok(),
        now < 1_794_787_200
    );
    let mut clean = finding();
    clean["vulnerabilities"] = json!({});
    clean["metadata"]["vulnerabilities"] =
        json!({"info":0,"low":0,"moderate":0,"high":0,"critical":0,"total":0});
    assert!(audit(&clean, 0, &lock(), "").is_ok());
    assert!(audit(&clean, 1, &lock(), "").is_err());
    assert!(audit(&finding(), 0, &lock(), "").is_err());
}

#[cfg(unix)]
#[test]
fn executable_captures_npm_findings_but_rejects_tool_and_registry_failures() {
    use std::{os::unix::fs::PermissionsExt, process::Command};
    let f = Fixture::new(&lock());
    fs::write(f.0.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    fs::create_dir(f.0.join("xtask")).unwrap();
    fs::write(f.0.join("xtask/Cargo.toml"), "").unwrap();
    let bin = f.0.join("bin");
    fs::create_dir(&bin).unwrap();
    for (name, script) in [
        ("cargo", "#!/bin/sh\nexit \"$FIXTURE_CARGO_STATUS\"\n"),
        (
            "npm",
            "#!/bin/sh\n: > \"$FIXTURE_CALLED\"\n/bin/cat \"$FIXTURE_AUDIT_REPORT\"\nexit \"$FIXTURE_AUDIT_STATUS\"\n",
        ),
    ] {
        let file = bin.join(name);
        fs::write(&file, script).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let input = f.0.join("report.json");
    let called = f.0.join("npm-called");
    let active = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        < 1_794_787_200;
    for (report, status, cargo_status, succeeds) in [
        (finding().to_string(), 1, 0, active),
        (finding().to_string(), 2, 0, false),
        ("not JSON".into(), 1, 0, false),
        (
            json!({"error":{"code":"EAI_AGAIN"}}).to_string(),
            1,
            0,
            false,
        ),
        (finding().to_string(), 1, 2, false),
    ] {
        let _ = fs::remove_file(&called);
        fs::write(&input, report).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .arg("audit")
            .current_dir(&f.0)
            .env("PATH", &bin)
            .env("CARGO", bin.join("cargo"))
            .env("FIXTURE_CALLED", &called)
            .env("FIXTURE_AUDIT_REPORT", &input)
            .env("FIXTURE_AUDIT_STATUS", status.to_string())
            .env("FIXTURE_CARGO_STATUS", cargo_status.to_string())
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            succeeds,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(called.exists(), cargo_status == 0);
        if succeeds {
            assert!(
                String::from_utf8(output.stdout)
                    .unwrap()
                    .contains("not a clean scan")
            );
        }
    }
}

#[test]
fn changed_advisories_severity_schema_status_and_evidence_fail_closed() {
    for (pointer, value) in [
        (
            "/vulnerabilities/deepmerge/via/0/url",
            json!("https://github.com/advisories/GHSA-other"),
        ),
        ("/vulnerabilities/deepmerge/severity", json!("moderate")),
        (
            "/vulnerabilities/deepmerge/via/0/severity",
            json!("moderate"),
        ),
        ("/vulnerabilities/deepmerge/fixAvailable", json!(true)),
        (
            "/vulnerabilities/deepmerge/nodes",
            json!(["node_modules/other/node_modules/deepmerge"]),
        ),
        (
            "/vulnerabilities/@sveltejs~1vite-plugin-svelte/via",
            json!(["deepmerge", "other"]),
        ),
        ("/metadata/vulnerabilities/total", json!(1)),
        ("/auditReportVersion", json!(3)),
    ] {
        let mut report = finding();
        *report.pointer_mut(pointer).unwrap() = value;
        assert!(audit(&report, 1, &lock(), "").is_err(), "{pointer}");
    }
    let mut extra = finding();
    extra["vulnerabilities"]["other"] = json!({"severity":"low"});
    assert!(audit(&extra, 1, &lock(), "").is_err());
    for report in [
        json!({}),
        json!({"error":{"code":"EAI_AGAIN"}}),
        Value::Null,
    ] {
        assert!(audit(&report, 1, &lock(), "").is_err());
    }
    assert!(audit(&finding(), 2, &lock(), "").is_err());
    assert!(audit(&finding(), 1, &lock(), "registry unavailable").is_err());
}

#[test]
fn changed_versions_and_additional_consumers_are_not_covered() {
    for (pointer, value) in [
        ("/packages/node_modules~1deepmerge/version", json!("4.3.2")),
        (
            "/packages/node_modules~1@sveltejs~1vite-plugin-svelte/version",
            json!("7.3.1"),
        ),
        (
            "/packages/node_modules~1@sveltejs~1vite-plugin-svelte/dependencies/deepmerge",
            json!("*"),
        ),
        ("/lockfileVersion", json!(2)),
    ] {
        let mut changed = lock();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert!(audit(&finding(), 1, &changed, "").is_err(), "{pointer}");
    }
    for changed in [
        json!({"version":"1.0.0","dependencies":{"deepmerge":"^4.3.1"}}),
        json!({"version":"1.0.0","devDependencies":{"deepmerge":"^4.3.1"}}),
        json!({"version":"1.0.0","optionalDependencies":{"deepmerge":"^4.3.1"}}),
        json!({"version":"1.0.0","peerDependencies":{"deepmerge":"^4.3.1"}}),
    ] {
        let mut changed_lock = lock();
        changed_lock["packages"]["node_modules/another-consumer"] = changed;
        assert!(audit(&finding(), 1, &changed_lock, "").is_err());
    }
    let mut nested = lock();
    nested["packages"]["node_modules/other/node_modules/deepmerge"] =
        json!({"version":"4.3.1"});
    assert!(audit(&finding(), 1, &nested, "").is_err());
    let mut malformed = lock();
    malformed["packages"]["node_modules/another"] = Value::Null;
    assert!(audit(&finding(), 1, &malformed, "").is_err());
}
