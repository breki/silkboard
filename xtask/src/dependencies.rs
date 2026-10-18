//! Registry cooldown and npm license/source verification.

use crate::Runner;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::Path,
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
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

// Small batches bound registry load and memory. A failed batch is the last;
// do not retry HTTP 429 or start more requests after unavailable evidence.
const CONCURRENCY: usize = 4;

struct CratePacer(Mutex<Option<Instant>>);

impl CratePacer {
    fn wait(&self, interval: Duration) -> Result<(), String> {
        let mut last = self.0.lock().map_err(|e| e.to_string())?;
        if let Some(previous) = *last {
            std::thread::sleep(interval.saturating_sub(previous.elapsed()));
        }
        *last = Some(Instant::now());
        Ok(())
    }
}

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
    runner: &Runner<'_>,
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
    let npm_claims = parse_npm_claims(npm)?;
    dependencies.extend(
        npm_claims
            .keys()
            .map(|(name, version)| (true, name.clone(), version.clone())),
    );
    let count = dependencies.len();
    let mut requests: BTreeMap<String, Vec<(bool, String, String)>> =
        BTreeMap::new();
    for (npm, name, version) in dependencies {
        let encoded = package_segment(&name)?;
        let url = if npm {
            format!("https://registry.npmjs.org/{encoded}")
        } else {
            format!("https://crates.io/api/v1/crates/{encoded}/{version}")
        };
        requests.entry(url).or_default().push((npm, name, version));
    }
    let (mut crates, mut npm): (VecDeque<_>, VecDeque<_>) = requests
        .into_iter()
        .partition(|(url, _)| url.starts_with("https://crates.io/"));
    let mut requests = Vec::new();
    // Let npm work overlap the paced crates.io requests instead of queuing
    // every npm package behind the complete crates.io phase.
    while let Some(request) = crates.pop_front() {
        requests.push(request);
        requests.extend(npm.drain(..npm.len().min(CONCURRENCY - 1)));
    }
    requests.extend(npm);
    let pacer = CratePacer(Mutex::new(None));
    let mut checked = 0;
    for batch in requests.chunks(CONCURRENCY) {
        let previous = checked;
        let results = std::thread::scope(|scope| {
            let workers: Vec<_> = batch
                .iter()
                .map(|(url, versions)| {
                    let claims = &npm_claims;
                    let pacer = &pacer;
                    scope.spawn(move || {
                        if url.starts_with("https://crates.io/") {
                            pacer.wait(Duration::from_secs(1))?;
                        }
                        let document = fetch(root, runner, url)?;
                        verify_versions(&document, versions, claims, now)
                            .map_err(|error| format!("{url}: {error}"))
                    })
                })
                .collect();
            // Join every worker, even on errors. Return errors in request
            // order rather than whichever worker happened to finish first.
            workers
                .into_iter()
                .map(|worker| {
                    worker.join().unwrap_or_else(|_| {
                        Err("registry worker panicked".into())
                    })
                })
                .collect::<Vec<_>>()
        });
        for result in results {
            checked += result?;
        }
        if checked / 25 != previous / 25 || checked == count {
            eprintln!("Checked dependency ages: {checked}/{count}");
        }
    }
    println!(
        "Dependency ages and npm license/sources passed ({count} versions, {} live requests, concurrency {CONCURRENCY}).",
        requests.len()
    );
    Ok(())
}

fn verify_versions(
    document: &Value,
    versions: &[(bool, String, String)],
    npm_claims: &NpmClaims,
    now: u64,
) -> Result<usize, String> {
    for (npm, name, version) in versions {
        let date = if *npm {
            let claim = npm_claims
                .get(&(name.clone(), version.clone()))
                .ok_or("missing npm lock claim")?;
            verify_npm_claim(
                &document["versions"][version],
                name,
                version,
                claim,
            )?;
            document["time"][version]
                .as_str()
                .ok_or_else(|| {
                    format!("no npm publish date: {name}@{version}")
                })?
                .to_owned()
        } else {
            text(&document["version"], "created_at")?.to_owned()
        };
        let published = timestamp(&date)?;
        // Exact urgent fix for RUSTSEC-2026-0346 through -0349. This
        // exempts only cooldown, never future timestamps or other gates.
        // See docs/dependency-security-exceptions.md; no configurable bypass.
        let rustls_security_fix = !*npm
            && name == "rustls"
            && version == "0.23.46"
            && published == timestamp("2026-10-14T09:54:15.075873Z")?;
        if now
            .checked_sub(published)
            .is_none_or(|age| age < 14 * 86400 && !rustls_security_fix)
        {
            return Err(format!(
                "{name}@{version} published {date}: within 14-day cooldown"
            ));
        }
    }
    Ok(versions.len())
}

type NpmClaims = BTreeMap<(String, String), (String, String, String)>;

fn parse_npm_claims(npm: &str) -> Result<NpmClaims, String> {
    let lock = json(npm)?;
    if lock["lockfileVersion"] != 3 {
        return Err("require npm lockfileVersion 3".into());
    }
    let packages = lock["packages"]
        .as_object()
        .filter(|p| p.len() > 1)
        .ok_or("missing npm packages")?;
    let mut npm_claims = BTreeMap::new();
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
        let integrity = text(package, "integrity")?;
        if !resolved.starts_with("https://registry.npmjs.org/")
            || !integrity.starts_with("sha512-")
        {
            return Err(format!("unapproved npm source/integrity: {path}"));
        }
        let license = text(package, "license")?;
        if !NPM_LICENSES.contains(&license) {
            return Err(format!("unapproved npm license: {path}"));
        }
        let claim = (
            license.to_owned(),
            resolved.to_owned(),
            integrity.to_owned(),
        );
        if let Some(previous) = npm_claims
            .insert((name.to_owned(), version.to_owned()), claim.clone())
            && previous != claim
        {
            return Err(format!(
                "inconsistent npm lock claims: {name}@{version}"
            ));
        }
    }
    Ok(npm_claims)
}

fn verify_npm_claim(
    published: &Value,
    name: &str,
    version: &str,
    (license, resolved, integrity): &(String, String, String),
) -> Result<(), String> {
    let published_license = published["license"]
        .as_str()
        .or_else(|| published["license"]["type"].as_str())
        .ok_or("missing registry npm license")?;
    let tarball = text(&published["dist"], "tarball")?;
    // Older registry records may spell the canonical npm URL with HTTP.
    // Installs still require the same URL over HTTPS, never another host.
    let tarball = tarball
        .strip_prefix("http://registry.npmjs.org/")
        .map_or_else(
            || tarball.to_owned(),
            |path| format!("https://registry.npmjs.org/{path}"),
        );
    if text(published, "name")? != name
        || text(published, "version")? != version
        || !NPM_LICENSES.contains(&published_license)
        || published_license != license
        || &tarball != resolved
        || published["dist"]
            .get("integrity")
            .is_some_and(|value| value.as_str() != Some(integrity))
    {
        return Err(format!(
            "npm lock claims differ from registry metadata: {name}@{version}"
        ));
    }
    // Legacy registry records without SRI still bind to the canonical tarball;
    // npm ci verifies its bytes using the lockfile's required sha512 digest.
    Ok(())
}

fn fetch(root: &Path, runner: &Runner<'_>, url: &str) -> Result<Value, String> {
    let response = runner(
        "curl",
        &[
            "--fail",
            "--silent",
            "--show-error",
            "--compressed",
            "--location",
            "--proto",
            "=https",
            "--connect-timeout",
            "10",
            "--max-time",
            "30",
            "--user-agent",
            "silkboard-dependency-check (https://github.com/breki/silkboard)",
            url,
        ],
        root,
    )
    .map_err(|error| format!("{url}: {error}"))?;
    json(&response).map_err(|error| format!("{url}: {error}"))
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

    fn many_npm() -> Value {
        let mut lock = json!({"lockfileVersion":3,"packages":{"":{}}});
        for i in 0..9 {
            lock["packages"][format!("node_modules/tool-{i}")] = json!({
                "version":"1.0.0", "license":"MIT", "integrity":"sha512-test",
                "resolved":format!("https://registry.npmjs.org/tool-{i}/-/tool-1.0.0.tgz")
            });
        }
        lock
    }

    fn registry_versions(name: &str) -> String {
        let version = json!({"name":name,"version":"1.0.0","license":"MIT","dist":{"tarball":format!("https://registry.npmjs.org/{name}/-/tool-1.0.0.tgz"),"integrity":"sha512-test"}});
        let mut other = version.clone();
        other["version"] = json!("2.0.0");
        json!({"time":{"1.0.0":"2020-01-01T00:00:00Z","2.0.0":"2020-01-01T00:00:00Z"},"versions":{"1.0.0":version,"2.0.0":other}}).to_string()
    }

    #[test]
    fn requests_overlap_with_a_bound_and_deduplicate_npm_versions() {
        use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let requests = AtomicUsize::new(0);
        let meta = metadata(&json!(
            "registry+https://github.com/rust-lang/crates.io-index"
        ));
        let mut lock = many_npm();
        let mut duplicate = lock["packages"]["node_modules/tool-0"].clone();
        duplicate["version"] = json!("2.0.0");
        lock["packages"]["node_modules/parent/node_modules/tool-0"] = duplicate;
        lock["packages"]["node_modules/other/node_modules/tool-0"] =
            lock["packages"]["node_modules/tool-0"].clone();
        check_at(
            Path::new("."),
            &|_, args, _| {
                requests.fetch_add(1, SeqCst);
                let concurrent = active.fetch_add(1, SeqCst) + 1;
                peak.fetch_max(concurrent, SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(30));
                active.fetch_sub(1, SeqCst);
                if args.last().unwrap().contains("crates.io") {
                    Ok(json!({"version":{"created_at":"2020-01-01T00:00:00Z"}})
                        .to_string())
                } else {
                    Ok(registry_versions(
                        args.last().unwrap().rsplit('/').next().unwrap(),
                    ))
                }
            },
            &meta,
            &lock.to_string(),
            u64::MAX,
        )
        .unwrap();
        assert_eq!(requests.load(SeqCst), 10);
        assert!(peak.load(SeqCst) > 1, "registry requests never overlapped");
        assert!(peak.load(SeqCst) <= 4);
    }

    #[test]
    fn failed_batch_is_joined_and_never_retried_or_followed() {
        use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
        let meta = json!({"workspace_members":["local"],"packages":[{"id":"local","name":"local","version":"1.0.0","source":null}]}).to_string();
        for response in [
            Err("HTTP 429 Too Many Requests; Retry-After: 60".into()),
            Err("HTTP 503 Service Unavailable".into()),
            Err("curl: (28) timeout".into()),
            Ok("malformed".into()),
            Ok("{}".into()),
        ] {
            let calls = AtomicUsize::new(0);
            let error = check_at(
                Path::new("."),
                &|_, _, _| {
                    calls.fetch_add(1, SeqCst);
                    response.clone()
                },
                &meta,
                &many_npm().to_string(),
                u64::MAX,
            )
            .unwrap_err();
            assert_eq!(calls.load(SeqCst), 4);
            assert!(error.contains("https://registry.npmjs.org/tool-0"));
            if let Err(reason) = response {
                assert!(error.contains(&reason));
            }
        }
    }

    #[test]
    fn crate_pacing_serializes_grants_and_has_no_catch_up_burst() {
        let pacer = CratePacer(Mutex::new(None));
        let grants = Mutex::new(Vec::new());
        let interval = Duration::from_millis(20);
        std::thread::scope(|scope| {
            for _ in 0..6 {
                scope.spawn(|| {
                    pacer.wait(interval).unwrap();
                    grants.lock().unwrap().push(Instant::now());
                });
            }
        });
        let mut grants = grants.into_inner().unwrap();
        grants.sort();
        for pair in grants.windows(2) {
            assert!(
                pair[1].duration_since(pair[0]) >= Duration::from_millis(15)
            );
        }
        std::thread::sleep(interval * 2);
        pacer.wait(interval).unwrap();
        let start = Instant::now();
        pacer.wait(interval).unwrap();
        assert!(start.elapsed() >= interval);
    }

    #[test]
    fn every_version_in_a_shared_npm_response_is_verified() {
        let name = "@scope/tool";
        let mut claims = parse_npm_claims(&npm().to_string()).unwrap();
        let claim = claims[&(name.into(), "1.0.0".into())].clone();
        claims.insert((name.into(), "2.0.0".into()), claim);
        let versions = vec![
            (true, name.into(), "1.0.0".into()),
            (true, name.into(), "2.0.0".into()),
        ];
        let good = json(&registry_versions(name)).unwrap();
        let now = timestamp("2020-01-15T00:00:00Z").unwrap();
        assert_eq!(verify_versions(&good, &versions, &claims, now), Ok(2));
        for invalid in [
            json!("2020-01-01T00:00:01Z"),
            json!("2020-01-16T00:00:00Z"),
            json!("not a date"),
            Value::Null,
        ] {
            let mut document = good.clone();
            document["time"]["2.0.0"] = invalid;
            assert!(
                verify_versions(&document, &versions, &claims, now).is_err()
            );
        }
        let mut document = good;
        document["versions"]["2.0.0"]["dist"]["integrity"] =
            json!("sha512-other");
        assert!(verify_versions(&document, &versions, &claims, now).is_err());
    }

    #[test]
    fn panicking_worker_cannot_produce_a_successful_verdict() {
        let meta = metadata(&json!(
            "registry+https://github.com/rust-lang/crates.io-index"
        ));
        let error = check_at(
            Path::new("."),
            &|_, _, _| panic!("failed adapter"),
            &meta,
            &npm().to_string(),
            u64::MAX,
        )
        .unwrap_err();
        assert!(error.contains("registry worker panicked"));
    }

    fn metadata(source: &Value) -> String {
        json!({"workspace_members":["workspace"],"packages":[{"id":"external","name":"axum","version":"0.8.9","source":source}]}).to_string()
    }
    fn npm() -> Value {
        json!({"lockfileVersion":3,"packages":{"":{},"node_modules/@scope/tool":{"version":"1.0.0","resolved":"https://registry.npmjs.org/@scope/tool/-/tool-1.0.0.tgz","integrity":"sha512-test","license":"MIT"}}})
    }
    fn run(meta: &str, lock: &Value, age: u64) -> Result<(), String> {
        check_at(
            Path::new("."),
            &|_, args, _| {
                assert_eq!(args[0], "--fail");
                assert!(args.contains(&"--compressed"));
                for pair in [
                    ["--connect-timeout", "10"],
                    ["--max-time", "30"],
                    ["--proto", "=https"],
                ] {
                    assert!(args.windows(2).any(|window| window == pair));
                }
                assert!(!args.iter().any(|arg| arg.starts_with("--retry")));
                if args.last().unwrap().contains("crates.io") {
                    Ok(json!({"version":{"created_at":"2026-01-01T00:00:00Z"}})
                        .to_string())
                } else {
                    assert!(args.last().unwrap().contains("@scope%2Ftool"));
                    Ok(json!({"time":{"1.0.0":"2026-01-01T00:00:00Z"},"versions":{"1.0.0":{"name":"@scope/tool","version":"1.0.0","license":"MIT","dist":{"tarball":"https://registry.npmjs.org/@scope/tool/-/tool-1.0.0.tgz","integrity":"sha512-test"}}}})
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
    fn only_the_exact_rustls_security_fix_can_bypass_cooldown() {
        let date = "2026-10-14T09:54:15.075873Z";
        let published = timestamp(date).unwrap();
        let document = json!({"version":{"created_at":date}});
        let check = |name: &str, version: &str, now| {
            verify_versions(
                &document,
                &[(false, name.into(), version.into())],
                &NpmClaims::new(),
                now,
            )
        };
        assert!(check("rustls", "0.23.46", published + 4 * 86400).is_ok());
        for (name, version) in [
            ("rustls", "0.23.45"),
            ("rustls", "0.23.47"),
            ("other", "0.23.46"),
        ] {
            assert!(
                check(name, version, published + 4 * 86400)
                    .unwrap_err()
                    .contains("cooldown")
            );
        }
        assert!(check("rustls", "0.23.46", published - 1).is_err());
        let changed_date =
            json!({"version":{"created_at":"2026-10-15T09:54:15.075873Z"}});
        assert!(
            verify_versions(
                &changed_date,
                &[(false, "rustls".into(), "0.23.46".into())],
                &NpmClaims::new(),
                published + 4 * 86400
            )
            .is_err()
        );
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
    fn lock_claims_must_match_authoritative_registry_metadata() {
        let metadata = json!({"workspace_members":["workspace"],"packages":[{"id":"workspace","name":"xtask","version":"0.1.0","source":null}]}).to_string();
        let good = json!({"name":"@scope/tool","version":"1.0.0","license":"MIT","dist":{"tarball":"https://registry.npmjs.org/@scope/tool/-/tool-1.0.0.tgz","integrity":"sha512-test"}});
        for (field, value) in [
            ("license", json!("GPL-3.0-only")),
            ("name", json!("another-package")),
            ("version", json!("2.0.0")),
            (
                "dist",
                json!({"tarball":"https://registry.npmjs.org/another/-/another-1.0.0.tgz","integrity":"sha512-test"}),
            ),
            (
                "dist",
                json!({"tarball":"https://registry.npmjs.org/@scope/tool/-/tool-1.0.0.tgz","integrity":"sha512-different"}),
            ),
        ] {
            let mut entry = good.clone();
            entry[field] = value;
            let response = json!({"time":{"1.0.0":"2026-01-01T00:00:00Z"},"versions":{"1.0.0":entry}}).to_string();
            assert!(
                check_at(
                    Path::new("."),
                    &|_, _, _| Ok(response.clone()),
                    &metadata,
                    &npm().to_string(),
                    u64::MAX
                )
                .is_err(),
                "registry {field} mismatch was accepted"
            );
        }
    }
    #[test]
    fn registry_binding_handles_legacy_metadata_and_conflicting_lock_entries() {
        let claim = (
            "MIT".into(),
            "https://registry.npmjs.org/tool/-/tool-1.0.0.tgz".into(),
            "sha512-test".into(),
        );
        let legacy = json!({"name":"tool","version":"1.0.0","license":{"type":"MIT"},"dist":{"tarball":"http://registry.npmjs.org/tool/-/tool-1.0.0.tgz"}});
        assert!(verify_npm_claim(&legacy, "tool", "1.0.0", &claim).is_ok());
        assert!(verify_npm_claim(&json!({}), "tool", "1.0.0", &claim).is_err());
        let mut lock = npm();
        let mut other = lock["packages"]["node_modules/@scope/tool"].clone();
        other["resolved"] =
            json!("https://registry.npmjs.org/wrong/-/wrong-1.0.0.tgz");
        lock["packages"]["node_modules/parent/node_modules/@scope/tool"] =
            other;
        assert!(
            run(
                &metadata(&json!(
                    "registry+https://github.com/rust-lang/crates.io-index"
                )),
                &lock,
                20 * 86400
            )
            .unwrap_err()
            .contains("inconsistent npm lock claims")
        );
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
                    &|_, _, _| response.clone(),
                    &meta,
                    &npm().to_string(),
                    u64::MAX
                )
                .is_err()
            );
        }
    }
}
