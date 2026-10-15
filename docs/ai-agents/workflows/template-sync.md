# template-sync

Inspect provenance, Git state, and the requested upstream revision. Fetch/read
rustbase into a separate temporary location or use read-only GitHub APIs.
Do not replace the current repository with a generated template or overwrite
project history. Never invent a last-synced watermark: this canon adaptation
is not a full rustbase import.

Compare upstream changes against their recorded baseline. Use configured
sync-candidates tooling when installed; before then, explicitly compare source
revisions and tracked paths manually. Classify apply, adapt, skip with reasons.
Keep Silkboard's shared layout, separate review flow, boundaries, and owner
decisions. Ask about high-impact conflicts rather than silently adopting them.

Apply relevant authorized changes, check their callers and documentation,
and run available validation. Record the exact source revision, imported
scope, and deliberate differences. Update a sync watermark only for the scope
actually reconciled; pending application code is not synchronized by this step.
