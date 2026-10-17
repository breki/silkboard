# Rust build-cache investigation

Issue [#24](https://github.com/breki/silkboard/issues/24) authorizes research
and measurement, not adoption. The experiment supports a follow-up using
three dependency-build caches. No Rust build cache has been added to normal
CI or release workflows.

## Experiment and evidence

On 2026-10-17, a temporary push-only branch ran the unchanged application
from main `220e3138` on fresh Ubuntu 24.04 GitHub runners with two available
CPUs, Rust 1.93.1 and cargo-llvm-cov 0.9.0. Existing pinned gate tools were
restored read-only. No new action or dependency was introduced.

- [Cold run, attempt 1](https://github.com/breki/silkboard/actions/runs/39071500257/attempts/1):
  revision `a6a56684`, three build-cache misses, then explicit saves.
- [Warm run, attempt 2](https://github.com/breki/silkboard/actions/runs/39071500257/attempts/2):
  identical revision, fresh runner, three exact hits and no saves.
- [Source-edit run](https://github.com/breki/silkboard/actions/runs/39071938643):
  revision `8c083cb8` added a Rust module-doc comment and required exact hits
  on the previous revision's caches. It rebuilt workspace code and ran tests.

The [original experiment workflow](https://github.com/breki/silkboard/blob/a6a56684fd856a4f88e0054fcf07c9a2d1ab033b/.github/workflows/cache-study.yml)
and [source-edit variant](https://github.com/breki/silkboard/blob/8c083cb8db24866c28a3529719fef1512036183c/.github/workflows/cache-study.yml)
record commands and keys. Both use `CARGO_INCREMENTAL=0` and distinct
directories for normal, instrumented coverage and ARM64 builds. The existing
CI does not set that incremental override. Cargo source fetching was outside
the measured compilation steps; Rust registry sources were not cached by
this experiment. Existing gate-tool restore time is part of total job time.

GNU time measured the complete commands and their subprocesses. Normal
checks were `cargo xtask clippy`, `doc` and `test`; coverage used the unchanged
`cargo xtask coverage`; ARM64 used
`cargo xtask clippy --target aarch64-unknown-linux-gnu`.
Both explicit and implicit coverage cleaning
remained enabled. This was compilation research, not full validation: frontend,
live registry policy, advisory and release-admission gates were not run here.

## Results

| Check | Cold seconds | Warm seconds | Source-edit seconds |
| --- | ---: | ---: | ---: |
| Normal lint, docs and tests | 84.12 | 35.66 | 36.00 |
| Coverage, including clean and tests | 68.80 | 30.44 | 32.01 |
| ARM64 lint and its host xtask build | 23.14 | 2.41 | 2.81 |
| Sum of measured commands | 176.06 | 68.51 | 70.82 |
| Entire experimental job | 210 | 105 | 103 |
| Elapsed runner-minutes | 3.50 | 1.75 | 1.72 |

The warm job used 105 fewer runner-seconds (50%), including setup and cache
overhead. Runner-minutes here mean job elapsed time divided by sixty, not
invoiced minutes or a prediction for full CI. All three successful runs
executed tests and reported 96.60% workspace line coverage, retaining the
90% overall and 85% per-file thresholds. Warm logs show workspace recompilation
without recompiling external dependencies.

| Cache | Compressed MiB | Warm restore seconds | Cold save seconds |
| --- | ---: | ---: | ---: |
| Normal dependency builds | 74.05 | 2 | 4 |
| Coverage dependency builds | 55.94 | 2 | 3 |
| ARM64 dependency builds | 42.94 | 3 | 3 |
| Total | 172.93 | 7 | 10 |

Archive sizes come from GitHub's cache API. Step timestamps give restore/save
durations rounded to seconds; they include compression/extraction and service
overhead. The source-edit restores totaled eight seconds. Cleanup took
0.58 seconds cold, 0.48 warm and 0.57 after the source edit. Uncompressed
dependency directories totaled 657,718,541 bytes after the cold cleanup.

| Resource across measured checks | Cold | Warm |
| --- | ---: | ---: |
| User plus system CPU seconds | 339.84 | 126.24 |
| Normal maximum RSS, KiB | 564,820 | 387,724 |
| Coverage maximum RSS, KiB | 569,512 | 366,596 |
| ARM64 maximum RSS, KiB | 481,656 | 294,224 |

CPU sums include child processes and exclude setup/cache actions. RSS is each
command's process high-water mark, not aggregate simultaneous job memory.
There is one sample per condition; runner/network variation is not controlled.
The source edit is deliberately small and does not establish performance for
dependency, feature, toolchain or configuration changes.
The source-edit probe forced prior keys; a proposed driver-code digest would
invalidate on that particular xtask edit. It verifies artifact compatibility,
not proposed key selection.

An [initial setup attempt](https://github.com/breki/silkboard/actions/runs/39071396541)
failed before benchmarks or cache saves because the version probe invoked
cargo-llvm-cov directly without its subcommand argument. The corrected
workflow invokes `cargo llvm-cov --version`. That failed attempt is excluded.

## Reusable work and coverage correctness

Normal lint and tests retain external build dependencies and libraries for
the dev/test profiles. Coverage needs its own instrumented dependencies;
ARM64 needs target artifacts plus host-side procedural macros/build scripts.
These boundaries follow [Cargo's build-cache layout](https://doc.rust-lang.org/cargo/reference/build-cache.html).
They must not share a cache with release application binaries or pinned tools.

In the pinned [cargo-llvm-cov clean implementation](https://github.com/taiki-e/cargo-llvm-cov/blob/v0.9.0/src/clean.rs),
`clean --workspace` removes workspace artifacts and coverage evidence while
preserving external dependency builds. The normal coverage invocation also
cleans workspace artifacts before testing. Both paths ran in every sample;
the experiment did not use `--no-clean` or reuse a coverage verdict.

Before saving, the experiment removed normal Rust docs, cleaned all three
workspace packages in each directory, and removed profraw/profdata evidence.
It checked for remaining workspace binaries/profile files and failed if found.
Only the cleaned dependency directories were archived. Fresh workspace code,
test binaries and coverage evidence were regenerated on both restored runs.

## Proposed follow-up, not adopted policy

Use the existing pinned `actions/cache/restore` and `actions/cache/save`
actions for three dependency-only directories. Keep `CARGO_INCREMENTAL=0`
as measured, explicit coverage cleaning, and workspace-output removal before
saves. Save only after all required validation and ARM64 checks succeed.
Normal builds, coverage and ARM64 each have positive savings after restore
cost; none needs a new cache service or compiler wrapper for this proposal.

Use immutable keys containing a schema epoch, runner OS/host architecture,
Ubuntu image version, exact Rust toolchain, build mode/target, profiles and
incremental setting, coverage-tool version where relevant, and a digest of
Cargo.lock, workspace/package manifests, Cargo configuration, CI workflow and
validation-driver code/flags. Include encoded Rust/Rustdoc flags and target
features. Prefer conservative misses to cross-mode or cross-configuration
fallbacks. Config-identical trusted dependency caches can span source-only
commits; the experiment's exact-revision keys were measurement controls.

Keep trusted and PR namespaces separate. Trusted runs must never consume or
promote PR-produced entries. A PR may restore matching trusted entries, then
its own exact-head/configuration entry on a miss; do not fall back across PR
heads. This retains reuse for ordinary source-only PRs while limiting reuse
of artifacts produced by earlier PR code. Key labels alone do not establish
trust: preserve GitHub's actual ref isolation and verify it in the follow-up.
The [GitHub cache reference](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)
describes immutable entries and ref access. The experiment's three entries
were confined to `refs/heads/issue-24-cache-probe`.

Exclude Cargo home, registry credentials/configuration, Git credentials,
advisory databases, workspace binaries, release outputs, reports and verdicts.
Continue every test, coverage calculation, live registry/advisory check and
exact release tag/version/commit check. Leave release admission and the final
release-backend build fresh and unchanged; no cache hit proves eligibility.

Before rollout, verify the candidate keys miss on toolchain/lock/config/target
changes, PR entries cannot influence trusted runs, and changed release identity
still rebuilds correctly. The experiment did not test those transitions or
the full frontend/audit/release pipeline with the proposed caches. A workflow
edit also invalidates the existing gate-tool cache's workflow hash, so initial
rollout has a separate cold-tool cost not represented by these warm results.

Cache eviction or unavailability must use a cold build, retaining all gates.
Archive storage grows per configuration and PR scope; avoid retaining
duplicate source-revision archives once a dependency configuration is seeded.
The roughly 173 MiB payload and seven-second warm restore support proceeding
to an owner-authorized implementation and full-pipeline verification.

## Cleanup and current state

The temporary remote branch and worktree were removed after preserving the
run data. The three experiment cache entries were deleted; readback showed
none remaining and all nine unrelated caches retained. No experiment workflow
or temporary Rust comment is part of this research change. CI, release job
identity, dependency versions and production configuration remain unchanged.
