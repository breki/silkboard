# Workflow catalog

Shared procedures are authoritative. Codex and Claude use the same workflow
names: invoke `$review` in Codex or `/review` in Claude Code. Claude exposes
`architect` and `web-dev` as skills; the others are commands.

Wrappers become discoverable when the client loads repository skills;
a current session may need to reload them. A procedure can also be read
directly. See [tooling status](tooling.md) for unavailable build helpers.

| Workflow | Purpose | Adapted from |
| --- | --- | --- |
| [check](workflows/check.md) | Type-check Silkboard targets without running tests. | Both |
| [test](workflows/test.md) | Run Silkboard behavioral tests, optionally filtered. | Both |
| [validate](workflows/validate.md) | Run Silkboard quality gates and report unrun checks. | Both |
| [implement](workflows/implement.md) | Plan and implement a requested Silkboard change or captured task. | Both |
| [commit](workflows/commit.md) | Commit authorized Silkboard changes without review or version bumps. | Bombyx (separate review); rustbase conventions |
| [release](workflows/release.md) | Prepare a validated Silkboard SemVer release and annotated tag. | Both; Hoard release separation |
| [review](workflows/review.md) | Review a fixed Silkboard change with reviewers matched to its content. | Bombyx, adapted reviewer selection |
| [code-reviewers](workflows/code-reviewers.md) | Explain or invoke Silkboard review roles for a requested change. | Both |
| [todo](workflows/todo.md) | Capture or list Silkboard backlog items without implementing them. | Both; explicit pre-tooling fallback |
| [issue](workflows/issue.md) | Handle an authorized GitHub issue through implementation and a draft PR. | Bombyx |
| [handoff](workflows/handoff.md) | Write a self-contained Silkboard session handoff from observed state. | Bombyx |
| [rundown](workflows/rundown.md) | Summarize the Silkboard session without changing repository state. | Bombyx |
| [retrospect](workflows/retrospect.md) | Review a Silkboard workflow for useful process improvements. | Both |
| [docs-audit](workflows/docs-audit.md) | Audit Silkboard documentation without editing or deleting it. | Bombyx |
| [html-report](workflows/html-report.md) | Create an offline local HTML report from requested material. | Both |
| [backlog-report](workflows/backlog-report.md) | Generate a local report of Silkboard backlog and GitHub issues. | Bombyx |
| [ask](workflows/ask.md) | Present the latest unresolved Silkboard choice through the question UI. | Bombyx |
| [short](workflows/short.md) | Restate the last reply or answer a supplied instruction in forty words. | Bombyx, without speech-specific rules |
| [architect](workflows/architect.md) | Explain or assess Silkboard architecture against its agreed roadmap. | Both |
| [web-dev](workflows/web-dev.md) | Develop or investigate Silkboard Axum, Svelte, and browser behavior. | rustbase |
| [update-deps](workflows/update-deps.md) | Update selected Silkboard dependencies with cooldown and audit checks. | Both |
| [template-sync](workflows/template-sync.md) | Selectively adapt rustbase updates while preserving Silkboard choices. | Both |
| [template-improve](workflows/template-improve.md) | Record feedback for rustbase without publishing it upstream. | Both |
| [template-backfeed](workflows/template-backfeed.md) | Prepare rustbase improvements from Silkboard without implicit publication. | rustbase; Bombyx template-maintenance rules |
| [simplify](workflows/simplify.md) | Simplify changed Silkboard code while preserving tested behavior. | Bombyx built-in intent, portable shared procedure |

The upstream `blue-pencil` agent is an editorial role, covered here by
fresh-reader rather than a fourth review pass. `code-reviewers` routes to
the same review procedure. `simplify` has a portable procedure instead of
depending on an upstream built-in being installed. Template backfeed is
conditional on an authorized upstream checkout, not a local xtask claim.

No command triggers hosting, VM control, external messages, or a release
just because another workflow completed. Commands preserve the active
client mode and its permissions.
