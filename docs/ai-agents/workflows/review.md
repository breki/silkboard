# review

Review never commits or releases. Default the comparison base to HEAD for
uncommitted work; for committed work use the requested base or the branch's
merge base. Include intended untracked files without staging unrelated work.

Create a fixed snapshot in a temporary directory: base and revision identity,
diff, path list, and copies of changed files, including untracked ones.
Include the supporting source/contracts each role needs in that frozen view.
Give reviewers snapshots, not a live working tree. Exclude review output from
the reviewed change. Use available targeted checks before review; record any
unavailable checks rather than claiming validation.

Read [reviewer roles](../reviewers.md). For code, config, automation, or
executable templates, run artisan once, then red-team. Add fresh-reader for
changed documentation, canon, comments that carry instructions, or user-facing
text. Prose-only changes use fresh-reader. Reviewers report; the coordinating
agent applies in-scope fixes between stages, validates them, and takes a fresh
snapshot. Changes to operational instructions can also justify red-team review.

Use independent agents when supported and authorized by the active workflow.
Run roles sequentially so each sees earlier fixes. If delegation is unavailable,
perform separate passes and disclose that they are not independent reviews.
Do not claim a reviewer ran merely because its instructions were read.

Give each reviewer prior findings and dispositions. Fix relevant defects and
log actionable deferred findings using [records](../records.md). Another
red-team round is warranted only if an actual behavior defect was fixed.
Stop when nothing found would be fixed, if fixes begin regressing, or after
three red-team rounds. At the cap, report unresolved issues rather than a clean
review. Resnapshot and revalidate after fixes; new behavior fixes after the
red-team stage need correctness review within that limit. Finish with findings,
fixes, deferrals, checked revision/snapshot, and verification limits.
