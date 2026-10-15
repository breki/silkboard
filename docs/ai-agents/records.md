# Durable records

Create records only when there is actual work or a finding to preserve.
Do not create empty review logs, fake backlog entries, or placeholder history.

- Backlog: `docs/todo.md`, with a unique lowercase hyphenated slug, summary,
  pending/in-progress/blocked/done state, and source/acceptance details.
- Working plans: `docs/issues/<slug>.md`, linked to the backlog or GitHub issue.
- Deferred review findings: `docs/developer/<role>-log.md`, where role is
  artisan, redteam, or fresh-reader. Include a unique `role-date-slug` ID,
  severity, source snapshot, consequence, disposition, and follow-up link.
- Template feedback: `docs/developer/template-feedback.md`, using unique
  `tf-date-slug` IDs and open/resolved/suggestion sections.
- Latest handoff: `docs/handoff.md`, written only by the handoff workflow.

Check uniqueness before writing. Preserve existing identifiers when updating
status; resolve findings with evidence rather than deleting their history.
Use links instead of maintaining duplicate task descriptions. Automated record
checking and mechanical editing are pending; manual edits must say so when
those helpers would otherwise have been used.
