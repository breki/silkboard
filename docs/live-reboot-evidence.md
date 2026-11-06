# Live guest reboot and interrupted-upgrade recovery

Both owner-approved phases for
[#18](https://github.com/breki/silkboard/issues/18) passed on 2026-11-06.
The [procedure](live-reboot-drill.md) remains the approval and abort guide for
future drills. This record documents the observed operation; it does not
authorize another reboot, publication, retry or privileged installation.

## Trusted access and installed identity

The independent operator route reached malina's libvirt-managed `silkboard`
guest, domain UUID `c58a0183-ee30-4364-a1f5-5521d782dbef`. The guest agent
provided its public ED25519 host key through that trusted channel. Strict SSH
then passed with a temporary pinned known-hosts file and the existing operator
key, including noninteractive administration. No unauthenticated key scan,
global SSH trust edit or host-verification bypass was used. Both routes worked
before and after the reboots.

Installed updater SHA-256 matched the historical reviewed source
`49756375c166026e6cd6e40cf3bdac5cfc2a18f4`:

```text
7cd60866d5a3bceaa37b98b5a8e07fbd7274fc7db0b4642875dd309bfb41b07c
```

Wrapper, stage/fetch helpers and updater units matched that source; the build
helper matched the separately installed
[PR #139 revision](production-build-cache-trial.md). Installed code, units,
signature trust and durable state format were not replaced or migrated.
The installed engine recovers a pending transaction before its refresh/admit
path and returns after recovery; its wrapper permits recovery without the
fixed GitHub credential.

Both immediate baselines had healthy `v0.14.0` at
`8d4c6ca4c9038c8567e794c4d5ccf44bba10f5fc`, no pending transaction and the
existing `v0.2.1` block. The timer was enabled/active with a successful last
idle poll. Local health/status and certificate-validated private HTTPS passed.
Persistent previous-boot journals, over 8 GiB free disk and retained release
identity were checked. Retained `v0.13.0` also passed an isolated loopback
health probe without production storage; its owned probe process was stopped.

## Phase A: ordinary reboot

The owner approved one ordinary guest reboot after the private baseline.
No newer stable release existed during the agreed publication pause. The
original enabled timer remained active before reboot; no manual updater poll
was used to establish scheduling afterward.

| Observation | Result, UTC |
| --- | --- |
| Orderly guest reboot requested | 06:33:05 |
| Strict SSH and healthy exact HTTPS identity confirmed | 06:33:46, 41 seconds after request |
| Automatic new-boot updater finished | 06:34:27, success |
| Automatic invocation ID | `71006228392640ca9a508addb58977d2` |

Boot changed from `d05fcd93-84cb-49eb-8a34-8b1fd6629623` to
`ff119a04-6c2c-4377-bed3-a20b904e002b`. The new-boot invocation reported no
newer eligible release and current `v0.14.0`. Both guest endpoints, HTTPS,
resolved current path/identity, durable updater state, installed hashes,
application/timer enablement and active state matched before/after. Pending
remained null and only `v0.2.1` was blocked. Both boot journals were retained.

## Phase B: genuine interrupted upgrade

After Phase A, the owner separately approved publication of signed `v0.14.1`
at `ac4c80da0ce15761d95c54255f5509aff9c42ab8`, interruption of its pending
build, temporary updater-credential relocation and one further guest reboot.
The tag contains the merged publisher fix plus separate release bookkeeping;
it was not a deliberately broken candidate. Full local validation, ARM64
Clippy, standalone audit and trusted signed-tag admission passed before
publication. The audit reported the existing approved Deepmerge exception.

Polling stopped before publishing the exact commit/tag while its enabled
state was preserved. Release-suite
[42179662721](https://github.com/breki/silkboard/actions/runs/42179662721),
attempt 1, completed successfully for workflow `387356305`, the agreed path,
push event, exact tag and exact commit. All three required jobs passed. The
installed updater independently reported that same run and admitted the
candidate. It was the only newer stable unblocked tag in the coordinated
publication window; no block/retry change was made to select it.

One normal updater service invocation created the genuine synchronized pending
pair: previous exact `v0.14.0`, candidate exact `v0.14.1`. While an
unprivileged Cargo process was active in that service's control group,
SIGSTOP was sent only to the group. All five recorded processes, including
the updater and Cargo, were stopped. The exact pending pair and candidate
block were rechecked, and the current symlink/application still matched healthy
`v0.14.0`. Only that frozen service group then received SIGKILL. No pending
state was synthesized or restored from an old snapshot.

The service finished at 07:27:12 with `Result=signal`, signal 9, invocation
`1395127eba10474ba3171a71a300e039`. All updater descendants were gone;
pending state and both `v0.2.1`/`v0.14.1` blocks survived. The application
remained healthy and the runtime credential copy was absent.

At 07:28:22 the fixed root-owned mode-0600 updater credential was atomically
moved to the agreed previously absent hold path, without reading its contents.
Original inode/ownership/mode were retained. Both admin routes could restore
it; the original and runtime credential paths were absent before reboot.

| Observation | Result, UTC |
| --- | --- |
| Orderly guest reboot requested with credential held | 07:28:32 |
| Automatic credential-free recovery finished | 07:29:05, success |
| Recovery invocation ID | `34adfb36ea7943c2a8e2a1cc65490acc` |
| Strict SSH and healthy exact HTTPS identity confirmed | 07:29:12, 40 seconds after request |
| Credential restored | 07:29:16, original inode and private metadata |
| Separate scheduled poll finished | 07:30:01, success |
| Scheduled invocation ID | `96b67e2c0fe14b448ca50e691b2d4ac7` |

Boot changed from `ff119a04-6c2c-4377-bed3-a20b904e002b` to
`64a3e226-2169-43b4-8872-a1f4352cc669`. In one automatic new-boot invocation,
the wrapper logged the unavailable GitHub credential, then the installed
engine logged restoration of `v0.14.0` after incomplete `v0.14.1`. The
invocation succeeded with no manual recovery poll. This matches the verified
installed recovery-before-refresh control flow and demonstrates recovery
without fresh credentialed admission. Tailscale/SSH networking remained
available; this was not an offline-network test.

Further polling was stopped while recording recovery. The original credential
was atomically restored, the temporary hold/runtime paths were absent, and
the original enabled/active timer state was restored. A distinct automatic
07:30 poll with the restored credential reported no newer eligible release.
At completion both local endpoints and HTTPS were healthy at exact `v0.14.0`,
pending was null, both blocks persisted and installed hashes were unchanged.
The candidate remains intentionally blocked; no retry or activation of
`v0.14.1` was performed.

## Evidence retention and limits

Root-owned mode-0700 guest directories retain mode-0600 state copies, hashes,
service results, frozen process records and previous/current boot journals:

- `/var/log/silkboard-issue18-20261106-phaseA-before`
- `/var/log/silkboard-issue18-20261106-phaseA-after-complete`
- `/var/log/silkboard-issue18-20261106-phaseB`

Raw journals remain private; no credential contents or application messages are
included here. Phase A collection needed retransmission of a collector cleared
from guest `/tmp` and dash-free journal boot IDs; an earlier partial evidence
directory is preserved separately. These collection corrections required no
additional reboot. Phase B controls were retained in persistent private storage.

The 41/40-second figures measure request-to-confirmed-health intervals from
sampled probes, not exact application outage durations. This record verifies
guest/application/updater reboot recovery, interrupted pre-activation recovery
without the credential, durable candidate blocking and cleanup. It does not
verify host reboot, offline networking, candidate-start rollback or live
pending-check reconsideration. The rollback/retry evidence remains in
[#17](live-rollback-drill.md); pending-check reconsideration still leaves
milestone 1 incomplete. Issue #18 remains open for review of this evidence.
