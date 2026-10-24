# Faster full validation

The owner authorized all five ranked proposals from the
[CI timing analysis](ci-time-analysis.md). These changes keep the full
required suite and release admission. Local measurements below establish
component behavior; they do not establish GitHub end-to-end savings.

## Changes

1. Reuse immutable Rust publication timestamps, keyed by registry package,
   version and lockfile checksum. Each invocation reevaluates age and exact
   security exceptions. npm source/license/integrity metadata and advisory
   audits stay live. Missing, invalid or unsafe evidence requires live Rust
   requests. An invalid cache directory is a configuration failure.
2. Normalize the resolved external Cargo graph, features, checksums and root
   build profiles for dependency-build keys. Workspace version bookkeeping
   and unrelated validator changes no longer invalidate compatible external
   builds. Compiler, flags, targets, system/image and trust boundaries remain.
3. Compile workspace tests at optimization level 0 with debug level 1; retain
   optimization level 1 and debug level 2 for third-party test dependencies.
   Development and release profiles retain their previous settings.
4. Have browser fixtures build the application with `--profile test`, reusing
   the compatible binary from normal tests. Cargo still checks freshness and
   builds it when the browser command runs independently.
5. Add `cargo xtask validate --check --parallel`. Dependency admission runs
   first, followed by canon, formatting and duplication. Two bounded lanes
   then run Rust lint/docs/tests alongside dependency policy and frontend
   checks. Both finish before browser, handoff, Bombyx, coverage and audit.
   Failures stop later scheduling and drain in-flight work. CI limits Cargo
   jobs, Rust test threads and frontend workers to two. The serial validation
   command remains available with the same required checks.

Filesystem-heavy fixtures remain outside the Rust-test overlap: an initial
unbounded-thread experiment exceeded existing publisher contention/deadline
limits. No retry budget, deadline, assertion or coverage threshold was relaxed.
`Validation timing:` records individual gates; overlapping durations must not
be summed as wall time. `Validation wall timing:` reports parallel-suite time.

## Local matched profile experiment

A frozen pre-optimization source snapshot plus the admitted parser lockfile
was used for both profiles. Both used two Cargo jobs, two Rust test threads,
no incremental compilation, warm compatible dependencies and newly cleaned
workspace artifacts. Coverage used its separate instrumented directory.
The shared Linux development host had eight CPUs; this was not an isolated
GitHub runner experiment.

| Command phase | Previous profile | Adopted profile |
| --- | ---: | ---: |
| Normal Rust tests, including workspace rebuild | 117.90 s | 43.11 s |
| Coverage, including workspace rebuild | 135.65 s | 46.86 s |
| Browser application build after tests | 6.63 s (dev) | 0.10 s (test) |

Both normal and instrumented suites passed unchanged tests. Line coverage was
97.28% in both trials, including every-file thresholds. GNU time reported
maximum child RSS of about 410/419 MiB for previous normal/coverage phases
and 368/384 MiB for the adopted profile. These are maximum individual child
measurements, not aggregate concurrent peak memory. Cold dependency profile
compilation has an initial cost; these results describe compatible warm builds.

The implemented validator subsequently passed its full read-only parallel
suite, current audits and separate ARM64 lint. Actual Rust line coverage was
97.29%, with every-file thresholds met; new cache-input, date-evidence and
scheduling modules each exceeded 96% line coverage. That first migration run
took 592.8 seconds locally, including cold admission and first compilation of
compatible third-party profile artifacts. It is verification, not a paired
full-pipeline performance comparison.

## Publication-date evidence

CI uses a separate private 0700 temporary directory and a regular evidence
file containing only package/version/checksum/timestamp facts. The parser
bounds file size, entry count and fields, rejects unknown schemas/fields and
duplicates, and never stores an admission verdict. Writes are atomic with
0600 permissions. The parent admission gate owns this optional path;
subprocesses do not inherit `SILKBOARD_RUST_DATES_FILE`.

Trusted and exact-head PR namespaces are separate, with no prefix fallback
or PR promotion. Evidence is saved only after full validation and ARM64 lint,
with a complete-current-lock verification immediately before the optional
save. Deleting an entry restores ordinary live requests. For a local opt-in:

```sh
install -d -m 0700 /tmp/silkboard-publication-evidence
SILKBOARD_RUST_DATES_FILE=/tmp/silkboard-publication-evidence/dates.json \
  cargo xtask dep-age-check
```

The evidence is trusted input to local admission: keep the directory private
and do not supply downloaded or unreviewed facts. Without this explicit path,
local checks continue to request Rust dates. The cache does not replace
registry license/source admission, current exception evaluation or audits.

Two successive checks of the same lockfile passed: cold admission took
136.873 seconds inside validation (405 live requests); the warm standalone
command took 37.19 seconds including 0.09 seconds of Cargo startup, reusing
all 134 Rust dates while issuing 271 live npm requests. Complete archive
verification also passed. Registry response time varies; these local timings
do not predict exact GitHub savings.

## Limits and follow-up evidence

The new build-key schema causes an initial miss. Workflow changes also cause
the existing pinned-tool cache to miss. Main must seed its trusted entries
before later PR heads can reuse them; a PR's own entries apply only to reruns
of that exact head. Record actual GitHub cold/warm totals, hits, transfer cost
and archive sizes in the implementation PR. Do not attribute the local profile
results to a measured full-CI improvement.

The original timing-analysis report remains historical evidence. In
particular, its runner banner/key ImageVersion discrepancy is unresolved;
image compatibility was retained, with allowlisted planner diagnostics added.
