//! The installed executable documents its interface without credentials.

#[test]
fn help_describes_the_standalone_publisher() {
    let output =
        std::process::Command::new(env!("CARGO_BIN_EXE_silkboard-publish"))
            .arg("--help")
            .output()
            .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("publish --config"));
    assert!(help.contains("stdin"));
}

#[test]
fn installed_binary_carries_its_third_party_notices() {
    let output =
        std::process::Command::new(env!("CARGO_BIN_EXE_silkboard-publish"))
            .arg("--licenses")
            .output()
            .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Community Data License Agreement"));
    assert!(text.contains("ring 0.17.14"));
}
