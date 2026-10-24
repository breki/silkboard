//! Normalized dependency/build compatibility, excluding workspace bookkeeping.
use crate::Runner;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::Path,
};
const REGISTRY: &str = "registry+https://github.com/rust-lang/crates.io-index";
pub(super) type Registry = BTreeMap<(String, String), String>;
fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("missing Cargo metadata field: {key}"))
}
fn document(root: &Path, name: &str) -> Result<toml::Table, String> {
    let mut bytes = Vec::new();
    fs::File::open(root.join(name))
        .map_err(|_| format!("cannot read compatibility input: {name}"))?
        .take(4_194_305)
        .read_to_end(&mut bytes)
        .map_err(|_| format!("cannot read compatibility input: {name}"))?;
    if bytes.len() > 4_194_304 {
        return Err("Cargo compatibility input too large".into());
    }
    std::str::from_utf8(&bytes)
        .map_err(|_| "invalid Cargo compatibility encoding")?
        .parse()
        .map_err(|_| format!("invalid Cargo compatibility TOML: {name}"))
}
/// Immutable registry content identities used by build and date evidence caches.
pub(super) fn registry(root: &Path) -> Result<Registry, String> {
    let lock = document(root, "Cargo.lock")?;
    if !matches!(
        lock.get("version").and_then(toml::Value::as_integer),
        Some(3 | 4)
    ) {
        return Err("unsupported Cargo lock format".into());
    }
    let packages = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or("missing Cargo lock packages")?;
    let mut result = BTreeMap::new();
    for package in packages {
        let package = package.as_table().ok_or("invalid Cargo package")?;
        let source = package
            .get("source")
            .map(|source| source.as_str().ok_or("invalid Cargo source"))
            .transpose()?;
        match source {
            None => continue,
            Some(REGISTRY) => {}
            Some(_) => return Err("unapproved Cargo cache source".into()),
        }
        let get = |key| {
            package
                .get(key)
                .and_then(toml::Value::as_str)
                .filter(|s| !s.is_empty())
                .ok_or("invalid Cargo registry identity")
        };
        let (name, version, checksum) =
            (get("name")?, get("version")?, get("checksum")?);
        if checksum.len() != 64
            || !checksum
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("invalid Cargo registry checksum".into());
        }
        if result
            .insert((name.into(), version.into()), checksum.into())
            .is_some()
        {
            return Err("duplicate Cargo registry identity".into());
        }
    }
    Ok(result)
}
fn strings(value: &Value) -> Result<Vec<String>, String> {
    let mut values = value
        .as_array()
        .ok_or("missing Cargo feature array")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| "invalid Cargo feature".into())
        })
        .collect::<Result<Vec<_>, String>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}
pub(super) fn inputs(
    root: &Path,
    runner: &Runner<'_>,
) -> Result<Value, String> {
    let manifest = document(root, "Cargo.toml")?;
    let registry = registry(root)?;
    let raw = runner(
        "cargo",
        &["metadata", "--locked", "--format-version", "1"],
        root,
    )?;
    if raw.len() > 16_777_216 {
        return Err("Cargo metadata too large".into());
    }
    let metadata: Value =
        serde_json::from_str(&raw).map_err(|_| "invalid Cargo metadata")?;
    let members = strings(&metadata["workspace_members"])?;
    if members.is_empty() {
        return Err("empty Cargo workspace".into());
    }
    let packages = metadata["packages"]
        .as_array()
        .ok_or("missing Cargo packages")?;
    if packages.is_empty() || packages.len() > 4096 {
        return Err("invalid Cargo package count".into());
    }
    let mut known = BTreeSet::new();
    let mut external = BTreeMap::new();
    for package in packages {
        let id = field(package, "id")?;
        if !known.insert(id.to_owned()) {
            return Err("duplicate Cargo package".into());
        }
        match package["source"].as_str() {
            Some(REGISTRY) => {
                let name = field(package, "name")?;
                let version = field(package, "version")?;
                let checksum = registry
                    .get(&(name.into(), version.into()))
                    .ok_or("Cargo metadata/lock mismatch")?;
                external.insert(id.to_owned(),json!({"id":id,"name":name,"version":version,"checksum":checksum}));
            }
            None if members.iter().any(|member| member == id) => {}
            _ => return Err("unapproved Cargo metadata source".into()),
        }
    }
    if members.iter().any(|id| !known.contains(id)) {
        return Err("unknown Cargo workspace member".into());
    }
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("missing Cargo resolution")?;
    let mut seen = BTreeSet::new();
    for node in nodes {
        let id = field(node, "id")?;
        if !known.contains(id) || !seen.insert(id.to_owned()) {
            return Err("inconsistent Cargo resolution".into());
        }
        if let Some(package) = external.get_mut(id) {
            let mut deps = Vec::new();
            for dep in node["deps"]
                .as_array()
                .ok_or("missing Cargo dependencies")?
            {
                if !known.contains(field(dep, "pkg")?) {
                    return Err("unknown Cargo dependency".into());
                }
                deps.push(dep.clone());
            }
            deps.sort_by_key(Value::to_string);
            package["features"] = json!(strings(&node["features"])?);
            package["dependencies"] = json!(deps);
        }
    }
    if known.iter().any(|id| !seen.contains(id)) {
        return Err("missing external Cargo resolution".into());
    }
    // Preserve profile and resolver policy, while ignoring package names,
    // local path edges and workspace membership that do not alter resolution.
    Ok(
        json!({"schema":2,"registry":registry.into_iter().map(|((name,version),checksum)|json!({"name":name,"version":version,"checksum":checksum})).collect::<Vec<_>>(),
        "resolved":external.into_values().collect::<Vec<_>>(),
        "profiles":manifest.get("profile"),"resolver":manifest.get("workspace").and_then(|w|w.get("resolver"))}),
    )
}
