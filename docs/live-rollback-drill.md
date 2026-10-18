# Live rollback and retry drill

This is the procedure for [issue #17](https://github.com/breki/silkboard/issues/17).
The owner selected two valid patch releases. Execution and publication remain
pending explicit approval; the read-only preflight below is observed evidence.
This is not evidence that production rollback or retry already works.

## Scope and observed baseline

Read-only checks on 2026-10-18 found the dedicated `silkboard` guest healthy:

- Current release: `v0.2.0`, commit
  `6a24145a6f5404e210f7597001d02e166b023889`.
- Installed updater: reviewed source `49756375c166026e6cd6e40cf3bdac5cfc2a18f4`,
  binary SHA256
  `7cd60866d5a3bceaa37b98b5a8e07fbd7274fc7db0b4642875dd309bfb41b07c`.
- Updater state: `blocked: []`, `pending: null`. Timer active; latest observed
  scheduled poll at 01:50 UTC preserved the current release.
- Both `/health` and `/api/status` passed on guest port 3000, with exact
  version/tag/commit identity. The retained v0.1.1 directory also exists;
  its health was not tested during this preflight.
- About 31 GiB free in `/opt/silkboard`'s filesystem. The v0.2.0 release is
  retained as the recovery target throughout this drill.

The installed updater uses its original state format. Application upgrades
do not replace it. Do not install the newer toolkit updater, migrate state,
reboot the guest, or modify another host service as part of this drill.

Use the existing private operator SSH connection with its pinned host key.
The operator-only guest agent on malina is an independent recovery channel.
Confirm both channels before starting. Never read or print the GitHub token.

## Release preparation and admission

Prepare v0.2.1 through the [release procedure](ai-agents/workflows/release.md),
including full local validation, ARM64 lint, advisory audit and a signed local
tag. Do not push it during preparation. It contains valid application code;
failure injection is guest configuration, not a broken published build.
Record the exact candidate commit before requesting publication approval.

Before publishing it, stop the updater timer and wait for any active updater
service to finish normally. Record the timer's prior enabled/active state.
Do not stop an active transaction to make room for the test. Recheck the
baseline state, checksum, current identity, health, free space and recovery
channels. Abort on an unexpected current release, block or pending transaction.
Retain a private copy of state, current identity, units and relevant journal
entries for diagnosis. A state backup is not a license to erase new state.

Publish only after approval and timer suspension. Keep polling suspended while
its checks run. Record the exact-revision release-suite run and attempt.
Use `sudo silkboard-update inspect v0.2.1` to prove the installed admission
path accepts the signed exact commit. Record successful signature and suite
evidence; a local tag alone is insufficient. The production token remains
limited to Contents/read and Actions/read.

## Candidate-only startup fault

After admission, install a new, root-owned runtime drop-in at
`/run/systemd/system/silkboard.service.d/90-issue17.conf`. Refuse to overwrite
an existing file. Its only content is:

```ini
[Service]
ExecStartPre=/bin/sh -c '! /usr/bin/test /opt/silkboard/current -ef /opt/silkboard/releases/v0.2.1-CANDIDATE_COMMIT'
```

Replace `CANDIDATE_COMMIT` with the admitted 40-character lowercase commit
before installation; no placeholder or shell-supplied tag is executable input.
Both paths are fixed operator-controlled paths. The check compares the actual
target directory, so it fails only while that exact candidate is current.
It runs as the application user under the existing sandbox, adds no authority,
and leaves the baseline and later releases able to start. Set the directory
mode to 755 and file to 644; root owns both. Record the resolved content.

Run `sudo systemd-analyze verify silkboard.service` and
`sudo systemctl daemon-reload`.
Confirm the effective unit adds only this check. Test the same comparison as
the application user against the current baseline: it must succeed. Merely
loading the drop-in does not restart the application. Do not proceed if its
condition would also prevent the recorded previous release from starting.

## Failure, durable block and retry

1. Start `silkboard-update.service` once and wait for its result. A build can
   take up to the unit's 30-minute timeout; keep the existing application
   available during compilation. The expected failure is specifically the
   injected startup check after successful build and repeated admission.
   A network/build/admission error does not establish activation rollback.
2. Record the service result and journal. Confirm the attempted candidate and
   its exact commit, activation failure, restoration of v0.2.0, both healthy
   endpoints and exact baseline identity. Expect `pending: null` and exactly
   v0.2.1 newly blocked. Polling can report failure even after healthy rollback.
   There will be a brief outage during attempted startup and restoration.
3. Start another poll with the fault still present. Confirm the block remains,
   no candidate rebuild/start is attempted, and v0.2.0 remains healthy. Retain
   before/after state and journal evidence from these separate invocations.
4. Run `sudo silkboard-update retry v0.2.1` through its installed locking wrapper.
   Compare state sets: only that tag is removed; current identity and pending
   work remain unchanged. Record the command and result without credential
   tracing. Re-run `sudo silkboard-update inspect v0.2.1`, then start a poll with the fault still
   present. It must pass ordinary admission, attempt the candidate again,
   fail at the same check, restore healthy v0.2.0 and block v0.2.1 again.
   This proves retry causes a real new attempt rather than granting admission.

After every poll, check both health endpoints and full identity. Stop on any
unexpected result. Do not manufacture blocked tags or pending transactions,
change tag targets, weaken release checks, or replace the installed updater.

## A newer eligible release

Remove the exact drill drop-in and reload systemd; leave other drop-ins alone.
Confirm the baseline still runs and v0.2.1 remains blocked. Prepare and publish
valid v0.2.2 through the same release procedure and explicit publication
approval, recording its exact commit and successful suite. Keep the timer
suspended until ready for the controlled poll; do not publish both candidates
up front, because the updater chooses the highest eligible version first.

Inspect v0.2.2, then start one normal updater service invocation. Verify it
builds and activates v0.2.2 while the v0.2.1 block remains durable. Require
both health endpoints, exact new version/tag/commit, no pending transaction,
and the preserved block. Do not infer this outcome from local selection tests.
This second patch release is an owner-selected verification release; its
bookkeeping must accurately describe its scope rather than invent a fix.

## Abort, recovery and cleanup

If rollback is unhealthy, keep the timer stopped and remove only the drill
drop-in. Reload systemd. If state still contains pending work, invoke one poll:
the installed engine restores its recorded previous release before network
admission. Verify actual health and identity. Do not clear pending state or
run `retry` to hide failed recovery. If automated restoration fails, use the
independent operator channel to inspect the retained baseline directory and
perform a separately recorded manual restoration; mark automatic acceptance
failed even if the application is recovered manually.

If state has no pending work but the wrong release is running, stop and
investigate instead of blindly replacing the symlink. Preserve state and
journals. Do not remove the retained baseline, build outputs or failed
candidate while investigating. No reboot is required for cleanup.

After successful newer-release verification, confirm the runtime drop-in is
absent and the effective application unit matches its original configuration.
Restore the recorded timer state, observe a subsequent normal poll, and check
private HTTPS plus the two guest endpoints and exact running identity. Keep
v0.2.1 blocked as the drill's durable result; any later cleanup of that block
is an explicit operator retry, not an automatic reset.

Retain redacted guest journals, service results, state snapshots, exact source
identities and suite references in the verification record. Separate successful
checks from unrun/failed cases. Update #17's evidence only after observing
these outcomes. Guest reboot/interrupted-transaction proof belongs to #18.
