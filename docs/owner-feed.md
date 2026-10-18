# Owner progress feed

The [#38](https://github.com/breki/silkboard/issues/38) feed shows published
agent progress using the existing [owner login](owner-login.md) and
[ingestion database](progress-ingestion.md). Release and health information
remains public. Production storage, secure phone credential entry, backups
and rollout remain [#39](https://github.com/breki/silkboard/issues/39) work.

## Using the page

Open the configured private HTTPS site, enter the owner password and choose
a project. Cards show the optional readable name, host, tool, native session
identity, child/parent identity, latest message and server receipt time.
Readable names never merge sessions. The latest non-null name persists when
later events omit it.

Sessions appear newest-created first. Expand one card to read messages,
highest sequence first, with server receipt timestamps. Retried/out-of-order
deliveries do not change sequence ordering; receipt time is not agent event
time. Earlier/latest controls page through 20 sessions or messages. Projects
also use bounded pages. Return to the first/latest page to see new entries
ahead of the current cursor.

The visible page refreshes every ten seconds, with at most one request per
list in flight. Hidden tabs pause polling and abort active list requests;
returning resumes it. One history can be expanded at once. Changing projects
cancels old reads. Failed refreshes retain displayed messages with a warning
and retry action. Empty, loading and unavailable states explain what happened.
Expired/revoked login clears the feed and asks the owner to sign in again.

Sign out hides the feed immediately and revokes the browser session. If the
request fails, the feed stays hidden with a retry action; the page does not
claim server logout succeeded. Passwords and messages are not saved in local
storage. Progress is plain text: embedded HTML, links and commands do not
become executable controls. Agents continue to publish only; the feed grants
no repository, development-file, VM-control, dispatch or execution authority.

## Read API

All routes require the owner cookie inside the protected `/api/owner` router
and return `Cache-Control: no-store`. Publisher tokens cannot read them;
owner cookies cannot publish progress.

| GET path | Optional cursor | Items |
| --- | --- | --- |
| `/api/owner/projects` | `after=PROJECT` | `{id}` |
| `/api/owner/projects/PROJECT/sessions` | `before=ID` | Session cards |
| `/api/owner/projects/PROJECT/sessions/ID/events` | `before=SEQUENCE` | Messages |

Responses contain `{items: [...], next_cursor: string | null}`, with at most
20 items. Project IDs use ingestion slug rules. IDs/sequences are canonical
positive signed-64-bit integers encoded as strings to preserve JavaScript
precision. Malformed cursors/unknown query parameters return 400. Missing
projects or sessions, including a session from another project, return 404.
Existing empty projects return empty pages. Invalid/expired owner sessions
return 401; storage/admission failures return 503.

A session contains `id`, `hostname`, `tool`, `session_id`, nullable
`subagent_id`, `parent_session_id`, `session_name`, and `latest`. A message
contains `event_id`, string `sequence`, numeric `received_at` (Unix
milliseconds), and `message`. Latest means highest sequence, not latest
delivery. Responses omit credential hashes and raw stored request payloads.
These reads introduce no schema migration.

## Verification

`cargo test -p silkboard-web --test feed` checks authorization, identity
separation, names, ordering, pagination and exact large integers. Frontend
component/API tests cover empty/error states, expiry, cancellation and unsafe
text; coverage gates apply to the new code.

`cargo xtask browser-test` uses built assets, Chromium and a disposable HTTPS
proxy to the real Rust backend. It requires OpenSSL and free dedicated
loopback ports 38124/38125. A private temporary directory holds the local
database, test password setup, publisher credential and self-signed certificate;
the fixture removes its files on shutdown. Only this fixture trusts its
certificate; production TLS/cookie policy is unchanged. Tests check real
secure-cookie login/logout, reset-driven expiry, safe text, history, refresh
and phone width. Local checks do not establish production dogfooding or
replace #39's live rollout evidence.
