//! Signed annotated tag admission; GitHub workflow evidence is checked by the updater.

use crate::Runner;
use serde_json::Value;
use std::path::Path;

pub(super) fn check(
    root: &Path,
    runner: &mut Runner<'_>,
    tag: &str,
) -> Result<(), String> {
    if !stable_tag(tag) {
        return Err(
            "require stable tag vMAJOR.MINOR.PATCH without leading zeros"
                .into(),
        );
    }
    if !runner(
        "git",
        &["status", "--porcelain", "--untracked-files=all"],
        root,
    )?
    .trim()
    .is_empty()
    {
        return Err("release requires a clean worktree".into());
    }
    let head = runner("git", &["rev-parse", "HEAD"], root)?;
    let reference = format!("refs/tags/{tag}");
    let revision = runner(
        "git",
        &["rev-parse", &format!("{reference}^{{commit}}")],
        root,
    )?;
    if head.trim() != revision.trim() {
        return Err("release tag does not reference HEAD".into());
    }
    let object = runner("git", &["cat-file", "tag", &reference], root)?;
    if !matches_target(&object, tag, head.trim()) {
        return Err("signed tag header must name the requested release and directly reference HEAD".into());
    }
    if !object
        .rsplit_once("\n-----BEGIN ")
        .is_some_and(|(_, signature)| {
            signature.starts_with("SSH SIGNATURE-----\n")
                && signature.ends_with("-----END SSH SIGNATURE-----\n")
        })
    {
        return Err("require an SSH-signed annotated release tag".into());
    }
    let signers = format!(
        "gpg.ssh.allowedSignersFile={}",
        root.join(".github/release-signers").display()
    );
    runner(
        "git",
        &[
            "-c",
            &signers,
            "-c",
            "gpg.ssh.program=ssh-keygen",
            "verify-tag",
            &reference,
        ],
        root,
    )?;
    let metadata = runner(
        "cargo",
        &["metadata", "--locked", "--no-deps", "--format-version", "1"],
        root,
    )?;
    let metadata: Value = serde_json::from_str(&metadata)
        .map_err(|e| format!("release metadata: {e}"))?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("missing packages in release metadata")?;
    versions_match(packages, tag)?;
    println!(
        "Signed release tag {tag} matches application/publisher versions and HEAD."
    );
    Ok(())
}

fn versions_match(packages: &[Value], tag: &str) -> Result<(), String> {
    for name in ["silkboard-web", "silkboard-publish"] {
        let package = packages
            .iter()
            .find(|p| p["name"] == name)
            .ok_or_else(|| format!("missing {name} package"))?;
        if package["version"].as_str() != tag.strip_prefix('v') {
            return Err(format!("release tag differs from {name} version"));
        }
    }
    Ok(())
}

fn matches_target(object: &str, tag: &str, head: &str) -> bool {
    // These fields are covered by the tag signature. Ref aliases are not.
    let mut headers = object.lines();
    headers.next() == Some(format!("object {head}").as_str())
        && headers.next() == Some("type commit")
        && headers.next() == Some(format!("tag {tag}").as_str())
}

fn stable_tag(tag: &str) -> bool {
    let Some(version) = tag.strip_prefix('v') else {
        return false;
    };
    let parts: Vec<_> = version.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_digit())
                && (p.len() == 1 || !p.starts_with('0'))
                && p.parse::<u64>().is_ok()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[allow(clippy::fn_params_excessive_bools)] // Fixture varies independent rejection conditions.
    fn run(
        tag: &str,
        dirty: bool,
        mismatch: bool,
        signed: bool,
        trusted: bool,
        version: &str,
    ) -> Result<(), String> {
        check(
            Path::new("."),
            &mut |program, args, _| {
                match (program,args[0]) {
                ("git","status")=>Ok(if dirty {" M changed"}else{""}.into()),
                ("git","rev-parse")=>Ok(if mismatch && args[1]!="HEAD" {"other"}else{"head"}.into()),
                ("git","cat-file")=>Ok(if signed {"object head\ntype commit\ntag v0.1.0\ntagger Fixture\n\n-----BEGIN SSH SIGNATURE-----\nbody\n-----END SSH SIGNATURE-----\n"}else{"object head\ntype commit\ntag v0.1.0\n\nunsigned"}.into()),
                ("git","-c")=>if trusted { Ok(String::new()) }else{Err("untrusted key".into())},
                ("cargo","metadata")=>Ok(json!({"packages":[{"name":"silkboard-web","version":version},{"name":"silkboard-publish","version":version}]}).to_string()),
                _=>panic!("unexpected {program} {args:?}"),
            }
            },
            tag,
        )
    }
    #[test]
    fn admission_checks_both_shipped_package_versions() {
        for packages in [
            json!([]),
            json!([{"name":"silkboard-web","version":"0.1.0"}]),
            json!([{"name":"silkboard-web","version":"0.1.0"},{"name":"silkboard-publish","version":"0.0.1"}]),
        ] {
            assert!(
                versions_match(packages.as_array().unwrap(), "v0.1.0").is_err()
            );
        }
    }
    #[test]
    fn signed_headers_bind_name_and_direct_commit() {
        assert!(matches_target(
            "object head\ntype commit\ntag v0.1.0\n",
            "v0.1.0",
            "head"
        ));
        for object in [
            "object other\ntype commit\ntag v0.1.0\n",
            "object head\ntype tag\ntag v0.1.0\n",
            "object head\ntype commit\ntag trial\n",
            "tag v0.1.0\nobject head\ntype commit\n",
            "",
        ] {
            assert!(!matches_target(object, "v0.1.0", "head"));
        }
    }
    #[test]
    fn rejects_invalid_stable_tags_before_executing() {
        for tag in [
            "",
            "1.2.3",
            "v01.2.3",
            "v1.2",
            "v1.2.3-rc.1",
            "v1.2.3+meta",
            "v1.2.3\n",
            "v1.2.999999999999999999999999",
        ] {
            assert!(!stable_tag(tag));
            assert!(
                check(
                    Path::new("."),
                    &mut |_, _, _| panic!("must not execute"),
                    tag
                )
                .is_err()
            );
        }
        for tag in ["v0.0.0", "v1.2.3", "v10.20.30"] {
            assert!(stable_tag(tag));
        }
    }
    #[test]
    fn only_clean_exact_version_signed_tag_is_admitted() {
        assert!(run("v0.1.0", false, false, true, true, "0.1.0").is_ok());
        for (dirty, mismatch, signed, trusted, version) in [
            (true, false, true, true, "0.1.0"),
            (false, true, true, true, "0.1.0"),
            (false, false, false, true, "0.1.0"),
            (false, false, true, false, "0.1.0"),
            (false, false, true, true, "0.2.0"),
        ] {
            assert!(
                run("v0.1.0", dirty, mismatch, signed, trusted, version)
                    .is_err()
            );
        }
    }
}
