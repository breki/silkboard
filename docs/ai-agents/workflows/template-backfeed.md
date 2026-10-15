# template-backfeed

Inspect Silkboard's feedback records and provenance. Identify generic changes
that belong in rustbase, separating project-specific deployment policy.
Prepare a candidate list with source commits, rationale, dependencies, and
validation evidence. If the upstream checkout/helpers are absent, deliver this
proposal locally rather than pretending a backfeed command ran.

With authorization to edit an upstream checkout, use its supported backfeed
diff/record helpers and create reviewable changes there. Advance its watermark
only after the corresponding changes are accounted for. External issues, PRs,
pushes, and messages require scope authorization; preparing candidates does
not imply publishing them. Do not mutate an unrelated repository merely because
it was used as a read-only source for canon.
