//! Root-only installed updater entry point; serialization is owned by its wrapper.

use silkboard_updater::{Backend, linux::Linux, poll, retry};
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
    let mut backend = Linux::production(command);
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let result = match arguments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["poll"] => poll(&mut backend),
        ["retry", tag] => retry(&mut backend, tag),
        ["status"] => backend
            .load()
            .and_then(|s| {
                serde_json::to_string_pretty(&s).map_err(|e| e.to_string())
            })
            .map(|s| println!("{s}")),
        ["inspect", tag] => backend
            .refresh()
            .and_then(|_| backend.admit(tag))
            .and_then(|release| {
                release.map_or_else(
                    || Err("release is not eligible".into()),
                    |r| {
                        println!("Eligible {} ({})", r.tag(), r.commit());
                        Ok(())
                    },
                )
            }),
        _ => Err(
            "usage: silkboard-update poll | status | retry TAG | inspect TAG"
                .into(),
        ),
    };
    if let Err(error) = result {
        eprintln!("silkboard updater: {error}");
        std::process::exit(1);
    }
}
