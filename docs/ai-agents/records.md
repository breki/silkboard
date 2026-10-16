# Durable records

Create records only when there is actual work or a finding to preserve.
Do not create empty review logs, fake backlog entries, or placeholder history.

- Backlog: GitHub issues in `breki/silkboard`. GitHub owns task details and
  status; use issue numbers/URLs as identifiers. Do not keep a parallel
  Markdown backlog or fall back to one when GitHub is unavailable.
- Working plans: `docs/issues/<slug>.md`, linked to the GitHub issue when
  applicable. These are implementation notes, not a separate task queue.
- Deferred review findings: `docs/developer/<role>-log.md`, where role is
  artisan, redteam, or fresh-reader. Include a unique `role-date-slug` ID,
  severity, source snapshot, consequence, disposition, and follow-up link.
- Template feedback: `docs/developer/template-feedback.md`, using unique
  `tf-date-slug` IDs and open/resolved/suggestion sections.
- Latest handoff: `.local/handoff.md`, gitignored temporary session context,
  written by the handoff workflow. Durable decisions belong in tracked canon
  and pending tasks in GitHub issues.

Check uniqueness before writing. Preserve existing identifiers when updating
status; resolve findings with evidence rather than deleting their history.
Use links instead of maintaining duplicate task descriptions. Automated record
checking and mechanical editing are pending; manual edits must say so when
those helpers would otherwise have been used.
