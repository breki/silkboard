# check

Read [tooling status](../tooling.md). Verify the configured `cargo xtask check`
entry point exists, then run it from the repository root. Report compiler
failures with actionable locations. Type-checking is not test execution.
If tooling is absent, report unavailable; do not fabricate a passing result.
