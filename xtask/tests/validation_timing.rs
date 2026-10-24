//! A failed validation gate still reports its own elapsed time.
#[test]
fn failed_gate_has_a_bounded_timing_record_before_validation_stops() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["validate", "--check"])
        .env("CARGO", "/nonexistent-silkboard-timing-test-cargo")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    let records: Vec<_> = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("Validation timing: "))
        .collect();
    assert_eq!(records.len(), 1, "{stderr}");
    let record: serde_json::Value = serde_json::from_str(records[0]).unwrap();
    assert_eq!(record["gate"], "dep-age-check");
    assert_eq!(record["outcome"], "fail");
    assert!(record["elapsed_ms"].as_u64().is_some());
    assert!(!stderr.contains("Checking canon-check"));
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("All required gates passed")
    );
}
