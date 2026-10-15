# Canon provenance

Adapted on 2026-10-15. These are guidance-source baselines, not a record that
Silkboard has imported the complete application template.

| Source | Revision | Scope |
| --- | --- | --- |
| [rustbase](https://github.com/breki/rustbase) | `6528907cea4e1e2e369fe260ccd0432b29c557b3` | Agent entry point, tool/script guidance, web patterns, workflow catalog |
| [Bombyx](https://github.com/breki/bombyx) | `38fe77286f55a0bcc6e12bd5d4500e72ae4ebb06` | Clean local checkout; working rules, quality, review separation, workflows, report template |

The [catalog](catalog.md) maps individual workflows to their source families.
The reused report template and adapted guidance retain the upstream
[MIT notice](upstream-license.txt).

This is a selective rewrite, not a promise of verbatim upstream behavior.
When syncing, inspect the upstream changes since these revisions, adapt each
relevant change, and record the scope actually reconciled. Do not advance a
whole-template watermark for a guidance-only update.

## Intentional differences

- One shared canon, with AGENTS and Claude entry points and thin wrappers.
- Both Codex and Claude supported; no assumption that their tools are identical.
- Review is separate from commit; reviewer selection follows the change.
- Behavior-focused TDD; deployment contracts can be tested without a separate
  approval question. Simple documentation work uses lightweight checks.
- Rustbase quality thresholds retained; absent automation marked pending.
- No CLI-only, Windows-only, host-specific, or speech-summary requirements.
- No copied incidents, private host addresses, stale crate names, or fake
  command availability. No hooks or permission overrides imported.
- Manual backlog/changelog/feedback capture allowed before helpers exist;
  missing release gates block releases rather than being skipped.
- Full workflow set retained, including reports and template maintenance,
  with supporting assets shared and capability limitations explicit.
- Deployment eligibility and platform choices remain the owner's decisions.

Check links, wrapper routing, workflow prerequisites, and source consistency
when changing this layout. Update the shared body once rather than editing
both wrappers. Record future generic improvements through template feedback.
