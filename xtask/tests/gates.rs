//! Observable quality-gate composition and failure contracts.

use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering::SeqCst},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use xtask::dispatch;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "silkboard-gates-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for directory in [
            "frontend",
            "docs",
            ".agents/skills",
            ".claude/commands",
            ".claude/skills",
        ] {
            fs::create_dir_all(root.join(directory)).unwrap();
        }
        for file in ["README.md", "AGENTS.md", "CLAUDE.md"] {
            fs::write(root.join(file), "# Fixture\n").unwrap();
        }
        fs::write(root.join("frontend/package-lock.json"), json!({"lockfileVersion":3,"packages":{"":{},"node_modules/tool":{"version":"1.0.0","resolved":"https://registry.npmjs.org/tool/-/tool-1.0.0.tgz","integrity":"sha512-fixture","license":"MIT"}}}).to_string()).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|s| (*s).into()).collect()
}

fn successful_output(program: &str, args: &[&str]) -> String {
    if program == "cargo" && args[0] == "metadata" {
        json!({"workspace_members":["local"],"packages":[{"id":"local","name":"xtask","version":"0.1.0","source":null},{"id":"external","name":"axum","version":"0.8.9","source":"registry+https://github.com/rust-lang/crates.io-index"}]}).to_string()
    } else if program == "curl" && args.last().unwrap().contains("crates.io") {
        json!({"version":{"created_at":"2020-01-01T00:00:00Z"}}).to_string()
    } else if program == "curl" {
        json!({"time":{"1.0.0":"2020-01-01T00:00:00Z"},"versions":{"1.0.0":{"name":"tool","version":"1.0.0","license":"MIT","dist":{"tarball":"https://registry.npmjs.org/tool/-/tool-1.0.0.tgz","integrity":"sha512-fixture"}}}}).to_string()
    } else {
        String::new()
    }
}

#[test]
fn validation_requires_every_stage_and_stops_at_each_failure() {
    let fixture = Fixture::new();
    let calls = Mutex::new(Vec::new());
    dispatch(
        &args(&["validate", "--check"]),
        &fixture.0,
        &mut |program, argv, cwd| {
            calls.lock().unwrap().push((
                program.to_owned(),
                argv.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
                cwd.to_owned(),
            ));
            Ok(successful_output(program, argv))
        },
    )
    .unwrap();
    let calls = calls.into_inner().unwrap();
    assert!(
        calls
            .iter()
            .any(|(p, a, _)| p == "npm" && a == &["run", "check:all"])
    );
    assert!(
        calls
            .iter()
            .any(|(p, a, _)| p == "npm" && a == &["run", "coverage"])
    );
    assert!(
        calls
            .iter()
            .any(|(p, a, _)| p == "npm" && a == &["run", "test:browser"])
    );
    assert!(
        calls
            .iter()
            .filter(|(p, a, _)| p == "cargo" && a[0] == "doc")
            .count()
            == 2
    );
    assert!(
        calls
            .iter()
            .any(|(p, a, _)| p == "cargo"
                && a == &["audit", "--deny", "warnings"])
    );
    assert!(
        calls
            .iter()
            .any(|(p, a, _)| p == "npm" && a == &["run", "test:handoff"])
    );
    for failure in 0..calls.len() {
        let count = AtomicUsize::new(0);
        let result = dispatch(
            &args(&["validate", "--check"]),
            &fixture.0,
            &mut |program, argv, _| {
                let current = count.fetch_add(1, SeqCst);
                if current == failure {
                    Err("required gate failed or unavailable".into())
                } else {
                    Ok(successful_output(program, argv))
                }
            },
        );
        assert!(
            result
                .unwrap_err()
                .contains("required gate failed or unavailable")
        );
        let expected = if calls[failure].0 == "curl" {
            calls
                .iter()
                .rposition(|(program, _, _)| program == "curl")
                .unwrap()
                + 1
        } else {
            failure + 1
        };
        assert_eq!(count.load(SeqCst), expected);
    }
}

#[test]
fn all_standalone_tasks_propagate_failure_and_accept_only_supported_arguments()
{
    for task in [
        "audit",
        "deny",
        "coverage",
        "dupes",
        "frontend",
        "frontend-check",
        "frontend-fmt",
        "frontend-test",
        "frontend-coverage",
        "frontend-dupes",
        "browser-test",
        "handoff-test",
        "check",
        "fmt",
        "clippy",
        "doc",
        "test",
    ] {
        assert_eq!(
            dispatch(&args(&[task]), Path::new("."), &mut |_, _, _| Err(
                "unavailable".into()
            )),
            Err("unavailable".into()),
            "{task}"
        );
        assert!(
            dispatch(&args(&[task]), Path::new("."), &mut |_, _, _| Ok(
                "tool output".into()
            ))
            .is_ok(),
            "{task}"
        );
    }
    for words in [
        vec![],
        vec!["unsupported"],
        vec!["validate", "--skip-audit"],
        vec!["test", "--ignored"],
        vec!["clippy", "--target"],
        vec!["clippy", "--target", "--bad"],
        vec!["fmt", "extra"],
        vec!["release-check"],
    ] {
        assert!(
            dispatch(&args(&words), Path::new("."), &mut |_, _, _| panic!(
                "must not execute"
            ))
            .is_err()
        );
    }
    assert!(
        dispatch(&args(&["--help"]), Path::new("."), &mut |_, _, _| panic!(
            "must not execute"
        ))
        .is_ok()
    );
}

#[test]
fn forwards_test_filter_cross_target_and_read_only_format_arguments() {
    for (task, expected) in [
        (
            vec!["test", "routing"],
            vec!["test", "--locked", "--workspace", "routing"],
        ),
        (
            vec!["clippy", "--target", "aarch64-unknown-linux-gnu"],
            vec![
                "clippy",
                "--locked",
                "--workspace",
                "--all-targets",
                "--target",
                "aarch64-unknown-linux-gnu",
                "--",
                "-D",
                "warnings",
            ],
        ),
        (
            vec!["fmt", "--check"],
            vec!["fmt", "--all", "--", "--check"],
        ),
        (vec!["frontend-fmt", "--check"], vec!["run", "format:check"]),
    ] {
        dispatch(&args(&task), Path::new("root"), &mut |_, argv, _| {
            assert_eq!(argv, expected);
            Ok(String::new())
        })
        .unwrap();
    }
}

#[test]
fn canon_checks_both_wrappers_and_reports_broken_links_and_missing_inputs() {
    let fixture = Fixture::new();
    let run = || {
        dispatch(&args(&["canon-check"]), &fixture.0, &mut |_, _, _| {
            panic!("no subprocess")
        })
    };
    fs::write(fixture.0.join("docs/target.md"), "# Target\n").unwrap();
    for file in [
        ".agents/skills/test.md",
        ".claude/commands/test.md",
        ".claude/skills/test.md",
    ] {
        fs::write(fixture.0.join(file),"[target](../../docs/target.md#target)\n[remote](https://example.com)\n[email](mailto:test@example.com)\n[anchor](#target)").unwrap();
    }
    assert!(run().is_ok());
    fs::create_dir_all(fixture.0.join("docs/ai-agents")).unwrap();
    let tooling = fixture.0.join("docs/ai-agents/tooling.md");
    fs::write(
        &tooling,
        "`cargo xtask test`\n## Pending interfaces\n`cargo xtask absent`\n",
    )
    .unwrap();
    assert!(run().is_ok());
    fs::write(&tooling, "`cargo xtask absent`\n").unwrap();
    assert!(run().unwrap_err().contains("unavailable command: absent"));
    fs::write(&tooling, "`cargo xtask test`\n").unwrap();
    fs::write(fixture.0.join("docs/broken.md"), "[broken](absent.md)").unwrap();
    assert!(run().unwrap_err().contains("missing relative link"));
    fs::write(fixture.0.join("docs/broken.md"), "[broken](unterminated")
        .unwrap();
    assert!(run().unwrap_err().contains("unclosed"));
    fs::remove_file(fixture.0.join("docs/broken.md")).unwrap();
    fs::remove_file(fixture.0.join("README.md")).unwrap();
    assert!(run().is_err());
    fs::remove_dir_all(fixture.0.join("docs")).unwrap();
    assert!(run().is_err());
}
