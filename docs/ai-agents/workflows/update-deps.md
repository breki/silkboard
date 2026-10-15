# update-deps

Require actual dependency manifests and installed helper commands; otherwise
report unavailable without creating lockfiles. Inspect requested scope,
current pins, toolchain constraints, and [quality rules](../quality.md).
Select compatible releases past the 14-day cooldown, including transitive
updates. Record explicit security-fix or reviewed exceptions narrowly.

Use configured age/preflight helpers before compiling newly adopted code.
Keep updates scoped and avoid unrelated lockfile churn. Run license/source,
advisory, build, and affected behavioral checks. Inspect migration notes for
major changes. Preserve other work; do not broadly delete dependency trees as
an automatic first step. Report exact changes and unrun checks. Commit and
release remain separate requests.
