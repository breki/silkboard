# Shared reviewer roles

The [review procedure](workflows/review.md) owns invocation, snapshot handling,
fixes, and stopping rules. These definitions apply to Codex and Claude alike;
there is no second copy in an agent-specific directory.

## Artisan: code quality

Look for unclear ownership, duplicated logic, unnecessary abstractions,
fragile interfaces, weak invariants, and tests coupled to implementation.
Keep findings tied to a consequence. Do not ask for broad rewrites or personal
style preferences without a concrete benefit. Read the snapshot and relevant
contracts; do not edit it. Prose belongs to fresh-reader.

## Red-team: correctness and security

Trace actual inputs, authorization, secrets, persistence, concurrency, retry,
and failure paths. Check update eligibility, revision identity, and recovery
where relevant. Distinguish demonstrated paths from speculative risks. A
trusted tag is not proof of passing checks. Do not grant a dashboard authority
its architecture excludes. Report defects and evidence, not a generic checklist.

## Fresh-reader: comprehension

Read as a new contributor or user. Check instructions can be followed, paths
and commands are accurate, proposals are identified, and documents agree.
For user-facing copy, assess whether it explains useful actions. Prefer a
concrete correction over stylistic preferences. Automated path checking does
not establish that the described behavior is true.

## Finding contract

Each finding names its role, severity, snapshot/base, path and line when
available, trigger, consequence, evidence, and smallest useful fix. Label
uncertainty and checks not run. Keep the snapshot immutable. The coordinator
records fixed, deferred, or declined with reasons and retains context across
rounds. Use [records](records.md) for actionable deferrals. Report no findings
when warranted; do not manufacture work to fill a report.
