//! Classification must preserve full validation for uncertain or mixed work.

use std::path::Path;
use xtask::ci::{Mode, mode};

const BASE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HEAD: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn classify(raw: &str) -> Mode {
    let mut runner = |_: &str, args: &[&str], _: &Path| {
        Ok(match args[0] {
            "rev-parse" => format!("{HEAD}\n"),
            "show" => format!("{BASE} {}\n", "c".repeat(40)),
            "diff" => raw.into(),
            _ => panic!("unexpected git probe {args:?}"),
        })
    };
    mode(Path::new("."), &mut runner, "pull_request", BASE, HEAD)
}

fn record(old: &str, new: &str, status: &str, path: &str) -> String {
    format!(":{old} {new} {BASE} {HEAD} {status}\0{path}\0")
}

#[test]
fn regular_documentation_changes_select_lightweight_checks() {
    for path in [
        "README.md",
        "CHANGELOG.md",
        "docs/guide.md",
        "docs/ai-agents/quality.md",
    ] {
        assert_eq!(
            classify(&record("100644", "100644", "M", path)),
            Mode::Docs
        );
        assert_eq!(
            classify(&record("000000", "100644", "A", path)),
            Mode::Docs
        );
        assert_eq!(
            classify(&record("100644", "000000", "D", path)),
            Mode::Docs
        );
    }
}

#[test]
fn code_workflows_mixed_changes_and_unsafe_names_require_every_gate() {
    for path in [
        "xtask/src/lib.rs",
        ".github/workflows/ci.yml",
        "Cargo.lock",
        "deploy/vm/install.sh",
        "AGENTS.md",
        "docs/snippet.sh",
        "docs/../Cargo.md",
        "docs/a\nb.md",
        "docs//a.md",
    ] {
        assert_eq!(
            classify(&record("100644", "100644", "M", path)),
            Mode::Full
        );
        assert_eq!(
            classify(
                &(record("100644", "100644", "M", "README.md")
                    + &record("100644", "100644", "M", path))
            ),
            Mode::Full
        );
    }
}

#[test]
fn uncertain_records_and_special_file_modes_never_skip_checks() {
    for raw in [
        String::new(),
        "malformed\0README.md\0".into(),
        record("100644", "120000", "T", "README.md"),
        record("100644", "100755", "M", "README.md"),
        record("100644", "100644", "R100", "README.md"),
        record("100644", "100644", "M", "README.md")
            .trim_end_matches('\0')
            .into(),
    ] {
        assert_eq!(classify(&raw), Mode::Full);
    }
}

#[test]
fn non_pr_events_and_unavailable_git_always_require_full_validation() {
    for event in [
        "push",
        "workflow_call",
        "workflow_dispatch",
        "pull_request_target",
        "",
    ] {
        let mut runner = |_: &str, _: &[&str], _: &Path| {
            panic!("must not inspect a non-PR diff")
        };
        assert_eq!(
            mode(Path::new("."), &mut runner, event, BASE, HEAD),
            Mode::Full
        );
    }
    let mut runner = |_: &str, _: &[&str], _: &Path| Err("unavailable".into());
    assert_eq!(
        mode(Path::new("."), &mut runner, "pull_request", BASE, HEAD),
        Mode::Full
    );
}

#[test]
fn identity_and_probe_failures_default_to_full() {
    for fail in ["rev-parse", "show", "diff"] {
        let mut runner = |_: &str, args: &[&str], _: &Path| {
            if args[0] == fail {
                return Err("probe failed".into());
            }
            Ok(match args[0] {
                "rev-parse" => HEAD.into(),
                "show" => format!("{BASE} {}", "c".repeat(40)),
                _ => unreachable!(),
            })
        };
        assert_eq!(
            mode(Path::new("."), &mut runner, "pull_request", BASE, HEAD),
            Mode::Full
        );
    }
    for parents in [
        BASE.to_owned(),
        format!("{HEAD} {BASE}"),
        format!("{BASE} invalid"),
        format!("{BASE} {HEAD} {HEAD}"),
    ] {
        let mut runner = |_: &str, args: &[&str], _: &Path| {
            Ok(if args[0] == "rev-parse" {
                HEAD.into()
            } else {
                parents.clone()
            })
        };
        assert_eq!(
            mode(Path::new("."), &mut runner, "pull_request", BASE, HEAD),
            Mode::Full
        );
    }
    let mut wrong_checkout = |_: &str, _: &[&str], _: &Path| Ok(BASE.into());
    assert_eq!(
        mode(
            Path::new("."),
            &mut wrong_checkout,
            "pull_request",
            BASE,
            HEAD
        ),
        Mode::Full
    );
    for invalid in [
        "",
        "main",
        "abc",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    ] {
        let mut runner = |_: &str, _: &[&str], _: &Path| {
            panic!("invalid identity must not reach git")
        };
        assert_eq!(
            mode(Path::new("."), &mut runner, "pull_request", invalid, HEAD),
            Mode::Full
        );
        assert_eq!(
            mode(Path::new("."), &mut runner, "pull_request", BASE, invalid),
            Mode::Full
        );
    }
}

#[test]
fn real_merge_diffs_classify_changes_and_renames_without_hiding_code() {
    use std::{
        fs,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };
    struct Repo(std::path::PathBuf);
    impl Drop for Repo {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn git(root: &Path, args: &[&str]) -> Result<String, String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into());
        }
        String::from_utf8(output.stdout).map_err(|e| e.to_string())
    }
    for (old, new, expected) in [
        ("docs/guide.md", "docs/guide.md", Mode::Docs),
        ("docs/guide.md", "docs/renamed.md", Mode::Docs),
        ("src/application.rs", "docs/old-code.md", Mode::Full),
        ("docs/guide.md", "src/new-code.rs", Mode::Full),
    ] {
        let repo = Repo(std::env::temp_dir().join(format!(
                "silkboard-ci-mode-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )));
        fs::create_dir_all(repo.0.join("docs")).unwrap();
        fs::create_dir_all(repo.0.join("src")).unwrap();
        git(&repo.0, &["init", "--quiet", "--initial-branch=main"]).unwrap();
        for (key, value) in [
            ("user.name", "Fixture"),
            ("user.email", "fixture@example.invalid"),
            ("commit.gpgsign", "false"),
            ("core.hooksPath", "/dev/null"),
        ] {
            git(&repo.0, &["config", key, value]).unwrap();
        }
        fs::write(repo.0.join("docs/guide.md"), "# Guide\n").unwrap();
        fs::write(repo.0.join("src/application.rs"), "fn main() {}\n").unwrap();
        git(&repo.0, &["add", "."]).unwrap();
        git(&repo.0, &["commit", "--quiet", "-m", "base"]).unwrap();
        let base = git(&repo.0, &["rev-parse", "HEAD"]).unwrap();
        git(&repo.0, &["checkout", "--quiet", "-b", "change"]).unwrap();
        if old == new {
            fs::write(repo.0.join(old), "# Updated guide\n").unwrap();
        } else {
            fs::rename(repo.0.join(old), repo.0.join(new)).unwrap();
        }
        git(&repo.0, &["add", "--all"]).unwrap();
        git(&repo.0, &["commit", "--quiet", "-m", "change"]).unwrap();
        git(&repo.0, &["checkout", "--quiet", "main"]).unwrap();
        git(
            &repo.0,
            &["merge", "--quiet", "--no-ff", "-m", "merge", "change"],
        )
        .unwrap();
        let head = git(&repo.0, &["rev-parse", "HEAD"]).unwrap();
        let mut runner = |_: &str, args: &[&str], root: &Path| git(root, args);
        assert_eq!(
            mode(
                &repo.0,
                &mut runner,
                "pull_request",
                base.trim(),
                head.trim()
            ),
            expected
        );
    }
}
