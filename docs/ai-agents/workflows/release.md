# release

Read [tooling status](../tooling.md) before making edits. If application or
release tooling is absent, report the missing prerequisites and stop without
creating a version, changelog, commit, or tag. Verify every required release
gate exists before proceeding; check actual tooling rather than trusting a
stale availability note.

1. Require a clean tree and nonempty Unreleased changelog. Keep release
   bookkeeping separate from substantive changes.
2. Use an explicitly requested version or bump. Otherwise choose the next
   SemVer version autonomously: major for breaking changes, minor for added
   functionality, patch for compatible fixes or refinements. Judge the actual
   changes rather than changelog labels alone. State the selected version and
   brief rationale; do not ask the owner to choose or confirm the number.
   Version selection does not itself authorize publication or deployment;
   preserve any authorization already given.
3. Resolve the version source from the actual workspace manifests. Bump it
   once, synchronize workspace lockfile entries without broad dependency
   updates, and promote Unreleased into a dated release section.
4. Run full validation, supported cross-target lint checks, and a standalone
   advisory audit that fails if unavailable. Verify checks cover the exact
   tree being tagged; changes after a check require relevant revalidation.
5. Commit only release bookkeeping with `release: vX.Y.Z` and create a
   signed annotated tag matching that version. Preserve configured signing
   and use the trusted SSH signer recorded in the roadmap. A failed
   gate leaves visible bookkeeping edits but no release commit or tag.
6. Report checks and next action. Do not push a tag merely because a local
   release was requested. If pushing is authorized, account for its ability
   to trigger deployment and check the agreed eligibility policy first.

An annotated tag is not proof of trusted signatures or passing GitHub checks.
Use the signer and release workflow selected in the roadmap; verify the local
tag with `cargo xtask release-check TAG` and require separate exact-revision
GitHub release-suite evidence for eligibility. Do not create
a GitHub Release object or deploy as an unstated extension of this workflow.
