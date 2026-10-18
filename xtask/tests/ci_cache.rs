//! CI cache plans bind configuration and isolate PR-produced artifacts.

use std::{collections::BTreeMap, fs, process::Command};

fn plan(extra: &[(&str, &str)]) -> std::process::Output {
    let temp = std::env::temp_dir().join(format!(
        "silkboard-cache-plan-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&temp).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("ci-cache-plan")
        .env("RUNNER_TEMP", &temp)
        .env("RUNNER_OS", "Linux")
        .env("RUNNER_ARCH", "X64")
        .env("ImageOS", "ubuntu24")
        .env("ImageVersion", "20261017.1.0")
        .env("GITHUB_EVENT_NAME", "push")
        .env("GITHUB_REF", "refs/heads/main")
        .env("CARGO_INCREMENTAL", "0")
        .env("CACHE_CONFIG_HASH", "a".repeat(64))
        .envs(extra.iter().copied())
        .output()
        .unwrap();
    assert_eq!(fs::read_dir(&temp).unwrap().count(), 0, "key inputs leaked");
    fs::remove_dir(temp).unwrap();
    result
}

fn keys(extra: &[(&str, &str)]) -> BTreeMap<String, String> {
    let result = plan(extra);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .map(|line| {
            let (key, value) = line.split_once('=').unwrap();
            (key.into(), value.into())
        })
        .collect()
}

#[test]
fn plans_reuse_dependencies_but_bind_configuration_flags_and_scope() {
    let base = keys(&[]);
    assert_eq!(base["enabled"], "true");
    assert_ne!(base["normal_trusted"], base["coverage_trusted"]);
    assert_ne!(base["normal_trusted"], base["arm64_trusted"]);
    for changed in [
        vec![(
            "CACHE_CONFIG_HASH",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )],
        vec![("ImageVersion", "20261018.1.0")],
        vec![("RUSTFLAGS", "-C target-cpu=x86-64-v2")],
        vec![("CARGO_ENCODED_RUSTDOCFLAGS", "-Dwarnings")],
        vec![("CARGO_PROFILE_TEST_OPT_LEVEL", "2")],
        vec![(
            "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
            "-C debuginfo=1",
        )],
    ] {
        assert_ne!(keys(&changed)["normal_trusted"], base["normal_trusted"]);
    }
    let head = "1".repeat(40);
    let pr = keys(&[
        ("GITHUB_EVENT_NAME", "pull_request"),
        ("GITHUB_REF", "refs/pull/31/merge"),
        ("PR_NUMBER", "31"),
        ("PR_HEAD_SHA", &head),
    ]);
    assert_eq!(pr["normal_trusted"], base["normal_trusted"]);
    assert_ne!(pr["normal_local"], pr["normal_trusted"]);
    let other = keys(&[
        ("GITHUB_EVENT_NAME", "pull_request"),
        ("GITHUB_REF", "refs/pull/31/merge"),
        ("PR_NUMBER", "31"),
        ("PR_HEAD_SHA", &"2".repeat(40)),
    ]);
    assert_ne!(pr["normal_local"], other["normal_local"]);
    assert!(
        pr.values()
            .all(|value| value.len() <= 512 && !value.contains('\n'))
    );
}

#[test]
fn uncertain_metadata_cannot_select_a_reusable_cache() {
    for invalid in [
        vec![("CACHE_CONFIG_HASH", "")],
        vec![("ImageVersion", "bad\nkey")],
        vec![("CARGO_INCREMENTAL", "1")],
        vec![("RUNNER_TEMP", "relative")],
        vec![("RUNNER_TEMP", "/dev/null")],
        vec![
            ("GITHUB_EVENT_NAME", "pull_request"),
            ("PR_NUMBER", "31"),
            ("PR_HEAD_SHA", "short"),
        ],
    ] {
        assert!(!plan(&invalid).status.success());
    }
    for event in ["workflow_dispatch", "pull_request_target", "unknown"] {
        assert_eq!(keys(&[("GITHUB_EVENT_NAME", event)])["enabled"], "false");
    }
    assert_eq!(
        keys(&[("GITHUB_REF", "refs/heads/topic")])["enabled"],
        "false"
    );
    assert_eq!(
        keys(&[("GITHUB_REF", "refs/tags/v0.2.0")])["normal_trusted"],
        keys(&[])["normal_trusted"]
    );
}
