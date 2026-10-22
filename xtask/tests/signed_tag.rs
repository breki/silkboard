//! Real Git/SSH verification in a disposable fixture, without owner keys.

use std::{
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use xtask::dispatch;

struct Fixture(std::path::PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn command(
    root: &Path,
    program: &str,
    args: &[&str],
) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8(output.stdout).unwrap())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into())
    }
}

#[test]
fn verifies_real_ssh_signature_and_rejects_another_signer() {
    let directory = std::env::temp_dir().join(format!(
        "silkboard-signature-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(directory.join("repo/.github")).unwrap();
    let _fixture = Fixture(directory.clone());
    let root = directory.join("repo");
    for name in ["trusted", "second", "other"] {
        command(
            &directory,
            "ssh-keygen",
            &[
                "-q",
                "-t",
                "ed25519",
                "-N",
                "",
                "-f",
                directory.join(name).to_str().unwrap(),
            ],
        )
        .unwrap();
    }
    let key = directory.join("trusted");
    fs::write(
        root.join(".github/release-signers"),
        format!(
            "silkboard-release {}silkboard-vm {}",
            fs::read_to_string(directory.join("trusted.pub")).unwrap(),
            fs::read_to_string(directory.join("second.pub")).unwrap()
        ),
    )
    .unwrap();
    command(&root, "git", &["init", "-q"]).unwrap();
    for (name, value) in [
        ("user.name", "Fixture"),
        ("user.email", "fixture@example.invalid"),
        ("gpg.format", "ssh"),
        ("user.signingkey", key.to_str().unwrap()),
        ("commit.gpgsign", "true"),
    ] {
        command(&root, "git", &["config", name, value]).unwrap();
    }
    command(&root, "git", &["add", ".github/release-signers"]).unwrap();
    command(&root, "git", &["commit", "-qm", "Fixture"]).unwrap();
    let admit = || {
        dispatch(
            &["release-check".into(), "v0.1.0".into()],
            &root,
            &mut |program, args, cwd| {
                if program == "cargo" {
                    Ok("{\"packages\":[{\"name\":\"silkboard-web\",\"version\":\"0.1.0\"}]}".into())
                } else {
                    command(cwd, program, args)
                }
            },
        )
    };
    // Tags exist only in this disposable repository, never in Silkboard.
    command(&root, "git", &["tag", "-s", "-m", "Fixture", "v0.1.0"]).unwrap();
    assert!(admit().is_ok());
    // The independent second signer has the same tag-admission authority.
    sign_tag(&root, &directory.join("second"), "v0.1.0");
    assert!(admit().is_ok());
    // An existing trusted non-release signature does not authorize a new name.
    command(&root, "git", &["tag", "-s", "-m", "Trial", "trial"]).unwrap();
    let trial =
        command(&root, "git", &["rev-parse", "refs/tags/trial"]).unwrap();
    command(
        &root,
        "git",
        &["update-ref", "refs/tags/v0.1.0", trial.trim()],
    )
    .unwrap();
    assert!(
        admit().is_err(),
        "a copied signed tag must not authorize a release"
    );
    sign_tag(&root, &directory.join("other"), "v0.1.0");
    assert!(admit().is_err());
}

fn sign_tag(root: &Path, key: &Path, tag: &str) {
    command(
        root,
        "git",
        &[
            "-c",
            &format!("user.signingkey={}", key.display()),
            "tag",
            "-fs",
            "-m",
            "Disposable signature fixture",
            tag,
        ],
    )
    .unwrap();
}
