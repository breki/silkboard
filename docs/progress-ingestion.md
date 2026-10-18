# Progress ingestion

The optional Rust/Axum ingestion service is the storage foundation for
[milestone 2](https://github.com/breki/silkboard/milestone/2), delivered by
[#34](https://github.com/breki/silkboard/issues/34). It accepts short agent
progress messages; there is no feed-reading API yet. Owner login, the feed,
the publisher and native agent integration are separate issues in that
milestone. This document describes local operation, not a production rollout.

## Local setup and credentials

Keep the database in an existing private directory outside the release tree
and static assets. On Unix the directory must have mode 0700 and the database
0600. Symlink files/directories and public permissions are rejected. The
operator must control that directory and its ancestors. SQLite journals need
write access there. Never place credentials or backups in public assets.

From the checkout root, for a local development instance:

```sh
install -d -m 0700 .local/progress
export SILKBOARD_DATABASE="$PWD/.local/progress/progress.sqlite"
cargo run --locked -p silkboard-web -- publisher issue silkboard dev-1 \
  "$PWD/.local/progress/dev-1.token"
cargo run --locked -p silkboard-web
```

The existing frontend build prerequisites in [development](development.md)
still apply. Without `SILKBOARD_DATABASE`, startup retains the status-only
service. A configured database failure fails startup rather than silently
disabling ingestion. Startup rejects a database inside the static directory.
The existing production unit is unchanged and does not enable ingestion;
service storage/configuration belongs to [#39](https://github.com/breki/silkboard/issues/39).

Issuance generates 32 random bytes, encoded as a 64-character hexadecimal
secret, and stores only its SHA-256 hash in the database. The command writes
the secret to a new mode-0600 file, never stdout or arguments. It refuses to
overwrite a file or reuse a project/label pair, including a revoked pair.
The token output must not be the database path or a sibling with the database
filename followed by `-` (reserved for SQLite sidecars); aliases are checked.
If file writing fails after issuance, it attempts revocation and reports
failure. Treat every nonzero result as unsuccessful provisioning.

Use a distinct label per publisher installation. Project IDs contain 1–64
lowercase ASCII letters, digits or hyphens. Labels use the identity limits
below. To revoke:

```sh
cargo run --locked -p silkboard-web -- publisher revoke silkboard dev-1
```

Revocation is idempotent, persists across restarts and leaves messages intact.
Rotate by issuing a new label, transferring its file privately, then revoking
the old label. Delete retired token files separately. Possession of a token
permits publishing as any reported session within its project; host/tool IDs
are descriptive, not cryptographically attested. Tokens grant no dashboard
read, repository, VM, updater or command-execution access. Keep them separate
from GitHub/updater credentials. Do not paste tokens into chat or command
arguments; the shared publishing client will consume the private file in #36.

## HTTP contract

Send `POST /api/projects/{project}/progress`, with `Content-Type:
application/json` and an `Authorization: Bearer` header containing the token.
Use HTTPS except for local loopback development. No token is accepted in a
query parameter or JSON field. No CORS permission is granted.

```json
{
  "event_id": "unique-event-1",
  "hostname": "development-host",
  "tool": "codex",
  "session_id": "native-session-id",
  "sequence": 1,
  "message": "Running tests"
}
```

Optional fields are `subagent_id`, `parent_session_id` and `session_name`.
Omitted optional fields and explicit null are equivalent. A parent ID is
allowed only with a subagent ID. Native tools that report the parent's
session ID for child hooks must include the separate native child ID.

Event IDs and identity fields are 1–256 printable ASCII bytes without spaces.
Messages are nonblank, single-line plain text up to 2,000 UTF-8 bytes; names
have the same text rules with a 120-byte limit. Unknown fields, control
characters and invalid types are rejected. The entire encoded JSON request
is limited to 8,192 bytes, including escapes and whitespace.

The session key is `(project, hostname, tool, session_id, subagent_id)`.
Names do not change identity. Parent metadata is immutable within that key;
send it consistently from the first event. Different hosts, tools, sessions,
children and projects retain separate histories. No local files, transcripts,
handoffs, task state or command logs are expected.

`sequence` is a positive signed 64-bit integer, unique within the session key.
Publishers must persist sequence allocation across resumed sessions and
coordinate writers to the same key. Deliveries may arrive out of order;
history order is ascending sequence, not receipt time. A newer sequence's
optional name describes that event, without rewriting earlier history.

An accepted request returns HTTP 200 with only:

```json
{"event_id":"unique-event-1","received_at":1792281600000}
```

`received_at` is server receipt time in Unix milliseconds. Retry the complete
event unchanged with its original ID and sequence. An exact retry returns
the original acknowledgement even after restart, with no second insertion.
Reusing a project-wide event ID with different content, a session sequence,
or conflicting parent metadata returns 409. Such conflicts must be resolved
locally rather than retried as successful delivery.

Invalid credentials (including wrong-project/revoked credentials) return 401.
Invalid values return 400; malformed JSON is 400, schema/type errors 422,
unsupported content type 415 and oversized bodies 413. Extraction errors
can precede authentication. Storage failure or saturated database admission
returns 503; retry with backoff. Successful acknowledgements and storage
errors are marked `Cache-Control: no-store`. Eight admitted database
operations run on blocking workers, outside Tokio's async worker threads.
This is bounded concurrency, not a per-publisher rate limit or disk quota.

There is no GET/history route; a publisher token cannot read another event.
`/health` and `/api/status` retain their existing unauthenticated responses.

## Persistence and schema compatibility

Schema 1 initializes projects, hashed credentials, sessions and events in one
SQLite transaction. Startup accepts schema 1 and refuses newer schema
versions; it never resets an incompatible database. Writes authenticate,
resolve identity and insert events in an immediate transaction, so concurrent
retries or revocation cannot partially insert messages. Database lock waits
are capped at two seconds. SQLite's default rollback journal and full
synchronization are retained; no WAL mode or relaxed durability is enabled.

Retain the database directory outside replaced release directories. A prior
status-only binary ignores this data; future schema changes need explicit
migration/rollback compatibility. This issue tests reopen/restart persistence,
not production upgrade or backup/restore procedures; those are part of #39.

Run `cargo test -p silkboard-web` for HTTP authorization, bounded input,
idempotency, session identity, persistence, schema and operator-file checks.
