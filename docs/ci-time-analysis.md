# Full CI time analysis — 2026-10-24

Analysis for [#86](https://github.com/breki/silkboard/issues/86), based on six
completed full CI runs. No benchmarking reruns were launched. This is a
small, deliberately selected snapshot of the current workflow, not a random
sample or an optimization experiment.

## What takes the time

A representative recent cache-miss job took **12m06s**. Its largest costs were
normal Rust test compilation/execution (151s), instrumented coverage (156s),
registry/cooldown checking (126s), and browser fixtures/tests (65s). Together
those consume **68.6% of the job**. The full validation step accounts for
598s, or **82.4%**. The queue/dispatch interval before this job started was
21s and is excluded from those percentages.

Warm Rust dependencies help, but the workflow deliberately rebuilds workspace
code and runs the tests in normal and instrumented configurations. Across
this sample, cache-hit jobs took 8m12s–8m50s; cache-miss jobs took
11m25s–13m58s. Even the cache-hit runs spent about 126s on paced registry
requests. Setup or cache uploading is not the main cause of these long runs.

## Sample and environment

All six runs succeeded on the same UTC day, using Ubuntu 24.04/X64,
pinned Rust 1.93.1 and Node 24.14.1. Gate-tool caches hit throughout; none of
these is a completely cold tool-install run. “Hit” means log-confirmed
restoration of all normal, coverage and ARM64 dependency archives. “Miss”
means their trusted keys, and PR fallbacks when applicable, explicitly
reported not found. Successful restore-step status alone was not used.

| Run / selected case | Event | Commit | Rust caches | Key image component | Before job | Job | Full validation |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [40121515147](https://github.com/breki/silkboard/actions/runs/40121515147) PR85 publisher/lock update | PR | `26982ea` | 3 misses | 20261023.334.3 | 21s | 12m06s | 9m58s |
| [40121451441](https://github.com/breki/silkboard/actions/runs/40121451441) main after API merge | main push | `d085d0a` | 3 misses | 20261012.334.1 | 2s | 11m30s | 9m27s |
| [40120086986](https://github.com/breki/silkboard/actions/runs/40120086986) PR84 API/workspace/lock update | PR | `178779c` | 3 misses | 20261023.334.3 | 2s | 11m25s | 9m23s |
| [40117417685](https://github.com/breki/silkboard/actions/runs/40117417685) main after UI reference merge | main push | `35c59e8` | 3 hits | 20261012.334.1 | 2s | 8m12s | 6m35s |
| [40116593219](https://github.com/breki/silkboard/actions/runs/40116593219) PR83 UI reference/HTML | PR | `0c0d0d9` | 3 hits | 20261012.334.1 | 2s | 8m50s | 7m23s |
| [40116144015](https://github.com/breki/silkboard/actions/runs/40116144015) main after contract merge | main push | `86faa66` | 3 misses | 20261023.334.3 | 3s | 13m58s | 11m31s |

The [run data](measurements/ci-full-validation-runs-2026-10-24.csv) retain
full head SHAs and component timings. The cache-key image component is shown
separately from the setup banner: **all six banners say 20261012.334.1**,
while three emitted keys contain 20261023.334.3. Source reads this component
from the runner's ImageVersion environment variable. These logs establish a
namespace difference, not why the banner and key disagree; do not call that
a proven image rollout or silently remove the compatibility boundary.

The two hit jobs have median 8m31s; the four misses median 11m48s. The overall
median is 11m27.5s. There are application/workspace changes and different
runners between some observations, so these are cohort summaries, not a
controlled estimate that caching alone saves their entire difference.
The main contract and main UI-reference runs have identical Cargo/registry,
Rust application/tooling and frontend source inputs (only docs/reference
content changed); they are a useful near-comparison, still confounded by
runner/cache namespace and machine variation.

## Every validation gate

Durations below are seconds from a gate's start marker to the next marker,
or the final success marker. Percentages use the latest job's 726s denominator.
The [gate data](measurements/ci-full-validation-gates-2026-10-24.csv) retain
start/end timestamps. Tiny values are display-rounded, not zero work.

| Gate | Latest seconds | % job | Hit median | Miss median | Observed range |
| --- | --- | --- | --- | --- | --- |
| dep-age-check | 126.2 | 17.4% | 126.3 | 126.4 | 126.2–126.5 |
| canon-check | <0.1 | 0.0% | <0.1 | <0.1 | <0.1–<0.1 |
| fmt --check | 0.2 | 0.0% | 0.2 | 0.2 | 0.2–0.2 |
| dupes | 0.1 | 0.0% | 0.1 | 0.1 | 0.1–0.1 |
| clippy | 34.4 | 4.7% | 4.1 | 32.7 | 3.6–43.3 |
| doc | 2.8 | 0.4% | 2.6 | 2.7 | 2.3–2.9 |
| test | 151.0 | 20.8% | 82.3 | 146.9 | 73.4–177.0 |
| deny | 0.5 | 0.1% | 0.3 | 0.3 | 0.2–0.5 |
| frontend | 16.6 | 2.3% | 19.5 | 15.8 | 14.0–23.3 |
| frontend-coverage | 10.9 | 1.5% | 13.1 | 10.5 | 9.4–15.1 |
| frontend-dupes | 0.1 | 0.0% | 0.2 | 0.1 | 0.1–0.2 |
| browser-test | 64.8 | 8.9% | 52.8 | 63.0 | 50.3–76.5 |
| handoff-test | 2.2 | 0.3% | 2.6 | 2.2 | 2.1–2.9 |
| bombyx-test | 24.3 | 3.3% | 24.6 | 23.9 | 23.3–24.8 |
| coverage | 156.1 | 21.5% | 82.3 | 153.5 | 72.5–190.8 |
| audit | 7.2 | 1.0% | 7.5 | 5.2 | 4.6–9.6 |

The oldest main run batches its bombyx-start and coverage-start messages at
virtually the same collector timestamp. Its handoff+bombyx interval is a
combined 28.289s; the separate attribution is unavailable. Those two median/
range rows exclude that observation (five runs, including three misses).
It is not evidence that VM tests took zero time or were skipped.

## Latest job timeline and overhead

One runner executes these phases sequentially; do not add any overlapping
inner compiler/test durations to their parent gate durations.

| Job component | Seconds | % job |
| --- | --- | --- |
| setup | 30 | 4.1% |
| tool cache restore | 2 | 0.3% |
| rust cache restore | 1 | 0.1% |
| frontend install | 32 | 4.4% |
| dependency fetch | 2 | 0.3% |
| validation | 598 | 82.4% |
| arm64 | 40 | 5.5% |
| cache cleanup | 3 | 0.4% |
| cache save | 14 | 1.9% |
| other | 4 | 0.6% |

```text
UTC       18:37:18    18:38:26                    18:48:24  18:49:04 18:49:24
          | setup68s | full validation598s         | ARM40s | finish20s |
Before job:21s (workflow creation to job start; not included in job duration)
```

Setup68s includes tool selection/restoration, npm/Chromium installation,
fetch and orchestration. Finish20s includes archive cleanup, save and final
job overhead. The detailed component table partitions the same 726s; the
timeline is a coarser grouping, not additional time. The six jobs occupy
66m01s of elapsed runner time in total. This is neither CPU usage nor a
billing calculation. PR and main validation both run; no policy change to
that required work is proposed by this measurement.

## Measured causes

### Registry checking has a deliberate two-minute floor

[Dependency policy](../xtask/src/dependencies.rs) checks 406 locked versions
in these revisions, including 127 Rust registry versions. Crates.io requests
are paced at one start per second. The first starts immediately, making the
spacing floor approximately 126s. npm metadata requests are grouped by package
and interleaved in bounded batches of four; the policy also verifies npm
license/source claims. Every sampled run spent 126.15–126.51s here, including
both cache-hit runs. This is mostly intentional request pacing, not evidence
of slow compilation or an advisory audit. The final advisory audit is a
separate 4.6–9.6s gate in this sample.

### Rust build work dominates normal tests and coverage

The [test profile](../Cargo.toml) has `opt-level = 1`.
[Validation](../xtask/src/lib.rs) runs normal workspace tests, then a separate
instrumented coverage build and tests with unchanged 90%/85% thresholds.
[Cache pruning](../xtask/src/ci_cache.rs) deliberately removes workspace
artifacts and coverage evidence; it retains compatible dependency builds.

| Latest gate | Whole gate | Cargo-reported build phase | Remaining gate time |
| --- | --- | --- | --- |
| Normal test | 151.0s | 133s | about 18.0s |
| Instrumented coverage | 156.1s | 138s | about 18.1s |

These build phases alone total 271s, or 37.3% of the latest job. “Remaining”
includes test execution, Cargo/process overhead, and coverage cleanup/reporting;
it is not a pure test-runtime measurement. Cargo rounds long build durations
in its output. Individual test-suite durations can overlap their internal
parallel tests; do not add them again to the whole gate. In the main cache-hit
run, the corresponding builds still took 59.14s and 58.06s: dependency caching
does not eliminate fresh workspace compilation or either required test run.

Clippy took 34.4s on the latest miss versus 3.6–4.6s on hits. Public/private
documentation together took about 2.3–2.9s. The browser gate also launches
[a real application build](../frontend/e2e/server.mjs): Cargo reported 17.57s
on the latest miss, versus 3.18s on the main hit. That build is already within
the 64.8s browser gate. Playwright's reported 1.1m includes setup; adding it
to the build or gate duration would double-count time.

The frontend check/lint/format/unit-test/build chain took 14.0–23.3s, followed
by a separate 9.4–15.1s coverage run. Logs expose Vitest/Vite durations but
buffer the compound npm output; individual type/lint/format/startup costs
cannot be reliably separated from these records. They are included in the
frontend total, not silently attributed to compilation. More granular npm
instrumentation is a lower priority than the measured Rust/registry costs.
The [browser configuration](../frontend/playwright.config.js) uses one worker
and shared fixtures; simply increasing workers would change isolation needs.

### Cache misses are real; key changes explain lost compatibility

The latest PR explicitly missed all six attempted Rust keys: three trusted
keys and three exact-head PR fallbacks. The earlier API PR did too. Their
registry versions did not change: the new shared workspace crate and later
publisher path dependency changed Cargo manifests/lockfile bookkeeping.
The configured fingerprint hashes those complete files, so known inputs
changed even without adopting another registry version. The image component
also differs between some runs, as noted above.

[Key construction](../xtask/src/ci_cache.rs) combines runner/image metadata,
compiler identity/settings, configuration fingerprint, profile and trust scope.
[The workflow](../.github/workflows/ci.yml) does not restore PR archives into
trusted main builds, and PR fallback keys include the exact head. Those are
intentional trust/freshness constraints. A main cache written under another
fingerprint or image component cannot satisfy an exact key. Logs establish
misses and namespace differences; they do not establish eviction, corruption,
or the full inventory of historical caches.

Verified warm restoration transferred about 342 MiB of compressed Rust archives
in the main hit run, taking 11s. Miss lookups took 1–2s; saving the latest
new archives took 14s, plus 3s pruning. Frontend/Chromium installation took
28–34s; locked Rust fetching took 1–2s. Gate-tool restoration took 0–2s at
Actions' whole-second precision and tool installation was skipped because of
verified hits. Cache/network maintenance is visible but much smaller than
validation. These observations do not measure the cost of a cold gate-tool
installation.

## Ranked follow-ups — proposals, not applied optimizations

| Rank | Investigation or change | Evidence / possible saving | Effort and constraints |
| --- | --- | --- | --- |
| 1 | Reuse verified immutable publication-date evidence safely | The paced registry phase costs about 126s on every run; this is a bound on the opportunity, not a promised saving | Medium. Design trusted evidence/version/policy binding and required npm license/source freshness; still check all locked versions, fail closed on missing required evidence, retain current advisory audits. Do not just raise request rates or drop cooldown checks. |
| 2 | Reduce unnecessary Rust dependency-cache invalidations; diagnose banner/key mismatch | Warm jobs were several minutes faster, but code/runner differences confound an exact saving. Benefit applies to misses, not already-warm runs | Medium/high. Distinguish dependency feature/build compatibility from workspace-only bookkeeping or unrelated validation output. Preserve compiler/flags/profile/target/system compatibility and trusted/PR separation. Do not add broad prefix fallbacks or remove image identity without evidence. Builds still compile workspace code freshly. Related #70 and #31. |
| 3 | Measure cheaper Rust test/coverage compilation profiles | Latest normal+instrumented Cargo build phases total 271s; even the main hit totals 117s. These are opportunity bounds, not forecasts | Medium. Extend the measurement approach of [#68](https://github.com/breki/silkboard/issues/68) to a separately agreed CI experiment. Compare build time plus test runtime and coverage correctness; keep every test and threshold, distinct compatible cache keys, and watch memory. |
| 4 | Measure reuse of a freshly built compatible application within browser fixtures | Browser application build accounts for 3–24s in these runs; savings cannot exceed that measured build portion | Low/medium. Verify binary identity/profile/fixture compatibility within the same job. No cross-run workspace artifact reuse, stale application, weakened authentication tests or shared-port worker races. |
| 5 | Evaluate parallel execution of independent gates after policy prerequisites | Could shorten elapsed critical path; this sample does not measure a safe parallel schedule | High/uncertain. Preserve admission ordering, all required results and fail behavior; avoid build-directory/fixture contention. More runners can duplicate setup/cache transfers and increase runner minutes. Release callers' required job set must remain intact. |

[#24](https://github.com/breki/silkboard/issues/24) and
[#31](https://github.com/breki/silkboard/issues/31) measured/implemented earlier
cache work; their older timings are not substituted for this snapshot.
Local worktree reuse [#66](https://github.com/breki/silkboard/issues/66) and
local sccache [#69](https://github.com/breki/silkboard/issues/69) concern a
different environment. They do not justify reusing CI workspace artifacts.
No required gate, timeout, threshold, cache policy or release rule is changed
by this issue's timing instrumentation and report. The edit to `xtask/src/lib.rs`
is itself an input to the existing conservative fingerprint and can cause a
one-time new cache namespace; this report does not broaden reuse to avoid it.

## Reproduce and interpret the measurements

From an existing private directory outside the checkout, retrieve metadata
and completed logs for each linked run. Replace `RUN_ID` with its table ID
and keep raw logs outside Git and public artifacts:

```sh
gh run view RUN_ID --repo breki/silkboard \
  --json databaseId,headSha,event,createdAt,updatedAt,jobs,url > run.json
gh run view RUN_ID --repo breki/silkboard --log > run.log
```

1. Use the successful Linux job's `startedAt`/`completedAt` for job occupancy;
   subtract workflow creation from job start for the pre-job interval. It
   includes scheduling/dispatch, not a measured CPU or pure queue duration.
2. For successful steps, subtract their metadata start/end timestamps. The
   component CSV groups setup, cache restoration, installation, fetching,
   validation, ARM64, archive cleanup/save, and residual job overhead. Sum
   components to the job total; skipped steps contribute no claimed work.
3. Within validation, find exact `Checking TASK...` markers in the timestamped
   log and subtract consecutive timestamps through `All required gates passed.`
   Verify all 16 expected gates and compare their sum with the parent step;
   these samples agree within 1s, including its launch and timestamp rounding.
   Logs that say `UNKNOWN STEP` still contain the markers; a missing friendly
   step label is not missing execution evidence.
4. Read actual cache-key hit/miss messages, not green step icons. Retain
   normal/instrumented/cross-target distinctions and trust scope. Compare
   Cargo's explicit build duration with the enclosing gate; never treat its
   printed log timestamp as the instant compilation finished. The subprocess
   adapter buffers stdout/stderr until exit, so individual output lines often
   share a timestamp. This also prevents reliable per-inner-command frontend
   timing and per-crate compilation attribution. No CPU/RSS tracing was collected.
5. Keep the oldest run's delayed handoff/bombyx markers combined. Do not turn
   the near-zero collector interval into a claim of skipped tests or faster VM
   execution. Cohorts contain only two hits/four misses and are not a controlled
   A/B benchmark; main and PR jobs ran on different machines.

The added validator record uses a monotonic clock around each gate:

```text
Validation timing: {"elapsed_ms":126246,"gate":"dep-age-check","outcome":"pass"}
```

This is a format example, not output from an additional CI experiment. Records
also appear on failure before validation stops, without including command
arguments, submitted content or error details. They measure the action before
emission and remain useful when collectors delay or batch messages. Existing
commands, order, prerequisites and fail-fast behavior stay unchanged. Tests
exercise successful/failed emission and a real executable failing its first
subprocess without network access. Panic/termination before returning a gate
result can still leave no completion record. Local validation verifies the
new instrumentation; sampled GitHub runs predate it.
