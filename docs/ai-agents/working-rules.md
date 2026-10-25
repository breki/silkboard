# Working rules

## Communication and decisions

Explain the outcome and mechanism in plain words. Name the subject, define
unfamiliar terms, and state what was actually verified. Keep tracked Markdown
near 80 columns; use one paragraph per line in GitHub descriptions. Avoid
machine-specific anecdotes and claims copied from another project.

Before asking a technical question, inspect what the repository can answer.
Explain meaningful tradeoffs, show an example when it helps, then recommend
an option. Use one interactive question at a time for requirements. A failed
question tool is a limitation to explain, not a reason to repeat plain-text
questions without context. Existing approval remains valid within its scope.

For UI acceptance, use the visible labels and explain where to find the
controls or information. Record the observations actually confirmed; a narrow
answer about history or timestamps does not confirm every interaction in a
larger checklist. Ask only for the checks still unresolved.

## Canon and memory

Shared rules belong in this repository. Personal memory can retain private,
user-specific context, but must not become the only copy of a project rule.
Promote useful general lessons into the relevant shared document; avoid
repeating them in both agent entry points or every workflow.

## Editing and execution

Inspect Git state and applicable instructions before editing. Preserve other
work. Prefer targeted searches and reads over dumping large files. Use the
configured working directory and safe argument arrays or proper shell quoting.
Never use JSON encoding as shell escaping. Keep secrets out of command output,
reports, prompts, and tracked files.

When sharing Cargo output across worktrees, verify that repository helpers
operate on the current worktree. A reused executable can retain another
worktree's build-time root even when invoked from the correct directory.

Validate invariants at the boundary that owns the value. Check sibling fields
and other instances of a bug pattern. After removing a capability, search its
help text, tests, wrappers, and documentation for stale claims. Deterministic
bookkeeping belongs in tested tooling once that tooling exists; judgment
belongs in the workflow. Do not add a parser merely to police prose style.

Scripts used by agents should be noninteractive: accept flags, check
preconditions, fail clearly, and be safe to rerun. This rule concerns scripts,
not interactive questions to the owner. Use concise success output and useful
failure details; long operations need durable logs and visible progress.
Use meaningful exit codes appropriate to the actual interface.

Missing per-step CI telemetry is not proof that a runner stalled. Use overall
job results and completed logs to establish which checks ran; do not cancel
and retry solely because step metadata is empty.

Keep nontrivial automation in Rust/xtask when practical. Shell wrappers should
be thin; bootstrap scripts that run before Cargo exists are a valid exception.
Do not rely on Python for repository tooling or tests.
Do not assume Windows-only or Bombyx host-specific restrictions apply here.
Inspect actual tool availability and platform behavior.

## Finish and hand off

Read the diff, run checks appropriate to the change, and distinguish passing,
failing, and unrun checks. Keep source documents consistent when decisions
change. Make a local handoff before relying on network publication. State
remaining work and the exact next action. Do not label a plan as operational.
