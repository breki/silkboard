# Local Rust test profile measurements

Issue [#68](https://github.com/breki/silkboard/issues/68), measured on
2026-11-02–03 against source revision
`1c8261daba9115bd3ccec6bb8094e90880172239`. This evaluates workspace
optimization levels 0 and 1 with the existing dependency override fixed.

## Method and scope

The development guest has two available x86-64 CPUs, about 3.82 GiB RAM,
Ubuntu 24.04.5 and Rust/Cargo 1.93.1. Commands ran sequentially with two
Cargo jobs and two Rust test threads. This shared development guest is not
an isolated benchmark machine. The initial profile ran before the alternative;
there is one cold build and one edit sample per profile, rather than an
interleaved statistical trial. Warm compilation, execution and authentication
have three observations per profile. Small timing differences are inconclusive.

Both variants retain workspace `debug = 1`, dependency `opt-level = 1` and
`debug = 2`, incremental compilation, assertions, overflow checks and the
same locked dependencies. `CARGO_PROFILE_TEST_OPT_LEVEL` changes only the
workspace test optimization choice; the package override retains dependency
optimization. No release setting is changed. See Cargo's
[profile documentation](https://doc.rust-lang.org/cargo/reference/profiles.html)
for override precedence and the compile/execution tradeoff.

Registry sources were fetched before measurement; all timed Cargo commands
used `--locked --offline`. A prerequisite attempt failed before compilation
because a locked Windows dependency was absent from the local registry.
That failure was retained separately and is excluded from successful build
timings. A fresh, worktree-local target directory was used for each profile.
These are cold compiled outputs, with downloaded registry sources and warm
OS caches; they do not represent a new machine or network setup.

`cargo test --workspace --no-run` separates compile/link time from subsequent
`cargo test --workspace` runs. Warm tests include Cargo freshness checks,
harness startup, unit/integration execution and doctests, rather than pure
CPU execution. Logs were checked for compilation and nonzero test counts.
The owner-authentication target exercises real Argon2id password hashing and
verification with the existing 64 MiB / three-pass parameters; it is a
CPU/memory-heavy subset, not an added synthetic benchmark. Network/timeouts,
process startup and fixture sleeps elsewhere can dominate whole-suite time.

The edit sample appends one temporary comment to
`crates/silkboard-web/src/lib.rs`, recompiles the workspace tests, then runs
them. It measures source invalidation and relinking without changing test
behavior; it does not forecast a large implementation edit. Original source
bytes were restored and compiled again before removing that variant's outputs.
Only the experiment's target directory was removed between profiles; no shared
cache was used or cleaned.

GNU time records wall/user/system seconds and maximum child RSS in KiB.
Maximum child RSS is not aggregate concurrent memory. Artifact size covers
the whole target tree, including incremental outputs and build scripts, at
the stated point; apparent bytes and allocated KiB are distinct measures.
Raw logs and machine-readable results are retained in private local storage.

## Results and decision

Keep the existing workspace `opt-level = 0` and third-party `opt-level = 1`.
The issue predates commit `0945255554afe68665c80bd7ec70fac3334bd760`, which
already introduced that split. This result records its tradeoff rather than
changing the profile again.

| Measurement | Workspace level 0 | Workspace level 1 |
| --- | ---: | ---: |
| Cold compile/link, s | 132.67 | 171.01 |
| Edit compile/link, s | 12.58 | 9.43 |
| Post-edit test run, s | 43.02 | 39.87 |
| Restored-source compile/link, s | 11.98 | 8.98 |
| Warm compile/link median, s | 0.10 | 0.11 |
| Warm test median, s | 34.13 | 34.72 |
| Owner authentication median, s | 2.01 | 1.95 |
| Cold compile + first test run, s | 168.65 | 204.69 |

Level 1 added 38.34 s (28.9%) to the cold compilation sample. Its warm
whole-suite runs (33.68–37.38 s) overlap level 0 (33.73–35.98 s); the
medians provide no whole-suite execution win. The eight authentication tests
were modestly faster at level 1 (median 1.95 versus 2.01 s), with the
expensive Argon2 dependency optimized in both variants. All eight whole-suite
runs passed the same 278 tests with zero ignored tests; all six focused
authentication runs passed eight tests. Warm and post-edit execution logs
contained no compilation.

The edit result favors level 1: 9.43 versus 12.58 s to compile, and 49.30
versus 55.60 s for compilation plus the following suite run. Restoring the
source also compiled faster at level 1. These are real countervailing results,
not evidence that level 0 wins every local loop. The single comment probe and
variable whole-suite times cannot establish a general edit-performance win.
For the default, retain the smaller cold-build cost and existing behavior;
a project dominated by repeated small edits could justify a broader,
interleaved edit trial before selecting level 1. No release, CI, coverage or
production performance improvement is claimed, and no quality threshold,
test selection or cryptographic parameter was weakened.

## Artifact footprint

| Point | Level | Apparent bytes | Allocated KiB |
| --- | ---: | ---: | ---: |
| cold | 0 | 2900845631 | 2851424 |
| edited | 0 | 3132359200 | 3078268 |
| cold | 1 | 2959038281 | 2905872 |
| edited | 1 | 3206996583 | 3148680 |

The level-1 cold output tree was about 2.0% larger in apparent bytes.
Both profiles used similar peak child memory during cold compilation; these
figures do not establish total guest peak memory.

## Per-command records

| Sample | Wall, s | User CPU, s | System CPU, s | Max child RSS, KiB | Passed tests |
| --- | ---: | ---: | ---: | ---: | ---: |
| `opt0-cold-compile` | 132.67 | 215.30 | 23.30 | 445676 | 0 |
| `opt0-warm-compile-1` | 0.38 | 0.08 | 0.11 | 51192 | 0 |
| `opt0-warm-test-1` | 35.98 | 23.71 | 5.47 | 205824 | 278 |
| `opt0-warm-compile-2` | 0.10 | 0.06 | 0.04 | 51184 | 0 |
| `opt0-warm-test-2` | 33.73 | 23.34 | 5.11 | 205952 | 278 |
| `opt0-warm-compile-3` | 0.08 | 0.05 | 0.02 | 50928 | 0 |
| `opt0-warm-test-3` | 34.13 | 23.48 | 4.93 | 205952 | 278 |
| `opt0-owner-1` | 2.01 | 2.23 | 0.33 | 140732 | 8 |
| `opt0-owner-2` | 2.01 | 2.27 | 0.34 | 140680 | 8 |
| `opt0-owner-3` | 2.10 | 2.28 | 0.36 | 140756 | 8 |
| `opt0-edit-compile` | 12.58 | 7.55 | 5.42 | 395552 | 0 |
| `opt0-edit-test` | 43.02 | 23.43 | 5.04 | 205952 | 278 |
| `opt0-restored-compile` | 11.98 | 7.24 | 5.34 | 396012 | 0 |
| `opt1-cold-compile` | 171.01 | 300.45 | 21.93 | 448520 | 0 |
| `opt1-warm-compile-1` | 0.24 | 0.07 | 0.09 | 51192 | 0 |
| `opt1-warm-test-1` | 33.68 | 22.66 | 4.83 | 204160 | 278 |
| `opt1-warm-compile-2` | 0.11 | 0.06 | 0.04 | 51184 | 0 |
| `opt1-warm-test-2` | 34.72 | 22.44 | 4.62 | 204032 | 278 |
| `opt1-warm-compile-3` | 0.08 | 0.06 | 0.02 | 51324 | 0 |
| `opt1-warm-test-3` | 37.38 | 22.12 | 4.67 | 204288 | 278 |
| `opt1-owner-1` | 1.95 | 2.16 | 0.29 | 138976 | 8 |
| `opt1-owner-2` | 1.90 | 2.14 | 0.28 | 138852 | 8 |
| `opt1-owner-3` | 1.95 | 2.18 | 0.29 | 138968 | 8 |
| `opt1-edit-compile` | 9.43 | 6.57 | 4.22 | 376896 | 0 |
| `opt1-edit-test` | 39.87 | 22.99 | 4.94 | 204288 | 278 |
| `opt1-restored-compile` | 8.98 | 6.63 | 3.95 | 377708 | 0 |

## Reproduction

Use an idle trusted checkout of the recorded revision with GNU time, Git and
Rust 1.93.1. Fetch locked registry sources before timing (`cargo fetch --locked`).
Use an ordinary worktree-local target, not the shared dependency wrapper;
changing its profile would introduce cleanup and lock costs into the trial.
The following commands illustrate one variant; use a fresh private directory
for each level and keep the workspace path and all other settings fixed:

```sh
measurement_dir=$(mktemp -d /tmp/silkboard-profile.XXXXXX)
export CARGO_TARGET_DIR="$measurement_dir/target"
export CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 CI=true
export CARGO_PROFILE_TEST_OPT_LEVEL=0 # Repeat separately with 1.
/usr/bin/time -f 'wall=%e user=%U system=%S max_rss_kib=%M' \
  cargo test --locked --offline --workspace --no-run
/usr/bin/time -f 'wall=%e user=%U system=%S max_rss_kib=%M' \
  cargo test --locked --offline --workspace
```

Unset any external compiler wrappers or Rust flags that alter compilation;
verify no other profile/config override changes the comparison. Repeat the
compile-only and full-test commands three times without cleaning, retaining
logs and checking executed counts. Run the authentication subset three times
with `cargo test --locked --offline -p silkboard-web --test owner`. Append
a temporary comment to the backend library, run compile-only then full tests,
restore its exact original bytes and compile once more. Record `du -sb` and
`du -sk` on the target tree after cold compilation and after the edited run.
Retain failures and prerequisites separately. Remove only the target directory
you created after recording its size; never clean a shared cache for this trial.

This report complements the [focused-feedback measurements](local-feedback-measurements.md)
and [local development guide](development.md). The full validation and release
requirements remain in [quality policy](ai-agents/quality.md).
