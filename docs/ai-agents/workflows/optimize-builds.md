# optimize-builds

Diagnose the supplied Silkboard slow path and recommend or implement a measured
improvement within the requested scope. Begin with existing evidence before
starting new experiments. Diagnosis alone does not authorize implementation;
a request to improve a pipeline permits in-scope changes through
[implement](implement.md). Review, commit and release remain separate workflows.
Carry forward existing authorization; this skill adds none.

## Locate the cost

Identify the command, workflow run or release tag and the user's goal. If the
symptom is vague, inspect available timing records first, then ask one focused
question only if the target still cannot be determined. Read only the guides
relevant to that path; follow their linked measurements/helpers as needed:

| Slow path | Existing sources |
| --- | --- |
| Local edit/check/test or Vite | [Development guide](../../development.md), [worktree reuse](../../local-build-reuse.md), [focused feedback measurements](../../local-feedback-measurements.md), [test-profile comparison](../../test-profile-measurements.md) |
| Hosted setup, downloads or cache misses | [CI setup investigation](../../ci-setup-downloads.md), [CI timing analysis](../../ci-time-analysis.md), [cache policy](../tooling.md#ci-cache-boundaries), [current workflow](../../../.github/workflows/ci.yml) |
| Normal, coverage or ARM64 compilation | [Dependency archives and key inputs](../../ci-rust-build-cache.md), [validation optimizations](../../ci-validation-optimizations.md), [registry checking](../../ci-registry-performance.md) |
| Release preparation, eligibility or production build | [Releases](../../releases.md), [updater admission](../../automatic-upgrades.md), [release workflow](release.md), [guest build helper](../../../deploy/vm/guest-build.sh), [production reuse issue #63](https://github.com/breki/silkboard/issues/63) |

These sources incorporate #24/#31/#70's CI cache work, #86's timing analysis,
#66's local dependency reuse, #67's focused feedback, #68's profile evaluation
and #130's setup investigation. Verify current manifests, helpers and GitHub
status before choosing follow-ups; issue descriptions and dated measurements
can be stale. The owner closed [#69](https://github.com/breki/silkboard/issues/69)
as not planned: sccache is not a standing next step to revive automatically.

Check prerequisites for the chosen evidence: installed toolchain and gate
tools from [tooling status](../tooling.md), authenticated GitHub reads for
hosted runs, and an existing approved read-only operator route for installed
updater state. Use supplied artifacts if live access is unavailable; state
their date and limits. Actions run/job access does not imply Checks annotation
access or permission to rerun jobs. Report a denied capability; do not change
credentials or push a dummy commit to bypass it.

Separate queue/dispatch or prerequisite waiting, tool/dependency download and
installation, cache lookup/restore/save/cleanup, dependency admission, Rust
compilation, test execution, frontend/browser work, ARM64 setup/lint, release
eligibility, and production build/activation. Use `Validation timing:` for
individual gates and `Validation wall timing:` for the parallel suite; do not
sum overlapping gates. A failed-to-start job is not slow compilation, and a
cancelled setup step does not establish that its lint ran.

## Establish a comparable baseline

Retain command/run identity, head and actual tested checkout (including a PR's
synthetic merge), toolchain, runner image/architecture, profiles/features,
resource limits and result. A successful cache action is not proof of a hit:
read its log/key/ref and identify the cached object. Distinguish downloaded
sources, dependency artifacts, tool executables and immutable publication facts
from fresh workspace code, test execution and passing verdicts.

Define cold/warm explicitly. Compare the same scope and settings; keep downloads
outside a compilation experiment when measuring compiler work. Retain failures
and counterexamples, not just the fastest samples. Record compilation and
execution separately, total elapsed time, CPU, maximum child RSS and artifact
size when relevant. RSS is not total concurrent memory. Check that focused
filters execute tests; zero matches can exit successfully.

For local experiments, use the existing budgets and a bounded plan such as one
clean build, a few warm observations and a reversible small edit. Prefer a
focused target over repeated full validation while investigating. Keep release
profiles fixed when measuring test profiles. Do not reinstall frontend tools
or rebuild static assets for each Vite source edit. Shared-worktree commands
must use the [existing wrapper](../../../dev/cargo) and its whole-command lock;
use isolated outputs for profile experiments or long-running servers. Never
clean another active build or replace its lock file. Restore edited source
bytes and stop only experiment-owned processes.

For hosted work, use completed logs before requesting more runner time. Record
confirmed cache hits/misses and transfer costs, installed packages, actual
resolved download URLs and phase wall times. Verify the effective package
source; a rewrite can be a no-op when runner images use mirror-file indirection.
Compare with healthy runs as well as outliers. When available, include the
healthy and after-change whole-job times even if the new run is slower.
Different runner/image/cache cohorts do not isolate the effect of an
optimization. Local probes cannot establish hosted download speed.

For production, separate tag publication, exact suite success, installed trust,
build start, activation and observed health/version. A signed tag or a GitHub
Release object alone is insufficient. Check live state only through existing
authorized access; repository helper/trust files do not prove what is installed.
Missing logs/access leave the explanation unverified. Production reuse in #63
requires ARM64 cold/warm measurements and a separately reviewed helper rollout;
a local result or application release does not install that helper.

## Choose and verify an improvement

Name the measured bottleneck, likely mechanism and uncertainty. Select the
smallest useful change; keeping the current setting is valid when benefits
are unclear. Use existing helpers rather than introducing a second cache,
validator or benchmark framework by default. Explain setup, transfer, disk and
maintenance costs, including cold migration separately for PR/trusted scopes.
A reliability improvement is distinct from a typical-run speedup.

Before changing anything, state a bounded before/after verification plan: exact
scope and settings, resource/time budget, expected cache behavior, source-change
freshness/identity check, and evidence needed to accept or reject the change.
For production retain the current memory/concurrency limits and define safe,
bounded cache retention before proposing a rollout.

[Quality policy](../quality.md), [working rules](../working-rules.md), existing
cache trust/freshness rules and [release admission](../../releases.md) constrain
any optimization. Preserve dependency age/license/source/security checks,
coverage/duplication limits, fresh workspace/test/coverage execution, and
normal/instrumented/ARM64 separation. Never substitute an old binary or cached
verdict. Do not promote PR artifacts to trusted scopes, change independent
signer trust, skip exact-revision release checks, or grant dashboard/publisher
accounts build, repository or execution authority. A performance request does
not itself authorize merge, release publication, deployment or operational
access; follow any separately existing authorization through its workflow.

After an authorized change, use the appropriate quality/review procedures.
Report the measured before/after, failures, counterevidence, checks passed or
unrun, and remaining next action. Keep observed results separate from hypotheses
and local correctness separate from hosted/production proof. If normal runs
are no faster, say so plainly; do not advertise an outlier comparison as a
general speedup. Preserve durable evidence in the relevant source guide rather
than adding a parallel backlog or repeating policy in this skill.
