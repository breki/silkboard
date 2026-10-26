# Publishing evolving task plans

Implemented for [#76](https://github.com/breki/silkboard/issues/76) in the
standalone Rust publisher. The first server release is `v0.7.0`; a compatible
publisher is now installed on the initial development machine. See the
[production rollout and its limits](task-plan-rollout.md).
The explicit [Codex workflow](ai-agents/codex-plans.md)
and `codex-plan` adapter implement
[#77](https://github.com/breki/silkboard/issues/77); current turn-summary hooks
continue independently. The [reporting contract](task-plan-contract.md) defines
graph semantics and the [private API](task-plan-api.md) defines server behavior.
GitHub remains the task tracker. Reported completion grants no execution,
merge or release permission.

## Installation order

Build/install as described in [publishing CLI](publishing-cli.md). The binary
requires no Rust, Cargo, Node, helper commands, daemon or queue on its target.
It reuses the existing operator-controlled endpoint/project/token/state
configuration and project-scoped publish-only token. Plan input cannot choose
a destination, credential, filesystem path or command to execute.

Upgrade the server to a reviewed release containing #75 before using these
commands, then install a compatible publisher containing #76 and, for the
Codex adapter, #77. The first release containing these features is `v0.7.0`.
Preserve private client state during
replacement. Verify that `--help` lists `plan {start|publish|handoff}`; the
publisher package version alone does not identify the server release.
Older servers cannot ingest plans; rejection is not success.
Existing single-line/multiline progress and Codex lifecycle reports remain
compatible and use their existing separate counters. Publishing plans does
not install the owner graph page, enable Codex's separate plan opt-in or change
hook trust. The graph is locally implemented in #78 and needs an authorized
server release. [Recovery evidence](task-plan-recovery.md) verifies #79.

## Publish a complete current plan

Start an independent task with `silkboard-publish plan start --config PATH`.
For subsequent reports use `silkboard-publish plan publish --config PATH`.
Both read one JSON object on stdin. Start refuses an existing local task;
publish refuses missing state, so a lost file cannot silently reset counters.
Include `schema_version: 1`, a canonical lowercase UUID v4 `task_id`,
`reporter`, `title` and the complete `steps` array. Optional fields are
`description` and `github_issue`, plus the reporter/step optionals in the
wire contract. Unknown fields and duplicate members are rejected. Do not
supply `sequence`, `plan_revision` or `writer_generation`: the CLI allocates
them from synchronized private working state.

Generate a fresh task UUID for an independent attempt. Reuse it for revisions,
reopening and an explicit session handoff. For example, Linux provides fresh
UUIDs through `/proc/sys/kernel/random/uuid`; the client itself invokes no
helper. The fixed UUID and identities below are fictional sample values:
replace them for real work. The example models this task-progress feature,
with reported states for illustration rather than production evidence.

```sh
silkboard-publish plan start --config /PRIVATE/config.json <<'JSON'
{
  "schema_version": 1,
  "task_id": "c53ba3b1-c55e-45a3-98af-a061aa972224",
  "reporter": {
    "hostname": "development-vm",
    "tool": "codex",
    "session_id": "native-session-a",
    "session_name": "Task-progress feature"
  },
  "title": "Add task-progress graphs",
  "github_issue": {"repository": "breki/silkboard", "number": "76"},
  "steps": [
    {"id":"contract","title":"Agree reporting contract",
     "depends_on":[],"state":"done"},
    {"id":"api","title":"Persist private plan snapshots",
     "depends_on":["contract"],"state":"done"},
    {"id":"publisher","title":"Extend installed publisher",
     "depends_on":["api"],"state":"active",
     "report":"Checking retries and local handoff."},
    {"id":"ui","title":"Build owner graph page",
     "depends_on":["api"],"state":"pending"},
    {"id":"verify","title":"Verify combined feature",
     "depends_on":["publisher","ui"],"state":"pending"}
  ]
}
JSON
```

After the initial start, send the entire updated graph with `plan publish`
on every invocation. Multiple active steps are
allowed when their prerequisites are done. Definition changes automatically
advance `plan_revision`; state, report and readable-name changes alone do
not. Each valid allocation advances the task sequence, including failed
network attempts. An explicit new-task start allocates sequence/revision/generation at 1.
Counters are decimal strings on the wire and checked through signed 64-bit
maximum. Exhaustion and malformed/newer local state fail closed.

Input and normalized wire snapshot each have a 262,144-byte limit, alongside
all graph/field limits. Ordinary progress retains its 65,536-byte input limit.
The existing verified HTTPS policy, disabled redirects/proxies, three-second
network budget and 1,024-byte acknowledgement cap apply unchanged.

Exit 0 with `Task plan acknowledged` means the server returned exactly the
allocated task ID and sequence with a valid receipt time. Other outcomes are
exit 1 with fixed secret-free diagnostics. They never print submitted text,
tokens or raw server responses. A timeout or invalid reply may follow a
successful server write. After a start attempt has committed state, use
`plan publish` even when start did not acknowledge; repeating start refuses
the existing state. Publishing the current draft again allocates a fresh
sequence and sends a full snapshot; it does not replay an old sequence or
retry in the background. Missing reports may leave incomplete received history.
A 409 conflict can mean stale writer/ordering; do not edit counters to force
acceptance. Stop and recover trusted working state or start a distinct attempt.

## Transfer the reporting role explicitly

Stop the previous writer before handoff. Run the following against its current
private state. Supply the complete native identities: hostname/tool/session ID
and child/parent IDs when applicable. The optional readable name does not
identify a writer. The replacement identity must differ from the current one.

```sh
silkboard-publish plan handoff --config /PRIVATE/config.json <<'JSON'
{
  "schema_version": 1,
  "task_id": "c53ba3b1-c55e-45a3-98af-a061aa972224",
  "from": {
    "hostname": "development-vm", "tool": "codex",
    "session_id": "native-session-a"
  },
  "to": {
    "hostname": "next-development-vm", "tool": "codex",
    "session_id": "native-session-b"
  }
}
JSON
```

`Task plan handed off locally` confirms only a synchronized local binding
change. This command makes no network request. It increments the writer
generation and keeps the task ID, current definition and sequence/revision
high-water marks. The server adopts the generation with the next accepted
publish from the replacement identity. Normal publishes from the old identity
are refused locally. Returning to a former native identity requires another
handoff and a higher generation; generations never reset.

On the same machine, the next session uses the same state directory. To move
to another machine, through an owner-approved private local transfer while
both writers are stopped, copy this task's `plan-HASH.json` state file into
the replacement's private state directory under its unchanged filename.
Preserve mode 0600 inside mode 0700 and keep the same configured endpoint and
project. Use the source's `task_id` inside the file to identify the right
record privately; do not paste its contents in chat or log them. Do not copy
lock files, rename hashed state files, fork active writers or restore stale
copies. Install the destination credential separately. This command does not
perform the transfer or authorize SSH, dashboard reads or remote execution.

## Private working state and failure safety

Each endpoint/project/task key has a stable `plan-HASH.lock` and a mode-0600
`plan-HASH.json`. State contains schema version 1 and the current normalized
`snapshot`, including allocated counters and reporter binding. Unlike progress
counters, plan state includes current descriptions/reports; protect it as
private agent content and keep it outside Git, static assets and build outputs.
It is the current working plan, not a durable list of requests awaiting delivery.
No automatic process reads it to resend old reports.

A nonblocking task lock serializes local allocation/handoff. Busy callers fail
promptly without sending. State is atomically replaced and synchronized before
network work. Concurrent sends can arrive out of order and receive 409; the
next valid full snapshot repairs current state without server reads. Invalid
input does not advance counters. Once storage has been committed, delivery or
reply failure does not roll it back. A storage synchronization error may also
have advanced the file: never guess or reuse an earlier sequence.

Preserve this state across restarts, binary upgrades and approved transfers.
Never use `plan start` to bypass missing state for an existing task.
Missing state for an existing task is not permission to reconstruct counters
from guesses. Recover a trusted current copy or use a fresh task UUID for a
separate attempt. There is no automatic retention or cleanup; operator removal
of retired task state needs both writers stopped and an understanding that
that attempt cannot safely resume without its current state.

Local tests exercise compiled commands without runtime tools, actual Axum
scope/revocation/conflict boundaries, failed allocations/delivery, concurrency,
return handoffs, private transfer and bounded acknowledgement validation.
They do not establish live phone visibility, installation or tagged upgrades.

## Automatic summary workflow context

Install a compatible publisher whose `--help` lists `codex-hook-context`,
`select` and `clear`, and
use the existing explicit `codex.plan_reporting` opt-in. The server must support
per-event workflow IDs (#104). Older publishers, including v0.10.0, do not
maintain this context. Upgrade and review/enable the synchronous local capture
handlers in [Codex setup](codex-publishing.md) before expecting automatic links.
The existing uploader remains asynchronous. Neither handler fetches dashboard
content or grants execution permission.

A Codex `codex-plan start`, or `plan start` with a matching configured Codex
reporter, explicitly selects that workflow. The selection becomes confirmed
only after its plan acknowledgement. The synchronous `codex-hook-context`
handler freezes that task ID at each native event boundary, keyed by the
native event kind and `turn_id`. The asynchronous `codex-hook` uploader reads
only that capture, never the current mutable selection. A delayed summary from
workflow A therefore keeps A after the session starts B. Final summaries may
link after the workflow is marked done. Completion
does not clear context. A new start replaces the prior selection; failed or
unconfirmed new reports leave summaries unlinked rather than pointing to the
previous task. Invalid/repeated starts do not replace a valid selection.

Publishing an older, different workflow does not select it. Publishing the
selected workflow refreshes its confirmation; failed delivery leaves it
unconfirmed until a later selected-plan publish is acknowledged. Preserve
normal best-effort behavior: do not retry the hook or restart task counters.

To resume an existing workflow as the current task, explicitly select it,
then publish its full current graph using the same task ID:

```sh
silkboard-publish codex-plan select --config /PRIVATE/config.json <<'JSON'
{"schema_version":1,"task_id":"c53ba3b1-c55e-45a3-98af-a061aa972224"}
JSON
# Now use codex-plan publish with the existing complete graph.
```

Selection alone remains unconfirmed. `codex-plan select` derives the same
operator/native identity as other explicit Codex commands; input cannot supply
`reporter`. The chosen task must already exist in private working state and
belong to that identity. To switch to unrelated work without a reported graph:

```sh
silkboard-publish codex-plan clear --config /PRIVATE/config.json
```

Clear is local, reads no stdin, and can run with plan reporting disabled.
It does not delete plan state/counters or change workflow status. Publishing
an old plan after clear does not reactivate it; use select then publish.

After an explicit writer handoff, new old-writer captures no longer resolve
the selection. A capture made before handoff keeps its historical association. The new writer must select the existing task and publish; handoff
does not silently assign a current task to that native session. A transfer
back to an earlier identity also needs fresh selection for the new generation.

Operator commands support the same local controls with an explicit reporter:

```sh
silkboard-publish plan select --config /PRIVATE/config.json <<'JSON'
{"schema_version":1,"task_id":"c53ba3b1-c55e-45a3-98af-a061aa972224","reporter":{"hostname":"development-vm","tool":"codex","session_id":"native-session-a"}}
JSON
silkboard-publish plan clear --config /PRIVATE/config.json <<'JSON'
{"schema_version":1,"reporter":{"hostname":"development-vm","tool":"codex","session_id":"native-session-a"}}
JSON
```

For a subagent, provide its full child and parent metadata and select only
when it owns the task's local reporting role. Ordinary delegated subagents
remain unlinked; a parent relationship or shared name alone is insufficient.

Each endpoint/project/host/tool/session/subagent key has a private stable
`context-HASH.lock` and mode-0600 `context-HASH.json`. The latter holds only a
schema version and optional task ID, writer generation and confirmation flag.
The local capture handler reads the explicitly selected task's private working
snapshot solely to validate writer identity/generation; it never scans a
directory for a task. Its mode-0600 `hook-context-HASH.json` stores at most 32
recent event/turn IDs with optional frozen task IDs (16 KiB read limit), using
the same native context lock. It stores no summary text or outgoing requests.
Old entries are evicted at the count or encoded-byte limit; missing, evicted
or conflicting captures yield no
association. There is no resend process. The uploader waits at most 500 ms
for concurrently started capture, then sends an unlinked summary. Missing or
invalid native turn IDs also remain unlinked; mutable selection is never a
fallback. No context lock is held during HTTP delivery.
Context locks precede task locks and use nonblocking acquisition. Unavailable,
corrupt, unsafe, busy, missing or stale context yields an ordinary unlinked
summary, not permission to inspect other files or reset state. Context records
are local selection metadata, not outgoing queues. Keep them private and do
not copy an old native session's selection as part of a writer transfer.

Disabling plan reporting suppresses hook associations but preserves selection.
Native `codex-plan start` rejects disabled reporting without changing context.
A generic `plan start` with a matching configured Codex reporter is still
allowed while disabled; it clears old selection without activating the new
one. Before unrelated work, explicitly clear context (which works while
disabled) so re-enabling cannot revive the earlier task. Unrelated tool
reporters never alter Codex context.

## Descriptions alongside titles

Use short workflow and step titles for lists and graph nodes. Supply optional
`description` on the workflow or individual steps for multi-paragraph Markdown
context (up to 2,000 UTF-8 bytes each). Step `report` is separate evolving
progress text, rather than the definition of the step. These existing fields
are carried by ordinary and explicit Codex plan commands; they are not
restricted to one line. Changing a description changes the plan definition
and advances its revision. Owner step selection opens Description and Agent
report; workflow descriptions appear in the workflow view.
