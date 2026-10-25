//! Reused repository helpers must act on the calling worktree.
#![cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn reused_xtask_checks_the_calling_worktree() {
    let dir = std::env::temp_dir()
        .join(format!("silkboard-worktree-root-{}", std::process::id()));
    fs::create_dir_all(dir.join("xtask/src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"xtask\"]\n",
    )
    .unwrap();
    fs::write(
        dir.join("xtask/Cargo.toml"),
        "[package]\nname = \"xtask\"\n",
    )
    .unwrap();
    let cargo = dir.join("fake-cargo");
    fs::write(&cargo, "#!/bin/sh\npwd\n").unwrap();
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("check")
        .current_dir(dir.join("xtask/src"))
        .env("CARGO", &cargo)
        .output()
        .unwrap();
    fs::remove_dir_all(&dir).unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        dir.to_str().unwrap()
    );
}

#[test]
fn runtime_root_rejects_unrelated_or_invalid_workspaces() {
    let dir = std::env::temp_dir()
        .join(format!("silkboard-root-errors-{}", std::process::id()));
    fs::create_dir_all(dir.join("child")).unwrap();
    assert!(xtask::workspace::root(&dir).is_err());
    fs::write(dir.join("Cargo.toml"), "[workspace]\n").unwrap();
    assert!(
        xtask::workspace::root(&dir)
            .unwrap_err()
            .contains("no xtask")
    );
    fs::write(dir.join("child/Cargo.toml"), "invalid toml !").unwrap();
    assert!(
        xtask::workspace::root(&dir.join("child"))
            .unwrap_err()
            .contains("parse")
    );
    fs::write(dir.join("child/Cargo.toml"), "[package]\nname = 'child'\n")
        .unwrap();
    fs::create_dir_all(dir.join("xtask")).unwrap();
    fs::write(dir.join("xtask/Cargo.toml"), "[package]\nname = 'xtask'\n")
        .unwrap();
    assert_eq!(xtask::workspace::root(&dir.join("child")).unwrap(), dir);
    fs::remove_dir_all(dir).unwrap();
}
