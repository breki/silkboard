# commit

Inspect branch, status, staged and unstaged diffs, and relevant history.
Identify the intended changes; preserve unrelated work. Read the diff and run
appropriate targeted checks under [quality](../quality.md). Documentation-only
work uses link/consistency checks and `git diff --check`.

Update an existing changelog's Unreleased section for user-visible changes;
use its configured helper when available. Before that helper exists, edit the
section directly and state the manual fallback. Do not create a changelog
just to describe a documentation-only save point. Do not bump versions, update
lockfiles, run reviewers, or repeat full release gates as part of committing.

Stage explicit intended paths, inspect the staged diff, and commit using a
Conventional Commit message describing the outcome. No emoji or invented
coauthors. Preserve repository signing configuration; never disable it to
force a commit. Report the hash and remaining worktree state. Push only within
the user's authorization. This procedure owns committing; its Git commit is
allowed, unlike ad hoc commits that skip the procedure.

After substantive work, use [retrospect](retrospect.md) briefly; skip it for
routine documentation-only changes. Do not make unrelated follow-up edits.
