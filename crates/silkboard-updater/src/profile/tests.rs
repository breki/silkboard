use super::*;
use serde_json::{Value, json};

#[test]
fn policy_rejects_ambiguous_identity_paths_and_disabled_gates() {
    let original = serde_json::to_value(Policy::silkboard()).unwrap();
    for (key, value) in [
        ("schema", json!(2)),
        ("application", json!("../foreign")),
        ("repository", json!("breki/../silkboard")),
        ("repository", json!("breki/..")),
        ("repository", json!("breki/repo?token=x")),
        ("repository_id", json!(0)),
        ("workflow_id", json!(0)),
        ("workflow_path", json!(".github/workflows/../ci.yml")),
        ("ci_path", json!("other.yml")),
        ("required_jobs", json!([])),
        ("required_jobs", json!(["gate", "gate"])),
        ("required_jobs", json!([" gate"])),
        ("required_jobs", json!(["gate\nname"])),
        ("unknown", Value::Null),
    ] {
        let mut bad = original.clone();
        bad[key] = value;
        assert!(
            Policy::from_json(&serde_json::to_vec(&bad).unwrap()).is_err(),
            "{key}"
        );
    }
    assert!(Policy::from_json(b"malformed").is_err());
    assert_eq!(
        Policy::from_json(&serde_json::to_vec(&original).unwrap()).unwrap(),
        Policy::silkboard()
    );
}

#[test]
fn protected_file_loading_rejects_replacement_links_and_writable_policy() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = std::env::temp_dir()
        .join(format!("deployment-policy-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.join("policy.json");
    let owner = fs::metadata(&root).unwrap().uid();
    assert_eq!(load(&path, owner).unwrap(), Policy::silkboard());
    fs::write(&path, serde_json::to_vec(&Policy::silkboard()).unwrap())
        .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(load(&path, owner).unwrap(), Policy::silkboard());
    assert!(load(&path, owner.wrapping_add(1)).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
    assert!(load(&path, owner).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(load(&path, owner).is_err());
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&path, "bad-json").unwrap();
    assert!(load(&path, owner).is_err());
    fs::remove_file(&path).unwrap();
    symlink("missing", &path).unwrap();
    assert!(load(&path, owner).is_err());
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(load(&path, owner).is_err());
    assert!(load(Path::new("/"), 0).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_missing_file_in_an_insecure_policy_directory_does_not_enable_defaults() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir()
        .join(format!("deployment-policy-unsafe-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o777)).unwrap();
    let owner = fs::metadata(&root).unwrap().uid();
    let result = load(&root.join("missing.json"), owner);
    fs::remove_dir(root).unwrap();
    assert!(result.is_err());
}
