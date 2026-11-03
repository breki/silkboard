# Production dependency-build reuse

Implementation for [#63](https://github.com/breki/silkboard/issues/63).
The owner subsequently authorized production-VM trials and narrow helper
installation. The [trial and installation record](production-build-cache-trial.md)
reports ARM64 measurements and the installed helper's exact identity. A normal
release build with the installed helper and rollout acceptance remain pending.
An application tag/release does not install this separately managed helper.

## Cache and identity contract

The reviewed [guest build helper](../deploy/vm/guest-build.sh) keeps the existing
`silkbuild` account, one Cargo job, sequential frontend/backend work, fixed VM
memory budget, independent installed tag trust and per-tag source/output trees.
The downloader, staging, activation, health checks and recovery are unchanged.
No application/publisher account gains cache access or build authority.

`/var/lib/silkboard-build/compile-cache-v1` is owned by `silkbuild`, mode 0700.
Its stable mode-0600 `build.lock` covers the whole helper invocation, including
source verification, compilation, output copy and cleanup. All cache users and
manual cleanup must take that lock; Cargo's shorter internal lock is insufficient.
Existing symlink/public cache directories and linked/nonprivate locks are
rejected rather than repaired. The service's existing build-root isolation
continues to cover this cache. This is private builder-owned executable input,
not a remote archive or a boundary against a compromised build account.

After independent signature/direct-target verification, the helper asks Cargo
for the current workspace. It checks backend version against tag and workspace
manifest locations against the candidate root, then cleans every workspace
package **before invoking any candidate xtask**. External dependency outputs
remain; even a same-version xtask cannot execute another checkout's embedded
path. Cargo retains responsibility for dependency, feature, compiler and flag
fingerprints. [Cargo's cache reference](https://doc.rust-lang.org/cargo/reference/build-cache.html)
and [package cleaning](https://doc.rust-lang.org/cargo/commands/cargo-clean.html)
describe the underlying behavior; neither replaces these workspace checks.

The backend build emits Cargo JSON. Only the current workspace backend's
single regular, canonical output in `target/release/silkboard-web` is copied;
workspace artifacts must report a rebuild, not a fresh-cache hit. Compiler
diagnostics and non-JSON tool output are preserved in build logs. Explicit
tag/commit build inputs are retained, and source release admission is checked
again after build scripts run. Frontend outputs remain candidate-local. The
identity file and protected root staging still bind the requested tag/commit;
normal activation health checks still verify running version/tag/commit.
`Build cache evidence:` logs actual fresh/rebuilt external package IDs, so
SQLite reuse can be checked from `libsqlite3-sys`/`rusqlite` records.

The cache's `target` tree has a **4 GiB retained-size limit**, using allocated
KiB from GNU `du`. Under the lock, an oversized tree is cleared before a valid
build and on exit after success/failure; the lock inode, source/output trees,
Cargo downloads and toolchains are preserved. Outputs are copied before final
pruning. A cleared cache remains valid and builds cold. This is not an in-flight
quota: one build can temporarily exceed the limit, and an interrupted build
can leave data until the next admitted invocation. Temporary metadata/artifact
records are inside the bounded tree. Workspace artifacts may remain between
runs but are never reused by a later candidate without the required cleanup.
Existing retained source/output/runtime-release directories are separate from
this cache budget; measure their disk use too. Do not delete the cache root or
lock while a command may hold or await it.

## Local verification and limits

The existing [deployment test target](../xtask/tests/vm_bootstrap.rs) executes
[fast helper contracts](../deploy/vm/tests/build-cache.sh) and
[real Cargo fixtures](../deploy/vm/tests/build-cache-real.sh). Both execute an
owned copy of the actual helper with production filesystem paths replaced;
the harness refuses residual production paths. No production route is used.

The pre-change helper failed the cross-release reuse contract. The new fast
cases cover dependency retention, workspace freshness, version/artifact-path
rejection, signature/clean/build failures, whole-command locking, size eviction,
lock-inode preservation, symlink/privacy refusal and cold fallback. The real
fixtures use generated signed tags, a same-version workspace xtask/library and
a stable external path dependency. Eight builds verify current source/output,
warm reuse, feature/version/flag changes, compiler-proxy identity changes and
a cleared cache. Compiler proxies delegate to the same installed compiler:
this is not evidence of a second real toolchain, bundled SQLite reuse or ARM64
performance. The five integrated deployment tests passed locally; full quality
and independent review results belong in the PR.

## Operator trial before installation

Use an existing approved guest/operator route; this issue does not provision
one. Keep the currently running application, independently installed trust,
credentials, updater policy and polling configuration unchanged. Inspect
`silkboard-update status`, installed helper/signers hashes, pending/blocked
candidates, disk/RAM/swap, tool versions and active `silkbuild` processes first.
Do not overlap trial builds with a normal update or another builder command.
Use the existing updater lock for the entire trial command, not just setup.
The existing guest transport and update service have 30-minute deadlines;
a transport timeout can leave a guest process running. Inspect before retrying
or deleting anything. Never reset a blocked release as part of this trial.

From the operator checkout, verify the chosen helper commit with independently
configured trust (`git verify-commit REVISION`), export its helper and record
SHA-256. Transfer that reviewed file with the existing
[guest transport](../deploy/vm/README.md), retaining its checksum verification.
Do not use a trust file supplied by the candidate. The baseline helper at
`866fa2c2e4f71ca88294a66c5b50c43d4654a156` has SHA-256
`cfd9503a2c45be0d64504dfa5431756a0ac8187c985bef4dd45076e11ae07a40`.
If the installed helper differs, review that difference before claiming a
matched production baseline or replacing it.

Prepare a new private `silkbuild`-owned trial root beneath the builder root,
with separate `baseline` and `proposed` subdirectories. Copy the verified
baseline/proposed helper files there. In these **trial copies only**, change
the single `base=/var/lib/silkboard-build` assignment to the matching trial
subdirectory; inspect the diff and record each trial-file checksum. All other
code, especially independent signer-path verification, must be unchanged.
Each trial subdirectory needs `cargo` and `rustup` symlinks to the existing
builder-owned download/toolchain homes; do not copy credentials or change
accounts/permissions. This isolates compiled output and per-tag paths while
keeping the same installed tools and already-downloaded sources. Refuse an
existing trial root; retain failed trials for inspection rather than overwriting.

Choose two independently admitted stable tag/commit bundles with the same
ARM64 external dependency/features/profile inputs (a UI-only release pair is
useful). Apply the [exact release-suite requirements](releases.md) to each;
local signature success alone is insufficient. Supply only those source
bundles, readable by `silkbuild`, with no GitHub credentials. Isolated trial
paths permit measuring even an already-running tag without rebuilding its
live release or touching retained production source/output paths.

Run baseline tag A, proposed cold tag A and proposed warm tag B sequentially,
as `silkbuild` with the same clean environment used by the updater:
`HOME=/var/lib/silkboard-build`, `PATH=/usr/bin:/bin`, no database/owner/publisher
settings. Wrap each whole helper command with GNU `/usr/bin/time` and retain
wall/user/system seconds, maximum child RSS, exit code and durable private
logs. If GNU time is unavailable, arrange that prerequisite explicitly; don't
claim a missing resource measurement passed. Only proposed tag A starts with
an empty proposed target; do not clear it before tag B. Record before/after
allocated/apparent cache, source/output and free-disk sizes. Cargo's reported
compilation duration excludes frontend/admission work; report it separately
from whole-helper wall time. Use the helper's package-ID evidence to confirm
SQLite and other dependencies were actually fresh, rather than inferring reuse
from elapsed time alone. Registry/network and OS-page-cache warmth are separate
conditions from a cold compiled-output tree.

For both outputs, verify identity-file tag/commit and temporarily run the
candidate on an unused loopback port with its own frontend, **no database or
owner configuration**. Compare `/api/status` health/version/tag/commit with the
candidate, then stop only that owned process. Do not call stage/activate or
point a trial at production storage. Record installed application health and
identity before/after; a trial must leave them unchanged. Check current-source
freshness and invalidation with the native fixture on ARM64 as well; its
compiler-proxy case remains a proxy, not a real toolchain change. Report any
additional real compiler/options experiment separately with its exact inputs.
Stop on identity mismatch, compilation failure, OOM/swap pressure, insufficient
recovery disk space or the command deadline. Preserve logs and inspect active
processes/locks before any cleanup or rerun.

## Narrow helper rollout and recovery

Install only after reviewing the PR and accepting the operator's trial results.
This is a separate operator action, not an effect of merging/tagging the app.
Confirm which installed helper the updater actually calls. For the documented
layout it is `/usr/local/libexec/silkboard/build`; do not blindly overwrite a
different layout or use the full updater installer just for this change.
Inspect pending candidates first: changing the helper affects the next normal
poll's build and may allow an already-eligible candidate to progress. It does
not clear a blocked tag, alter eligibility or itself activate an application.

While holding `/var/lib/silkboard-updater/update.lock`, confirm no direct
builder process is active, check the installed baseline hash and keep a
root-owned backup. Check the incoming reviewed helper's recorded checksum.
Install it as a new root-owned mode-0755 file beside the active helper, then
atomically rename it over `build`. Refuse existing backup/temporary paths until
inspected. Do not copy the trial's modified base assignment into production.
Do not replace signers, token, policy, updater binary, systemd units or timer.
Record source commit, old/new hashes, owner/mode and installed path. The next
normal admitted build should create its private cache itself; no pre-seeding
from development, CI or a PR is supported.

After the next authorized build, inspect logs, retained size, private owner/modes,
expected fresh dependencies, current candidate identity, staging and normal
health/recovery outcome. Keep #63 open until real ARM64 evidence and rollout
acceptance are recorded. To roll back the helper, wait for the same lock and
an idle builder, then atomically restore the verified root-owned backup.
Leave the private cache inert for inspection: the old helper uses its own
per-source targets. Helper rollback is separate from application rollback or
retry; follow the [existing updater recovery procedure](automatic-upgrades.md)
for those actions under their own authorization. A successful local test or
trial is not proof that the production helper was installed.
