# Task-plan presentation design

Status: accepted milestone 5 presentation design for
[issue #74](https://github.com/breki/silkboard/issues/74), building on the
[reporting contract](task-plan-contract.md). Issue #78 implements the owner
view against the private APIs; release and production acceptance remain
separate. This document defines the presentation requirements.
The [interactive reference](mockups/task-plan.html) opens locally in a browser
without dependencies or a server. Its simulated statuses are example data.

## Entry points and navigation

The owner selected a separate **Task plans** page. The later
[issue #96](https://github.com/breki/silkboard/issues/96) moves **Events** and
**Task plans** into direct links in the top header row, replacing their
hamburger-menu entries. Events remains the starting view and retains the
all-project event table. Application status, credential management and Sign
out remain in the hamburger menu; no task execution controls are added.

Proposed owner browser paths:

| View | Path |
| --- | --- |
| All-project task summaries | `/plans` |
| Current task | `/plans/PROJECT/TASK_ID` |
| Exact historical snapshot | `/plans/PROJECT/TASK_ID?sequence=SEQUENCE` |

These are browser paths, not publisher API routes. Use the contract's project,
UUID and decimal-string rules. Deep links and reloads must reach the normal
application shell. Login returns to the requested same-origin plan view;
expiry/logout clears task and history data and shows login. Never accept an
arbitrary return URL or place credentials/plan text in links.

Task plans opens a compact table across projects, most recently accepted report
first. Show Last report, Project, Task and Reported state; keep step counts and
active/blocked counts with the state rather than adding many columns. The task
title is the accessible link to its current graph. Reporter information belongs
in task details. Preserve the existing bounded pagination pattern, and restore
the earlier list position when returning with browser Back.

Use local `HH:mm` above `MM-DD` in the summary table, and full local
`yyyy-mm-dd HH:mm:ss` in task/history details, following the event feed. A
stable task identity is not a human-facing title or a GitHub issue number.
If present, show the validated GitHub issue as a link, never fetched content.

## Task heading and progress summary

Keep the compact Silkboard header. The task view has breadcrumbs back to
Task plans, project and task title, and an optional description/issue link.

The summary shows:

- The overall **last reported** state, derived by the contract. Active work
  can coexist with a blocked branch; show both instead of calling the whole
  task blocked while other steps remain active.
- `N of M steps marked done`, plus active/blocked counts. A progress bar means
  that count only; give it an accessible count description rather than a
  default percent announcement. Do not label it an effort percentage, ETA or
  verified success.
- The reporting tool/host and optional readable session name, and when the
  current snapshot was received. Describe these as reported identity, not
  authenticated proof of which machine or agent performed work.
- A Graph / Steps presentation switch and revision/history selection.

When one step is active, **Locate active step** selects/focuses it and brings
it into view. When several are active, expose an active-step chooser with all
reported active steps. Choosing one focuses its node/row; it does not execute
or resume work. If none is active, do not invent a current step.

## Graph and ordered step view

A node is one stable step. A directed arrow goes from a prerequisite to the
step that depends on it. Support separate branches, several roots and multiple
active nodes. Keep a consistent legend with explicit text and symbols:

| State | Label and visual treatment |
| --- | --- |
| Pending | Hollow marker and neutral border |
| Active | Filled marker and warm highlighted border |
| Done | Checkmark and green accent |
| Blocked | Exclamation marker, distinct accent and a blocking reason |

Color is supplementary. Selected and keyboard-focused nodes have their own
visible treatment; a selected pending node must not appear to be active.
Graph geometry remains stable on status-only updates. A changed definition
may require a new layout; preserve selected identity and viewport context when
possible. Do not rely on layout positions or array indices as identity.

The Steps view presents the same data in readable prerequisite order, with
stable tie ordering from the plan. Each entry identifies its prerequisites in
text. It is a complete accessible alternative, not a separate task state.

Selection opens details alongside the graph on desktop. On phones use a
scrollable details dialog with a visible Close action, native Escape behavior,
contained keyboard focus and focus return to the originating node/row. If the
node has disappeared in a revision, return to an appropriate task heading
and explain the removal instead of silently selecting unrelated work.

## Step details and evidence

Show the step title, full description, state, prerequisite titles/states,
current snapshot's report, blocking reason when applicable and reported
identity. Render Markdown with the application's existing safe renderer;
labels and identity remain plain text. Use no raw HTML, image loading, code
execution or fetched link previews.

Label receipt time **Snapshot received**, not “step completed at” or “last
changed at” unless received history actually establishes that transition.
A full snapshot repeats unchanged steps; its receipt does not prove all steps
changed at that moment. Keep task sequences and reporting generations in the
API rather than the primary owner UI.

A reopened step keeps its ID. Its current state changes while received history
retains earlier completion. Make prior completion/reopening accessible through
bounded history; only show transition labels when backed by received snapshots.
Do not infer a completion that was never received. Dependency consistency is
validated at ingestion; the browser does not silently rewrite reported states.

## Revisions, snapshots and session continuity

A plan-definition revision differs from a status report. The revision chooser
shows distinct received plan revisions with their receipt context. Selecting
an earlier revision resolves to its latest accepted snapshot and pins the
historical view to that exact sequence. The explicit **Current plan** choice
returns to live updates. Received-history entries can also open another
snapshot within the same definition revision, including before a reopening.

Historical views have a persistent **Earlier snapshot** label and receipt time.
Their states come from that snapshot, not today's graph. They remain pinned
when new reports arrive; offer a clear return to Current plan. A newer-report
notice must not silently replace the historical view.

Show added/removed steps and changed prerequisites when the adjacent received
snapshots provide the comparison. Retain readable historical identity for
removed work; current step counts use the current graph only. If reports or
revisions are missing, say that received history is incomplete rather than
inventing an exact account of unobserved changes.

An explicit session handoff keeps the same task page. Reported tool/host/session
can change on the next accepted generation; show the new reporter and any
received handoff history without duplicating the task or resetting its steps.
A generation jump does not establish the identities of missed sessions.
Independent attempts remain separate task rows even for the same issue.

## Phone, keyboard and large plans

Keep readable labels at 320-pixel viewport width and avoid horizontal page
overflow. The graph may use a bounded internally scrollable region and a
simpler vertical layout on narrow screens. Never shrink the full graph into
unreadably small text. Preserve direction and prerequisite meaning, not merely
a decorative timeline. The Steps alternative remains one action away.

Support up to the contract's 128 nodes and 512 edges without an unbounded
canvas, history fetch or rerender loop. Preserve scroll/focus on report-only
updates. Make the current-step summary and active-step chooser usable without
scrolling through every completed node. No minimap, editor, zoom toolbar or
additional filtering is required for this first implementation.

Use native buttons/links, visible focus, accessible node names including state,
keyboard-operable revision/active selectors and appropriate dialog semantics.
Announce concise meaningful updates, not the entire graph on every refresh.
Respect reduced motion and never steal focus when a report arrives.

## Loading, errors and freshness

- A loading view identifies that task plans are being fetched. Existing content
  stays visible during refresh; do not reset selection merely to show loading.
- An empty list explains that no task-plan reports have arrived yet. It does
  not create tasks or invite publishers to fetch dashboard instructions.
- Authentication failure clears private data and asks the owner to log in.
  Distinguish unavailable plan support from an unknown/removed task or snapshot.
- A refresh failure retains the last received content, with a clear warning
  and Retry action. Do not relabel that content as freshly received.
- Always display the last accepted report time and a readable age. Silence
  changes no reported step state. Avoid unagreed “offline” thresholds or claims
  that the agent is still executing. An identical replay does not refresh age.
- Incomplete history and request failures are different conditions; report them
  separately. A later full snapshot repairs current display, not lost history.

Reuse visibility-aware ten-second polling with one current-plan request in
flight. Pause/abort in hidden tabs and clear private state on logout. Load
history only on demand with bounded pagination. Do not persist plan bodies in
localStorage, log them to analytics or request development-repository access.

## Reviewable reference and implementation boundary

The linked HTML is based on the self-contained prototype from the owner
conversation, formatted and checked for focus preservation during resize.
It is preserved for implementers on other machines. It illustrates
this feature as a nine-step task, step details, phone dialog, plan revisions,
blocked/resumed status and simulated progress. It sends no network requests.
Its controls advance fictional data only; they are not production controls.

The prototype starts with one active step and is a visual reference, not a
complete implementation of the agreed contract. The normative design above
adds multi-active selection, reopening/history, task-list navigation and
session continuity. Do not copy its demo state machine as reporting logic or
mistake the fictional API/storage completion for delivered milestone work.

[#78](https://github.com/breki/silkboard/issues/78) implements this presentation
against #75's real owner APIs after this design and the contract are accepted.
It must add meaningful component/real-API browser coverage for login/deep links,
all states, multi-active selection, revisions/reopening, lost reports, phone
and keyboard behavior, and maximum-sized graphs. This design changes no
production route, publisher permission, database schema or release authority.
