//! Capture explicit build identity without runtime Git access.

mod build_identity;

fn main() {
    for name in ["SILKBOARD_RELEASE_TAG", "SILKBOARD_COMMIT"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    println!("cargo:rerun-if-changed=build_identity.rs");
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let release = std::env::var("PROFILE").unwrap() == "release";
    let tag = std::env::var("SILKBOARD_RELEASE_TAG").ok();
    let commit = std::env::var("SILKBOARD_COMMIT").ok();
    let (tag, commit) = build_identity::validate_identity(
        &version,
        release,
        tag.as_deref(),
        commit.as_deref(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    println!("cargo:rustc-env=SILKBOARD_BUILD_TAG={tag}");
    println!("cargo:rustc-env=SILKBOARD_BUILD_COMMIT={commit}");
}
