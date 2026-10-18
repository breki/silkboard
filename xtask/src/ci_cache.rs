//! Configuration-bound CI dependency caches and verified workspace cleanup.

use crate::Runner;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MODES: &[(&str, &str)] = &[
    ("normal", "host-dev-test-inc0"),
    ("coverage", "host-test-inc0-llvmcov0.9.0"),
    ("arm64", "aarch64-unknown-linux-gnu-dev-test-inc0"),
];
static NEXT_INPUT: AtomicU64 = AtomicU64::new(0);

fn required<'a>(
    env: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, String> {
    env.get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing CI cache metadata: {key}"))
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn compiler_setting(key: &str) -> bool {
    matches!(
        key,
        "RUSTC"
            | "RUSTDOC"
            | "RUSTFLAGS"
            | "RUSTDOCFLAGS"
            | "RUSTC_WRAPPER"
            | "RUSTC_WORKSPACE_WRAPPER"
            | "CARGO_INCREMENTAL"
            | "CARGO_ENCODED_RUSTFLAGS"
            | "CARGO_ENCODED_RUSTDOCFLAGS"
            | "CI_ARM64_TARGET_DIR"
            | "CC"
            | "CXX"
            | "AR"
            | "CFLAGS"
            | "CXXFLAGS"
            | "CPPFLAGS"
            | "LDFLAGS"
    ) || [
        "CARGO_PROFILE_",
        "CARGO_TARGET_",
        "CARGO_BUILD_",
        "CARGO_LLVM_COV_",
        "LLVM_",
        "CC_",
        "CXX_",
        "AR_",
    ]
    .iter()
    .any(|prefix| key.starts_with(prefix))
}

struct InputFile(PathBuf);
impl Drop for InputFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn digest(
    root: &Path,
    runner: &Runner<'_>,
    env: &BTreeMap<String, String>,
) -> Result<String, String> {
    let config = required(env, "CACHE_CONFIG_HASH")?;
    if !hex(config, 64) {
        return Err("invalid cache configuration digest".into());
    }
    let flags: BTreeMap<_, _> = env
        .iter()
        .filter(|(key, _)| compiler_setting(key))
        .collect();
    let rustc = runner("rustc", &["-vV"], root)?;
    if !rustc.contains("release:") || !rustc.contains("host:") {
        return Err("missing Rust compiler identity".into());
    }
    let temp = Path::new(required(env, "RUNNER_TEMP")?);
    if !temp.is_absolute() {
        return Err("RUNNER_TEMP must be absolute".into());
    }
    let input_path = temp.join(format!(
        "silkboard-cache-key-{}-{}",
        std::process::id(),
        NEXT_INPUT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&input_path)
        .map_err(|e| format!("cache fingerprint input: {e}"))?;
    let input = InputFile(input_path);
    let payload =
        serde_json::json!({"config":config,"rustc":rustc,"flags":flags});
    file.write_all(&serde_json::to_vec(&payload).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    drop(file);
    let output = runner(
        "sha256sum",
        &[input.0.to_str().ok_or("cache input path is not UTF-8")?],
        root,
    )?;
    let digest = output
        .split_whitespace()
        .next()
        .filter(|value| hex(value, 64))
        .ok_or("invalid cache fingerprint output")?;
    Ok(digest.into())
}

fn plan(
    root: &Path,
    runner: &Runner<'_>,
    env: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, String> {
    let event = required(env, "GITHUB_EVENT_NAME")?;
    let reference = required(env, "GITHUB_REF")?;
    let pr = event == "pull_request";
    if !pr
        && !(event == "push"
            && (reference == "refs/heads/main"
                || reference.starts_with("refs/tags/")))
    {
        return Ok(BTreeMap::from([("enabled".into(), "false".into())]));
    }
    if required(env, "CARGO_INCREMENTAL")? != "0" {
        return Err("CI dependency caches require CARGO_INCREMENTAL=0".into());
    }
    let mut identity = Vec::new();
    for key in ["RUNNER_OS", "RUNNER_ARCH", "ImageOS", "ImageVersion"] {
        let value = required(env, key)?;
        if value.len() > 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err(format!("invalid cache runner metadata: {key}"));
        }
        identity.push(value);
    }
    let local = if pr {
        let number = required(env, "PR_NUMBER")?;
        let id: u64 = number.parse().map_err(|_| "invalid cache PR number")?;
        let head = required(env, "PR_HEAD_SHA")?;
        if id == 0
            || id.to_string() != number
            || !hex(head, 40)
            || reference != format!("refs/pull/{number}/merge")
        {
            return Err("invalid cache PR identity".into());
        }
        format!("pr-{number}-head-{head}")
    } else {
        "trusted".into()
    };
    let fingerprint = digest(root, runner, env)?;
    let prefix = format!("rust-build-v1-{}-{fingerprint}", identity.join("-"));
    let mut outputs = BTreeMap::from([("enabled".into(), "true".into())]);
    for (mode, profile) in MODES {
        for (kind, scope) in [("trusted", "trusted"), ("local", local.as_str())]
        {
            let key = format!("{prefix}-{profile}-{scope}");
            if key.len() > 512 {
                return Err("cache key exceeds GitHub's limit".into());
            }
            outputs.insert(format!("{mode}_{kind}"), key);
        }
    }
    Ok(outputs)
}

pub(super) fn print_plan(
    root: &Path,
    runner: &Runner<'_>,
) -> Result<(), String> {
    for (key, value) in plan(root, runner, &environment()?)? {
        println!("{key}={value}");
    }
    Ok(())
}

fn environment() -> Result<BTreeMap<String, String>, String> {
    let metadata = [
        "GITHUB_EVENT_NAME",
        "GITHUB_REF",
        "RUNNER_TEMP",
        "RUNNER_OS",
        "RUNNER_ARCH",
        "ImageOS",
        "ImageVersion",
        "CACHE_CONFIG_HASH",
        "PR_NUMBER",
        "PR_HEAD_SHA",
    ];
    std::env::vars_os()
        .filter_map(|(key, value)| {
            let key = key.into_string().ok()?;
            if !metadata.contains(&key.as_str()) && !compiler_setting(&key) {
                return None;
            }
            Some(
                value
                    .into_string()
                    .map(|value| (key, value))
                    .map_err(|_| "non-UTF-8 CI cache metadata".to_string()),
            )
        })
        .collect()
}

pub(super) fn prune_checked(
    root: &Path,
    runner: &Runner<'_>,
    temp: &Path,
    expected_key: &str,
) -> Result<(), String> {
    let env = environment()?;
    prune_with_env(root, runner, temp, expected_key, &env)
}

fn prune_with_env(
    root: &Path,
    runner: &Runner<'_>,
    temp: &Path,
    expected_key: &str,
    env: &BTreeMap<String, String>,
) -> Result<(), String> {
    if plan(root, runner, env)?
        .get("normal_local")
        .map(String::as_str)
        != Some(expected_key)
    {
        return Err("CI cache configuration changed during validation; refusing to save".into());
    }
    let base = temp.join("silkboard-ci-rust");
    for (variable, mode) in [
        ("CARGO_TARGET_DIR", "normal"),
        ("CARGO_LLVM_COV_TARGET_DIR", "coverage"),
        ("CI_ARM64_TARGET_DIR", "arm64"),
    ] {
        if Path::new(required(env, variable)?) != base.join(mode) {
            return Err(format!(
                "cache archive path does not match cleanup: {variable}"
            ));
        }
    }
    if temp != Path::new(required(env, "RUNNER_TEMP")?) {
        return Err("cache cleanup runner temp changed".into());
    }
    prune(root, runner, temp)
}

pub(super) fn prune(
    root: &Path,
    runner: &Runner<'_>,
    temp: &Path,
) -> Result<(), String> {
    if !temp.is_absolute() || temp.parent().is_none() {
        return Err(
            "cache cleanup requires an absolute runner temp directory".into()
        );
    }
    let temp = fs::canonicalize(temp).map_err(|e| e.to_string())?;
    let base = temp.join("silkboard-ci-rust");
    let repository = fs::canonicalize(root).map_err(|e| e.to_string())?;
    if base.starts_with(&repository) || repository.starts_with(&base) {
        return Err("cache cleanup must be outside the repository".into());
    }
    let paths: Vec<_> = MODES.iter().map(|(mode, _)| base.join(mode)).collect();
    for path in std::iter::once(&base).chain(paths.iter()) {
        if fs::symlink_metadata(path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
            || !path.is_dir()
        {
            return Err(format!("unsafe cache directory: {}", path.display()));
        }
    }
    for path in &paths {
        check_tree(path, path, None)?;
    }
    let metadata = runner(
        "cargo",
        &["metadata", "--locked", "--no-deps", "--format-version", "1"],
        root,
    )?;
    let (packages, names) = workspace(&metadata)?;
    let mut package_args = Vec::new();
    for package in &packages {
        package_args.extend(["--package", package.as_str()]);
    }
    for (index, path) in paths.iter().enumerate() {
        let target = path.to_str().ok_or("cache directory is not UTF-8")?;
        if index == 1 {
            let directory = format!("CARGO_LLVM_COV_TARGET_DIR={target}");
            runner(
                "env",
                &[&directory, "cargo", "llvm-cov", "clean", "--workspace"],
                root,
            )?;
        } else {
            let mut args = vec!["clean", "--target-dir", target];
            if index == 0 {
                let mut docs = args.clone();
                docs.push("--doc");
                runner("cargo", &docs, root)?;
            }
            args.extend_from_slice(&package_args);
            runner("cargo", &args, root)?;
            if index == 2 {
                args.extend(["--target", "aarch64-unknown-linux-gnu"]);
                runner("cargo", &args, root)?;
            }
        }
        check_tree(path, path, Some(&names))?;
    }
    println!(
        "Verified dependency caches: workspace artifacts and coverage evidence removed."
    );
    Ok(())
}

fn workspace(
    metadata: &str,
) -> Result<(Vec<String>, BTreeSet<String>), String> {
    let metadata: serde_json::Value =
        serde_json::from_str(metadata).map_err(|e| e.to_string())?;
    let members = metadata["workspace_members"]
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or("missing cache-clean workspace members")?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("missing cache-clean packages")?;
    let mut ids = Vec::new();
    let mut names = BTreeSet::new();
    for package in packages
        .iter()
        .filter(|package| members.contains(&package["id"]))
    {
        ids.push(
            package["id"]
                .as_str()
                .ok_or("missing workspace package ID")?
                .to_owned(),
        );
        let name = package["name"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("missing workspace package name")?;
        names.extend([name.to_owned(), name.replace('-', "_")]);
        for target in package["targets"]
            .as_array()
            .ok_or("missing workspace targets")?
        {
            if target["kind"]
                .as_array()
                .is_some_and(|k| k.iter().any(|v| v == "custom-build"))
            {
                continue;
            }
            let name = target["name"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("missing workspace target name")?;
            names.extend([name.to_owned(), name.replace('-', "_")]);
        }
    }
    if ids.len() != members.len() {
        return Err("incomplete cache-clean workspace metadata".into());
    }
    Ok((ids, names))
}

fn workspace_artifact(component: &str, names: &BTreeSet<String>) -> bool {
    [Some(component), component.strip_prefix("lib")]
        .into_iter()
        .flatten()
        .any(|component| {
            names.iter().any(|name| {
                component.strip_prefix(name).is_some_and(|tail| {
                    tail.is_empty()
                        || tail.starts_with('.')
                        || tail.strip_prefix('-').is_some_and(|hash| {
                            let hash =
                                hash.split('.').next().unwrap_or_default();
                            hash.len() >= 7
                                && hash.bytes().all(|b| b.is_ascii_hexdigit())
                        })
                })
            })
        })
}

fn check_tree(
    base: &Path,
    directory: &Path,
    names: Option<&BTreeSet<String>>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let kind = fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type();
        if kind.is_symlink() || !(kind.is_file() || kind.is_dir()) {
            return Err(format!("unsafe cache entry: {}", path.display()));
        }
        if kind.is_dir() {
            check_tree(base, &path, names)?;
        } else if let Some(names) = names {
            let relative =
                path.strip_prefix(base).map_err(|e| e.to_string())?;
            let parts: Vec<_> =
                relative.iter().map(|part| part.to_string_lossy()).collect();
            if matches!(
                path.extension().and_then(|v| v.to_str()),
                Some("profraw" | "profdata")
            ) || parts.first().is_some_and(|v| {
                [
                    "doc",
                    "html",
                    "text",
                    "llvm-cov",
                    "llvm-cov-target",
                    "release",
                ]
                .contains(&v.as_ref())
            }) || parts.iter().any(|v| workspace_artifact(v, names))
            {
                return Err(format!(
                    "non-dependency artifact remains in cache: {}",
                    relative.display()
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, atomic::AtomicUsize};

    struct Fixture {
        home: PathBuf,
        root: PathBuf,
        temp: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let home = std::env::temp_dir().join(format!(
                "silkboard-cache-test-{}-{}",
                std::process::id(),
                NEXT_INPUT.fetch_add(1, Ordering::Relaxed)
            ));
            let root = home.join("repository");
            let temp = home.join("runner");
            fs::create_dir_all(&root).unwrap();
            for (mode, _) in MODES {
                fs::create_dir_all(temp.join("silkboard-ci-rust").join(mode))
                    .unwrap();
            }
            Self { home, root, temp }
        }
        fn file(&self, path: &str) -> PathBuf {
            let path = self.temp.join("silkboard-ci-rust").join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "test artifact").unwrap();
            path
        }
        fn env(&self) -> BTreeMap<String, String> {
            let mut values: BTreeMap<_, _> = [
                ("GITHUB_EVENT_NAME", "push"),
                ("GITHUB_REF", "refs/heads/main"),
                ("RUNNER_OS", "Linux"),
                ("RUNNER_ARCH", "X64"),
                ("ImageOS", "ubuntu24"),
                ("ImageVersion", "test.1"),
                ("CARGO_INCREMENTAL", "0"),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
            values.insert("CACHE_CONFIG_HASH".into(), "a".repeat(64));
            values.insert(
                "RUNNER_TEMP".into(),
                self.temp.to_str().unwrap().into(),
            );
            for (variable, mode) in [
                ("CARGO_TARGET_DIR", "normal"),
                ("CARGO_LLVM_COV_TARGET_DIR", "coverage"),
                ("CI_ARM64_TARGET_DIR", "arm64"),
            ] {
                values.insert(
                    variable.into(),
                    self.temp
                        .join("silkboard-ci-rust")
                        .join(mode)
                        .to_str()
                        .unwrap()
                        .into(),
                );
            }
            values
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.home).unwrap();
        }
    }

    fn metadata() -> String {
        serde_json::json!({"workspace_members":["member-id"],"packages":[
            {"id":"member-id","name":"sample-workspace","targets":[{"name":"sample-workspace","kind":["lib"]},{"name":"custom-binary","kind":["bin"]},{"name":"build-script-build","kind":["custom-build"]}]},
            {"id":"external-id","name":"external","targets":[]}
        ]}).to_string()
    }

    fn fingerprint(
        program: &str,
        args: &[&str],
        _: &Path,
    ) -> Result<String, String> {
        match program {
            "rustc" => {
                Ok("release: 1.93.1\nhost: x86_64-unknown-linux-gnu\n".into())
            }
            "sha256sum" => {
                let output = std::process::Command::new(program)
                    .args(args)
                    .output()
                    .map_err(|e| e.to_string())?;
                if !output.status.success() {
                    return Err("fingerprint command failed".into());
                }
                String::from_utf8(output.stdout).map_err(|e| e.to_string())
            }
            _ => Ok(if args[0] == "metadata" {
                metadata()
            } else {
                String::new()
            }),
        }
    }

    #[test]
    fn cleanup_keeps_dependencies_and_cleans_every_workspace_mode() {
        let fixture = Fixture::new();
        let artifacts: Vec<_> = ["normal/doc/index.html", "normal/debug/deps/libsample_workspace-abcdef012.rlib", "coverage/stale.profraw", "coverage/stale.profdata", "coverage/debug/deps/custom_binary-abcdef012", "arm64/debug/deps/libsample_workspace-abcdef012.rmeta", "arm64/aarch64-unknown-linux-gnu/debug/deps/libsample_workspace-abcdef012.rmeta"].iter().map(|path| fixture.file(path)).collect();
        let dependencies = [fixture.file("normal/debug/deps/libexternal-abcdef012.rlib"), fixture.file("coverage/debug/build/external-abcdef012/build_script_build-abcdef012"), fixture.file("arm64/debug/deps/libexternal-abcdef012.rmeta")];
        let calls = Mutex::new(Vec::new());
        let env = fixture.env();
        let expected =
            plan(&fixture.root, &fingerprint, &env).unwrap()["normal_local"]
                .clone();
        prune_with_env(
            &fixture.root,
            &|program, args, cwd| {
                if matches!(program, "rustc" | "sha256sum") {
                    return fingerprint(program, args, cwd);
                }
                calls.lock().unwrap().push((
                    program.to_owned(),
                    args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
                ));
                if args[0] == "metadata" {
                    return Ok(metadata());
                }
                for artifact in &artifacts {
                    if artifact.exists() {
                        fs::remove_file(artifact).unwrap();
                    }
                }
                Ok(String::new())
            },
            &fixture.temp,
            &expected,
            &env,
        )
        .unwrap();
        assert!(artifacts.iter().all(|path| !path.exists()));
        assert!(dependencies.iter().all(|path| path.exists()));
        let calls = calls.into_inner().unwrap();
        assert_eq!(calls.len(), 6);
        assert!(calls.iter().any(|(p, a)| p == "env"
            && a[0].contains("/coverage")
            && a.ends_with(&["clean".into(), "--workspace".into()])));
        assert!(calls.iter().any(|(_, a)| a.ends_with(&[
            "--target".into(),
            "aarch64-unknown-linux-gnu".into()
        ])));
        assert_eq!(
            calls
                .iter()
                .filter(|(_, a)| a.contains(&"member-id".into()))
                .count(),
            3
        );
    }

    #[test]
    fn cleanup_stops_on_every_command_failure() {
        let fixture = Fixture::new();
        for failure in 0..6 {
            let calls = AtomicUsize::new(0);
            let result = prune(
                &fixture.root,
                &|_, args, _| {
                    if calls.fetch_add(1, Ordering::SeqCst) == failure {
                        Err("required cleanup unavailable".into())
                    } else {
                        Ok(if args[0] == "metadata" {
                            metadata()
                        } else {
                            String::new()
                        })
                    }
                },
                &fixture.temp,
            );
            assert_eq!(result.unwrap_err(), "required cleanup unavailable");
            assert_eq!(calls.load(Ordering::SeqCst), failure + 1);
        }
    }

    #[test]
    fn cleanup_rejects_every_kind_of_leftover_workspace_evidence() {
        for path in [
            "normal/debug/deps/libsample_workspace-abcdef012.rlib",
            "normal/debug/.fingerprint/sample-workspace-abcdef012/lib-sample_workspace",
            "normal/debug/build/sample-workspace-abcdef012/build-script-build",
            "normal/debug/deps/custom_binary-abcdef012",
            "coverage/stale.profraw",
            "coverage/stale.profdata",
            "normal/doc/index.html",
            "normal/release/deps/libexternal-abcdef012.rlib",
        ] {
            let fixture = Fixture::new();
            fixture.file(path);
            let error =
                prune(&fixture.root, &fingerprint, &fixture.temp).unwrap_err();
            assert!(
                error.contains("non-dependency artifact"),
                "{path}: {error}"
            );
        }
    }

    #[test]
    fn cleanup_refuses_unsafe_roots_and_links_before_running_commands() {
        let fixture = Fixture::new();
        let no_commands = |_: &str, _: &[&str], _: &Path| {
            panic!("unsafe paths must not run cleanup commands")
        };
        for temp in [
            Path::new("relative"),
            Path::new("/"),
            fixture.root.as_path(),
        ] {
            assert!(prune(&fixture.root, &no_commands, temp).is_err());
        }
        let nested = fixture.temp.join("silkboard-ci-rust/normal/checkout");
        fs::create_dir(&nested).unwrap();
        assert!(prune(&nested, &no_commands, &fixture.temp).is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let outside = fixture.home.join("preserve");
            fs::write(&outside, "keep").unwrap();
            let link = fixture.temp.join("silkboard-ci-rust/normal/linked");
            symlink(&outside, &link).unwrap();
            assert!(prune(&fixture.root, &no_commands, &fixture.temp).is_err());
            assert_eq!(fs::read_to_string(&outside).unwrap(), "keep");
            fs::remove_file(&link).unwrap();
            let mode = fixture.temp.join("silkboard-ci-rust/arm64");
            fs::remove_dir(&mode).unwrap();
            symlink(&outside, &mode).unwrap();
            assert!(prune(&fixture.root, &no_commands, &fixture.temp).is_err());
        }
    }

    #[test]
    fn changed_configuration_or_archive_paths_cannot_be_published() {
        let fixture = Fixture::new();
        let env = fixture.env();
        let expected =
            plan(&fixture.root, &fingerprint, &env).unwrap()["normal_local"]
                .clone();
        let mut changed = env.clone();
        changed.insert("CACHE_CONFIG_HASH".into(), "b".repeat(64));
        assert!(
            prune_with_env(
                &fixture.root,
                &fingerprint,
                &fixture.temp,
                &expected,
                &changed
            )
            .unwrap_err()
            .contains("configuration changed")
        );
        for key in [
            "CARGO_TARGET_DIR",
            "CARGO_LLVM_COV_TARGET_DIR",
            "CI_ARM64_TARGET_DIR",
        ] {
            let mut changed = env.clone();
            changed.insert(
                key.into(),
                fixture.home.join("elsewhere").to_str().unwrap().into(),
            );
            let expected = plan(&fixture.root, &fingerprint, &changed).unwrap()
                ["normal_local"]
                .clone();
            assert!(
                prune_with_env(
                    &fixture.root,
                    &fingerprint,
                    &fixture.temp,
                    &expected,
                    &changed
                )
                .unwrap_err()
                .contains("archive path")
            );
        }
    }

    #[test]
    fn compiler_identity_is_bound_and_private_inputs_are_removed_on_failure() {
        let fixture = Fixture::new();
        let env = fixture.env();
        let base = plan(&fixture.root, &fingerprint, &env).unwrap();
        let changed = plan(
            &fixture.root,
            &|p, a, c| {
                if p == "rustc" {
                    Ok("release: 1.93.2\nhost: x86_64-unknown-linux-gnu\n"
                        .into())
                } else {
                    fingerprint(p, a, c)
                }
            },
            &env,
        )
        .unwrap();
        assert_ne!(base["normal_local"], changed["normal_local"]);
        for rustc_failure in [true, false] {
            assert!(
                plan(
                    &fixture.root,
                    &|p, _, _| if p == "rustc" {
                        Ok(if rustc_failure {
                            "bad compiler"
                        } else {
                            "release: 1.93.1\nhost: x86_64-unknown-linux-gnu\n"
                        }
                        .into())
                    } else {
                        Err("hash tool failed".into())
                    },
                    &env
                )
                .is_err()
            );
            assert_eq!(
                fs::read_dir(&fixture.temp).unwrap().count(),
                1,
                "private input file leaked"
            );
        }
        let mut irrelevant = env;
        irrelevant
            .insert("UNRELATED_CREDENTIAL".into(), "test-sentinel".into());
        assert_eq!(
            base,
            plan(&fixture.root, &fingerprint, &irrelevant).unwrap()
        );
    }

    #[test]
    fn malformed_workspace_metadata_fails_before_cleanup() {
        let fixture = Fixture::new();
        for value in [
            "bad JSON".to_owned(), "{}".into(),
            serde_json::json!({"workspace_members":[],"packages":[]}).to_string(),
            serde_json::json!({"workspace_members":["missing"],"packages":[]}).to_string(),
            serde_json::json!({"workspace_members":[null],"packages":[{"id":null}]}).to_string(),
            serde_json::json!({"workspace_members":["x"],"packages":[{"id":"x","name":null}]}).to_string(),
            serde_json::json!({"workspace_members":["x"],"packages":[{"id":"x","name":"workspace"}]}).to_string(),
            serde_json::json!({"workspace_members":["x"],"packages":[{"id":"x","name":"workspace","targets":[{}]}]}).to_string(),
        ] {
            assert!(prune(&fixture.root, &|p,args,_| {
                assert_eq!(p, "cargo"); assert_eq!(args[0], "metadata"); Ok(value.clone())
            }, &fixture.temp).is_err());
        }
    }

    #[test]
    fn cleanup_cannot_accept_successful_commands_with_stale_evidence() {
        let temp = std::env::temp_dir()
            .join(format!("silkboard-prune-red-{}", std::process::id()));
        let artifact = temp.join("silkboard-ci-rust/coverage/stale.profraw");
        for (mode, _) in MODES {
            fs::create_dir_all(temp.join("silkboard-ci-rust").join(mode))
                .unwrap();
        }
        fs::create_dir_all(artifact.parent().unwrap()).unwrap();
        fs::write(&artifact, "stale evidence").unwrap();
        let result = prune(
            Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap(),
            &|_, args, _| {
                if args[0] == "metadata" {
                    Ok(r#"{"workspace_members":["workspace"],"packages":[{"id":"workspace","name":"sample-workspace","targets":[{"name":"sample-workspace","kind":["lib"]}]}]}"#.into())
                } else {
                    Ok(String::new())
                }
            },
            &temp,
        );
        fs::remove_dir_all(temp).unwrap();
        assert!(
            result.is_err(),
            "successful clean commands left coverage evidence cacheable"
        );
    }
}
