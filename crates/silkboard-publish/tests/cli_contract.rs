//! Capability discovery and quiet hooks for installed-client callers.
use silkboard_publish::{Error, run};
use std::ffi::OsString;
fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}
#[test]
fn capability_discovery_needs_no_configuration_or_stdin() {
    let help = run(&args(&["--help"]), &b""[..]).unwrap();
    for capability in ["codex-hook-context", "select", "clear", "handoff"] {
        assert!(help.contains(capability));
    }
    assert!(
        run(&args(&["--licenses"]), &b""[..])
            .unwrap()
            .contains("License")
    );
    let version = run(&args(&["--version"]), &b""[..]).unwrap();
    assert!(
        version.starts_with(concat!(
            "silkboard-publish ",
            env!("CARGO_PKG_VERSION")
        ))
    );
}
#[test]
fn explicit_commands_report_bad_setup_but_native_hooks_remain_quiet() {
    for family in ["codex-plan", "plan"] {
        for command in ["start", "publish", "select", "clear"] {
            assert_eq!(
                run(
                    &args(&[
                        family,
                        command,
                        "--config",
                        "/nonexistent/silkboard.json"
                    ]),
                    &b""[..]
                ),
                Err(Error::PrivateFile)
            );
        }
    }
    for hook in ["codex-hook-context", "codex-hook"] {
        assert_eq!(
            run(
                &args(&[hook, "--config", "/nonexistent/silkboard.json"]),
                &b"private invalid payload"[..]
            ),
            Ok("{}")
        );
        assert_eq!(
            run(
                &args(&[hook, "--unsupported", "/nonexistent/silkboard.json"]),
                &b""[..]
            ),
            Err(Error::Usage)
        );
    }
}
