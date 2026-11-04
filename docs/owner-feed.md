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

Open the configured private HTTPS site and enter the owner password. Agents
is the home page at `/`. Events is at `/events`; Workflows remains at `/plans`,
and existing workflow snapshot links remain valid.
Applied filters, paging and selections are [bookmarkable](deep-links.md);
reload, login and browser Back/Forward restore their URL state.

Agents shows one card per main/root agent session, preserving project, host,
tool and native session identity. [Issue #120](https://github.com/breki/silkboard/issues/120)
revises the earlier mixed session/subagent table. Explicit child and parent
metadata exclude subagents; names never select or merge sessions. The API
selects main agents before cursor pagination so child reports consume no slots.
A child's activity is never rolled into a parent's latest activity. Child
events remain in Events and its session hierarchy; workflow snapshots remain
available in Workflows and its received history.

Cards use one column on narrow phones and an adaptive grid on larger screens.
Each card shows its readable name or tool/host fallback, tool/host context,
project, latest activity and browser-local receipt date and time including
seconds and year. Expand Report & identity to see the exact native session.
Long labels, titles and IDs wrap within the card.

Agents, Events and the Workflows list use a desktop project index. Phones use
a searchable Project chooser instead. Selecting a project resets the dependent
agent choice and returns to the latest page. More projects loads the next
bounded set of choices; project counts are not inferred from the current page.
The Agents chooser searches the complete main-session history by latest
readable name, project, activity text, tool or host. Its choices include project,
tool/host and native session identity so equal names remain distinguishable.
Selecting one filters by project, host, tool and native session together.

Latest activity is selected across all stored events and workflow snapshots
before pagination, using server receipt time rather than session counters.
Historical workflow writers keep their own activity after a handoff. A workflow
card opens the reported snapshot, matching its displayed status. A later-received
report is the newest observation even if the work happened earlier. Equal
receipt times prefer workflows, then descending item key and workflow sequence;
main agent cards use descending full identity to break receipt-time ties.

An event card opens Events with its exact full identity in the URL. The visible
Exact agent session scope distinguishes the primary session from each child,
remains through paging and view switching, and survives reload and browser
Back/Forward. Applied time ranges still narrow this exact scope. Clear session filter
removes that scope and restores Project/Agent selection. Incomplete identity
links show an error and fetch no event data until cleared.

Agents pages contain up to 20 main identities with Older agents and Latest agents.
New activity can move an identity toward the latest page while paging; return
to Latest agents to see it. Visible pages poll every ten seconds, pause while
hidden, retain stale cards with a retry warning on failure, and clear private
content on login expiry. Publisher tokens cannot read this owner-only API.

Events shows one table of individual events across all projects, with Time,
Project, Agent (tool and host), and a short Message preview. Long labels and
previews stay compact; full values are available in expanded details.
No project selection is required.

The compact header has a top-right Navigation menu. Open it by touch or with
Enter/Space; Tab reaches its links and owner actions. Escape closes it and
returns focus to the menu button. Clicking or moving focus outside also closes
it. Agents, Events, Workflows and Todos are always visible primary links.
Desktop uses one header row; phones place all four links beneath the brand and
menu button. The phone menu opens beneath both rows so primary links remain
reachable while it is open. Each link remains a touch target, with a visible keyboard focus
outline. The current primary page has bold text and a bottom marker, and is
announced as the current page by assistive technology. Workflow detail and
snapshot links keep Workflows active; Application status has no active primary.
Application status opens `/status`,
which shows public health, version, release tag and commit, with navigation
back to the primary pages. It supports direct links, reloads and browser history.

The [Todos checklist](todos.md) is a primary link, with no duplicate in the menu.
When signed in, the menu also contains Manage publisher credentials and Sign
out. The credential manager opens inline on the current page; Close credential
manager removes it and any disclosed token. Logout or expiry also removes the
manager's private content. Its URL can reopen the panel after signing back in.
Owner actions still require the existing authenticated APIs. The
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

When an event explicitly names an existing workflow in its project, an
Open workflow link appears beside its message preview. Touch it or focus it
with Tab and press Enter to open the workflow's existing graph view. The
message button still expands details independently. Events with no explicit
association, including older events, show no link. Compatible opted-in Codex
publishers with reviewed local capture hooks attach confirmed workflow context
frozen at the native event boundary to automatic summaries;
missing or unconfirmed context remains unlinked. Sharing a session or name
is not enough. Associations stay with each event across pagination and
workflow session handoffs. A missing workflow can become linked after its
first report arrives. See [the ingestion contract](progress-ingestion.md).

Project and Agent open searchable selection lists. Search narrows available
choices without changing the active filter; selecting a choice applies it
immediately. All projects and All agents remove that filter. Choices come
from the complete Events dataset, with More projects/agents for longer lists,
not just the displayed page. Agent labels show tool and host. Escape closes
a picker and returns focus to its button; Tab and Enter reach and choose items.

Open All time (or the current range) beside the clock icon to show the
Received time dialog, with recent-period presets and a custom range editor.
Presets apply immediately and end at their selection time. Custom From/To dates use
YYYY-MM-DD and 24-hour HH:mm in the browser's local timezone; Apply range
applies both endpoints together. Leave both fields of an endpoint empty for
no bound. Invalid dates, nonexistent local daylight-saving times and reversed
ranges show a warning and retain the last valid bounds. Cancel and Escape
discard unapplied edits. Project and Agent can still change independently.
Clear filters resets all controls and returns to the latest page.
Failed choice loading offers Retry; existing selections remain active.

Flat list displays events newest-received first. Time is server receipt time, displayed
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
including headings, emphasis, lists, quotes, code blocks, tables and safe
links. Messages may include line breaks and tabs under the existing size
limits; see the [input contract](progress-ingestion.md). Table
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
| `/api/owner/agents` | `before=CURSOR` | Latest activity per native identity |
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

An Agents item represents a main session with null child/parent identifiers.
It contains `project`, `hostname`, `tool`, `session_id`, nullable
`subagent_id`, `parent_session_id`, `session_name`, numeric `received_at` and
`latest`. An event activity has `kind: "event"`, `event_id` and `message`;
a workflow activity has `kind: "workflow"`, `task_id`, string `sequence`,
`title` and reported `state`. Return its opaque JSON cursor unchanged,
URL-encoded as `before`; it contains receipt time and full native identity.
The Agents API accepts optional `project`, `hostname`, `tool` and `session_id`
exact filters and a bounded, case-insensitive literal `q` search over the
project, latest name, activity text, tool and host. Filtering occurs after latest-main
activity selection and before pagination. Agent project choices use
`/api/owner/filter-options?view=agents&kind=project` across event and workflow
history; subagent-only projects are excluded from that view.

Exact Events scope requires `project`, `hostname`, `tool`, `session_id` and
`subagent_id` together. Use an explicit empty `subagent_id=` for a primary
session, or the child's native identifier for a subagent. Incomplete sets
return 400. Existing optional `agent` text and `from`/`to` receipt-millisecond
bounds remain supported; the UI uses time bounds with exact scope.

The global event cursor is an opaque JSON string, URL-encoded as a query value.
It contains receipt time, project and event ID and must be returned unchanged.
It does not use SQLite row IDs, so database maintenance does not invalidate
its ordering. A global event contains `project`, `hostname`, `tool`,
`session_id`, nullable `subagent_id`, `parent_session_id`, `session_name`,
and the message fields below. Full owner-visible details arrive in the
bounded response; the UI reveals them on activation. Raw payloads and
credentials are never returned. Compatible project/session/history routes
remain available, though the starting view no longer uses project cards.

A session contains `id`, `hostname`, `tool`, `session_id`, nullable
`subagent_id`, `parent_session_id`, `session_name`, and `latest`. A message
contains `event_id`, string `sequence`, numeric `received_at` (Unix
milliseconds), and `message`, plus optional `description` and resolved
`workflow_task_id`. Absent optional fields are omitted. Latest means highest
sequence, not latest
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

## Detailed descriptions

Events keep a compact message preview. Activate its Event details button
(or click the row) to read the full summary and optional Description beneath
it; the button also works with the keyboard. Descriptions render safe
Markdown, including multiple paragraphs. Existing events without descriptions
retain their full message.

Select a workflow step to see its Description and separate Agent report.
The step title remains the compact graph label. Workflow-level descriptions
already appear in the workflow view. Description text is publisher-reported
context, not an instruction or verified completion evidence.

## Session hierarchy

Events initially opens as **Flat list**, preserving chronological receipt
order. Use **Session hierarchy** beside it to group the current page into
session rows, with explicitly linked subagent sessions beneath their primary
session. The owner selected this grouped-row layout from the #109 proposals.
The view and collapse choices are saved in the URL and restore on reload
or browser Back/Forward. Switching views keeps the current filters, page and
selected event details and does not request additional reports.

All groups initially open. Each session header has its own collapse control,
separate from event details. Collapsing a parent also hides its nested child
sessions; expanding it restores the children's own collapse choices. Polling
preserves choices for sessions still on the page. Changing filters or moving
to another page resets group expansion.

Primary-session families come first, ordered by their newest visible event,
including child reports. Unmatched child groups follow in receipt order.
Within each family, child groups follow receipt order; events within a session
retain page receipt order. The highest visible
sequence provides the readable session label. Group counts cover that
session's events on this page, not its full history or its children's counts.

Only explicit native parent metadata within the same project, hostname and
tool can link a child to a visible primary session. Names and other subagents
never select a parent. Children without a matching parent remain accessible
as **Parent outside current results**, or **Parent not reported** when no
parent metadata was supplied. Filtering or pagination can therefore change
which relationships appear. The view never fetches missing parent history
or implies the page contains a complete session tree.

Existing workflow links and safe Markdown details remain available in both
views. Optional event descriptions render when supplied by a compatible
server; the publishing contract for those descriptions is separate #107.
