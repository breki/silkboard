# Task-plan recovery and freshness

Locally verified for [#79](https://github.com/breki/silkboard/issues/79), using
the installed publisher, actual Axum publisher/owner routes and private test
storage. The [wire contract](task-plan-contract.md) supplies the ordering rules;
this work adds composition checks, not a new reconciliation service.
[Combined local acceptance](task-plan-acceptance.md) is recorded in #80;
production dogfooding remains #81.

## How recovery works

Every report contains the complete current plan. Before sending it, the
publisher reserves a fresh sequence and saves the current draft and counters
in its private working state. If delivery fails, the next explicit publish
sends a new full snapshot with a higher sequence. It needs no dashboard read
and does not resend a stored queue of old reports. See the
[publisher guide](task-plan-publisher.md) for invocation and handoff.

The server uses task sequence, definition revision and writer generation,
rather than arrival time, to decide whether a report can become current.
The next valid snapshot can repair the displayed graph after missed status,
definition or handoff reports. Earlier received snapshots remain immutable.
The server never fills missing history with guessed intermediate work.

## Controlled failure evidence

The integration test in
[plan_recovery.rs](../crates/silkboard-publish/tests/plan_recovery.rs) runs the
actual installed executable with no helper commands on its PATH. Test-only
middleware interrupts writes around the normal authenticated routes:

| Scenario | Observed result |
| --- | --- |
| Initial full report, sequence 1 | Stored and acknowledged |
| Status 2 and definition updates 3–4 rejected before write | Local high-water marks advance; server remains at 1 |
| Fresh full report 5, definition revision 3 | Current graph repaired, including completed work and revised prerequisites; only received sequences 5 and 1 appear in history |
| Delayed unseen report 2, conflicting sequence or stale revision | Rejected; current graph and receipt remain unchanged |
| Orphan prerequisite in a new revision | Rejected atomically; no partial graph or history |
| Exact replay of a received snapshot | Original acknowledgement and receipt returned; current pointer and freshness unchanged |
| Write 6 accepted but its acknowledgement replaced by an error | Client reports failure while the owner can read the accepted snapshot |
| Full report 7 after the ambiguous failure | Current graph advances normally; reported blocked reason remains explicit |
| Completed step removed in revision 4 | Removed from the current graph; its earlier received completion remains readable |
| Handoff report 9 lost, later writer generation 3 received at 10 | Same task continues; earlier-generation writer cannot replace it, even with a higher sequence |
| Peer stays silent beyond the network budget for 11 | Client returns its fixed timeout diagnostic; one attempt, no write or queued resend |
| Fresh full report 12 | Current graph advances; history contains only 12, 10, 8, 7, 6, 5 and 1 |

Failed invocations make one attempt. Controlled failures return within the
fixture's five-second bound; transport retains its existing three-second
network budget. The test checks counters and exact received history, rather
than treating an exit status alone as proof of server state. Its faults and
credentials exist only inside the disposable fixture.

## What the owner sees

The Task plans page shows the last received report time and age. Repeated
reads and identical replays do not make an old report fresh. Reported states
are not verified results or a live connection status. In particular, a quiet
reporter is not automatically blocked, complete, offline or still working.
A reported blocked step keeps its explicit blocking reason.

Component checks advance the clock through quiet time and owner-read failures:
receipt time, active/blocked states and selection remain intact while age grows.
A failed refresh adds a warning. The real-API browser check skips reports and
a definition revision, then verifies that the next full snapshot repairs the
graph without losing the selected step or inventing history. It also checks
that a later failed read retains the received graph and blocked reason.

## Best-effort limits

- A successful receipt acknowledges storage, not completion or verification.
  A timeout or invalid/error acknowledgement can follow a successful write.
- Recovery requires a later explicit full report. No later report means no
  repair. Missing intermediate reports, completions and writer bindings cannot
  be reconstructed. A complete current graph does not imply complete history.
- Keep the same conceptual step IDs and transfer private working state through
  the explicit local handoff. Missing state is not permission to guess counters;
  recover trusted state or start a distinct attempt.
- Do not edit counters or replay different content under an old sequence to
  force a conflicting writer through. Exact replays still require a valid,
  unrevoked project credential and grant no owner-read authority.

There is no background sender, durable outgoing queue, task-content download,
VM-control path, production deployment or new recovery/backup policy here.
