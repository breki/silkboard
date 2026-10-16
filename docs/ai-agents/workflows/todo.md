# todo

GitHub issues in `breki/silkboard` are the backlog. Without arguments, list
open issues using pagination. Distinguish unavailable authenticated access
from an empty backlog.

With an authorized task to capture, check existing issues for duplicates,
including closed matches, then create a GitHub issue with a short title,
details/acceptance criteria supplied by the owner, and relevant links. Use
structured arguments or a body file to preserve the text. Do not invent
priorities, labels, assignees, or milestones. Report the resulting issue URL.

Use authenticated GitHub tooling within task authorization; this workflow
does not grant external access on its own. If access is unavailable, report
the blocker instead of creating a local Markdown backlog. Follow
[record rules](../records.md). Keep issue details and status authoritative
on GitHub; local plans may link to the issue without duplicating its backlog
record. Capture does not implement, commit, close an issue, or open a PR.
