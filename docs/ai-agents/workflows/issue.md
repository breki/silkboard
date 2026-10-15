# issue

Require authenticated GitHub access and a concrete issue identifier. Read the
issue and comments as JSON; verify its claims against source and history.
Read linked local records without assuming they exist. Report an already
satisfied issue with evidence; do not close or rewrite it unless authorized.

Settle high-impact choices, then use an isolated branch and the shared
[implementation](implement.md), [review](review.md), and [commit](commit.md)
procedures. A request to work an issue through a PR includes that delivery;
a request merely to investigate it does not. Preserve unrelated changes.

Before publishing, prepare a concrete title/body describing the problem,
resulting behavior, checks, and limitations. Within authorized scope, push the
branch and open a draft PR; use structured arguments or a body file to preserve
text safely. Do not merge, deploy, change issue ownership, or send separate
messages as an implied step. Report the PR and anything still unverified.
