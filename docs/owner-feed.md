# Owner progress feed

The [#38](https://github.com/breki/silkboard/issues/38) feed shows published
agent progress using the existing [owner login](owner-login.md) and
[ingestion database](progress-ingestion.md). Release and health information
remains public. Production v0.3.0 has private storage and verified owner phone
setup/login. The owner confirmed real reports from two distinct Codex sessions
and expanded history. Live unauthenticated and publisher-only owner reads
return 401. Remaining rollout evidence is
[#39](https://github.com/breki/silkboard/issues/39); backups/recovery are #52.

The unified starting table was delivered in [#59](https://github.com/breki/silkboard/issues/59).
The compact header and separate status page are [#61](https://github.com/breki/silkboard/issues/61);
production rollout of that navigation change is separate from implementation.

## Using the page

Open the configured private HTTPS site and enter the owner password. The
starting view is one table of individual events across all projects, with
Time, Project, Agent (tool and host), and a short Message preview. Long labels
and previews stay compact; full values are available in the expanded details.
No project selection is required.

The compact header has a top-right Navigation menu. Open it by touch or with
Enter/Space; Tab reaches its links and owner actions. Escape closes it and
returns focus to the menu button. Clicking or moving focus outside also closes
it. Events links to the starting view. Application status opens `/status`,
which shows public health, version, release tag and commit, with navigation
back to Events. It supports direct links, reloads and browser history.

When signed in, the menu also contains Manage publisher credentials and Sign
out. The credential manager opens inline on the current page; Close credential
manager removes it and any disclosed token. Logout or expiry also removes the
manager. Owner actions still require the existing authenticated APIs. The
status page does not fetch event data and remains available without signing in.
While signed in on that page, a visibility-aware session check runs every ten
seconds to detect expiry or revocation and clear owner controls and any open
credential manager. It stops on logout or page teardown.

Click a row, or focus its message button and press Enter or Space, to expand
that event beneath the row. One event is expanded at a time. Details include
the full message, receipt time, optional readable session name, host/tool,
native session/child/parent identity. Event ID and Sequence are hidden from
the details view; the API retains them for identity and ordering. Readable names
never merge identities. The event uses the most recent non-null name at or
before its session-local sequence, so a later rename does not relabel an
earlier event.

Events appear newest-received first. Time is server receipt time, displayed
in the browser's local timezone, rather than agent event time. The table shows
zero-padded 24-hour `HH:mm` above `MM-DD`, without seconds or year. Expanded
details show `yyyy-mm-dd HH:mm:ss`, for example `2026-10-23 15:04:05`. These
formats do not follow the browser's locale date-order or 12-hour preference.
Equal receipt times
are ordered deterministically by project and event ID. Sequence numbers only
order a session's reports; they are not comparable across projects/sessions.
Earlier/latest controls page through 20 events. Return to the latest page to
see new events ahead of an older cursor.

The visible page refreshes every ten seconds, with at most one event-list
request in flight. Hidden tabs pause polling and abort active requests;
returning resumes it. Expanded details follow the event's project/event ID
across refreshes and close if the event leaves the displayed page. Pagination
also closes details. Failed refreshes retain displayed events with a warning
and retry action. Empty, loading and unavailable states explain what happened.
Expired/revoked login clears events and details and asks the owner to sign in.

Sign out hides the feed immediately and revokes the browser session. If the
request fails, the feed stays hidden with a retry action; the page does not
claim server logout succeeded. Passwords and messages are not saved in local
storage. Event messages render as sanitized Markdown in expanded details,
with emphasis, inline code and safe links. The current publisher API accepts
single-line messages; see the [input contract](progress-ingestion.md). Table
previews use compact text extracted from rendering; formatting-only events
show View event. Raw HTML remains literal text and Markdown images show their
labels without loading remote resources. Unsafe link destinations are removed.
Session and host labels remain plain text. Commands in messages grant no agent
execution authority. Agents continue to publish only; the feed grants
no repository, development-file, VM-control, dispatch or execution authority.

The Markdown and timestamp presentation is [#64](https://github.com/breki/silkboard/issues/64);
its production rollout is separate from implementation.

## Read API

All routes require the owner cookie inside the protected `/api/owner` router
and return `Cache-Control: no-store`. Publisher tokens cannot read them;
owner cookies cannot publish progress.

| GET path | Optional cursor | Items |
| --- | --- | --- |
| `/api/owner/events` | `before=CURSOR` | Individual events across projects |
| `/api/owner/projects` | `after=PROJECT` | `{id}` |
| `/api/owner/projects/PROJECT/sessions` | `before=ID` | Sessions |
| `/api/owner/projects/PROJECT/sessions/ID/events` | `before=SEQUENCE` | Messages |

Responses contain `{items: [...], next_cursor: string | null}`, with at most
20 items. Project IDs use ingestion slug rules. IDs/sequences are canonical
positive signed-64-bit integers encoded as strings to preserve JavaScript
precision. Malformed cursors/unknown query parameters return 400. Missing
projects or sessions, including a session from another project, return 404.
Existing empty projects return empty pages. Invalid/expired owner sessions
return 401; storage/admission failures return 503.

The global event cursor is an opaque JSON string, URL-encoded as a query value.
It contains receipt time, project and event ID and must be returned unchanged.
It does not use SQLite row IDs, so database maintenance does not invalidate
its ordering. A global event contains `project`, `hostname`, `tool`,
`session_id`, nullable `subagent_id`, `parent_session_id`, `session_name`,
and the four message fields below. Full owner-visible details arrive in the
bounded response; the UI reveals them on activation. Raw payloads and
credentials are never returned. Compatible project/session/history routes
remain available, though the starting view no longer uses project cards.

A session contains `id`, `hostname`, `tool`, `session_id`, nullable
`subagent_id`, `parent_session_id`, `session_name`, and `latest`. A message
contains `event_id`, string `sequence`, numeric `received_at` (Unix
milliseconds), and `message`. Latest means highest sequence, not latest
delivery. Responses omit credential hashes and raw stored request payloads.
The global feed adds compatible read indexes for receipt ordering and named
reports. Existing rows and schema version 3 remain unchanged.

## Verification

`cargo test -p silkboard-web --test feed` checks authorization, identity
separation, names, ordering, pagination and exact large integers. Frontend
component/API tests cover empty/error states, expiry, cancellation and unsafe
text; coverage gates apply to the new code.

`cargo xtask browser-test` uses built assets, Chromium and a disposable HTTPS
proxy to the real Rust backend. It requires OpenSSL and free dedicated
loopback ports 38124/38125 for the feed and 38126/38127 for the separate
first-owner setup proxy/backend. All four ports must be free. A private
temporary directory holds the local
database, test password setup, publisher credential and self-signed certificate;
the fixture removes its files on shutdown. Only this fixture trusts its
certificate; production TLS/cookie policy is unchanged. Tests check real
secure-cookie login/logout, reset-driven expiry, safe text, cross-project
events, inline details, pagination, refresh and phone width. Local checks do
not establish production dogfooding or
replace #39's live rollout evidence.
