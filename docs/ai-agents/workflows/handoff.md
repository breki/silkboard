# handoff

Inspect current status, branch/upstream relationship, recent commits, changed
paths, roadmap, and any working task documents. Do not reconstruct Git facts
from memory. Include the user's objective, decisions, completed work, unfinished
work, blockers, relevant files, checks with results, uncommitted changes, and
exact next action. Distinguish local commit, pushed commit, and deployed
release.

Use the user-specified local path; otherwise write `.local/handoff.md` as
the latest handoff, creating `.local/` when needed and preserving useful
unfinished context. This directory is gitignored for temporary session
context. Keep durable decisions in tracked canon and pending work in GitHub
issues. Link canonical decisions rather than duplicating their full text;
resolve relative links from the handoff's actual directory. Avoid credentials
and unrestricted logs. Write locally before attempting any authorized network
publication.
No commit, push, or dashboard upload is implied. Report the path and next step.
