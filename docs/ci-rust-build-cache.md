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
xtask source/tests. Compiler, Rustdoc, profile, target, coverage and native
compiler environment overrides are included. Mode-specific keys separate
host dev/test, instrumented coverage with the pinned tool version, and ARM64.
Key-input metadata is hashed through a private temporary file outside the
archive directories; it is removed on success or failure.

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
