//! Minimal repository checks; application and release tooling remain pending.

use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, ExitCode};

const USAGE: &str = "Usage: cargo xtask <clippy|doc|test>";

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    match dispatch(&args, |args, docs| run_cargo(&cargo, args, docs)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch(
    args: &[OsString],
    mut run: impl FnMut(&[&str], bool) -> Result<(), String>,
) -> Result<(), String> {
    match args {
        [task] if task == "clippy" => run(
            &[
                "clippy",
                "--locked",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
            false,
        ),
        [task] if task == "doc" => {
            run(&["doc", "--locked", "--workspace", "--no-deps"], true)?;
            run(
                &[
                    "doc",
                    "--locked",
                    "--workspace",
                    "--no-deps",
                    "--document-private-items",
                ],
                true,
            )
        }
        [task] if task == "test" => {
            run(&["test", "--locked", "--workspace"], false)
        }
        [task] if task == "--help" || task == "-h" => {
            println!("{USAGE}");
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn run_cargo(
    cargo: &OsString,
    args: &[&str],
    docs: bool,
) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace member");
    let mut command = Command::new(cargo);
    command.args(args).current_dir(root);
    if docs {
        // Append to existing flags so callers retain their rustdoc settings.
        // Encoded flags take precedence over RUSTDOCFLAGS in Cargo.
        let mut flags = std::env::var_os("CARGO_ENCODED_RUSTDOCFLAGS")
            .unwrap_or_else(|| {
                std::env::var("RUSTDOCFLAGS")
                    .unwrap_or_default()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join("\x1f")
                    .into()
            });
        if !flags.is_empty() {
            flags.push("\x1f");
        }
        flags.push("-Dwarnings");
        command.env("CARGO_ENCODED_RUSTDOCFLAGS", flags);
    }
    let status = command
        .status()
        .map_err(|error| format!("could not run cargo {}: {error}", args[0]))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {} failed ({status})", args[0]))
    }
}

#[cfg(test)]
mod tests {
    use super::dispatch;
    use std::ffi::OsString;

    #[test]
    fn rejects_unknown_tasks_and_extra_arguments_without_running_cargo() {
        for args in [vec![], vec!["validate"], vec!["test", "ignored"]] {
            let args: Vec<OsString> =
                args.into_iter().map(Into::into).collect();
            assert!(
                dispatch(&args, |_, _| panic!("must not execute")).is_err()
            );
        }
    }

    #[test]
    fn each_task_propagates_a_failed_cargo_stage() {
        for task in ["clippy", "doc", "test"] {
            let mut calls = 0;
            let result = dispatch(&[task.into()], |_, _| {
                calls += 1;
                Err("failed stage".into())
            });
            assert_eq!(result, Err("failed stage".into()));
            assert_eq!(calls, 1, "{task} must stop at the first failure");
        }
    }

    #[test]
    fn documentation_checks_both_visibility_scopes_and_propagates_failure() {
        let mut private_passes = Vec::new();
        let result = dispatch(&["doc".into()], |args, deny_warnings| {
            assert!(deny_warnings);
            let private = args.contains(&"--document-private-items");
            private_passes.push(private);
            if private {
                Err("private docs failed".into())
            } else {
                Ok(())
            }
        });
        assert_eq!(private_passes, [false, true]);
        assert_eq!(result, Err("private docs failed".into()));
    }
}
