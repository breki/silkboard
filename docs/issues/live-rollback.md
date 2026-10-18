# Live rollback and retry (#17)

The owner requested issue #17 and selected two valid patch releases for the
drill: v0.2.1 for an exact-candidate startup failure and retry, then v0.2.2
for a newer eligible release while the first remains blocked. The owner
approved phase 1; v0.2.2 publication still requires a separate decision.

Follow the [drill procedure](../live-rollback-drill.md). Retain its preflight,
failure, block persistence, retry, newer-release and cleanup evidence. Update
the automatic-upgrade verification record with observations, not predictions.
The installed updater must remain unchanged; current toolkit code uses a
different state contract and is not an implicit upgrade prerequisite.

Read-only preflight on 2026-10-18 found a healthy v0.2.0, no pending work or
blocked tags, active polling, 31 GiB free, and the recorded updater checksum.
Signed v0.2.1 was published and passed its exact release suite. Two controlled
startup failures restored healthy v0.2.0; an intervening poll respected the
block, and manual retry cleared only that block before ordinary readmission.
The fault has been removed; units and installed updater are unchanged. State
has no pending work and v0.2.1 is blocked. Polling remains paused for phase 2.
Local v0.2.2 preparation is separate from this documentation change; it has
not been published. Its newer-release acceptance remains unrun.
No production reboot is part of this issue. Issue #18 remains separate.
Remove this working document when verification is complete.
