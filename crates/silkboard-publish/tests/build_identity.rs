//! Publisher build provenance contracts, independent of installed Git.

#[path = "../build_identity.rs"]
mod build_identity;

use build_identity::identity;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
fn development_is_explicit_with_or_without_source_metadata() {
    assert_eq!(
        identity("1.2.3", None, None).unwrap(),
        "development; commit unknown"
    );
    assert_eq!(
        identity("1.2.3", None, Some(SHA)).unwrap(),
        format!("development; commit {SHA}")
    );
}

#[test]
fn tagged_build_requires_matching_version_and_complete_commit() {
    assert_eq!(
        identity("1.2.3", Some("v1.2.3"), Some(SHA)).unwrap(),
        format!("tag v1.2.3; commit {SHA}")
    );
    for tag in ["", "v1.2.4", "1.2.3", "v1.2.3-rc.1", "v1.2.3\ninjected"] {
        assert!(identity("1.2.3", Some(tag), Some(SHA)).is_err());
    }
    assert!(identity("1.2.3", Some("v1.2.3"), None).is_err());
    for commit in [
        "",
        "abc123",
        "g123456789abcdef0123456789abcdef01234567",
        "0123456789abcdef0123456789abcdef0123456789\n",
    ] {
        assert!(identity("1.2.3", None, Some(commit)).is_err());
        assert!(identity("1.2.3", Some("v1.2.3"), Some(commit)).is_err());
    }
}
