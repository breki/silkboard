# Deep links

The address bar identifies the current Silkboard view. Copy its URL to return
to applied filters, paging and selections. Direct entry, reload and browser
Back/Forward restore that state; signing in keeps the requested destination.
A different browser still needs its own owner login. Links grant no access.

Applied time ranges use absolute Unix milliseconds and display in the browser's
local timezone. A Last hour choice becomes the fixed range selected at that
moment. Search text inside an open picker and unsaved form drafts remain local
editing state; they are not applied filters or bookmark contents.

## Views and fields

| Page | Bookmark state |
| --- | --- |
| Agents `/` | `project`, exact `hostname`/`tool`/`session_id`, `before`, `agent_details` |
| Events `/events` | `project`, `agent`, `from`/`to`, exact session, `before`, `view`, `event`, `collapsed` |
| Workflows `/plans` | `project`, `agent`, `state`, `before` |
| Workflow `/plans/PROJECT/TASK` | `sequence`, `step`, `presentation`, `details`, `history`, `history_before`, `compare` |
| Todos `/todos` | `project`, `todo`, `mode` |
| Application status `/status` | Public status; optional owner credential panel |
| Setup `/setup` | Existing one-time setup link; optional `theme` |

For example, `/plans?project=silkboard&state=active` opens active Silkboard
workflows. `/events?project=silkboard&view=hierarchy` opens that project's
session hierarchy. Existing workflow snapshot and exact-session links remain
valid. Exact Events identity includes project, host, tool, native session and
`subagent_id`; an empty child ID means the primary session.

The app writes encoded JSON identities for expanded agent/event targets and
collapsed session groups. These use stable project/native IDs, never readable
names or row positions. Linked agents and events remain accessible when newer
reports move them off the requested page. Missing targets offer a clear action
to remove that selection. Pagination cursors identify a position in received
history; the contents of a page may change as activity arrives. Extremely
long group identities can reach the bookmark size limit. The app keeps the
last valid collapse state and offers Clear collapsed groups rather than
applying a change that cannot be restored.

Workflow `presentation=steps` selects Steps; omission selects Graph.
`step=ID` selects its details, with `details=closed` keeping a phone dialog
closed. `history=reports` or `history=revisions` opens received history;
`history_before` pages it. `compare=previous` requests comparison with the
previous received report for the displayed snapshot. Missing steps or snapshots
show recovery information rather than substituting invented content.

Todo `mode=edit` focuses stored text in the task document; `mode=child` creates
an empty child draft under the selected task. Neither saves a change until a
valid edit is made. Selecting saved task text updates its URL; the shared menu
can copy its link. Missing or ineligible targets can be
cleared while keeping the checklist available.
Project filters keep matching items and their ancestors visible. A linked item
outside the active filter, or a request to add a child in a filtered view,
requires returning to the full list before editing that target.

## Credentials, appearance and recovery

`panel=credentials` opens publisher management on an owner page.
`credential_project` and `credential_after` restore its metadata selection.
Opening a link never issues or revokes a credential. Disclosed tokens cannot
be restored: closing the panel, leaving the page, logout and expiry remove them.
The panel's URL remains available for signing back in.

Setup consumes its one-time fragment and strips unrelated query fields,
retaining only a validated appearance choice.

`theme=system`, `theme=light` or `theme=dark` overrides that browser's saved
appearance for the linked view. Choosing a theme updates the URL and the saved
browser preference. Primary navigation carries the explicit appearance and
credential-panel selection to its destination.

Unknown, duplicate or malformed query fields produce a recoverable invalid
link. Reset view opens the same page without query selections. Unknown paths
show Page not found with an Agents link and retain HTTP 404. An unavailable
owner API still requires authentication; the URL never bypasses that boundary.
Passwords, token material, draft text, destructive confirmations and commands
have no bookmark fields. URLs can contain private project and native identity
metadata, so share them within the same private context as the dashboard.
