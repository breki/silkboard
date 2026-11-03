# Build optimization skill evaluation

Issue [#131](https://github.com/breki/silkboard/issues/131), evaluated
2026-11-03 against frozen source base
`4ebea67b46774056fc603f92a709ea7cb999d126` plus the new
[shared procedure](ai-agents/workflows/optimize-builds.md) and wrappers.
This tests guidance and decisions, not a new pipeline optimization.

## Method

Three independent Codex agents received the skill path, a realistic request
and frozen raw artifacts. They started without the coordinator conversation,
expected answers, findings or proposed fixes. Each could read the snapshot
and linked canon; builds/tests, network access, configuration changes and
external writes were outside the exercise scope. Historical artifacts were
explicitly identified as such. No live production access was exercised.

The cases used the #68 profile-run JSON, #130 Actions job metadata and
cache/download log excerpts, and a frozen v0.12.0 release observation with
the operator-copied billing annotation. The release artifact also included
the historical 606-second v0.5.1 build and healthy v0.11.0 status. These
artifacts are input evidence, not current service claims.

The coordinator assessed the actual responses for correct bottleneck
attribution, useful bounded verification, counterevidence, current helper
reuse, and preserved quality/authority. Responses were returned in agent
messages and assessed directly by the coordinator, without separate response
files. This was a manual behavioral
assessment, not a wording/heading matcher or proof of general agent behavior.
The later document review checked the record and raw inputs, without
independently verifying those response summaries.

## Initial cases

| Request | Observed response and assessment |
| --- | --- |
| Local Rust tests: assess a suggested switch from level 1 to 0 | Checked the actual manifest: workspace already 0, dependencies 1. Reproduced timing totals/medians, retained the faster level-1 edit counterexample, and recommended an existing focused status target. Proposed a 15-minute future comparison with two jobs/threads and nonzero test/freshness checks. No profile change or new benchmark was executed. Meets the local case criteria. |
| Forty-minute Chromium/compiler CI timeout: cache browsers or cut validation? | Identified slow APT downloads and tool-key misses despite npm/Rust hits; excluded cancelled lint from passing evidence. Read actual final URLs/cache keys and preserved all gates. Recommended inspecting ordinarily scheduled runs rather than launching reruns. It discussed healthy download observations but omitted the healthy whole-job time from its summary. Useful diagnosis, with the comparison refinement below. |
| Signed v0.12.0 passed locally but production remains v0.11.0 | Identified failed release admission and the billing/spending annotation, without calling it proven quota exhaustion or a build failure. Distinguished publication, exact-suite eligibility, unknown installed trust and eventual production activation. Kept #63 cache work separate, proposed bounded ARM64 measurements under existing limits and a separately reviewed/operator-installed helper. No live state, trust change or deployment was inferred. Meets the release case criteria. |

All three supplied concrete next actions and stated unverified facts. The
agents reported read-only inspection and calculations, without performing
builds, CI reruns or operational actions. Local and release requests followed
their relevant guides; the CI request used the CI guides and actual workflow.
None introduced a new cache, weakened a threshold or revived declined #69.

## Refinement and repeat

The CI answer's missing healthy whole-job comparison motivated one narrow
clarification: when available, show healthy and after-change job elapsed time
even if the new run is slower. A fresh independent agent received the revised
skill and old, healthy-base and final raw CI artifacts, with the same request
and read-only limits. No intended answer or earlier conclusion was supplied.
The repeated answer included all three job times (40m22s cancelled, 5m05s
healthy base, 5m55s final), explicitly reported the 50-second regression
against the healthy base and retained the tool-cache reliability benefit
without a general speedup claim. It preserved the gates and proposed one
naturally occurring trusted cold run plus two warm runs, rather than new
benchmark reruns. This meets the refined CI comparison criteria.

The local/release guidance was unchanged by this CI-specific refinement;
those branches were not rerun. These are a small set of simulated requests,
not exhaustive behavioral or adversarial testing, hosted timing validation,
production rollout evidence or a guarantee of correct future tool use.

## Structural checks

The skill uses the repository's existing `.agents/skills/` discovery layout
and shared workflow pattern; a matching Claude command forwards to the same
canon. The catalog explains that a running client may need to reload skills.
No global installation or UI-specific metadata is needed for this layout.

Node checks validated the required frontmatter, name/folder, description and
unfinished-scaffold constraints; the existing locked Prettier YAML parser
validated syntax. The bundled Python validator was not run because repository
rules exclude Python tooling/tests. Canon/relative-link and diff checks cover
the wrappers and shared guide. No application, dependency, cache or release
configuration changed, so no full application validation is claimed by these
local documentation checks.

The matching Claude command was checked for forwarding/link consistency;
Claude execution and live client skill reload were not exercised. The
new skill is discoverable from its repository layout after the client loads
that catalog; no global installation was performed.
