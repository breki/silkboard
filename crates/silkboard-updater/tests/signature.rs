//! Real Git/SSH verification precedes GitHub evidence and any build.
use silkboard_updater::{Backend, linux::Linux};
use std::{fs, path::Path, process::Command};

fn command(
    program: &str,
    args: &[&str],
    root: &Path,
) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap();
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    Ok(String::from_utf8(output.stdout).unwrap())
}

#[test]
fn real_signed_tag_is_checked_before_api_and_aliases_and_untrusted_keys_fail() {
    let root = std::env::temp_dir()
        .join(format!("silkboard-signature-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let result = std::panic::catch_unwind(|| {
        command("git", &["init", "."], &root).unwrap();
        command("git", &["commit", "--allow-empty", "-m", "fixture"], &root)
            .unwrap();
        command(
            "ssh-keygen",
            &["-q", "-t", "ed25519", "-N", "", "-f", "key"],
            &root,
        )
        .unwrap();
        let key = root.join("key").display().to_string();
        command(
            "git",
            &[
                "-c",
                "gpg.format=ssh",
                "-c",
                &format!("user.signingkey={key}"),
                "tag",
                "-s",
                "v0.2.0",
                "-m",
                "fixture",
            ],
            &root,
        )
        .unwrap();
        let public = fs::read_to_string(root.join("key.pub")).unwrap();
        let trust = root.join("signers");
        fs::write(&trust, format!("silkboard-release {public}")).unwrap();
        let repo = root.join(".git").display().to_string();
        let mut api_calls = 0;
        let mut verified = false;
        let mut backend = Linux::production(|program: &str, args: &[&str]| {
            if program == "timeout" {
                let start = args.iter().position(|a| *a == "git").unwrap() + 1;
                let mut git_args = args[start..]
                    .iter()
                    .map(|a| (*a).to_owned())
                    .collect::<Vec<_>>();
                let pos =
                    git_args.iter().position(|a| a == "--git-dir").unwrap();
                git_args[pos + 1] = repo.clone();
                for arg in &mut git_args {
                    if arg.starts_with("gpg.ssh.allowedSignersFile=") {
                        *arg = format!(
                            "gpg.ssh.allowedSignersFile={}",
                            trust.display()
                        );
                    }
                }
                let output = command(
                    "git",
                    &git_args.iter().map(String::as_str).collect::<Vec<_>>(),
                    &root,
                );
                if git_args.iter().any(|a| a == "verify-tag") {
                    verified = output.is_ok();
                }
                output
            } else {
                assert!(
                    verified,
                    "GitHub evidence must follow trusted signature verification"
                );
                api_calls += 1;
                Ok("{\"id\":387356305,\"path\":\".github/workflows/release.yml\",\"state\":\"active\",\"workflow_runs\":[]}".into())
            }
        });
        // Passing trust reaches API; no successful run means pending, not eligible.
        assert!(backend.admit("v0.2.0").unwrap().is_none());
        command(
            "git",
            &["update-ref", "refs/tags/v0.3.0", "refs/tags/v0.2.0"],
            &root,
        )
        .unwrap();
        assert!(backend.admit("v0.3.0").unwrap().is_none());
        fs::write(&trust, "").unwrap();
        assert!(backend.admit("v0.2.0").unwrap().is_none());
        command("git", &["tag", "-d", "v0.2.0"], &root).unwrap();
        command("git", &["tag", "v0.2.0"], &root).unwrap();
        assert!(backend.admit("v0.2.0").unwrap().is_none());
        assert_eq!(api_calls, 2, "only the trusted tag may reach GitHub");
    });
    fs::remove_dir_all(&root).unwrap();
    result.unwrap();
}
