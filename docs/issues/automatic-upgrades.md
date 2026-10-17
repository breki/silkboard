# Automatic upgrades and recovery

Issue [#6](https://github.com/breki/silkboard/issues/6). In progress.

## Goal and agreed approach

Complete milestone 1: poll every ten minutes, select the highest newer stable
eligible release, verify the installed SSH signer and release workflow
`387356305` for the exact tag/commit, build as `silkbuild`, activate with health
checks, and recover the working release on failure or interrupted activation.
Failed tags remain blocked until manual retry; newer tags remain eligible.

The owner selected a fine-grained GitHub token restricted to this repository
with Contents/read and Actions/read on 2026-10-17. Installation is pending.
The token belongs only to the downloader, never the builder or application.

Implement the policy and orchestration in a standalone Rust workspace crate.
Install an operator-controlled copy; release-controlled code runs only as the
builder and runtime users. Persist a transaction before build/activation and
recover it before considering another upgrade. Keep the previous release.

## Acceptance checks

- Behavioral red/green tests for selection, exact workflow/tag binding,
  pending checks, failed build/start/health, retry, and interrupted recovery.
- Signature verification before any release-controlled build command.
- Credential/account separation, bounded network commands, serialized runs.
- Full read-only validation, ARM64 lint, sequential independent reviews.
- Authorized live checks on the dedicated guest: automatic upgrade, rollback,
  retry, and guest reboot persistence. Release publication and deployment
  remain explicit actions requiring the appropriate authorization.

## Progress and open facts

- Started from merged PR #10, main `4df661725d5a5a82b2295466cf1c720c10fc2dec`.
- Production currently serves verified `v0.1.1` at `c68a539`.
- Reviewed local MuTrack updater, Phren pre-build signature verification,
  Hoard release command and proposed pull updater; proposal is not operational.
- GitHub read-back confirms repository ID `1417054470`, workflow ID
  `387356305`, and the three successful jobs in the initial release suite.
- Credential creation/installation is awaiting the owner's Ready response.
- Local implementation complete: full read-only validation, ShellCheck,
  ARM64 lint, and actual Git/SSH signature tests pass. Full Rust coverage is
  96.4%; every production module exceeds the per-file threshold.
- Sequential independent artisan, red-team, and fresh-reader reviews found no
  actionable findings. Frozen snapshots are retained locally.
- Read-only guest checks confirm the existing `v0.1.1` service and Tailscale
  remain active. No updater installation, enablement, or application upgrade
  has occurred. Production acceptance and release publication remain pending.
