# Proposed guest reboot and updater recovery drill

Preparation for [#18](https://github.com/breki/silkboard/issues/18), not live
acceptance evidence. The issue requires agreement on the concrete drill before
production reboots or interruptions. Approve each phase after its baseline is
recorded. Preparing this document does not authorize a release, a reboot,
credential relocation, process signals or changes to SSH trust.

## Current evidence and access

On 2026-11-06, certificate-validated private HTTPS reported healthy v0.14.0,
commit `8d4c6ca4c9038c8567e794c4d5ccf44bba10f5fc`. This proves the running
application identity only. Strict operator SSH failed because this workspace
has no trusted ED25519 host key for the production hostname. No guest command
ran and no production state was changed.

Restore the existing operator-approved, pinned SSH route; obtain any missing
pin through the independently trusted operator channel. Never trust an
unauthenticated key scan or disable host verification. Confirm the independent
operator-only guest-agent channel on malina works before either phase; a
working application endpoint is not an administrative recovery channel.

The [historical installation](automatic-upgrades.md#verification-status)
records updater source `49756375c166026e6cd6e40cf3bdac5cfc2a18f4`, checksum
`7cd60866d5a3bceaa37b98b5a8e07fbd7274fc7db0b4642875dd309bfb41b07c`.
Application upgrades do not install a new updater. Verify the current binary,
wrapper, helpers and units before relying on either historical or current
repository source. Stop and reassess any mismatch; do not replace installed
code or migrate state as part of the drill.

## Baseline and evidence

Through trusted operator access, record these values immediately before each
phase in a new root-owned mode-0700 guest evidence directory. Keep files
mode 0600; use UTC times and boot IDs to associate observations.

- Guest hostname/domain identity, boot ID and UTC time; verify the dedicated
  `silkboard` guest, not its host or the separate trial guest.
- Both guest endpoints (`http://127.0.0.1:3000/health` and `/api/status`) and
  private HTTPS health/status, with exact version, tag and full commit.
- Resolved `/opt/silkboard/current`, its `identity` file and retained release
  directory. Check free space on the state and release filesystems.
- Installed updater/wrapper/helper hashes and their reviewed source identity;
  effective unit files, drop-ins, application/timer enablement, active state,
  timer deadlines and latest updater invocation result.
- Root-only `/var/lib/silkboard-updater/state.json` and installed
  `silkboard-update status`. Require no pending transaction for initial entry;
  preserve the existing blocked set and never invent an empty baseline.
- Persistent journal availability across reboot, relevant previous/current
  boot journals, the working independent recovery route and its operator.

Do not record tokens, credential-file contents, process environments or shell
traces. Credential checks record ownership/mode/existence only. Keep raw
journals private and inspect for secrets before sharing a bounded summary.
If journals are not persistent, prepare an agreed persistent evidence method
before reboot rather than claiming missing earlier-boot logs as success.

Abort preparation if recovery paths are unavailable, the previous release is
missing/unhealthy, space is insufficient for durable state/log writes, another
transaction is active, or observed state differs from the approved baseline.
Wait for active work to finish normally; do not interrupt it to create a slot.
Require the updater timer already enabled/active and its last idle poll healthy.
If scheduling or credential setup needs repair, agree that separately first.

## Phase A: ordinary guest reboot

This phase needs its own approval after the private baseline is available.
Expected identity is the recorded healthy release; v0.14.0 above is an initial
observation, not a substitute for the immediate baseline.

1. Verify no newer eligible release can change the expected identity during
   this phase; agree a release-publication pause for the approved drill window. If one exists, resolve the release ordering with the owner.
   Keep the original enabled updater timer enabled so boot scheduling is
   exercised. Require no pending work and no active updater invocation.
2. Record the old boot ID, timer state and healthy identity. Request one
   orderly reboot of the dedicated guest through the verified operator route.
   Do not reboot malina or another guest.
3. Allow up to five minutes for trusted administrative access and application
   health to return; record actual outage duration. Require a different boot
   ID, both local endpoints and HTTPS healthy at the exact baseline identity,
   and application enablement/active state preserved.
4. Observe a new automatic updater invocation in the new boot. Require the
   timer's original enabled/active state and a recorded next deadline. The
   installed timer is expected to run after boot and on ten-minute boundaries;
   record the actual invocation rather than inferring it from enablement.
5. Retain before/after state, both boot journals and service results. Require
   no pending transaction and the unchanged blocked set. Stop if any part is
   unexpected; do not proceed automatically to the interruption phase.

## Phase B: interrupt a real pending build, then reboot

This needs separate approval after Phase A and recording an exact, locally
prepared candidate. It includes suspending polling before the approved release
publication; interruption still waits for successful installed admission.
It adds a guest reboot and a recovery restart of the existing application.
It tests interruption before candidate activation. Candidate-start rollback
is separate [#17 evidence](live-rollback-drill.md), not a result of this phase.

A newer, valid signed release with a successful exact tag-triggered suite is
required. v0.14.1 containing the merged publisher fix is a proposed candidate;
its preparation/publication and precise tag/commit are not authorized by this
document. Do not publish a deliberately broken build, reuse an old tag,
manufacture pending state or bypass ordinary admission.

1. Prepare the exact signed candidate through the
   [release procedure](ai-agents/workflows/release.md), leaving it unpublished.
   Record previous/candidate identities and agree the publication/drill window.
   Stop the timer **before publishing** while retaining its original enabled
   state; wait for an idle service. Only then publish the separately approved
   candidate and wait for its successful exact tag-triggered suite. Record the
   run/attempt and inspect the exact candidate with the installed updater.
   Require this to be the only newer stable unblocked tag: inspection does not
   pin a later poll. Coordinate a publication pause and no block/retry changes
   for the entire phase. Abort on another newer tag or changed ref/check state;
   do not let a normal poll choose a different release. A locally valid tag is
   insufficient.
2. Start one normal updater invocation. Observe its genuine, synchronized
   pending record with the exact previous/candidate pair and candidate block.
   While its unprivileged build is active, suspend **only** the updater service
   control group with SIGSTOP through systemd. Re-read durable state and verify
   the current symlink and application still match the healthy previous release.
   Confirm the service is frozen before proceeding. If it finished, activated
   the candidate or left the expected checkpoint, resume it with SIGCONT and
   stop this phase; do not rewrite state to recreate the checkpoint.
3. Capture the pending state, blocked set, invocation ID, build/process state
   and healthy baseline. Kill only that frozen updater control group with
   SIGKILL. Verify no updater descendants remain and capture its failure result.
   Confirm the pending record and candidate block survived. No application,
   host or unrelated process receives these signals.
4. To test credential-free recovery, atomically move the fixed root-owned
   mode-0600 `/etc/silkboard/updater-token` to a new, otherwise absent
   `/etc/silkboard/issue18-updater-token.hold`. Preserve ownership/mode on the
   same filesystem; never read its contents. Verify the original path and
   runtime `/run/silkboard-fetch/token` are absent. If runtime cleanup failed,
   stop and inspect through the operator channel before reboot. Confirm both
   trusted recovery channels can restore the held credential independently.
5. With the timer originally enabled, request one orderly guest reboot. The
   new boot's automatic updater invocation must restore the recorded previous
   release before any fresh fetch/admission, even with its GitHub credential
   unavailable. Require exact healthy identity on both guest endpoints and
   HTTPS, `pending: null`, and the existing blocks plus the interrupted
   candidate's block. Capture its invocation journal/result and new boot ID.
   Recovered state or a manual operator poll alone does not prove automatic
   scheduling; retain the boot-triggered invocation evidence.
6. After that recovery invocation finishes, stop further timer polling while
   collecting this result. Restore the held
   credential atomically only if the original path is still absent, preserving
   its private metadata. Restore the original timer enabled/active state and
   observe a separate scheduled poll with no pending work and the candidate
   still blocked. Never clear its block or retry it without a separate decision.

Missing credentials demonstrate recovery without fresh credentialed admission.
This does not establish an offline boot: Tailscale/SSH networking stays
available. Verify recovery ordering against the installed source and retained
journal; do not infer zero network traffic merely from a healthy endpoint.

## Abort, restoration and acceptance

If administrative access or health fails to return within five minutes, stop
further drill actions and use the previously verified independent operator
channel to inspect the same guest. Do not loop reboots. Keep polling stopped
during diagnosis. Remove no state, release directory or failed-tag block.
Never restore a saved state file over newer state to make a test pass.

If a frozen service must resume before interruption, use SIGCONT on that same
control group and allow the original transaction to finish normally. After
SIGKILL, preserve pending work for normal installed recovery. If automatic
recovery fails, retain the failure and pending state, restore the credential
through the independent channel, and attempt separately recorded operator
recovery. Manual success does not count as automatic recovery acceptance.

Restore any held credential even when a phase fails, using the verified
recovery channel; inspect any newly appeared token path instead of overwriting
it. Restore the recorded timer state only after exact health and cleared
pending work are verified. Otherwise leave polling paused, record that
exception and agree recovery before resuming it. Keep journals, blocked state
and baseline release directories. Record any cleanup that could not be
completed.

Only close #18 after both approved phases have live evidence: old/new boot
IDs, exact release identity, application health, automatic timer invocation,
real interrupted transaction, recovery before fresh admission without its
credential, durable candidate blocking, and verified cleanup. Local tests and
this procedure alone leave milestone 1's reboot proof incomplete.
