# Owner Todos

[Issue #116](https://github.com/breki/silkboard/issues/116) adds an owner-managed
checklist at `/todos`. Choose Todos in the visible primary navigation and sign
in using the existing owner account. Agents stays home; Events and Workflows
keep their links. [Issue #121](https://github.com/breki/silkboard/issues/121)
moves Todos out of the hamburger menu and keeps it directly available from
each primary page. Todos do not sync with GitHub or drive agents, workflows or execution.

[Issue #124](https://github.com/breki/silkboard/issues/124) replaces per-item
forms with one editable task document. The owner approved the
[mockups](mockups/todo-editor/README.md) before implementation. Select a saved
task to update its [bookmark](deep-links.md), then use Copy selected task link
in the shared menu. Unsaved text never appears in URLs.

## Editing the list

Click task text and type directly. Native arrows and selection follow wrapped
lines and neighboring tasks. Enter splits at the caret and creates an unchecked
sibling after the original task's subtree; children stay with the original.
Backspace at the start or Delete at the end joins adjacent tasks. Selecting text
across tasks replaces that range. Removing a parent promotes its surviving
children and preserves their completion states. Undo and Redo restore text and
structure while the editor stays open, including after a save; routine deletion
has no confirmation dialog.

Tab nests the active task and its whole subtree under its preceding sibling.
Shift+Tab lifts that subtree after its parent's whole subtree, keeping later
siblings under their original parent. Invalid moves leave the list unchanged
and explain why. Shared New task, Indent, Outdent, Undo and Redo tools provide
the same actions on phones. Plain-text multiline paste imports tasks; tabs
represent nesting and optional `[ ]`/`[x]` markers set completion. Dragging is
not supported. Every checkbox is independent of its ancestors and descendants.

Escape leaves editing; then Tab moves to other controls. Checkboxes and other
controls use ordinary Tab and Space. Keyboard help explains the commands.
The list supports five levels, 1,000 tasks and 1,000 UTF-8 bytes of single-line,
nonempty text per task. Incomplete or invalid text remains an unsaved draft.

Changes save after a 600 ms pause, one request at a time. Typing remains available
during a save; acknowledgements preserve the editor and caret. Saved means the
server acknowledged the exact submitted document. A stale revision or failed
request keeps the draft and stops further writes. Review saved list compares
both versions. If the saved version is unchanged, Retry save is explicit. An
exact read-back can recognize a save whose acknowledgement was lost. Use saved
list confirms discarding local edits, rereads the server, and refuses replacement
if typing occurred during that read. Copy my draft provides a plain-text copy.

Drafts stay only in the currently loaded page's memory, including through
in-page view changes and login expiry. Sign in to recover a retained draft;
logged-out views show no task text. Explicit sign-out warns before discarding
unsaved work. Following another page link, reloading or closing this page warns
about unsaved changes; proceeding can lose the draft. No task
text is stored in localStorage or sessionStorage. Saved text, order, hierarchy
and checkboxes survive reload and application restart. The editor does not poll
and replace an in-progress document.

## Private API and storage

All routes are inside the guarded `/api/owner` router. Reads require an owner
cookie; mutations also require the exact configured Origin and fresh
`X-Silkboard-CSRF`. Publisher tokens have no todo read or write capability.
Mutations reauthenticate the owner inside the same immediate SQLite transaction
that checks the revision and writes data. Todo text renders as plain text.

| Method and path | JSON body | Result |
| --- | --- | --- |
| `GET /api/owner/todos` | none | Current checklist |
| `PUT /api/owner/todos/document` | `revision`, ordered `items` with `key`, nullable `id`, nullable `parent_key`, `text`, `completed` | 200, checklist and client-key/server-ID map |
| `POST /api/owner/todos` | `revision`, `text`, optional nullable `parent_id` | 201 and checklist |
| `PATCH /api/owner/todos/ID` | `revision`, `text`, `completed` boolean | 200 and checklist |
| `DELETE /api/owner/todos/ID` | `revision` | 200 and checklist with promoted children |

A checklist is `{revision, items}`. Revision is a canonical nonnegative signed
64-bit integer string; IDs are canonical positive signed 64-bit integer strings.
Items contain `id`, nullable `parent_id`, `text` and boolean `completed`, ordered
in depth-first document order; IDs need not increase. The complete bounded list
is returned, so all ancestors are
available for hierarchy display. Every successful mutation increments the global
revision. Unknown body fields and invalid IDs, text or depth return 400; missing
items/parents return 404; stale revisions return 409. Authorization and integrity
failures retain the existing 401/403 behavior, with storage failures returning
503. Responses use `Cache-Control: no-store`.

The document endpoint applies the complete list atomically, allocating IDs for
new keys and deleting omitted rows. Keys are unique ASCII letters, digits,
underscores or hyphens, up to 64 characters. A parent key names an active prior
ancestor in preorder. Existing IDs must exist and appear once. The endpoint
allows up to 3,000,000 body bytes for a maximally escaped bounded list; other
owner routes retain their 4 KiB limit. Original per-item endpoints remain
available for compatibility.

The original `todo_schema` stays at version 1 and base `user_version` stays 3.
Additive `todo_layout_schema` version 1 and `todo_layout` retain actual hierarchy
and document order. Original rows retain text, completion, IDs and a parent
projection compatible with earlier apps. When an earlier app writes, the new
reader reconciles the revision lag, preserves surviving order, promotes children
of removed parents and incorporates newly added tasks within the depth limit.
This keeps rollback readers and writes usable without dropping private data.
An earlier app can show a reparented item at top level when its actual parent
has a higher ID; returning to this version restores the retained layout.
The private backup validator accepts legacy databases and valid stale layout
metadata, but rejects partial, corrupt or future metadata. Reopening reconciles
legacy writes transactionally. Existing owner, progress and workflow data remain.
See [private application data](application-data.md) for operator recovery.

Local API, persistence and browser tests establish implementation behavior.
They do not demonstrate production installation or recovery acceptance.
