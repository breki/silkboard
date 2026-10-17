//! Build identity validation contracts.

#[path = "../build_identity.rs"]
mod build_identity;

use build_identity::validate_identity;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
fn development_may_be_unknown_but_release_requires_identity() {
    assert_eq!(
        validate_identity("0.1.0", false, None, None),
        Ok(("unknown", "unknown"))
    );
    assert!(validate_identity("0.1.0", true, None, None).is_err());
    for release in [false, true] {
        assert_eq!(
            validate_identity("0.1.0", release, Some("v0.1.0"), Some(COMMIT)),
            Ok(("v0.1.0", COMMIT))
        );
        assert!(
            validate_identity("0.1.0", release, None, Some(COMMIT)).is_err()
        );
        assert!(
            validate_identity("0.1.0", release, Some("v0.1.0"), None).is_err()
        );
        for tag in ["", "v0.2.0", "0.1.0", "v0.1.0-rc.1", "v0.1.0\ninjected"] {
            assert!(
                validate_identity("0.1.0", release, Some(tag), Some(COMMIT))
                    .is_err()
            );
        }
        for commit in [
            "",
            "abc123",
            "0123456789abcdef0123456789abcdef0123456g",
            "0123456789abcdef0123456789abcdef012345678",
        ] {
            assert!(
                validate_identity(
                    "0.1.0",
                    release,
                    Some("v0.1.0"),
                    Some(commit)
                )
                .is_err()
            );
        }
    }
}
