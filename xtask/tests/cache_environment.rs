//! Operator cache settings must not become test/subprocess write authority.
#![cfg(unix)]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
#[test]
fn publication_cache_location_is_not_inherited_by_subprocesses() {
    let root = std::env::temp_dir().join(format!(
        "silkboard-cache-env-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let cargo = root.join("cargo");
    fs::write(
        &cargo,
        "#!/bin/sh\n[ -z \"${SILKBOARD_RUST_DATES_FILE+x}\" ]\n",
    )
    .unwrap();
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("check")
        .env("CARGO", cargo)
        .env("SILKBOARD_RUST_DATES_FILE", root.join("facts.json"))
        .output()
        .unwrap();
    fs::remove_dir_all(root).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
