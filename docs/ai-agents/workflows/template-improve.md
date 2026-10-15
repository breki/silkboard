# template-improve

With a specific finding, read the relevant upstream behavior and local
divergence. With a sweep request, establish the requested commit/date range
(or the last documented sweep), inspect actual diffs and existing feedback,
and present evidence-backed candidates before recording the selected ones. If
that range cannot be established, ask rather than inventing a watermark.

For each accepted finding, capture a concise feedback entry under
`docs/developer/template-feedback.md` using the configured feedback helper, or
explicitly use manual bookkeeping before that helper exists. Follow [record
rules](../records.md): unique ID, source revision, problem, consequence,
proposed change, and status. Distinguish open divergence, resolved feedback,
and suggestions to flow upstream. Link existing entries rather than
duplicating them. Do not create an upstream issue or send a message
implicitly.
