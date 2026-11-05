# Owner Todos

[Issue #116](https://github.com/breki/silkboard/issues/116) adds an owner-managed
checklist at `/todos`. Choose Todos in the visible primary navigation and sign
in using the existing owner account. Agents stays home; Events and Workflows
keep their links. [Issue #121](https://github.com/breki/silkboard/issues/121)
moves Todos out of the hamburger menu and keeps it directly available from
each primary page. Todos do not sync with GitHub or drive agents, workflows or execution.

Each item has a Link action for its [bookmark](deep-links.md). Edit and
Add child also update the URL; opening it restores stored text or an empty child
form without saving a change. Unsaved text never appears in the URL.

## Editing the list

Enter a single-line task in New todo and choose Add todo. Add child on any
item selects it as the parent and focuses the same form. Use top level returns
the draft to the root. Children appear in nested, indented lists. Each sibling
level keeps creation order, including children promoted after deletion.
The list supports five levels, 1,000 total items and 1,000 UTF-8 bytes per task.
There is no drag-and-drop or reparenting control.

Every checkbox is independent: checking or unchecking a parent does not change
its children, and partial completion does not alter its checkbox. Edit opens a
text form; Save text changes its label while preserving its current completion
state. Cancel edit discards that text draft. Per-item actions name their target
for assistive technology; all controls support keyboard and phone use.

Delete opens a confirmation identifying the item. Deleting a parent keeps its
direct children and moves them to the deleted parent's level, under its former
parent if present. Those children's descendants and completion states remain
unchanged. Their position follows creation order among their new siblings.
Confirm delete commits the operation; Cancel delete keeps the list unchanged.
Deletion cannot be undone.

Reload todos reads the current list. Reload and application restart preserve
saved text, hierarchy and checkboxes. The page does not poll and replace an
in-progress checklist; a separate owner-session check detects login expiry.
Navigation, logout and expiry discard local unsaved drafts rather than storing
private checklist text in the browser.

Changes appear as saved only after the server acknowledges them. During a save,
mutation controls are disabled. A stale tab cannot overwrite a newer edit: the
server rejects its revision, and the page asks for Reload todos. If a network
failure leaves a save unconfirmed, the previous rows and local drafts remain;
reload and inspect the result before trying again. A successful write may have
reached storage even when its reply was lost. Reload preserves drafts and
explains when a selected parent or edited item has been deleted elsewhere.

## Private API and storage

All routes are inside the guarded `/api/owner` router. Reads require an owner
cookie; mutations also require the exact configured Origin and fresh
`X-Silkboard-CSRF`. Publisher tokens have no todo read or write capability.
Mutations reauthenticate the owner inside the same immediate SQLite transaction
that checks the revision and writes data. Todo text renders as plain text.

| Method and path | JSON body | Result |
| --- | --- | --- |
| `GET /api/owner/todos` | none | Current checklist |
| `POST /api/owner/todos` | `revision`, `text`, optional nullable `parent_id` | 201 and checklist |
| `PATCH /api/owner/todos/ID` | `revision`, `text`, `completed` boolean | 200 and checklist |
| `DELETE /api/owner/todos/ID` | `revision` | 200 and checklist with promoted children |

A checklist is `{revision, items}`. Revision is a canonical nonnegative signed
64-bit integer string; IDs are canonical positive signed 64-bit integer strings.
Items contain `id`, nullable `parent_id`, `text` and boolean `completed`, ordered
by ascending ID. The complete bounded list is returned, so all ancestors are
available for hierarchy display. Every successful mutation increments the global
revision. Unknown body fields and invalid IDs, text or depth return 400; missing
items/parents return 404; stale revisions return 409. Authorization and integrity
failures retain the existing 401/403 behavior, with storage failures returning
503. Responses use `Cache-Control: no-store`.

The additive `todo_schema` component marker is version 1, alongside `todos`
and its parent index; base `user_version` remains 3. Existing progress, owner,
publisher and workflow rows are retained. The private backup/restore validator
accepts legacy databases without Todos and checks present component version,
shape, depth and size. Older app versions can retain these additive tables but
cannot expose the feature. Newer component versions are rejected by this version.
See [private application data](application-data.md) for operator recovery.

Local API, persistence and browser tests establish implementation behavior.
They do not demonstrate production installation or recovery acceptance.
