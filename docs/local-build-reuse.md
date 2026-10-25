# Local Cargo build reuse

Issue [#66](https://github.com/breki/silkboard/issues/66) adds an opt-in local
wrapper. From any Silkboard worktree, replace `cargo` with `./dev/cargo`:

```sh
./dev/cargo test --locked --workspace
./dev/cargo xtask validate --check --parallel
./dev/cargo --arm64 xtask clippy --target aarch64-unknown-linux-gnu
```

The wrapper requires Bash, GNU `stat`, `jq` and util-linux `flock` (present in the
Ubuntu development guest). No installation or shell startup change is needed;
each worktree must contain the wrapper. It uses the same installed Cargo and
toolchain as ordinary commands. Use only trusted local worktrees belonging to
the same operator; build outputs are executable code, not verified downloads.

## Directories and correctness

The default root is `${XDG_CACHE_HOME:-$HOME/.cache}/silkboard-build`. Override
it with an absolute `SILKBOARD_DEV_BUILD_ROOT` for experiments or separate
projects. The root must be an owned nonsymlink directory with mode 0700. Its
`normal`, `coverage` and `arm64` children hold normal host output, instrumented
coverage output and cross-target output respectively. The wrapper sets
`CARGO_TARGET_DIR` and `CARGO_LLVM_COV_TARGET_DIR` for the entire command;
`--arm64` selects a directory, so supply Cargo's target argument too. It does
not select a compiler or execute ARM64 binaries on an x86-64 guest.

On each workspace-path switch, the wrapper cleans every current workspace
package from all existing modes before running the requested command. It also
cleans coverage workspace output and profiles. External dependencies remain.
A single path marker is written only after cleanup succeeds. Cargo can otherwise
reuse a same-version workspace binary from the wrong source path, even under
a lock; runtime root discovery alone does not prevent that stale-code failure.
The shell bootstrap performs cleanup before using xtask, which may itself be
stale. Cargo metadata supplies the workspace package names, avoiding a second
maintained package list.

Cargo still evaluates dependency versions, features, source changes within
one worktree, compiler identity, profiles and build flags. This is primarily
dependency reuse; switching worktrees rebuilds workspace code. Profile or flag
changes can add output or rebuild it; clearing the cache remains valid.
Tests execute on every `cargo test` invocation. Coverage retains its existing
workspace clean, fresh profiles and 90%/85% thresholds. The browser fixture
asks Cargo metadata for the selected output directory. xtask discovers its
workspace from the calling directory at runtime, including direct invocation
of a binary built in another worktree.

A single advisory lock covers the *whole command*, including tests, browser
fixtures and coverage cleanup, across all three directories. A second wrapper
command waits until the first finishes. Cargo's own build lock alone does not
protect a binary used later by a fixture. All users of these directories must
use the wrapper; do not point ordinary Cargo, CI or another account at them.
Do not override target/build directories or instrumented output in Cargo
arguments/configuration while using the wrapper, or redirect its manifest path.
Invoke it from the intended workspace instead. Separate experiments should
use a different root. Avoid recursively invoking the wrapper from a wrapped
command: its lock is deliberately not reentrant.

Long-running `run` commands retain the lock until the application exits. Use
ordinary worktree-local Cargo output for simultaneous application servers or
parallel work that should not wait. Call tools from their intended worktree;
the wrapper does not change directory or choose a worktree on your behalf.
Unrelated workspaces should use a separate cache root.

## Disk use and cleanup

There is no eviction daemon or automatic deletion. Check `du -sh` on the root
and clean during an idle period, using the wrapper so ongoing commands finish
first:

```sh
./dev/cargo clean
./dev/cargo --arm64 clean
./dev/cargo llvm-cov clean --workspace
```

The last command cleans workspace coverage artifacts, preserving eligible
instrumented dependencies. For a completely cold coverage directory, run
`./dev/cargo llvm-cov clean` without `--workspace`. Do not delete the root or
`command.lock` while another command is running or waiting: replacing the
lock file would allow two different locks. To stop using reuse, use ordinary
`cargo` commands again; existing worktree-local artifacts remain separate.
No cache entry replaces validation, current dependency/advisory admission,
release checks or exact production build identity. CI and production cache
policies are unchanged.

The [Cargo build-cache reference](https://doc.rust-lang.org/cargo/reference/build-cache.html)
describes target and intermediate directories; it does not guarantee
workspace-source freshness when sharing them across worktrees.

## Measurements

Measured on 2026-10-25 in the Ubuntu 24.04 x86-64 development guest with two
vCPUs, about 4 GiB RAM, Rust 1.93.1, two Cargo jobs and two Rust test threads.
The registry downloads and tools were already installed; “cold” means empty
compiled-output directories. This was a shared guest, not an isolated runner.
Each condition has one successful sample; concurrent host activity and fixture
runtime vary. These results do not establish CI or production performance.

The worktrees started at base `3b96f89` with this wrapper and runtime xtask
root patch copied in. Full-workspace tests used the existing test profile,
including incremental compilation. GNU time measured wall time, child CPU
time and maximum individual child RSS; RSS is not aggregate concurrent memory.

| Condition | Wall seconds | CPU seconds (user + system) | Max child RSS KiB |
| --- | ---: | ---: | ---: |
| Empty build cache, full workspace tests in A | 156.37 | 247.00 | 446284 |
| Same A, full workspace tests again | 28.39 | 20.19 | 205440 |
| Switch to B, clean workspace and run full tests | 54.11 | 64.72 | 383636 |
| Add B-only test, run its workspace-wide filter | 1.41 | 2.30 | 202696 |
| Return to A, same filter after cleanup | 25.32 | 45.48 | 383672 |
| Separate ARM64 workspace check, cold | 26.84 | 45.56 | 362684 |
| Same ARM64 check, warm | 0.19 | 0.13 | 48804 |

The filter rows are freshness probes, not full-test performance comparisons.
B ran the added test and A did not. The real wrapper test also creates two
same-version crates with different output before the first build, verifies
A → B → A across normal and ARM64 directories, and runs A/B concurrently.
Workspace packages rebuild on a switch; unchanged external dependencies,
including bundled SQLite, were absent from the switched build's compile list.

An initial switch attempt failed in the existing publisher test
`missing_fields_invalid_config_and_delivery_errors_are_quiet`: its local
server reported “Client never connected.” The attempt took 36.04 seconds.
The isolated test passed, followed by the successful fresh A → B switch
reported above. No timeout or assertion was relaxed. Earlier experiments
without workspace cleanup ran stale code and are excluded from performance
claims. Their failure motivated the real round-trip regression.

The [measurement data](measurements/local-build-reuse-2026-10-25.csv) retain
CPU/RSS and cumulative cache-disk readings, including the failed attempt.
After the cold full test run the cache occupied 2395044 KiB; after adding
ARM64, a test-profile variant and a compiler-flag variant it occupied
4595900 KiB. These are cumulative directories, not per-command allocations
or a retention limit. Coverage output is additional.

The profile probe scoped `CARGO_PROFILE_TEST_OPT_LEVEL=1` to the filtered
workspace command; Cargo rebuilt workspace code and passed in 67.43 seconds.
The flag probe scoped `RUSTFLAGS=-Cdebuginfo=0` to `check --locked -p xtask`;
it rebuilt compatible inputs and passed in 6.29 seconds. These verify
configuration invalidation, not a recommended profile change. Release settings
were unchanged. Only Rust 1.93.1 was installed/tested; cross-toolchain reuse
was not experimentally measured. Cargo retains responsibility for compiler
compatibility, and operators can choose a separate cache root for experiments.

The first full-validation attempt passed live dependency admission and Rust
linting, then stopped because the guest's old frontend installation lacked
locked `marked` and `dompurify` packages. `npm ci --prefix frontend` installed
the existing lockfile without changing versions; the isolated frontend gate
then passed. A later attempt exposed a public-permission test fixture affected
by the wrapper's private umask; explicitly setting its intended permissions
fixed the fixture, and its complete tooling gate passed under umask 077.
ARM64 Clippy also passed. The final
`./dev/cargo xtask validate --check --parallel` passed every required gate in
333.46 seconds with warm normal builds and a cold instrumented directory. Rust
line coverage was 97.34% overall and 95.45% for the new workspace helper, with
all per-file thresholds met. Coverage itself took 147.265 seconds. Browser,
configuration/wrapper fixtures and live advisory audits passed. This is
verification, not an A/B full-pipeline performance comparison. After validation
the cumulative cache occupied 5,932,712 KiB, including coverage output.

To repeat the full-test comparison, create two worktrees of this revision with
both source trees present before the first build. From each worktree root,
use the same new private cache and settings:

```sh
export SILKBOARD_DEV_BUILD_ROOT=/tmp/silkboard-build-study
export CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2
/usr/bin/time -f 'wall=%e user=%U system=%S max_rss_kib=%M' \
  ./dev/cargo test --locked --workspace
du -sk "$SILKBOARD_DEV_BUILD_ROOT"
```

Choose an unused absolute cache path, run A cold, A warm, then B. For freshness,
add a uniquely named test only in B and run its filter with `--workspace` in B
then A; B must execute it and A must not. Do not compare these filtered runs
with full-test timings. Keep failed attempts and resource/context differences
in the resulting report. The integrated real-Cargo regression performs the
same-version source round trip automatically during `test:bombyx`.
