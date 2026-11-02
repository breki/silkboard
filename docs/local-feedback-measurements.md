# Local feedback measurements

Issue [#67](https://github.com/breki/silkboard/issues/67), measured
2026-10-26 against source revision
`7f7b6543649600a1900885817989a7bfda4c9821` (v0.12.0). The
[development guide](development.md) uses these existing commands to select
feedback appropriate to an edit. No helper, profile, dependency or quality
threshold was changed.

## Method and limits

The development guest ran Ubuntu 24.04.5 x86-64, Rust 1.93.1 and Node
24.14.1, with two available CPUs and about 3.82 GiB RAM. Rust commands used
two Cargo jobs and two test threads. Frontend commands used `CI=true`,
which selects the existing two-worker Vitest limit. This shared guest was
not an isolated performance runner.

Locked frontend dependencies were installed once before timing. Rust registry
downloads and external compiled dependencies already existed in the private
local cache. The first backend command included the wrapper's workspace-path
switch cleanup; subsequent first crate commands compiled additional inputs.
These are mixed initial samples, not cold builds. Each repeated crate command
had no compilation or cleanup lines in its log. Tests executed on both runs.

Each table entry is one observation, not a statistical distribution or a
guaranteed time. GNU time measured wall time, user/system CPU time and
maximum child RSS in KiB; RSS is not aggregate concurrent memory. Measurements
ran sequentially, with the same test profile and lockfiles. All timed commands
exited zero; no timed samples were discarded. Raw logs and machine-readable
results remain in private local session storage.

## Scope and elapsed time

The crate rows use `./dev/cargo test --locked -p PACKAGE`.

| Crate | Initial sample, s | Repeated sample, s | Passed tests per invocation |
| --- | ---: | ---: | ---: |
| `silkboard-web` | 34.52 | 15.56 | 92 |
| `silkboard-publish` | 48.08 | 16.26 | 60 |
| `silkboard-updater` | 20.60 | 0.25 | 31 |
| `xtask` | 15.02 | 2.19 | 79 |
| `silkboard-plan` | 1.37 | 0.16 | 5 |

After those crate runs, `./dev/cargo test --locked --workspace` passed
267 tests in 44.72 seconds. This invocation also compiled `silkboard-updater`,
`xtask` and `silkboard-plan`; it is not a fully warm workspace baseline.
A workspace test run is only the Rust test portion of full validation.

Focused integration targets ran after the workspace sample:

| Package / target | Wall, s | Passed tests |
| --- | ---: | ---: |
| `silkboard-web` / `status` | 0.24 | 3 |
| `silkboard-publish` / `publishing` | 3.58 | 11 |
| `silkboard-updater` / `policy_evidence` | 0.09 | 2 |
| `xtask` / `ci_mode` | 0.25 | 6 |

These commands add `--test TARGET` to the package command. Their test counts
were checked so a zero-match filter could not be mistaken for passing tests.
The two frontend file samples used `npm test --prefix frontend -- Header.test.js`:
10 tests passed in 5.06 and 2.58 seconds. The full frontend check
(`npm run check:all --prefix frontend`) took 35.83 seconds, including types,
lint, formatting, 290 component tests and a production build.

The scopes exercise different behavior. The small commands are useful while
editing contained code; they do not replace full-crate, cross-component or
full-validation evidence. No hosted CI or production speedup was measured.

## Source-edit and hot-reload probes

A temporary comment was appended to the backend library source, then the
`status` integration target passed three tests in 3.05 seconds, including
compilation. Restoring the exact original bytes and running the same target
passed in 1.05 seconds. Both runs retained their compilation work. This
is a small source-change probe, not a forecast for larger Rust edits.

For real frontend HMR, a worktree-local backend and Vite ran on their default
loopback ports after dependencies and initial assets were prepared. A browser
opened `/status`, observed Healthy, and verified that Vite's `/api/status`
proxy returned HTTP 200 with backend version 0.12.0. A temporary change to
the Header's visible heading appeared after 87.8 ms; restoring it appeared
after 76.8 ms. The browser recorded zero main-frame navigations, and
the backend remained running. These two observations exclude initial build,
backend/Vite startup and browser startup time.

The source bytes were restored and only the probe's own backend/Vite process
groups were stopped. No production state or owner credentials were used. The
HTTP Vite setup does not prove authenticated Secure-cookie flows; those
retain their existing HTTPS browser fixtures.

## Timing records

The following rows retain CPU and memory context for each timed invocation.

| Sample | Wall, s | User CPU, s | System CPU, s | Max child RSS, KiB |
| --- | ---: | ---: | ---: | ---: |
| `silkboard-web-first` | 34.52 | 39.72 | 8.66 | 378964 |
| `silkboard-web-repeat` | 15.56 | 19.67 | 2.96 | 205824 |
| `silkboard-publish-first` | 48.08 | 48.95 | 6.19 | 466184 |
| `silkboard-publish-repeat` | 16.26 | 1.70 | 0.81 | 100364 |
| `silkboard-updater-first` | 20.60 | 33.53 | 3.11 | 316904 |
| `silkboard-updater-repeat` | 0.25 | 0.10 | 0.08 | 87908 |
| `xtask-first` | 15.02 | 19.32 | 3.72 | 276504 |
| `xtask-repeat` | 2.19 | 1.58 | 1.18 | 90964 |
| `silkboard-plan-first` | 1.37 | 1.53 | 0.43 | 171236 |
| `silkboard-plan-repeat` | 0.16 | 0.11 | 0.04 | 84148 |
| `workspace-repeat` | 44.72 | 35.20 | 8.03 | 242624 |
| `frontend-header-first` | 5.06 | 3.16 | 1.01 | 328880 |
| `frontend-header-repeat` | 2.58 | 2.60 | 0.46 | 283416 |
| `frontend-all` | 35.83 | 52.06 | 6.71 | 476364 |
| `silkboard-web-focused` | 0.24 | 0.08 | 0.09 | 45112 |
| `silkboard-publish-focused` | 3.58 | 0.11 | 0.09 | 46752 |
| `silkboard-updater-focused` | 0.09 | 0.03 | 0.04 | 38328 |
| `xtask-focused` | 0.25 | 0.09 | 0.14 | 38068 |
| `backend-comment-edit` | 3.05 | 1.05 | 1.04 | 265096 |
| `backend-restored` | 1.05 | 0.83 | 0.54 | 266116 |

The raw identifier `workspace-repeat` names the workspace sample described
above; it included three compilation steps. The frontend rows' test counts
are recorded in the scope section, rather than the Rust-only count parser.

## Repeating the checks

Use an idle development period, a trusted checkout and the
[local wrapper setup](local-build-reuse.md). Keep resource limits and source
revision fixed. Run the same command twice from the same workspace and retain
both outputs, including failures:

```sh
export CI=true CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2
/usr/bin/time -f 'wall=%e user=%U system=%S max_rss_kib=%M' \
  ./dev/cargo test --locked -p silkboard-web --test status
/usr/bin/time -f 'wall=%e user=%U system=%S max_rss_kib=%M' \
  ./dev/cargo test --locked -p silkboard-web --test status
```

Inspect compilation/cleanup output before describing a sample as warm.
Choose an existing behavior-specific target and check its executed count.
For a source-edit comparison, retain the starting bytes, make a small local
edit, measure the same command, restore the file and verify it again.
For frontend HMR, follow the development guide, make and undo a harmless
visible source edit, and distinguish live updates from a full navigation.

The previous [worktree-reuse study](local-build-reuse.md#measurements) covers
cross-worktree correctness and dependency reuse. Test-profile tuning (#68),
sccache evaluation (#69) and hosted CI setup/download work (#130) remain
separate investigations.
