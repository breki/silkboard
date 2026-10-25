# Codex task-plan reporting

For Codex development work, use the installed publisher's explicit
`codex-plan` commands when the operator has enabled plan reporting. Keep the
working plan aligned with actual decisions and work. Reporting does not grant
permission to run checks, start reviewers, merge, release or execute tasks.
GitHub remains the task tracker. Claude integration remains separate.

## Prerequisites

The operator must install a compatible server with the plan API and a publisher
with `codex-plan` support, then explicitly set `codex.plan_reporting` to `true`
in the existing private publisher configuration. Existing summary settings do
not opt in. See [configuration](../codex-publishing.md) and
[upgrade order](../task-plan-publisher.md#installation-order).
Do not install credentials, enable reporting or change hook trust merely
because a development task mentions a plan.

Use the operator-selected executable/configuration paths, as for ordinary
progress reporting. The defaults are `$HOME/.local/bin/silkboard-publish` and
`$HOME/.config/silkboard-publish/config.json`; operator environment overrides
are `SILKBOARD_PUBLISH_BIN` and `SILKBOARD_PUBLISH_CONFIG`.
The executable must already be installed. Do not substitute a repository
executable, download a helper or read token contents to make reporting work.

Run from the configured workspace or a descendant. The command derives its
host/name from operator configuration, fixes the tool to `codex`, and reads
`CODEX_THREAD_ID` from the native shell environment. Installed Codex 0.163.0
was verified to supply the actual native thread ID. This environment variable
is a locally verified integration detail, not a promised universal Codex API.
If it is missing, invalid or no longer matches the runtime, stop plan reporting
and verify the integration. Never invent an identity or fall back to a
transcript, conversation text, a previous session or `CODEX_SESSION_ID`.

One root Codex session maintains a task graph. Child agents report findings
to that writer; they do not publish changes to its graph. Several steps may
be active while work overlaps. Independent attempts use different task UUIDs.

## Start and maintain a graph

For a substantive task, choose a fresh canonical UUID v4 and record it with
stable step IDs in the task's local working record. On Linux a fresh UUID is
available from `/proc/sys/kernel/random/uuid`. Never reuse the sample UUID in
the publisher guide for real work. Do not change an existing task ID to hide
lost state, an ordering error or an unfinished attempt.

Publish the complete current graph with the installed command:

```sh
publisher=${SILKBOARD_PUBLISH_BIN:-"$HOME/.local/bin/silkboard-publish"}
config=${SILKBOARD_PUBLISH_CONFIG:-"$HOME/.config/silkboard-publish/config.json"}
"$publisher" codex-plan start --config "$config" <<'JSON'
{
  "schema_version": 1,
  "task_id": "c53ba3b1-c55e-45a3-98af-a061aa972224",
  "title": "Add task-progress graphs",
  "github_issue": {"repository": "breki/silkboard", "number": "77"},
  "steps": [
    {"id":"surface","title":"Verify Codex integration",
     "depends_on":[],"state":"done"},
    {"id":"adapter","title":"Publish complete native plans",
     "depends_on":["surface"],"state":"active"},
    {"id":"verify","title":"Verify workflow and boundaries",
     "depends_on":["adapter"],"state":"pending"}
  ]
}
JSON
```

These example states are illustrative. For real work, use the actual task UUID,
structure and observations. Unlike `plan start`, `codex-plan start` does not
accept `reporter`; the command supplies native/operator identity. Both commands
use the same synchronized private state and wire contract. Input cannot select
a destination, token, state path, executable or allocated counters.

After start has allocated working state, use `codex-plan publish` with the
entire current graph and the same task ID. This remains true if start timed
out or did not acknowledge delivery: it may already have committed state.
Do not repeatedly start, delete files or edit counters to recover.

Publish at meaningful boundaries: initial plan, starting or completing a step,
a real blocker, a revised definition, reopening completed work, and a final
accurate state. Stable IDs survive title/description edits and reopening.
Dependencies must remain valid; active/done steps require done prerequisites.
Use multiple active steps when work actually overlaps. A blocked step needs
its actual reason; silence or delivery failure is not a work blocker by itself.
Full snapshots retain completed work and repair missed intermediate updates.
The publisher manages sequence, definition revision and writer generation.

A native plan-tool update is not a complete Silkboard graph. Its text/status
list does not supply stable IDs, prerequisite edges or blocked reasons.
Maintain those explicitly rather than guessing identities from titles or
inventing a linear graph. No transcript content, source file, prompt, shell
log or private handoff is automatically uploaded. Choose bounded report text
that is appropriate for the private dashboard; this is not a secret scanner.

## Failures, continuation and handoff

Delivery is one bounded attempt with no automatic retry or queue. A failed
report does not prevent development; record delivery as unconfirmed and
continue authorized work. Do not claim phone visibility or server receipt
without evidence. A later meaningful update can send the complete current
snapshot again. Do not loop or report on every shell command.

Preserve task ID, stable step IDs and current local working state during
compaction and same-session continuation. Task state is private and outside
Git. The task's local handoff record may name the task ID and state location;
it must not contain tokens or authorize a remote transfer. Do not upload that
record or treat it as a durable outgoing message queue.

When a different native session takes over, stop the old writer and use the
existing [explicit writer handoff](../task-plan-publisher.md#transfer-the-reporting-role-explicitly).
The replacement must use the same current state and operator-approved
endpoint/project. The ordinary `plan handoff` command supports explicit old
and new native identities; subsequent `codex-plan publish` derives the new
session identity and must match that binding. Cross-machine transfer and
credential installation still require their own operator authorization.

Never interpret Stop, a final response, child completion, session shutdown or
missing reports as automatic task completion. Mark a step done only when its
actual work is complete. Reopen validation/review steps if new changes require
rechecking them. An agent's done state is a report, not proof of passing checks
or permission to merge. Keep ordinary short progress hooks enabled; this
workflow does not replace or change their trust definitions.
