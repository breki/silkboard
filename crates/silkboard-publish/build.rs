//! Embed explicit provenance; the installed CLI never needs Git.

mod build_identity;

fn main() {
    for name in ["SILKBOARD_RELEASE_TAG", "SILKBOARD_COMMIT"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    println!("cargo:rerun-if-changed=build_identity.rs");
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let tag = std::env::var("SILKBOARD_RELEASE_TAG").ok();
    let commit = std::env::var("SILKBOARD_COMMIT").ok();
    let identity =
        build_identity::identity(&version, tag.as_deref(), commit.as_deref())
            .unwrap_or_else(|error| panic!("{error}"));
    println!("cargo:rustc-env=SILKBOARD_PUBLISH_IDENTITY={identity}");
}
