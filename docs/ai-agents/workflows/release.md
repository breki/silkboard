# release

Read [tooling status](../tooling.md) before making edits. If application or
release tooling is absent, report the missing prerequisites and stop without
creating a version, changelog, commit, or tag. Verify every required release
gate exists before proceeding; check actual tooling rather than trusting a
stale availability note.

1. Require a clean tree and nonempty Unreleased changelog. Keep release
   bookkeeping separate from substantive changes.
2. Use an explicit requested bump; otherwise infer major for BREAKING/Removed,
   minor for Added, patch otherwise. Explain the inference and ask the owner
   to choose the release version. Do not infer agreement from silence.
3. Resolve the version source from the actual workspace manifests. Bump it
   once, synchronize workspace lockfile entries without broad dependency
   updates, and promote Unreleased into a dated release section.
4. Run full validation, supported cross-target lint checks, and a standalone
   advisory audit that fails if unavailable. Verify checks cover the exact
   tree being tagged; changes after a check require relevant revalidation.
5. Commit only release bookkeeping with `release: vX.Y.Z` and create an
   annotated tag matching that version. Preserve configured signing. A failed
   gate leaves visible bookkeeping edits but no release commit or tag.
6. Report checks and next action. Do not push a tag merely because a local
   release was requested. If pushing is authorized, account for its ability
   to trigger deployment and check the agreed eligibility policy first.

An annotated tag is not proof of trusted signatures or passing GitHub checks.
Those deployment-policy decisions remain open in the roadmap. Do not create
a GitHub Release object or deploy as an unstated extension of this workflow.
