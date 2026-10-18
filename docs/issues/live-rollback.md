# Live rollback and retry (#17)

The owner requested issue #17 and selected two valid patch releases for the
drill: v0.2.1 for an exact-candidate startup failure and retry, then v0.2.2
for a newer eligible release while the first remains blocked. Publication
and production disruption still require approval of the concrete procedure.

Follow the [drill procedure](../live-rollback-drill.md). Retain its preflight,
failure, block persistence, retry, newer-release and cleanup evidence. Update
the automatic-upgrade verification record with observations, not predictions.
The installed updater must remain unchanged; current toolkit code uses a
different state contract and is not an implicit upgrade prerequisite.

Read-only preflight on 2026-10-18 found a healthy v0.2.0, no pending work or
blocked tags, active polling, 31 GiB free, and the recorded updater checksum.
The fault has not been installed. Local v0.2.1 preparation is separate from
this planning change; neither patch release is published. No production reboot
is part of this issue.
Issue #18 remains separate. Remove this working document when verification is
complete.
