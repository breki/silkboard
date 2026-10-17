//! Registry cooldown and npm license/source verification.

use crate::Runner;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const NPM_LICENSES: &[&str] = &[
    "MIT",
    "MIT-0",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "MPL-2.0",
    "BlueOak-1.0.0",
    "CC0-1.0",
    "0BSD",
    "(MIT OR CC0-1.0)",
    "MIT OR Apache-2.0",
];

pub(super) fn check(
    root: &Path,
    runner: &mut Runner<'_>,
) -> Result<(), String> {
    let metadata = runner(
        "cargo",
        &["metadata", "--locked", "--format-version", "1"],
        root,
    )?;
    let npm = fs::read_to_string(root.join("frontend/package-lock.json"))
        .map_err(|e| format!("frontend lockfile: {e}"))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    check_at(root, runner, &metadata, &npm, now)
}

fn json(text: &str) -> Result<Value, String> {
    serde_json::from_str(text)
        .map_err(|e| format!("invalid dependency JSON: {e}"))
}

fn array<'a>(value: &'a Value, field: &str) -> Result<&'a Vec<Value>, String> {
    value[field]
        .as_array()
        .ok_or_else(|| format!("missing {field} array"))
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value[field]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("missing {field} string"))
}

fn check_at(
    root: &Path,
    runner: &mut Runner<'_>,
    metadata: &str,
    npm: &str,
    now: u64,
) -> Result<(), String> {
    let metadata = json(metadata)?;
    let members = array(&metadata, "workspace_members")?;
    let packages = array(&metadata, "packages")?;
    if packages.is_empty() || members.is_empty() {
        return Err("empty Cargo metadata".into());
    }
    let mut dependencies = BTreeSet::new();
    for package in packages {
        let name = text(package, "name")?;
        let version = text(package, "version")?;
        match package["source"].as_str() {
            Some("registry+https://github.com/rust-lang/crates.io-index") => {
                dependencies.insert((
                    false,
                    name.to_owned(),
                    version.to_owned(),
                ));
            }
            None if members.contains(&package["id"]) => {}
            _ => {
                return Err(format!(
                    "unapproved Cargo source: {name}@{version}"
                ));
            }
        }
    }
    let lock = json(npm)?;
    if lock["lockfileVersion"] != 3 {
        return Err("require npm lockfileVersion 3".into());
    }
    let packages = lock["packages"]
        .as_object()
        .filter(|p| p.len() > 1)
        .ok_or("missing npm packages")?;
    for (path, package) in packages {
        if path.is_empty() {
            continue;
        }
        let name = path
            .rsplit("node_modules/")
            .next()
            .filter(|_| path.starts_with("node_modules/"))
            .ok_or("unexpected npm package path")?;
        let version = text(package, "version")?;
        let resolved = text(package, "resolved")?;
        if !resolved.starts_with("https://registry.npmjs.org/")
            || !text(package, "integrity")?.starts_with("sha512-")
        {
            return Err(format!("unapproved npm source/integrity: {path}"));
        }
        if !NPM_LICENSES.contains(&text(package, "license")?) {
            return Err(format!("unapproved npm license: {path}"));
        }
        dependencies.insert((true, name.to_owned(), version.to_owned()));
    }
    let mut npm_times = BTreeMap::<String, Value>::new();
    let count = dependencies.len();
    for (index, (npm, name, version)) in dependencies.into_iter().enumerate() {
        let encoded = package_segment(&name)?;
        let date = if npm {
            if !npm_times.contains_key(&name) {
                let value = fetch(
                    root,
                    runner,
                    &format!("https://registry.npmjs.org/{encoded}"),
                )?;
                npm_times.insert(name.clone(), value["time"].clone());
            }
            npm_times[&name][&version]
                .as_str()
                .ok_or_else(|| {
                    format!("no npm publish date: {name}@{version}")
                })?
                .to_owned()
        } else {
            let value = fetch(
                root,
                runner,
                &format!("https://crates.io/api/v1/crates/{encoded}/{version}"),
            )?;
            text(&value["version"], "created_at")?.to_owned()
        };
        let published = timestamp(&date)?;
        if now
            .checked_sub(published)
            .is_none_or(|age| age < 14 * 86400)
        {
            return Err(format!(
                "{name}@{version} published {date}: within 14-day cooldown"
            ));
        }
        if (index + 1) % 25 == 0 {
            eprintln!("Checked dependency ages: {}/{count}", index + 1);
        }
    }
    println!(
        "Dependency ages and npm license/sources passed ({count} versions)."
    );
    Ok(())
}

fn fetch(
    root: &Path,
    runner: &mut Runner<'_>,
    url: &str,
) -> Result<Value, String> {
    let response = runner(
        "curl",
        &[
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--connect-timeout",
            "10",
            "--max-time",
            "30",
            "--user-agent",
            "silkboard-dependency-check",
            url,
        ],
        root,
    )?;
    json(&response)
}

fn package_segment(name: &str) -> Result<String, String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"@/._-".contains(&b))
    {
        return Err(format!("invalid registry package name: {name:?}"));
    }
    Ok(name.replace('/', "%2F"))
}

fn timestamp(date: &str) -> Result<u64, String> {
    fn parse(value: Option<&str>) -> Result<u64, String> {
        value
            .ok_or("short registry timestamp")?
            .parse()
            .map_err(|_| "invalid registry date number".into())
    }
    let year = parse(date.get(0..4))?;
    let month = parse(date.get(5..7))?;
    let day = parse(date.get(8..10))?;
    let hour = parse(date.get(11..13))?;
    let minute = parse(date.get(14..16))?;
    let second = parse(date.get(17..19))?;
    let suffix = date.get(19..).ok_or("short registry timestamp")?;
    let fraction = suffix
        .strip_suffix('Z')
        .or_else(|| suffix.strip_suffix("+00:00"))
        .ok_or("registry date must be UTC")?;
    let leap = |year: u64| {
        year.is_multiple_of(4)
            && (!year.is_multiple_of(100) || year.is_multiple_of(400))
    };
    let lengths = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1970..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || day == 0
        || day > lengths[usize::try_from(month - 1).unwrap_or(12).min(11)]
        || hour > 23
        || minute > 59
        || second > 59
        || date.get(4..5) != Some("-")
        || date.get(7..8) != Some("-")
        || date.get(10..11) != Some("T")
        || date.get(13..14) != Some(":")
        || date.get(16..17) != Some(":")
        || !(fraction.is_empty()
            || fraction.strip_prefix('.').is_some_and(|v| {
                !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit())
            }))
    {
        return Err(format!("invalid registry timestamp: {date}"));
    }
    let years: u64 =
        (1970..year).map(|y| if leap(y) { 366 } else { 365 }).sum();
    let months: u64 =
        lengths[..usize::try_from(month - 1).unwrap()].iter().sum();
    Ok((((years + months + day - 1) * 24 + hour) * 60 + minute) * 60 + second)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn metadata(source: &Value) -> String {
        json!({"workspace_members":["workspace"],"packages":[{"id":"external","name":"axum","version":"0.8.9","source":source}]}).to_string()
    }
    fn npm() -> Value {
        json!({"lockfileVersion":3,"packages":{"":{},"node_modules/@scope/tool":{"version":"1.0.0","resolved":"https://registry.npmjs.org/@scope/tool/-/tool-1.0.0.tgz","integrity":"sha512-test","license":"MIT"}}})
    }
    fn run(meta: &str, lock: &Value, age: u64) -> Result<(), String> {
        check_at(
            Path::new("."),
            &mut |_, args, _| {
                assert_eq!(args[0], "--fail");
                if args.last().unwrap().contains("crates.io") {
                    Ok(json!({"version":{"created_at":"2026-01-01T00:00:00Z"}})
                        .to_string())
                } else {
                    assert!(args.last().unwrap().contains("@scope%2Ftool"));
                    Ok(json!({"time":{"1.0.0":"2026-01-01T00:00:00Z"}})
                        .to_string())
                }
            },
            meta,
            &lock.to_string(),
            timestamp("2026-01-01T00:00:00Z").unwrap() + age,
        )
    }
    #[test]
    fn cooldown_covers_both_registries_and_exact_boundary() {
        let meta = metadata(&json!(
            "registry+https://github.com/rust-lang/crates.io-index"
        ));
        assert!(run(&meta, &npm(), 14 * 86400).is_ok());
        for age in [0, 14 * 86400 - 1] {
            assert!(run(&meta, &npm(), age).unwrap_err().contains("cooldown"));
        }
        assert!(run(&meta, &npm(), 100 * 86400).is_ok());
    }
    #[test]
    fn rejects_unapproved_sources_licenses_and_missing_lock_fields() {
        for source in [
            Value::Null,
            json!("git+https://example.com"),
            json!("registry+https://example.com"),
        ] {
            assert!(run(&metadata(&source), &npm(), 20 * 86400).is_err());
        }
        let meta = metadata(&json!(
            "registry+https://github.com/rust-lang/crates.io-index"
        ));
        for (key, value) in [
            ("license", json!("GPL-3.0-only AND MIT")),
            ("resolved", json!("https://registry.npmjs.org.evil/x")),
            ("integrity", json!("sha1-old")),
            ("version", Value::Null),
        ] {
            let mut lock = npm();
            lock["packages"]["node_modules/@scope/tool"][key] = value;
            assert!(run(&meta, &lock, 20 * 86400).is_err());
        }
        for lock in [
            json!({}),
            json!({"lockfileVersion":2}),
            json!({"lockfileVersion":3,"packages":{}}),
        ] {
            assert!(run(&meta, &lock, 20 * 86400).is_err());
        }
        assert!(run("{}", &npm(), 20 * 86400).is_err());
        assert!(run("bad json", &npm(), 20 * 86400).is_err());
    }
    #[test]
    fn dates_validate_calendar_and_full_utc_time() {
        assert_eq!(timestamp("1970-01-01T00:00:00Z"), Ok(0));
        assert_eq!(timestamp("1970-01-02T01:02:03.123+00:00"), Ok(90123));
        assert!(timestamp("2000-02-29T00:00:00Z").is_ok());
        for date in [
            "",
            "garbage",
            "2026-02-29T00:00:00Z",
            "2100-02-29T00:00:00Z",
            "2026-00-01T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-01-00T00:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01T00:60:00Z",
            "2026-01-01T00:00:60Z",
            "2026-01-01X00:00:00Z",
            "2026-01-01T00:00:00+01:00",
            "2026-01-01T00:00:00.badZ",
        ] {
            assert!(timestamp(date).is_err(), "{date}");
        }
        assert_eq!(package_segment("@scope/name"), Ok("@scope%2Fname".into()));
        assert!(package_segment("a?token=b").is_err());
    }
    #[test]
    fn unavailable_or_malformed_registry_response_is_fatal() {
        let meta = metadata(&json!(
            "registry+https://github.com/rust-lang/crates.io-index"
        ));
        for response in [
            Err("network unavailable".into()),
            Ok("bad".into()),
            Ok("{}".into()),
        ] {
            assert!(
                check_at(
                    Path::new("."),
                    &mut |_, _, _| response.clone(),
                    &meta,
                    &npm().to_string(),
                    u64::MAX
                )
                .is_err()
            );
        }
    }
}
