# todo

Without arguments, list open items in `docs/todo.md`, or report none if the
file is absent. With a task, preserve its meaning and choose a unique lowercase
hyphenated slug. Capture a short summary, details/acceptance criteria supplied
by the owner, and relevant links. Do not invent priorities or milestones.

Use the configured xtask helper once available. Until then, explicitly use
manual Markdown bookkeeping: create the file only when first capturing an
item, record a pending entry with slug and summary, and check for duplicate
slugs before writing. Follow [record rules](../records.md). A GitHub issue's
body stays authoritative for its details; keep a pointer instead of a copy.
Capture never implements, commits, or publishes an issue.
