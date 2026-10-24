//! Optional immutable Rust publication evidence, never a cached policy verdict.
use crate::ci_inputs::{self, Registry};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
const LIMIT: u64 = 1_048_576;
static NEXT: AtomicU64 = AtomicU64::new(0);
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    version: String,
    checksum: String,
    created_at: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    schema: u8,
    entries: Vec<Entry>,
}
pub(super) struct Dates {
    path: Option<PathBuf>,
    registry: Registry,
    facts: Mutex<BTreeMap<(String, String), String>>,
}
impl Dates {
    pub(super) fn load(
        root: &Path,
        path: Option<&Path>,
    ) -> Result<Self, String> {
        let Some(path) = path else {
            return Ok(Self {
                path: None,
                registry: BTreeMap::new(),
                facts: Mutex::new(BTreeMap::new()),
            });
        };
        let registry = ci_inputs::registry(root)?;
        let parent = path.parent().ok_or("invalid publication cache path")?;
        let meta = parent
            .symlink_metadata()
            .map_err(|_| "publication cache directory unavailable")?;
        if !path.is_absolute()
            || !meta.is_dir()
            || parent
                .canonicalize()
                .map_err(|_| "publication cache directory unavailable")?
                != parent
        {
            return Err("unsafe publication cache directory".into());
        }
        #[cfg(unix)]
        if meta.permissions().mode() & 0o777 != 0o700 {
            return Err("publication cache directory must be private".into());
        }
        let mut dates = BTreeMap::new();
        let mut writable = true;
        match path.symlink_metadata() {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("publication cache unavailable".into()),
            Ok(meta) => {
                let mut safe = meta.is_file();
                #[cfg(unix)]
                {
                    safe = safe
                        && meta.nlink() == 1
                        && meta.permissions().mode() & 0o022 == 0;
                }
                if safe {
                    if let Ok(found) = Self::read(path, &registry) {
                        dates = found;
                    } else {
                        eprintln!(
                            "Invalid Rust publication cache; live verification required."
                        );
                    }
                } else {
                    eprintln!(
                        "Unsafe Rust publication cache ignored; live verification required."
                    );
                    writable = false;
                }
            }
        }
        Ok(Self {
            path: writable.then(|| path.to_owned()),
            registry,
            facts: Mutex::new(dates),
        })
    }
    fn read(
        path: &Path,
        registry: &Registry,
    ) -> Result<BTreeMap<(String, String), String>, String> {
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|_| "publication cache unreadable")?
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "publication cache unreadable")?;
        if bytes.len() as u64 > LIMIT {
            return Err("publication cache oversized".into());
        }
        let e: Evidence = serde_json::from_slice(&bytes)
            .map_err(|_| "invalid publication cache")?;
        if e.schema != 1 || e.entries.len() > 2048 {
            return Err("unsupported publication cache".into());
        }
        let mut dates = BTreeMap::new();
        for entry in e.entries {
            if entry.name.len() > 256
                || entry.version.len() > 128
                || entry.created_at.len() > 64
            {
                return Err("invalid publication cache field".into());
            }
            crate::dependencies::timestamp(&entry.created_at)?;
            let key = (entry.name, entry.version);
            if registry.get(&key) != Some(&entry.checksum) {
                continue;
            }
            if dates.insert(key, entry.created_at).is_some() {
                return Err("duplicate publication evidence".into());
            }
        }
        Ok(dates)
    }
    pub(super) fn get(
        &self,
        name: &str,
        version: &str,
    ) -> Result<Option<String>, String> {
        Ok(self
            .facts
            .lock()
            .map_err(|_| "publication cache lock unavailable")?
            .get(&(name.into(), version.into()))
            .cloned())
    }
    pub(super) fn remember(
        &self,
        name: &str,
        version: &str,
        date: &str,
    ) -> Result<(), String> {
        crate::dependencies::timestamp(date)?;
        let key = (name.into(), version.into());
        if self.path.is_some() && self.registry.contains_key(&key) {
            self.facts
                .lock()
                .map_err(|_| "publication cache lock unavailable")?
                .insert(key, date.into());
        }
        Ok(())
    }
    pub(super) fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let entries = self
            .facts
            .lock()
            .map_err(|_| "publication cache lock unavailable")?
            .iter()
            .filter_map(|((name, version), date)| {
                self.registry.get(&(name.clone(), version.clone())).map(
                    |checksum| Entry {
                        name: name.clone(),
                        version: version.clone(),
                        checksum: checksum.clone(),
                        created_at: date.clone(),
                    },
                )
            })
            .collect();
        let bytes = serde_json::to_vec(&Evidence { schema: 1, entries })
            .map_err(|_| "publication cache encoding failed")?;
        if bytes.len() as u64 > LIMIT {
            return Err("publication cache oversized".into());
        }
        let parent = path.parent().ok_or("invalid publication cache path")?;
        let temporary = parent.join(format!(
            ".publication-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let persist = || -> Result<(), String> {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o600);
            let mut file = options
                .open(&temporary)
                .map_err(|_| "cannot create publication evidence")?;
            file.write_all(&bytes)
                .and_then(|()| file.sync_all())
                .map_err(|_| "cannot persist publication evidence")?;
            fs::rename(&temporary, path)
                .map_err(|_| "cannot publish publication evidence")?;
            File::open(parent)
                .and_then(|f| f.sync_all())
                .map_err(|_| "cannot sync publication evidence")?;
            Ok(())
        };
        let result = persist();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

/// Validate complete current facts before publishing an optional cache archive.
pub(super) fn verify(root: &Path, path: &Path) -> Result<(), String> {
    let cache = Dates::load(root, Some(path))?;
    if cache.path.as_deref() != Some(path) {
        return Err("unsafe publication evidence archive".into());
    }
    let dates = Dates::read(path, &cache.registry)?;
    if dates.len() != cache.registry.len() {
        return Err("incomplete publication evidence archive".into());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "invalid system time")?
        .as_secs();
    for ((name, version), date) in dates {
        crate::dependencies::verify_cached_rust(&name, &version, &date, now)?;
    }
    println!("Rust publication evidence archive verified.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root =
                std::env::temp_dir().canonicalize().unwrap().join(format!(
                    "silkboard-publication-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
            fs::create_dir(&root).unwrap();
            #[cfg(unix)]
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                .unwrap();
            fs::write(root.join("Cargo.lock"),format!("version=4\n[[package]]\nname=\"external\"\nversion=\"1.0.0\"\nsource=\"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum=\"{}\"\n","a".repeat(64))).unwrap();
            Self(root)
        }
        fn path(&self) -> PathBuf {
            self.0.join("rust-publication-dates.json")
        }
        fn write(&self, bytes: &[u8]) {
            fs::write(self.path(), bytes).unwrap();
            #[cfg(unix)]
            fs::set_permissions(self.path(), fs::Permissions::from_mode(0o600))
                .unwrap();
        }
        fn evidence() -> serde_json::Value {
            serde_json::json!({"schema":1,"entries":[{"name":"external","version":"1.0.0","checksum":"a".repeat(64),"created_at":"2020-01-01T00:00:00Z"}]})
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn live_facts_round_trip_without_caching_verdicts() {
        let f = Fixture::new();
        let cache = Dates::load(&f.0, Some(&f.path())).unwrap();
        assert!(cache.get("external", "1.0.0").unwrap().is_none());
        assert!(cache.remember("external", "1.0.0", "malformed").is_err());
        cache
            .remember("external", "1.0.0", "2020-01-01T00:00:00Z")
            .unwrap();
        cache.save().unwrap();
        assert_eq!(fs::read_dir(&f.0).unwrap().count(), 2);
        verify(&f.0, &f.path()).unwrap();
        let reopened = Dates::load(&f.0, Some(&f.path())).unwrap();
        assert_eq!(
            reopened.get("external", "1.0.0").unwrap().as_deref(),
            Some("2020-01-01T00:00:00Z")
        );
        let date = reopened.get("external", "1.0.0").unwrap().unwrap();
        assert!(
            crate::dependencies::verify_cached_rust(
                "external",
                "1.0.0",
                &date,
                crate::dependencies::timestamp("2020-01-02T00:00:00Z").unwrap()
            )
            .is_err()
        );
        let disabled = Dates::load(&f.0, None).unwrap();
        disabled.remember("external", "1.0.0", &date).unwrap();
        disabled.save().unwrap();
        assert!(disabled.get("external", "1.0.0").unwrap().is_none());
    }
    #[test]
    fn malformed_partial_or_mismatched_facts_require_fresh_evidence() {
        let f = Fixture::new();
        for bytes in [
            b"invalid JSON".to_vec(),
            vec![b' '; usize::try_from(LIMIT).unwrap() + 1],
            br#"{"schema":1,"schema":1,"entries":[]}"#.to_vec(),
        ] {
            f.write(&bytes);
            let cache = Dates::load(&f.0, Some(&f.path())).unwrap();
            assert!(cache.get("external", "1.0.0").unwrap().is_none());
            assert!(verify(&f.0, &f.path()).is_err());
        }
        for case in [
            "version",
            "checksum",
            "date",
            "schema",
            "extra",
            "duplicate",
            "long",
            "count",
        ] {
            let mut data = Fixture::evidence();
            match case {
                "version" => {
                    data["entries"][0]["version"] = serde_json::json!("2.0.0");
                }
                "checksum" => {
                    data["entries"][0]["checksum"] =
                        serde_json::json!("b".repeat(64));
                }
                "date" => {
                    data["entries"][0]["created_at"] = serde_json::json!("bad");
                }
                "schema" => data["schema"] = serde_json::json!(2),
                "extra" => data["verdict"] = serde_json::json!(true),
                "duplicate" => {
                    let entry = data["entries"][0].clone();
                    data["entries"].as_array_mut().unwrap().push(entry);
                }
                "long" => {
                    data["entries"][0]["name"] =
                        serde_json::json!("x".repeat(257));
                }
                _ => {
                    data["entries"] = serde_json::json!(vec![
                        data["entries"][0]
                            .clone();
                        2049
                    ]);
                }
            }
            f.write(&serde_json::to_vec(&data).unwrap());
            let cache = Dates::load(&f.0, Some(&f.path())).unwrap();
            assert!(
                cache.get("external", "1.0.0").unwrap().is_none(),
                "{case}"
            );
            assert!(verify(&f.0, &f.path()).is_err(), "{case}");
        }
    }
    #[cfg(unix)]
    #[test]
    fn unsafe_paths_are_not_read_or_overwritten() {
        use std::os::unix::fs::symlink;
        let f = Fixture::new();
        let outside = f.0.join("preserve");
        fs::write(&outside, b"preserve").unwrap();
        symlink(&outside, f.path()).unwrap();
        let cache = Dates::load(&f.0, Some(&f.path())).unwrap();
        cache
            .remember("external", "1.0.0", "2020-01-01T00:00:00Z")
            .unwrap();
        cache.save().unwrap();
        assert!(verify(&f.0, &f.path()).is_err());
        assert_eq!(fs::read(&outside).unwrap(), b"preserve");
        fs::remove_file(f.path()).unwrap();
        fs::hard_link(&outside, f.path()).unwrap();
        assert!(Dates::load(&f.0, Some(&f.path())).unwrap().path.is_none());
        fs::remove_file(f.path()).unwrap();
        f.write(&serde_json::to_vec(&Fixture::evidence()).unwrap());
        fs::set_permissions(f.path(), fs::Permissions::from_mode(0o666))
            .unwrap();
        assert!(Dates::load(&f.0, Some(&f.path())).unwrap().path.is_none());
        let alias = f.0.join("alias");
        symlink(&f.0, &alias).unwrap();
        assert!(Dates::load(&f.0, Some(&alias.join("facts.json"))).is_err());
        fs::set_permissions(&f.0, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(Dates::load(&f.0, Some(&f.path())).is_err());
        assert!(Dates::load(&f.0, Some(Path::new("relative.json"))).is_err());
    }
}
