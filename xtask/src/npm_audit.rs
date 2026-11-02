//! Exact temporary disposition of one advisory whose CVE was rejected.
use crate::Runner;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

// 2026-11-09T00:00:00Z. No CLI/environment extension or generic ignore list.
const EXPIRES: u64 = 1_794_182_400;
const ADVISORY: &str = "https://github.com/advisories/GHSA-gcqx-w48m-w2h4";
const PLUGIN: &str = "node_modules/@sveltejs/vite-plugin-svelte";
const DEEPMERGE: &str = "node_modules/deepmerge";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Output {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

pub(super) fn check(root: &Path, runner: &Runner<'_>) -> Result<(), String> {
    let output = runner(
        "npm",
        &["audit", "--json", "--audit-level=low"],
        &root.join("frontend"),
    )?;
    let lock = fs::read_to_string(root.join("frontend/package-lock.json"))
        .map_err(|error| format!("npm audit lockfile: {error}"))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    if evaluate(&output, &lock, now)? {
        println!(
            "npm audit: accepted exact GHSA-gcqx-w48m-w2h4 (CVE-2026-93753 rejected) for deepmerge 4.3.1 via Svelte plugin 7.2.0; temporary exception expires 2026-11-09T00:00:00Z (two low propagated entries, not a clean scan)."
        );
    } else {
        println!("npm audit: no findings.");
    }
    Ok(())
}

fn keys(value: &Value, expected: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == expected.len()
            && expected.iter().all(|key| object.contains_key(*key))
    })
}

fn counts(low: u64) -> Value {
    json!({"info":0,"low":low,"moderate":0,"high":0,"critical":0,"total":low})
}

fn approved_findings() -> Value {
    json!({
        "deepmerge": {
            "name":"deepmerge", "severity":"low", "isDirect":false,
            "via":[{
                "source":1_243_505,"name":"deepmerge","dependency":"deepmerge",
                "title":"deepmerge mergeObject() allows prototype poisoning of returned objects",
                "url":ADVISORY,"severity":"low","cwe":["CWE-1321"],
                "cvss":{"score":5.3,"vectorString":"CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:L/A:N"},
                "range":">=4.2.0 <=4.3.1"
            }],
            "effects":["@sveltejs/vite-plugin-svelte"],
            "range":">=4.2.0","nodes":[DEEPMERGE],"fixAvailable":false
        },
        "@sveltejs/vite-plugin-svelte": {
            "name":"@sveltejs/vite-plugin-svelte","severity":"low","isDirect":true,
            "via":["deepmerge"],"effects":[],"range":"*",
            "nodes":[PLUGIN],"fixAvailable":false
        }
    })
}

fn approved_lock(lock: &str) -> Result<(), String> {
    let lock: Value = serde_json::from_str(lock)
        .map_err(|error| format!("npm audit lockfile JSON: {error}"))?;
    let packages = lock["packages"]
        .as_object()
        .ok_or("npm audit requires locked package bindings")?;
    if lock["lockfileVersion"] != 3
        || lock["packages"][DEEPMERGE]["version"] != "4.3.1"
        || lock["packages"][PLUGIN]["version"] != "7.2.0"
        || lock["packages"][PLUGIN]["dependencies"]["deepmerge"] != "^4.3.1"
    {
        return Err(
            "npm advisory exception does not cover these locked versions"
                .into(),
        );
    }
    for (path, package) in packages {
        if !package.is_object() {
            return Err(
                "npm audit requires valid locked package records".into()
            );
        }
        if (path.ends_with("/deepmerge") || package["name"] == "deepmerge")
            && path != DEEPMERGE
        {
            return Err(
                "npm advisory exception refuses additional Deepmerge copies"
                    .into(),
            );
        }
        for field in [
            "dependencies",
            "devDependencies",
            "optionalDependencies",
            "peerDependencies",
        ] {
            if let Some(dependencies) = package.get(field) {
                let dependencies = dependencies
                    .as_object()
                    .ok_or("npm audit requires valid dependency maps")?;
                if dependencies.contains_key("deepmerge") && path != PLUGIN {
                    return Err("npm advisory exception refuses another Deepmerge consumer".into());
                }
            }
        }
    }
    Ok(())
}

fn evaluate(output: &str, lock: &str, now: u64) -> Result<bool, String> {
    if output.len() > 2 * 1024 * 1024 {
        return Err("npm audit response exceeds the verification limit".into());
    }
    let output: Output = serde_json::from_str(output)
        .map_err(|error| format!("npm audit capture JSON: {error}"))?;
    if !matches!(output.exit_code, 0 | 1) || !output.stderr.is_empty() {
        return Err(
            "npm audit command failed or emitted unexpected diagnostics".into(),
        );
    }
    let report: Value = serde_json::from_str(&output.stdout)
        .map_err(|error| format!("npm audit report JSON: {error}"))?;
    let metadata = &report["metadata"];
    let dependencies = &metadata["dependencies"];
    if !keys(
        &report,
        &["auditReportVersion", "vulnerabilities", "metadata"],
    ) || report["auditReportVersion"] != 2
        || !keys(metadata, &["vulnerabilities", "dependencies"])
        || !keys(
            dependencies,
            &["prod", "dev", "optional", "peer", "peerOptional", "total"],
        )
        || !dependencies
            .as_object()
            .is_some_and(|object| object.values().all(Value::is_u64))
    {
        return Err(
            "npm audit returned an unsupported or incomplete report schema"
                .into(),
        );
    }
    if output.exit_code == 0
        && report["vulnerabilities"] == json!({})
        && metadata["vulnerabilities"] == counts(0)
    {
        return Ok(false);
    }
    if output.exit_code != 1
        || report["vulnerabilities"] != approved_findings()
        || metadata["vulnerabilities"] != counts(2)
    {
        return Err("npm audit has findings outside the exact approved Deepmerge exception".into());
    }
    if now >= EXPIRES {
        return Err(
            "Deepmerge audit exception expired at 2026-11-09T00:00:00Z".into(),
        );
    }
    approved_lock(lock)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(report: &Value, status: i32) -> String {
        json!({"exit_code":status,"stdout":report.to_string(),"stderr":""})
            .to_string()
    }

    #[test]
    fn expiry_is_inclusive_and_clean_scans_still_pass_afterwards() {
        let report: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/npm-audit-deepmerge.json"
        ))
        .unwrap();
        let lock = json!({"lockfileVersion":3,"packages":{
            DEEPMERGE:{"version":"4.3.1"},PLUGIN:{"version":"7.2.0","dependencies":{"deepmerge":"^4.3.1"}}
        }}).to_string();
        assert_eq!(evaluate(&output(&report, 1), &lock, EXPIRES - 1), Ok(true));
        for now in [EXPIRES, EXPIRES + 1] {
            assert!(
                evaluate(&output(&report, 1), &lock, now)
                    .unwrap_err()
                    .contains("expired")
            );
        }
        let mut clean = report;
        clean["vulnerabilities"] = json!({});
        clean["metadata"]["vulnerabilities"] = counts(0);
        assert_eq!(evaluate(&output(&clean, 0), &lock, EXPIRES + 1), Ok(false));
    }

    #[test]
    fn malformed_captures_and_oversized_reports_never_pass() {
        for output in [
            "not-json".into(),
            json!({"exit_code":null,"stdout":"{}","stderr":""}).to_string(),
            "x".repeat(2 * 1024 * 1024 + 1),
        ] {
            assert!(evaluate(&output, "{}", EXPIRES - 1).is_err());
        }
        assert!(evaluate(&output(&json!({}), 1), "{}", EXPIRES - 1).is_err());
        let report: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/npm-audit-deepmerge.json"
        ))
        .unwrap();
        for lock in ["invalid", "{}", r#"{"packages":{}}"#] {
            assert!(evaluate(&output(&report, 1), lock, EXPIRES - 1).is_err());
        }
    }
}
