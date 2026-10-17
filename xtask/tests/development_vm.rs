//! Exercise native Bombyx provisioning without changing the host.

#[test]
fn development_vm_provisioning() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let result = std::process::Command::new("node")
        .args(["--test", ".bombyx/tests/provision.test.mjs"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
