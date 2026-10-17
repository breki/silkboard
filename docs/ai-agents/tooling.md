# Tooling status and prerequisites

The workspace contains the Axum status application, shared xtask gates, and
Stop hook tests. The Svelte/Vite frontend builds and tests locally. CI and
release workflow definitions are present; a definition is not evidence of a
successful GitHub run or a working production deployment.

## Available commands

Run from the repository root. Rust edition 2024 requires the Rust toolchain;
CI pins Rust 1.93.1 with rustfmt, Clippy, and llvm-tools-preview. Node 24.14.1
is pinned in `.mise.toml` and CI. Install frontend dependencies with
`npm ci --prefix frontend`. The hook tests additionally require Bash, Git,
and jq; signature tests require ssh-keygen. Registry checks require curl.

Install the external gates before full validation:

- `cargo install --locked cargo-llvm-cov@0.9.0`
- `cargo install --locked code-dupes@0.2.1`
- `cargo install --locked cargo-deny@0.20.2`
- `cargo install --locked cargo-audit@0.22.2`
- Run `npm exec -- playwright install --with-deps chromium` inside `frontend/`
  for the isolated browser suite. The platform dependencies may require sudo.

Missing required tools, registry responses, or advisory checks fail validation.
No gate silently skips an absent frontend, coverage tool, or audit database.
Long runs print stage progress; redirect output to a local log when needed.

| Capability | Entry point |
| --- | --- |
| Full read-only validation | `cargo xtask validate --check` |
| Rust type check, all targets | `cargo xtask check` |
| Rust formatting | `cargo xtask fmt --check` |
| Rust lint, denied warnings | `cargo xtask clippy` |
| Cross-target lint | `cargo xtask clippy --target TARGET` |
| Public and private Rust docs | `cargo xtask doc` |
| All workspace tests | `cargo xtask test` |
| Filtered workspace tests | `cargo xtask test FILTER` |
| Rust coverage, 90% overall / 85% per file | `cargo xtask coverage` |
| Rust duplication, at most 6% | `cargo xtask dupes` |
| Cargo license/source policy | `cargo xtask deny` |
| Current Rust and npm advisories | `cargo xtask audit` |
| All locked versions' cooldown and npm license/source policy | `cargo xtask dep-age-check` |
| Frontend types, lint, format, tests, build | `cargo xtask frontend` |
| Frontend types only | `cargo xtask frontend-check` |
| Frontend formatting | `cargo xtask frontend-fmt --check` |
| Frontend component tests | `cargo xtask frontend-test` |
| Frontend coverage | `cargo xtask frontend-coverage` |
| Frontend duplication | `cargo xtask frontend-dupes` |
| Isolated Chromium tests | `cargo xtask browser-test` |
| Signed stable tag/version/checkout checks | `cargo xtask release-check TAG` |
| Relative Markdown links and tooling command claims | `cargo xtask canon-check` |

`cargo xtask --help` documents supported arguments. `validate` is also
read-only without `--check`. Only standalone `fmt` and `frontend-fmt` rewrite
files when run without `--check`. Tasks reject unsupported arguments and stop
on failure. Cargo stages use locked resolution and run from the workspace
root, including when invoked from a subdirectory. Rustdoc denies warnings
for both visibility scopes and preserves caller flags.

Coverage includes application and xtask library logic. The Rust gate excludes
only `src/main.rs` entry points: subprocess/listener/signal I/O is kept there,
with behavior checked by hook, signature, and browser integration tests. The
frontend coverage gate excludes only the DOM mount adapter and declaration
file, with the mount path exercised by Chromium. Thresholds are beside the
implementation in [xtask](../../xtask/src/lib.rs) and
[Vitest configuration](../../frontend/vitest.config.js). Duplication scans
production Rust in both crates and frontend source, excluding tests.

`dep-age-check` verifies every locked registry version, including optional
and transitive versions, against current registry publication timestamps.
It enforces fourteen full days and has no environment bypass. It also rejects
non-workspace Cargo path dependencies, non-crates.io sources, non-npm npm
sources, missing sha512 integrity, and npm licenses outside the explicit
list in [the dependency gate](../../xtask/src/dependencies.rs). npm package
name, version, canonical tarball URL, and license must match registry metadata;
registry integrity must also match when published. Legacy registry records
without integrity still bind to the canonical tarball, whose bytes npm ci
verifies against the required lockfile sha512 digest. Package aliases are not
supported. Cargo license
expressions and sources are checked by [cargo-deny](../../deny.toml).
Unavailable registries fail; there is no offline success claim or date cache.
Check prospective dependency publication dates before compiling newly adopted
code; the gate runner itself must be built to check an updated workspace.

`canon-check` checks inline relative Markdown link targets in `docs/`, both
agent wrapper trees, and root entry points. It checks implemented inline
`cargo xtask` command claims in this document before the Pending heading.
URL/anchor links and the Pending section's future commands are exempt. It
does not validate anchors, arbitrary inline paths, reference-style links,
shell syntax, or whether prose describes a correct product policy.

The [Git-status hook](git-status-hook.md) is informational. The
[Stop hook](stop-check-hook.md) calls formatting, Clippy, documentation,
and tests for changed Rust; it does not run full release validation.

See [local development](../development.md) for application commands and
[release preparation](../releases.md) for signing and workflow evidence.
The [VM bootstrap](../../deploy/vm/README.md) provides operator scripts for
the verified initial production deployment, separate from xtask and the app.
The standalone [updater](../automatic-upgrades.md) has local behavioral checks;
it is installed with verified admission and scheduled polling. Live upgrade
and recovery evidence remain pending. It is not
an xtask deployment command. The [in-repository toolkit](../deployment-toolkit.md)
adds a reusable engine, policy evaluator and structured operator interface;
its newer installed-code/state format has not been deployed to production.

## Pending interfaces and deployment

| Capability | Pending work |
| --- | --- |
| Dependency preflight | `cargo xtask dep-preflight`, registry version selection |
| Records | `cargo xtask records-check`, structured records tooling |
| Changelog | `cargo xtask changelog add` |
| Template maintenance | sync-candidates, feedback-add, backfeed helpers |
| Production delivery | Verify live automatic upgrades and recovery |

Version and changelog bookkeeping remain manual under the release workflow.
GitHub release workflow registration and successful run evidence must be
read back after publication; no numeric workflow ID is invented locally.
A release must not be declared eligible unless both trusted tag verification
and the complete release suite succeed for its exact revision.
