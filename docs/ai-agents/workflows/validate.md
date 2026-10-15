# validate

Read [quality](../quality.md) and [tooling status](../tooling.md). Inspect the
actual configured gates. Prefer the read-only validation form; do not claim
`--check` exists without checking help/source. If the only validator rewrites
files, say so and account for those edits before taking a review snapshot.
Run the full configured pipeline for code/release validation. On failure,
iterate on the failed gate, then rerun the full pipeline when warranted.
For documentation-only changes, check links, consistency, and Git whitespace.
Report exactly which validation ran. Missing release gates block a release.
