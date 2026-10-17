//! Machine-readable operator failures must precede deployment work.

use std::process::Command;

#[test]
fn unknown_profile_returns_a_structured_error_without_deployment_work() {
    let output = Command::new(env!("CARGO_BIN_EXE_silkboard-updater"))
        .args(["--json", "--profile", "unknown", "status"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["outcome"], "error");
    assert_eq!(report["error"]["stage"], "configuration");
}
