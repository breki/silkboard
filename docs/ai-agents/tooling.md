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
CI cache helpers and their tests require coreutils `sha256sum` and `env`.
The installed [Codex plan publisher](codex-plans.md) uses explicit agent
commands and separately enabled operator configuration; it needs no target
Rust/Node runtime or new hook definition. CI uses compiled CLI/API fixtures;
the installed Codex 0.163.0 identity probe is separate local evidence.
Bundled SQLite requires a C compiler. On Debian/Ubuntu, ARM64 cross-target
lint also needs `gcc-aarch64-linux-gnu` and `libc6-dev-arm64-cross`; CI installs
both explicitly before that gate, including when recommendations are disabled.

Install the external gates before full validation:

- `cargo install --locked cargo-llvm-cov@0.9.0`
- `cargo install --locked code-dupes@0.2.1`
- `cargo install --locked cargo-deny@0.20.2`
- `cargo install --locked cargo-audit@0.22.2`
- Run `npm exec -- playwright install --with-deps chromium` inside `frontend/`
  for the isolated browser suite. The platform dependencies may require sudo.
- OpenSSL for the disposable HTTPS browser fixtures; loopback ports 38124–38131
  must be free (feed, first-owner setup, plans and combined acceptance use
  separate proxy/backend pairs).
  No production credentials are used.

Missing required tools, registry responses, or advisory checks fail validation.
No gate silently skips an absent frontend, coverage tool, or audit database.
Long runs print stage progress; redirect output to a local log when needed.
Validation also emits a `Validation timing:` JSON record per completed gate
(`gate`, integer `elapsed_ms`, and `outcome` pass/fail), including failures
before stopping. The [CI time analysis](../ci-time-analysis.md) records the
measured baseline, interpretation limits and proposed follow-ups.

For opt-in dependency reuse across local worktrees, replace `cargo` with
`./dev/cargo`. The [local build procedure](../local-build-reuse.md) documents
workspace cleanup, locking, separate coverage/ARM64 output and measurements.
The same validation gates still run.

The local [Codex task-title helper](codex-plans.md#native-session-titles) uses
`node dev/codex-title.mjs --task-title TEXT`. Its protocol and CLI checks run
with `npm run test:codex-title --prefix frontend` and the Bombyx gate.

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
| Private owner handoff | `cargo xtask handoff-test` |
| Local build/title helpers and Bombyx configuration/secret/trust tests | `cargo xtask bombyx-test` |
| Signed stable tag/version/checkout checks | `cargo xtask release-check TAG` |
| Relative Markdown links and tooling command claims | `cargo xtask canon-check` |
| Conservative PR validation scope | `cargo xtask ci-mode EVENT BASE HEAD` |
| CI dependency-cache keys | `cargo xtask ci-cache-plan` |
| Verified CI dependency-cache cleanup | `cargo xtask ci-cache-prune RUNNER_TEMP EXPECTED_NORMAL_KEY` |

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
Unavailable required live metadata fails. An explicit optional Rust publication-date
evidence cache reuses immutable facts and reevaluates current policy; npm metadata
and audits remain live. See [validation optimizations](../ci-validation-optimizations.md).
Requests run in batches of at most four, with one npm metadata request per
package across all its locked versions. Each response is verified and discarded
inside its worker. Every worker in a batch finishes before another batch
starts; any fetch, JSON, source, license, integrity or age error stops later
batches. Up to three other requests in that batch may still run. There are no
automatic retries, including for HTTP 429; rerun after the registry's advised
wait. Errors identify the registry URL. Curl retains its 10-second connection
and 30-second request timeouts.
Curl requests compressed responses, and batches interleave the registries so
npm work can proceed during crates.io pacing.
The [crates.io API policy](https://crates.io/data-access#api) requires at most
one request per second. A shared per-invocation pacer spaces crates.io request
starts by at least one second; npm requests use the four-worker bound.
Separate gate invocations do not share a rate limiter. See
[the measurement record](../ci-registry-performance.md) for observed savings
and resource limits.
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
it is installed with verified admission, scheduled upgrades and live
rollback/retry/newer-release evidence. Updater reboot recovery remains pending.
It is separate from xtask. The [in-repository toolkit](../deployment-toolkit.md)
adds a reusable engine, policy evaluator and structured operator interface;
its newer installed-code/state format has not been deployed to production.

## Documentation-only PR checks

The owner authorized issue #22's lighter PR policy. CI retains the single
`Full validation (Linux)` required job, which always reports a result. Its
documentation path runs `canon-check` and `git diff --check` after installing
minimal pinned Rust. It does not install Node/frontend dependencies, gate
tools, LLVM tools or the ARM64 target, and does not claim application tests,
coverage or audits passed. The job summary explicitly records that scope.

`ci-mode` selects documentation only for a `pull_request` event whose exact
checked-out SHA is the expected two-parent merge, with the event base as its
first parent. It reads Git's NUL-delimited raw diff with renames disabled,
including both sides of moves. Only regular non-executable Markdown files
under `docs/`, plus `README.md` and `CHANGELOG.md`, qualify. Paths must use
simple ASCII names without hidden/traversal/empty components. Symlinks,
executable modes, code/workflow/dependency/deployment files, mixed or empty
diffs, invalid identities, malformed output and failed Git probes select
full validation. Agent entry points and executable instruction wrappers are
outside this allowlist.

Classification compilation/command failures also fall back to full; a
subsequent failing gate still fails the job. Documentation check failures
fail the required job rather than being treated as a reason to skip checks.
The classifier and canon checks run repository tooling; this optimization is
not a security sandbox for unreviewed PR code.

Main pushes and all non-PR/release calls always take the full path, regardless
of changed files or classifier output. Release workflow identity and its
exact three successful job requirements are unchanged. A documentation-only
PR result is never release eligibility evidence. The standalone
`validate --check` command continues to run every gate locally.

## Superseded PR runs

CI groups PR runs by the caller's workflow reference and PR number. A newer
run for that same group cancels the older running/pending work. Different PRs
and caller workflow files/refs have separate groups; the `silkboard-ci-`
prefix keeps this policy distinct from caller-level concurrency policies.
Callers must not reuse this internal group namespace for their own policy.

Main pushes and other non-PR events, including release calls, use unique
run-ID/attempt groups and disable cancellation. This also prevents GitHub's
single-pending-run replacement from dropping an unrelated main/release run.
Release jobs and admission requirements remain unchanged. Cancellation does
not make an older commit eligible or replace the latest required check; a
superseding commit still needs its own successful validation.

Manual reruns share the PR's group. Rerun the latest commit: starting an old
attempt can cancel newer work in that group. This policy follows run arrival,
not an independent comparison of commit age.

## CI cache boundaries

The owner authorized CI caching after gate-tool installation took 6m 42s in
run 39044784280. CI restores only four pinned gate executables from a
dedicated temporary tool root. An exact key includes OS, architecture,
Ubuntu/Rust versions and the CI workflow hash, which binds the pinned tool
versions and installation logic. There are no broad restore prefixes. PR
keys include the PR number and are distinct from main/tag keys; GitHub also
scopes PR caches to their merge refs. PRs first try the matching trusted cache
from main, then their own isolated entry. A PR cannot populate the trusted
key used by main or release runs through this workflow.

On a miss, tools are built with `cargo install --locked` using explicit
Rust 1.93.1 outside the candidate checkout. Every run checks their expected
version output before use. The explicit cache save runs before frontend
dependency scripts, application builds and tests. Only the tool `bin/`
directory is saved, excluding Cargo configuration/registry metadata, project
build output and credentials. Version checks detect incompatible entries;
they do not independently authenticate binary contents. Cache trust depends
on GitHub's isolation and the reviewed workflow that creates the entry.

Both CI and the release frontend build use setup-node's npm download cache,
keyed by `frontend/package-lock.json`; `node_modules` is not restored.
`npm ci` still installs from the lockfile and checks package integrity.
Rust registry downloads and Chromium are not cached. Separate
[Rust dependency-build caches](../ci-rust-build-cache.md) retain normal,
instrumented coverage and ARM64 dependencies, with trusted/exact-head PR
namespaces and verified workspace/profile cleanup before publication.
Rust publication dates can use separately verified immutable evidence; npm
registry checks and advisory audits still contact their live sources. No cache
stores passing validation results. ARM64 lint and signed exact-revision release checks remain required.

The [Rust build-cache investigation](../ci-rust-cache-study.md) preserves the
temporary-branch measurements behind the subsequently adopted caches above.

An unavailable or evicted cache can take the normal cold path. A restored
tool-version mismatch fails the job; remove the affected GitHub cache entry
and rerun rather than bypassing the check. Changes to the workflow or pinned
tool/Rust/runner versions invalidate the tool key. A new workflow version
needs an initial cold run. Before main has a matching trusted entry, PRs can
reuse their own entry on later runs; once main populates it, new PRs can
restore that trusted entry directly.

## Pending interfaces and deployment

| Capability | Pending work |
| --- | --- |
| Dependency preflight | `cargo xtask dep-preflight`, registry version selection |
| Records | `cargo xtask records-check`, structured records tooling |
| Changelog | `cargo xtask changelog add` |
| Template maintenance | sync-candidates, feedback-add, backfeed helpers |
| Production delivery | Live pending-check reconsideration and updater reboot recovery |

Version and changelog bookkeeping remain manual under the release workflow.
GitHub release workflow registration and successful run evidence must be
read back after publication; no numeric workflow ID is invented locally.
A release must not be declared eligible unless both trusted tag verification
and the complete release suite succeed for its exact revision.

The Bombyx gate runs native bootstrap helper behavior/privacy tests with
90% line/function and 85% branch coverage. The two thin fixed-error CLI entry
points are excluded from module coverage and exercised through subprocesses;
provisioning shell/platform execution remains a separate live guest check.
