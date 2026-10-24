# Task-plan reporting contract

Status: milestone 5 design contract for
[issue #73](https://github.com/breki/silkboard/issues/73).
The endpoints, payloads and local handoff below are planned, not implemented.
The existing [progress API](progress-ingestion.md) remains unchanged.

## Agreed behavior and authority

The owner selected an agent's evolving task-specific plan, rather than a fixed
workflow. A directed edge means that a step requires another step first.
Several steps may be active at once. A completed step can be reopened under
its existing identity; its earlier completion remains in received history.
A new Codex session can continue the same graph through an explicit handoff.
An independent attempt gets a new task identity even for the same GitHub issue.

GitHub remains authoritative for tasks and acceptance. Silkboard stores the
agent's observations and presents them to the owner. It does not assign work,
fetch tasks for agents, run commands, control VMs, or verify reported completion.
Plan text and GitHub references never confer execution permission.

Use the existing project-scoped publisher credential for writes and separate
owner authentication for reads. Keep the first-owner setup gate. Neither
owner cookies nor tailnet membership alone permit publishing; publisher tokens
cannot read plans. Host/tool/session fields are descriptive, not attested.
A compromised publisher can misreport within its authorized project.

## Task, step and reporting identity

A task is one tracked undertaking, identified by `(project, task_id)`.
`task_id` is a client-generated canonical lowercase UUID v4. It is independent
of a native session ID, title and optional GitHub issue reference. Several
tasks may refer to one issue, and one session may work on several tasks.

A step has a task-local `id`: 1–64 ASCII characters, matching
`[a-z0-9][a-z0-9_-]*`. Use the same ID when continuing, clarifying or reopening
the same conceptual work. Do not infer identity from labels, array positions
or graph layout. Do not reuse an old ID for different work. Removed steps and
their observed states remain in historical snapshots; a revision may restore
the same conceptual step under its original ID.

Exactly one reporting writer maintains a task at a time. This does not prevent
multiple active steps or ordinary agent delegation; this milestone does not
add independent child writers or multi-agent task assignment. A writer binding
has a positive task-local `writer_generation` and reporter metadata using the
existing hostname/tool/native-session/child/parent/name conventions. A native
session change requires a new generation. The optional readable name may
change; other reporter identity fields are fixed within one generation.

### Explicit session handoff

Maintain local task reporting state: task ID, last allocated sequence, current
plan revision, current plan and writer generation/binding. This is the current
working plan and its counters, not a queue of outbound reports. Keep it private
and transfer it through an owner-approved local handoff, separately from tokens.
Silkboard supplies no task-content download or handoff-upload API.

Stop the previous reporting session before transferring the state. The next
session retains the task ID and counters and increments the writer generation.
Normal updates preserve that generation. The local publisher must refuse
reports from a native identity that no longer owns its local binding. Preserve
the generation high-water mark across handoffs, including a return to an earlier
native session. Do not fork the local task state between active reporters.

For an existing task, a new snapshot must have a higher task sequence. Then:

- A lower generation is a conflict, even if its sequence is higher.
- The same generation requires the same reporter identity, excluding its
  optional readable name.
- A greater generation establishes a new reporter binding atomically with the
  accepted snapshot. Generations may skip: several explicit session handoffs
  may have happened while reports were lost. Do not fabricate missing bindings.

Competing native identities using the same generation cannot both be accepted.
Once a higher generation is accepted, delayed normal reports from earlier
generations cannot become current. An identical previously accepted replay
can still receive its original acknowledgement without changing current state.

These counters are correctness fences for cooperating publishers, not proof of
ownership or new authentication/execution authority. A project credential can
misreport a generation just as it can misreport other project observations.
The contract does not implement a distributed task-claim service. Losing local
task state is not permission to guess counters or silently overwrite an existing
graph. Recover trusted handoff state or start a distinct task attempt.

## Complete snapshot payload

Reserve `POST /api/projects/{project}/plans` for authenticated full snapshots.
Use JSON, the existing bearer-token placement and verified HTTPS transport.
Do not overload the 2,000-byte message field or change the existing progress
request's 8,192-byte limit. Plan payloads have their own limits below.

Example: two implementation branches are active after agreeing the contract.

```json
{
  "schema_version": 1,
  "task_id": "c53ba3b1-c55e-45a3-98af-a061aa972224",
  "writer_generation": "1",
  "sequence": "4",
  "plan_revision": "2",
  "reporter": {
    "hostname": "development-vm",
    "tool": "codex",
    "session_id": "native-session-example",
    "subagent_id": null,
    "parent_session_id": null,
    "session_name": "Task-plan feature"
  },
  "title": "Add task-progress graphs",
  "description": "Show the evolving plan and the work currently covered.",
  "github_issue": {"repository": "breki/silkboard", "number": "73"},
  "steps": [
    {
      "id": "contract",
      "title": "Define task-plan reporting",
      "description": "Agree identity, revisions and dependency rules.",
      "depends_on": [],
      "state": "done",
      "report": "Contract agreed.",
      "blocked_reason": null
    },
    {
      "id": "api",
      "title": "Implement storage and API",
      "description": "Persist authenticated plan snapshots.",
      "depends_on": ["contract"],
      "state": "active",
      "report": "Working on snapshot validation.",
      "blocked_reason": null
    },
    {
      "id": "ui",
      "title": "Implement graph view",
      "description": "Render the owner view using the agreed contract.",
      "depends_on": ["contract"],
      "state": "active",
      "report": "Working on graph layout.",
      "blocked_reason": null
    },
    {
      "id": "verify",
      "title": "Verify the combined feature",
      "description": "Exercise the complete reporting and viewing path.",
      "depends_on": ["api", "ui"],
      "state": "pending",
      "report": null,
      "blocked_reason": null
    }
  ]
}
```

`schema_version` is the integer 1. All identity fields, `title`, `steps`,
`sequence`, `plan_revision`, `writer_generation`, `reporter` and each step's
identity/title/state/dependency array are required. `description`, `report`,
`github_issue`, reporter optional identity/name fields and `blocked_reason`
may be omitted or null where applicable; both forms mean absence. Unknown
fields and duplicate JSON object members are rejected.

`sequence` is the strictly increasing order of snapshots for the whole task,
including across handoffs. `plan_revision` identifies a plan definition.
These and `writer_generation` are canonical decimal strings from 1 through
9223372036854775807, with no leading zeros. Use strings in all requests,
acknowledgements and owner responses so JavaScript does not round identities or order values.

The plan definition consists of task title/description/GitHub reference and
the ordered step array with each ID, title, description and prerequisite set.
Changing this definition requires increasing `plan_revision`. State, report,
blocked reason or reporting-session changes alone do not require increasing
it. Every snapshot still needs a new task sequence. Prerequisite ordering is
not meaningful; duplicate prerequisite IDs are invalid. Step array ordering
is a presentation hint, not dependency order.

### Bounds and content

| Value | Limit |
| --- | --- |
| Encoded JSON request, including escapes | 262,144 bytes (256 KiB) |
| Steps in a snapshot | 1–128 |
| Directed prerequisite edges in total | 512 |
| Prerequisites on one step | 32 |
| Task/step title | Nonblank, 1–160 UTF-8 bytes, single line |
| Optional task/step description | Nonblank, at most 2,000 UTF-8 bytes |
| Optional step report | Nonblank, at most 2,000 UTF-8 bytes |
| Blocked reason | Nonblank, at most 1,000 UTF-8 bytes |
| Reporter identity/name fields | Existing progress identity/name limits |

All field limits and the total body limit apply independently; using the
maximum length for every field may exceed the body limit. Plain titles must
have no control characters. Multiline text follows existing message rules:
LF, CR and tab are allowed; other control characters are rejected. Preserve
accepted text rather than silently truncating or normalizing it. Render any
Markdown using the existing safe renderer; titles and identity remain text.
No images, attachments, raw executable markup or remote-content fetching.

A GitHub reference is optional metadata, not a URL supplied by an agent.
`repository` has exactly two nonempty ASCII segments joined by `/`, at most
100 bytes in total; each segment uses letters, digits, `.`, `_` and `-`, but
is neither `.` nor `..`. `number` uses the positive decimal-string rule above.
Construct a fixed `https://github.com/` issue link from these validated values.
Do not fetch the issue or add repository credentials to Silkboard.

## Graph states, revisions and reopening

Reject duplicate step IDs, unknown prerequisite IDs, self-edges and cycles.
Disconnected branches and multiple roots are valid. Validate the complete
snapshot atomically; never partially accept nodes or edges.

| Step state | Meaning and constraint |
| --- | --- |
| `pending` | Not currently reported as started |
| `active` | Work underway; all direct prerequisites must be `done` |
| `done` | Agent reports completion; all direct prerequisites must be `done` |
| `blocked` | Cannot continue; a blocked reason is required |

A blocked reason is allowed only for `blocked`; absence/null is required for
other states. Blocking may describe failed checks or waiting for owner input.
The initial contract has no separate failed, skipped or cancelled state.
A step report gives context but is not trusted verification evidence.

Reopening uses the same ID and changes its current state to pending, active
or blocked. If a prerequisite is reopened, the reporter must update dependent
active/done steps so the full snapshot satisfies the dependency constraints.
The server rejects an inconsistent snapshot instead of silently rewriting
states or scheduling work. All earlier accepted completions remain readable.

A scope revision may add, edit, remove or restore steps. Removing a node must
also remove/revise references to it. Removed work and its previous completions
remain in history; the current graph's denominator covers its current steps.
A label edit does not authorize reusing an ID for different conceptual work.

Compute the overall presentation from reported step states: all done means
`completed`; otherwise any active means `active`; otherwise any blocked means
`blocked`; otherwise `pending`. A task may therefore show active work alongside
a blocked branch. A later valid snapshot can reopen a completed task. Neither
completion nor receipt closes a GitHub issue, approves a merge or publishes a
release. A count of completed steps is not an estimate of remaining effort.

## Ordering, replay and missed updates

After authentication, parse and validate the request before storing it. Within
one transaction apply these rules in order:

1. If `(project, task_id, sequence)` already exists, compare the validated
   payload. An identical replay returns its original acknowledgement and
   receipt time, without changing current state or freshness. Different
   content at the same sequence is a conflict.
2. For a new sequence on a known task, require it to be higher than the latest
   accepted sequence. Lower unseen sequences are conflicts, not backfilled
   history. Enforce writer-binding and handoff rules.
3. Require `plan_revision` to be at least the latest accepted revision. At the
   same revision the plan definition must match. A changed definition needs
   a greater revision. Gaps are permitted; a greater revision may have an
   identical definition if unreceived intermediate edits were undone.
4. Store the complete snapshot and reporting binding, then move the task's
   current pointer to that snapshot. Keep earlier accepted snapshots immutable.

Comparison ignores JSON member order and insignificant JSON whitespace;
omitted/null optional fields are equivalent. Compare prerequisite sets without
ordering. Preserve step order and the actual accepted text. Authentication is
still checked for retries, including token revocation and project scope.

A task's first received snapshot may have any positive sequence, revision and
writer generation: initial reports or an initial handoff may have been lost.
Accept a valid full snapshot without fabricating missing history. Do not infer or verify earlier
reporting sessions from a generation counter. Expose gaps as incomplete received history.

Allocating a sequence does not imply delivery. The publisher must reserve
counters safely, keep their allocated high-water mark across failures and
handoffs, and never reuse a sequence for different content. Recovery needs no
server reads: the next fresh snapshot carries the full current plan. There
is no automatic retry loop, daemon or durable queue of previous requests.
A manually retried snapshot must reuse its exact identity and content.

A successful write returns HTTP 200 with only `task_id`, string `sequence`
and original server `received_at` (Unix milliseconds). No current plan,
credentials, commands or permission-bearing response data is returned.
Invalid graph/fields yield 400, wrong content type 415 and an oversized body
413. Invalid/revoked/out-of-scope publisher credentials yield 401. Ordering,
writer or revision conflicts yield 409; uninitialized owner, unavailable
storage or exhausted admission yield 503. Do not disclose current task state
in conflict responses. Error precedence for malformed unauthenticated bodies
must not reveal whether a task exists.

## Owner reads and freshness

Owner APIs need bounded task summaries across projects, a current full graph,
exact earlier snapshots and revision/history navigation. Use the existing
owner-only, no-store, same-origin/CSRF and safe rendering conventions. Never
expose token hashes, raw request logs or updater/development credentials.
Route names and storage indexes are implementation choices in #75; the
following observable behavior is required:

- List at most 20 summaries per page, with stable opaque cursors. A summary
  includes identity, title, reported overall state/counts, active/blocked step
  IDs, current revision and last accepted receipt time, not every snapshot body.
- Fetch one bounded full snapshot at a time. Historical selection names its
  exact task sequence or a revision's latest accepted sequence. It shows states
  as last reported at that snapshot, not states silently copied from today.
- Paginate received history and distinct plan-revision summaries. Do not invent
  unreceived intermediate plans or require an unbounded history download to
  find an earlier revision. Unknown task/snapshot yields owner-side 404.
- Preserve server receipt time separately from reporter content. Show last
  accepted report time; an identical replay does not refresh it. Silence alone
  changes no reported state and proves neither activity nor completion.
- Mark historical views and incomplete received history clearly. Staleness
  is a presentation of age, not a background agent-health inference.

The first version does not automatically delete received snapshots. Request
limits bound individual work, not lifetime database growth. Implementation
must document capacity and bounded read/admission behavior; independent backup
and recovery policy remains [#52](https://github.com/breki/silkboard/issues/52).

## Delivery and verification boundary

[#75](https://github.com/breki/silkboard/issues/75) implements storage/APIs and
migration/recovery compatibility; #76 extends the installed publisher; #77
integrates actual Codex plan reporting; #78 implements the owner view; #79
exercises reconciliation/freshness; #80 owns combined verification; #81 owns
an authorized release and real dogfooding. These are future implementations,
not claims that current turn-summary hooks already expose complete plans.

A new data schema needs explicit compatibility and recovery handling before
release; switching an old binary back cannot undo a migration. This contract
does not change schema 3, existing publisher requests, hook trust, production
configuration or release authorization. The mockup's completed steps are
fictional example data.
