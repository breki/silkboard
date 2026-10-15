# handoff

Inspect current status, branch/upstream relationship, recent commits, changed
paths, roadmap, and any working task documents. Do not reconstruct Git facts
from memory. Include the user's objective, decisions, completed work, unfinished
work, blockers, relevant files, checks with results, uncommitted changes, and
exact next action. Distinguish local commit, pushed commit, and deployed
release.

Use the user-specified local path; otherwise write `docs/handoff.md` as the
latest handoff, preserving useful unfinished context. Link canonical decisions
rather than duplicating their full text. Avoid credentials and unrestricted
logs. Write locally before attempting any authorized network publication.
No commit, push, or dashboard upload is implied. Report the path and next step.
