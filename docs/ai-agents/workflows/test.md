# test

Read [quality](../quality.md) and [tooling status](../tooling.md). Verify the
xtask test command exists. Forward the requested filter and supported flags
without dropping them; use help to check syntax. Run the requested tests and
report counts, failures, and skips. Do not turn a filtered run into a claim
that the full suite passed. Absent tooling means this workflow is unavailable.

For a standalone browser run, check the
[built-asset prerequisites](../../development.md#checks) first. A fresh
worktree does not have `frontend/dist` merely because it shares installed
frontend tools or Cargo output with another worktree. Build the assets in the
tested worktree before starting its browser fixture.
