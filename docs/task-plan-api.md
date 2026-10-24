# Private task-plan API

Implemented for [#75](https://github.com/breki/silkboard/issues/75), with local
verification only. This does not install a production migration. The installed
publisher, Codex hooks and owner pages do not yet send or display plans; those
are separate milestone 5 issues. The [wire contract](task-plan-contract.md)
defines fields, graph validation, ordering and writer handoff.

## Publishing

`POST /api/projects/{project}/plans` accepts one complete JSON snapshot with
`Authorization: Bearer PUBLISHER_TOKEN` and `Content-Type: application/json`.
Use the existing project-scoped publisher token; first-owner setup must be
complete. Owner cookies cannot publish. The server authenticates before
reading/decoding plan content and checks revocation again in the write
transaction. It never returns the current plan to a publisher.

Success is HTTP 200 with `task_id`, decimal-string `sequence`, and the original
server `received_at` in Unix milliseconds. An exact normalized retry returns
the same acknowledgement, including after newer reports arrive. Prerequisite
order and omitted versus null optional fields normalize identically; node
order remains meaningful. A conflicting retry or stale sequence, revision or
writer generation returns 409. Invalid content is 400, unauthorized access 401,
a body over 262,144 bytes 413, non-JSON content 415, and unavailable storage or
exhausted worker admission 503. Errors are fixed messages without submitted
content or credentials. Responses use `Cache-Control: no-store`.

Both encoded request and normalized snapshot must fit 262,144 bytes. All
individual field, node and edge limits still apply. Existing progress requests
retain their separate 8,192-byte limit. Eight plan requests may be admitted at
once; database work runs off the async executor. Delivery retries and local
writer-state management belong to the future publisher integration.

## Owner reads

These GET routes use the existing owner session cookie and owner security
headers. Publisher tokens and tailnet membership cannot read them. No token
is accepted in a query string.

| Route | Result |
| --- | --- |
| `/api/owner/plans` | Latest summary for each task across projects |
| `/api/owner/projects/{project}/plans/{task_id}` | Current snapshot, summary and prior completions |
| Same task route with `?sequence=N` | Exact received snapshot |
| Same task route with `?revision=N` | Latest received snapshot within that definition revision |
| Same task route with `/history` | Received snapshot summaries, newest sequence first |
| Same task route with `/revisions` | One latest summary per received revision, newest revision first |

Sequence and revision selectors are mutually exclusive, canonical positive
decimal strings up to signed 64-bit maximum. Unknown queries and malformed
IDs/cursors return 400; absent tasks, reports and revisions return 404.
Unauthorized owner requests return 401. Storage failure is 503.

Pages contain `items` and nullable `next_cursor`, with at most 20 summaries.
Pass a non-null cursor back as the URL-encoded `before` query value on the same
route. The all-task cursor is opaque; history/revision cursors are decimal
strings. All-task ordering is receipt time descending, then project and task
ID descending. Updates can move tasks between list pages: pagination does not
freeze the database. History and revision pages do not transfer full snapshots.

Each summary contains project/task identity, string sequence/revision/generation,
title, reporter, derived state, counts, active/blocked step IDs, server receipt
time and `history_incomplete`. It contains no full step descriptions/reports.
Detail contains `snapshot`, `summary` and `prior_completions`. For a currently
non-done step, the latter records its latest received transition to done at or
before the selected snapshot: step ID, sequence and receipt time. It does not
invent unreceived transitions or imply independent verification. Full received
snapshots preserve earlier definitions, removed nodes and reports.

## Storage and compatibility

Plans share the existing private SQLite file, permissions and backup lifecycle.
The base schema remains version 3. An independently versioned additive component
`plan_schema` is version 1, with current tasks, immutable snapshots, latest
revision summaries and indexed step-state transitions. Initialization creates
all tables transactionally. Partial components and newer component versions
fail closed; existing base data and credentials are not rewritten.

A local drill used the actual signed `v0.6.0` application source at
`07026bbd0d6a1feff9d04a4e47856d1611afa00d`: it opened a database containing
plans, served health/feed, published ordinary progress, and backed up/restored
all tables. The new application read the retained plan afterwards. Restore
revoked all publisher tokens and browser sessions as before. This demonstrates
compatibility with that retained revision, not every past/future binary or a
production rollback. The older application cannot validate the new component;
use a current compatible binary for operator snapshots.

Current backup/restore checks the optional plan component without migrating
it and copies its tables consistently. Restore retains plans while invalidating
authority. Before a production rollout, take a verified independent private
snapshot and follow [application-data recovery](application-data.md). Reverting
application code is not a general database downgrade. No production deployment,
backup schedule, retention policy or off-machine destination is introduced here.
