//! Thin subprocess adapter for Silkboard repository tasks.

use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let root = match std::env::current_dir()
        .map_err(|error| error.to_string())
        .and_then(|cwd| xtask::workspace::root(&cwd))
    {
        Ok(root) => root,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let mut runner = |program: &str, args: &[&str], cwd: &Path| {
        let executable = if program == "cargo" {
            std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into())
        } else {
            program.into()
        };
        let mut command = Command::new(executable);
        command.args(args).current_dir(cwd);
        // Only the parent admission gate may read/write operator date evidence.
        command.env_remove("SILKBOARD_RUST_DATES_FILE");
        if program == "cargo" && args.first() == Some(&"doc") {
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
        let output = command.output().map_err(|e| {
            format!("could not run {program} {}: {e}", args.join(" "))
        })?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| format!("{program} output is not UTF-8: {e}"))?;
        if !stderr.trim().is_empty() {
            eprint!("{stderr}");
        }
        if output.status.success() {
            Ok(stdout)
        } else {
            Err(format!(
                "{program} {} failed ({}):\n{stdout}\n{stderr}",
                args.join(" "),
                output.status
            ))
        }
    };
    let dates = std::env::var_os("SILKBOARD_RUST_DATES_FILE")
        .filter(|value| !value.is_empty());
    match xtask::dispatch_with_publication_cache(
        &args,
        &root,
        &mut runner,
        dates.as_deref().map(Path::new),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
