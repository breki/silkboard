# Rust dependency-build caches

The owner authorized [issue #31](https://github.com/breki/silkboard/issues/31)
after the [research in PR #30](https://github.com/breki/silkboard/pull/30).
This configures dependency-build reuse in CI. The research's experimental
job halved its elapsed time; it was not a full-CI benchmark. Implementation
run evidence and remaining verification limits belong in issue #31's PR.

## Contents and boundaries

CI uses three directories beneath the runner's temporary
`silkboard-ci-rust` directory, each with its own archive:

| Directory | Reusable work |
| --- | --- |
| `normal` | Host dev/test dependencies used by lint, docs, tests and browser builds |
| `coverage` | Instrumented host dependencies for fresh coverage execution |
| `arm64` | ARM64 lint dependencies and host build-script/procedural-macro dependencies |

Incremental compilation is disabled, matching the measured experiment.
Pinned gate tools remain in their separate binary-only cache; their source
installation unsets application target directories. Cargo home, registry
downloads/configuration, credentials, advisory databases and the final release
build are outside these archives. Documentation-only PRs do not access them.

The key planner fingerprints the exact `rustc -vV` identity, image OS/version,
runner OS/architecture and configuration digest. That digest covers Cargo.lock,
workspace/package manifests, `.cargo`, toolchain files, the CI workflow and
the three xtask files that own cache planning, cleanup and command execution. Compiler, Rustdoc, profile, target, coverage and native
compiler environment overrides are included. Mode-specific keys separate
host dev/test, instrumented coverage with the pinned tool version, and ARM64.
Key-input metadata is hashed through a private temporary file outside the
archive directories; it is removed on success or failure.

### Configuration inputs

[Issue #70](https://github.com/breki/silkboard/issues/70) narrows the former
blanket `xtask/**` input. The restore plan and pre-save verification use the
same input set:

| Input | Reason to invalidate |
| --- | --- |
| `Cargo.lock`, `**/Cargo.toml` | Resolved dependencies, features, workspace membership and build profiles |
| `.cargo/**`, `rust-toolchain*` | Cargo configuration and compiler/toolchain selection |
| `.github/workflows/ci.yml` | Build commands, environment, tool versions, archive paths and restore/save policy |
| `xtask/src/ci_cache.rs` | Key schema, mode identity, planning and dependency-only cleanup/verification |
| `xtask/src/lib.rs` | Task dispatch, Cargo arguments, coverage cleaning and validation orchestration |
| `xtask/src/main.rs` | Subprocess execution and inherited/overridden build environment |

The planner additionally binds compiler identity, runner/image identity and
compiler-related environment overrides as described above. Embedded tests in
the retained Rust files still invalidate caches; unrelated modules such as
`xtask/src/dependencies.rs` and standalone tooling tests do not. If build or
cache execution moves to another file, add that file to both workflow hash
expressions. Bump the key schema epoch for changed archive semantics.

A security-policy-only edit can therefore reuse compatible dependency builds,
while the changed xtask workspace code is rebuilt and current security checks
still execute. This is dependency reuse, not reuse of policy verdicts. The
workflow change itself causes one initial cold run; this change does not
recover entries written with the previous fingerprint.

Local mutation checks verified that security-policy, PR-classifier, standalone
test and application-source edits leave the selected input digest unchanged;
lockfile, manifest, Cargo configuration, toolchain, workflow and each retained
xtask-file edit change it. These checks inspect the input set, not GitHub
runner ordering or cache transfer performance. Record actual cold/warm CI
timing, hit/miss and archive-size evidence in the implementation PR for #70;
a first run cannot establish the warm-run benefit.

There is no prefix fallback across configurations. Trusted configuration
entries can span application source-only revisions. A PR first tries the
matching trusted key, then its own key bound to its number and exact head.
Changing the head invalidates its local entry. Trusted runs never look up or
promote PR keys; GitHub's ref isolation supplies the access boundary, not the
key's text. Other workflow contexts use cold builds without build-cache access.

## Validation and publication

Every cache hit still runs the complete selected gate suite. Workspace builds,
tests and coverage calculations remain fresh. The coverage command retains
both workspace cleaning and its 90% overall / 85% per-file thresholds.
Registry-age/source/license/integrity checks and advisory audits remain live.

Restores and saves are optional network operations. A restore action failure
discards all partial Rust build directories before validation proceeds cold.
A miss or eviction also builds normally. A save failure does not turn passing
checks into a validation failure. Invalid key metadata or failed content
verification prevents archive publication and fails the configuration check.

An explicit save is eligible only after full validation and ARM64 lint pass,
and only for modes without a matching cache entry. Before saving, the helper
checks that configuration and flags still match the original key and that
archive paths match cleanup paths. It rejects roots overlapping the checkout,
symlinks and non-regular entries. It discovers workspace packages/targets from
locked Cargo metadata, cleans their artifacts and normal Rust docs, runs the
existing coverage workspace clean, and cleans both host and target ARM64
workspace artifacts. It then rejects leftover workspace binaries/fingerprints,
coverage profiles, reports and release outputs. External dependency builds
remain reusable; workspace binaries, reports and passing verdicts are not saved.

The release workflow's admission, required job identities and final fresh
backend build are unchanged. A cache hit cannot admit a release or substitute
for its signed tag, version, exact commit or successful required suite.

## Helpers and recovery

`cargo xtask ci-cache-plan` emits validated GitHub step outputs using the CI
environment and `CACHE_CONFIG_HASH`.
`cargo xtask ci-cache-prune RUNNER_TEMP EXPECTED_NORMAL_KEY`
revalidates that plan and cleans/verifies the three
directories. These are CI helpers, not commands for reusing a developer's
ordinary target directory. They require `sha256sum` and `env` from coreutils.

An initial workflow change also invalidates the pinned-tool cache, so the
first run may include its established cold installation cost. Ordinary source
changes can use main's matching dependency entries after main seeds them;
before then, only reruns of the same PR head can reuse its local entries.
Separate PR heads or configuration changes may temporarily store additional
archives. Monitor storage/eviction rather than broadening restore prefixes.

For a suspected bad entry, delete that exact GitHub cache entry and rerun.
The normal cold path retains every required gate. Bump the key schema epoch
when changing archive semantics; do not work around corruption by weakening
tests, cleaning, audit or release checks.
