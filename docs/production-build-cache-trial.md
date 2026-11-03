# Production build-cache trial and installation

Observed on 2026-11-03 for [#63](https://github.com/breki/silkboard/issues/63).
The owner authorized isolated ARM64 trials, then merging
[PR #139](https://github.com/breki/silkboard/pull/139) and installing its helper.
The helper is installed; normal release-build and activation acceptance remain
pending. Keep #63 open until that evidence is recorded.

## Inputs and method

Helper source: `e76a4f7dea69417a17996e5842fbde7fba5b1410`, merged unchanged
at `cb1eda419de53ab0d3825da6963f30fb9fde9c1a`. Its commit signature passed
against a copy of the independently installed guest public trust. The installed
baseline matched the SHA-256 in the [operator guide](production-build-cache.md).

The source inputs were signed `v0.10.0` at
`e03650ef0faeb378f9b87486d7cb537ee4f477a2` and `v0.11.0` at
`e220db0babe04c086158c07bee3ed4e49f3837d1`. Their external Rust dependencies
matched; workspace package versions differed. Exact tag-triggered release runs
[40229760272](https://github.com/breki/silkboard/actions/runs/40229760272) and
[40261444811](https://github.com/breki/silkboard/actions/runs/40261444811)
passed all three required jobs. Read-back matched workflow ID `387356305`,
the agreed path, push event, tag, exact commit and completed success. Each
helper independently verified its source tag against installed guest trust.

Trials ran sequentially as `silkbuild` with one Cargo job, clean environments,
the existing toolchain/download homes and isolated source/output directories.
Only the trial helpers' base-directory assignment changed; reversing that
substitution matched the original helper bytes. The updater lock covered each
whole trial command. No trial staged or activated an application or used
production storage. Tools were Rust/Cargo 1.93.1 on
`aarch64-unknown-linux-gnu`, Node 24.14.1 and npm 11.11.0.

## Measurements and correctness

GNU time measured complete successful helper invocations, all exiting zero:

| Build | Wall seconds | User seconds | System seconds | Peak child RSS, KiB |
| --- | ---: | ---: | ---: | ---: |
| Baseline, v0.10.0 | 899.34 | 854.19 | 52.77 | 488516 |
| Proposed cold, v0.10.0 | 813.72 | 769.04 | 52.00 | 483612 |
| Proposed warm, v0.11.0 | 208.65 | 201.01 | 16.08 | 434280 |

The warm invocation took 74.36% less wall time than the proposed cold one.
Cargo's backend compilation durations were 11m27s for baseline and 2m41s for
warm. Registry/download and OS-page caches were not reset. These single trials
do not isolate cache overhead or establish a statistical timing distribution.

Actual Cargo artifact records reported zero fresh and 86 rebuilt external
dependencies in the cold build, then 86 fresh and zero rebuilt in the warm
build. Both `libsqlite3-sys` 0.38.2 and `rusqlite` 0.40.2 were rebuilt cold and
fresh warm. Workspace cleanup removed 248 files / 99.4 MiB before warm
candidate tooling; the current workspace crates rebuilt.

The native ARM64 real-Cargo fixture passed all eight signed-candidate cases:
reuse, workspace freshness, dependency feature/version changes, flags,
compiler-proxy identity and cleared-cache fallback. The proxy delegates to the
same installed compiler; a second real compiler was not tested.

All three outputs passed identity-file and loopback `/api/status` checks for
health, version, tag and commit. Each temporary server used its own frontend
on an unused loopback port without database or owner configuration. Its served
index matched its output file. All owned servers exited afterward.

Across 393 resource samples, minimum available RAM was 1275 MiB, maximum swap
use was 1 MiB and OOM kills stayed zero. Disk remained above the 8 GiB stop
threshold, with about 22 GiB free afterward. The final trial cache occupied
364672 allocated KiB, about 356 MiB, below its 4 GiB retention limit. Its cache
and target directories were `silkbuild`-owned mode 0700; the stable single-link
lock was mode 0600. Actual 4 GiB eviction pressure was not exercised live.

The harness initially queried updater status while holding the same lock;
that failed before starting a build. Status was moved before lock acquisition.
After baseline timing showed the sequence deadline was too short, the partial
cold run was stopped and retained. The successful cold run used a new empty
compiled-output tree and an individual deadline. The fixture initially lacked
explicit builder toolchain-home variables; the corrected environment passed.
These failed or interrupted attempts are retained separately from the three
successful measurements.

Scheduled updater attempts during held trial locks returned failure. After
release, the ordinary 18:30:02 UTC poll succeeded, reporting no newer eligible
release. Pending remained null and the existing `v0.2.1` block was preserved.

## Narrow installation and remaining acceptance

The exact proposed helper SHA-256 is:

```text
90ff80f3f77abc1741f1ceb4d109ebb1633d7369e9673ccf37615bb529f8d84f
```

After merging the CI-green exact PR head, installation held the updater lock,
confirmed no pending candidate or active builder, checked both helper hashes,
and atomically replaced only `/usr/local/libexec/silkboard/build`. The installed
file is root-owned mode 0755. The verified baseline remains root-owned mode
0755 at `/usr/local/libexec/silkboard/build.pre-pr139-e76a4f7` for the
[documented rollback](production-build-cache.md#narrow-helper-rollout-and-recovery).
No trial cache was copied into the production cache.

Before/after checks preserved healthy `v0.13.0` at
`022adb6dbc2b01167e329447fdcf0549a44f0d95`. Hash checks preserved installed
trust, credentials, updater binary and service/timer files. This installation
neither restarted nor activated the application. Private installation records
are under `/var/log/silkboard-helper-pr139-67pWD5`; isolated trial logs and
outputs remain under `/var/lib/silkboard-build/trial-pr139-67pWD5`, about
1.64 GiB including interrupted attempts. Local exported evidence was retained
outside the checkout; no credentials or application messages are recorded here.

The next normal admitted release build must verify the installed helper's
production cache creation, private modes and retained size, exact candidate
identity, protected staging and normal activation/health outcome. A later
compatible release must confirm warm reuse through that installed path.
The isolated cross-tag trials establish real ARM64 dependency reuse; they do
not establish those installed-path outcomes or close #63.
