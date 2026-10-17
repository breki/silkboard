//! Operator bootstrap transport contracts without SSH or VM access.

use std::{path::Path, process::Command};

#[test]
fn operator_transport_preserves_boundaries_and_failures() {
    run_fixture("transport.sh");
}

#[test]
fn initial_activation_cleans_up_every_failure_path() {
    run_fixture("activation.sh");
}

fn run_fixture(name: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let output = Command::new("bash")
        .arg(root.join("deploy/vm/tests").join(name))
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
