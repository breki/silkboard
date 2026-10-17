//! Root-only installed updater entry point; serialization is owned by its wrapper.

use silkboard_updater::{
    cli::{self, Operation},
    linux::Linux,
    outcome::{Error, Stage},
    profile::Policy,
};
use std::process::{Command, Stdio};

fn command(program: &str, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
        .env("HOME", "/root")
        .env("LC_ALL", "C")
        .stdin(Stdio::null());
    if args.contains(&"/usr/local/libexec/silkboard/build") {
        command.stdout(std::io::stderr());
        let status = command.status().map_err(|e| e.to_string())?;
        return if status.success() {
            Ok(String::new())
        } else {
            Err("unprivileged build failed; see journal".into())
        };
    }
    let output = command.output().map_err(|e| format!("{program}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

fn main() {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let json = arguments.iter().any(|arg| arg == "--json");
    let result = (|| {
        let request = cli::parse(&arguments)?;
        if request.operation == Operation::Help {
            return Ok(cli::Report::Help {
                usage: cli::USAGE.into(),
            });
        }
        let policy = Policy::installed()
            .map_err(|e| Error::new(Stage::Configuration, e))?;
        let mut backend = Linux::configured(command, policy.clone())
            .map_err(|e| Error::new(Stage::Configuration, e))?;
        cli::execute(
            &request.operation,
            &mut backend,
            &policy,
            silkboard_updater::storage::SILKBOARD_ADAPTER,
        )
    })()
    .and_then(|report| {
        if json {
            cli::machine(&report)
        } else {
            cli::human(&report)
        }
    });
    match result {
        Ok(report) => println!("{report}"),
        Err(error) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({"schema":1,"outcome":"error", "error":error})
                );
            } else {
                eprintln!("silkboard updater: {error}");
            }
            std::process::exit(1);
        }
    }
}
