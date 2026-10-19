//! Silkboard quality gates and release admission checks.

use std::path::Path;

pub mod ci;
mod ci_cache;
mod dependencies;
mod release;

/// Subprocess adapter shared by real execution and behavioral tests.
/// Registry checks may invoke it concurrently; other gates run sequentially.
pub type Runner<'a> =
    dyn Fn(&str, &[&str], &Path) -> Result<String, String> + Sync + 'a;

/// Supported task syntax. Validation is read-only with or without `--check`.
pub const USAGE: &str = "Usage: cargo xtask <check|fmt [--check]|clippy \
[--target TARGET]|doc|test [FILTER]|validate [--check]|coverage|dupes|deny|\
audit|dep-age-check|frontend|frontend-check|frontend-fmt [--check]|\
frontend-test|frontend-coverage|frontend-dupes|browser-test|handoff-test|\
release-check TAG|canon-check|ci-mode EVENT BASE HEAD|ci-cache-plan|\
ci-cache-prune RUNNER_TEMP EXPECTED_NORMAL_KEY>";

/// Run a supported repository task.
///
/// # Errors
/// Returns an error for unsupported arguments or a failed required gate.
pub fn dispatch(
    args: &[String],
    root: &Path,
    runner: &mut Runner<'_>,
) -> Result<(), String> {
    let args: Vec<_> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["--help" | "-h"] => {
            println!("{USAGE}");
            Ok(())
        }
        ["validate"] | ["validate", "--check"] => validate(root, runner),
        ["dep-age-check"] => dependencies::check(root, runner),
        ["release-check", tag] => release::check(root, runner, tag),
        ["canon-check"] => canon_check(root),
        ["ci-cache-plan"] => ci_cache::print_plan(root, runner),
        ["ci-cache-prune", temp, expected_key] => {
            ci_cache::prune_checked(root, runner, Path::new(temp), expected_key)
        }
        ["ci-mode", event, base, head] => {
            println!("{}", ci::mode(root, runner, event, base, head).as_str());
            Ok(())
        }
        ["fmt"] => cargo(root, runner, &["fmt", "--all"]),
        ["fmt", "--check"] => {
            cargo(root, runner, &["fmt", "--all", "--", "--check"])
        }
        ["check"] => cargo(
            root,
            runner,
            &["check", "--locked", "--workspace", "--all-targets"],
        ),
        ["clippy"] => clippy(root, runner, &[]),
        ["clippy", "--target", target] if !target.starts_with('-') => {
            clippy(root, runner, &["--target", target])
        }
        ["doc"] => {
            cargo(
                root,
                runner,
                &["doc", "--locked", "--workspace", "--no-deps"],
            )?;
            cargo(
                root,
                runner,
                &[
                    "doc",
                    "--locked",
                    "--workspace",
                    "--no-deps",
                    "--document-private-items",
                ],
            )
        }
        ["test"] => cargo(root, runner, &["test", "--locked", "--workspace"]),
        ["test", filter] if !filter.starts_with('-') => {
            cargo(root, runner, &["test", "--locked", "--workspace", filter])
        }
        ["deny"] => cargo(
            root,
            runner,
            &[
                "deny",
                "--offline",
                "--locked",
                "--all-features",
                "check",
                "licenses",
                "bans",
                "sources",
            ],
        ),
        ["audit"] => {
            cargo(root, runner, &["audit", "--deny", "warnings"])?;
            npm(root, runner, &["audit", "--audit-level=low"])
        }
        ["coverage"] => coverage(root, runner),
        ["dupes"] => {
            for directory in [
                "crates/silkboard-web",
                "crates/silkboard-updater",
                "crates/silkboard-publish",
                "xtask",
            ] {
                report(&runner(
                    "code-dupes",
                    &[
                        "-p",
                        directory,
                        "--exclude-tests",
                        "--exclude",
                        "tests/**",
                        "check",
                        "--max-exact-percent",
                        "6",
                    ],
                    root,
                )?);
            }
            Ok(())
        }
        ["frontend"] => npm(root, runner, &["run", "check:all"]),
        ["frontend-check"] => npm(root, runner, &["run", "check"]),
        ["frontend-fmt"] => npm(root, runner, &["run", "format"]),
        ["frontend-fmt", "--check"] => {
            npm(root, runner, &["run", "format:check"])
        }
        ["frontend-test"] => npm(root, runner, &["test"]),
        ["frontend-coverage"] => npm(root, runner, &["run", "coverage"]),
        ["frontend-dupes"] => npm(root, runner, &["run", "dupes"]),
        ["handoff-test"] => npm(root, runner, &["run", "test:handoff"]),
        ["browser-test"] => npm(root, runner, &["run", "test:browser"]),
        _ => Err(USAGE.into()),
    }
}

fn report(output: &str) {
    if !output.trim().is_empty() {
        println!("{output}");
    }
}

fn coverage(root: &Path, runner: &mut Runner<'_>) -> Result<(), String> {
    cargo(root, runner, &["llvm-cov", "clean", "--workspace"])?;
    cargo(
        root,
        runner,
        &[
            "llvm-cov",
            "--locked",
            "--workspace",
            "--ignore-filename-regex",
            r"src[/\\]main\.rs$",
            "--fail-under-lines",
            "90",
            "--fail-under-file-lines",
            "85",
            "--summary-only",
        ],
    )
}

fn cargo(
    root: &Path,
    runner: &mut Runner<'_>,
    args: &[&str],
) -> Result<(), String> {
    report(&runner("cargo", args, root)?);
    Ok(())
}

fn npm(
    root: &Path,
    runner: &mut Runner<'_>,
    args: &[&str],
) -> Result<(), String> {
    report(&runner("npm", args, &root.join("frontend"))?);
    Ok(())
}

fn clippy(
    root: &Path,
    runner: &mut Runner<'_>,
    extra: &[&str],
) -> Result<(), String> {
    let mut args = vec!["clippy", "--locked", "--workspace", "--all-targets"];
    args.extend_from_slice(extra);
    args.extend_from_slice(&["--", "-D", "warnings"]);
    cargo(root, runner, &args)
}

fn validate(root: &Path, runner: &mut Runner<'_>) -> Result<(), String> {
    // Shared dispatch guarantees standalone gates and aggregate validation agree.
    for task in [
        "dep-age-check",
        "canon-check",
        "fmt --check",
        "dupes",
        "clippy",
        "doc",
        "test",
        "deny",
        "frontend",
        "frontend-coverage",
        "frontend-dupes",
        "browser-test",
        "handoff-test",
        "coverage",
        "audit",
    ] {
        eprintln!("Checking {task}...");
        let args: Vec<_> = task.split_whitespace().map(String::from).collect();
        dispatch(&args, root, runner)?;
    }
    println!("All required gates passed.");
    Ok(())
}

fn canon_check(root: &Path) -> Result<(), String> {
    // Check the shared workflow sources and both sets of wrappers; no prose lint.
    fn walk(root: &Path, directory: &Path) -> Result<(), String> {
        for entry in std::fs::read_dir(directory)
            .map_err(|e| format!("{}: {e}", directory.display()))?
        {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                walk(root, &path)?;
            } else if path.extension().is_some_and(|e| e == "md") {
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())?;
                check_links(&path, &text)?;
                if path == root.join("docs/ai-agents/tooling.md") {
                    check_documented_commands(&text)?;
                }
            }
        }
        Ok(())
    }
    for directory in [
        "docs",
        ".agents/skills",
        ".claude/commands",
        ".claude/skills",
    ] {
        walk(root, &root.join(directory))?;
    }
    for file in ["README.md", "AGENTS.md", "CLAUDE.md"] {
        let path = root.join(file);
        check_links(
            &path,
            &std::fs::read_to_string(&path).map_err(|e| e.to_string())?,
        )?;
    }
    Ok(())
}

fn check_links(file: &Path, text: &str) -> Result<(), String> {
    for tail in text.split("](").skip(1) {
        let Some((link, _)) = tail.split_once(')') else {
            return Err(format!("{}: unclosed Markdown link", file.display()));
        };
        if link.starts_with('#')
            || link.contains("://")
            || link.starts_with("mailto:")
        {
            continue;
        }
        let target = link.split('#').next().unwrap_or_default();
        if target.is_empty() || !file.parent().unwrap().join(target).exists() {
            return Err(format!(
                "{}: missing relative link {link}",
                file.display()
            ));
        }
    }
    Ok(())
}

fn check_documented_commands(text: &str) -> Result<(), String> {
    // Only inline code in the implemented section is a command claim.
    // Pending interfaces are explicitly exempt, as are prose and code blocks.
    let implemented = text.split("## Pending").next().unwrap_or_default();
    for command in implemented.split('`').skip(1).step_by(2) {
        if let Some(task) = command.strip_prefix("cargo xtask ") {
            let task = task.split_whitespace().next().unwrap_or_default();
            if !matches!(
                task,
                "check"
                    | "fmt"
                    | "clippy"
                    | "doc"
                    | "test"
                    | "validate"
                    | "coverage"
                    | "dupes"
                    | "deny"
                    | "audit"
                    | "dep-age-check"
                    | "frontend"
                    | "frontend-check"
                    | "frontend-fmt"
                    | "frontend-test"
                    | "frontend-coverage"
                    | "frontend-dupes"
                    | "browser-test"
                    | "handoff-test"
                    | "release-check"
                    | "canon-check"
                    | "ci-mode"
                    | "ci-cache-plan"
                    | "ci-cache-prune"
                    | "--help"
                    | "-h"
            ) {
                return Err(format!(
                    "tooling status documents an unavailable command: {task}"
                ));
            }
        }
    }
    Ok(())
}
