# npm audit assessment and temporary disposition

Assessed 2026-11-02 against the security maintenance branch based on
`21265b593d3c7e852fe69b93d4eda54fbbcff8cc`. This separates a patched
dependency from an advisory whose CVE authority has rejected the finding.

## Minimatch: update the affected matcher

[GHSA-g3hv-75wg-m249](https://github.com/advisories/GHSA-g3hv-75wg-m249)
affects versions before 11.0.1. The locked 10.2.5 was shared by ESLint
10.7.0, config-array 0.23.5 and TypeScript ESLint typescript-estree 8.64.0.
An exact 11.0.1 override removes the affected version without npm's suggested
downgrade of eslint-plugin-svelte to 1.1.2. Its new matcher dependency and
short-cooldown rationale are recorded in the
[security dependency record](../dependency-security-exceptions.md#minimatch-pattern-amplification-fix-2026-11-02).

Version 11 removes regular-expression/internal matching APIs. The installed
consumers use the retained constructor, match/partial-match and minimatch
function APIs. Verify actual ESLint file discovery, ignored files, Svelte
and TypeScript parser selection, normal frontend build/tests and full gates.
This is a security fix, not permission to refresh unrelated dependencies.

## Deepmerge: authority rejection and local reachability

The locked chain is `@sveltejs/vite-plugin-svelte@7.2.0` to
`deepmerge@4.3.1`. npm still reports
[GHSA-gcqx-w48m-w2h4](https://github.com/advisories/GHSA-gcqx-w48m-w2h4),
with no patched version. Updating to the current plugin alone does not remove
its Deepmerge dependency.

The authoritative [CVE record at observed revision 790fe3b](https://github.com/CVEProject/cvelistV5/blob/790fe3b1147c6f11580e9b00b18abcf2c7534af4/cves/2026/93xxx/CVE-2026-93753.json)
marks CVE-2026-93753 **REJECTED**, dated 2026-11-02T16:33:19.357Z, because
the record does not describe a security vulnerability. This is stronger
evidence than an unconfirmed comment; the
[upstream discussion](https://github.com/TehShrike/deepmerge/issues/273)
also records the dispute and reproduction. At assessment time the npm/GitHub
advisory feed had not caught up. Do not infer withdrawal of the GitHub entry
from the CVE's status or classify the current npm audit as passing.

The local input path was inspected in the locked plugin source:

- `src/utils/load-svelte-config.js` dynamically imports the repository's
  executable Svelte configuration before merging.
- `src/utils/options.js` merges plugin defaults, loaded Svelte configuration,
  inline Vite options and locally derived build/server options. It performs
  both pre-resolution and final-resolution configuration merging.
- Silkboard's [Svelte configuration](../../frontend/svelte.config.js) selects
  the Vite preprocessor; its [Vite configuration](../../frontend/vite.config.js)
  invokes `svelte()` with no inline plugin options. No dashboard report,
  request, token, database row or rendered Markdown supplies merge inputs.
- No application-source import of Deepmerge or Minimatch was found. They
  belong to the build/lint toolchain; serving the compiled frontend and the
  Rust application does not execute this plugin's configuration merge.

This conclusion assumes trusted source and executable build configuration.
Validation of a hostile PR is already execution of that PR's configuration,
not a sandbox provided by Deepmerge. The reported case changes properties
visible on the returned object, rather than globally changing all object
prototypes. It is not evidence of a live Silkboard compromise, credential
disclosure or a reason to change production credentials.

## Approved temporary exception

**Status: approved on 2026-11-02; explicitly renewed on 2026-11-09.**
The owner accepted the exact proposal after reviewing draft PR #135. The
gate still runs RustSec with denied warnings and live npm audit at the low
threshold; it accepts only the advisory's exact recorded finding
and its one propagated plugin entry before the expiry below.

| Bound | Approved value |
| --- | --- |
| Advisory | Only GHSA-gcqx-w48m-w2h4 / CVE-2026-93753 |
| Dependency | Only Deepmerge 4.3.1 through Svelte Vite plugin 7.2.0 |
| Context | Repository-controlled build configuration only |
| Reason | Authoritative CVE rejection plus inspected local input path |
| Expiry | 2026-11-16T00:00:00Z; review earlier if the feed changes |
| Default | Every other advisory and all unavailable/malformed evidence fail |

The [evaluator](../../xtask/src/npm_audit.rs) parses the live npm JSON audit
result and the committed lockfile. The subprocess adapter captures exit code,
stdout and stderr only for the fixed JSON audit invocation. A clean report
requires status zero and zero counts. Status one is accepted only for the
complete recorded two-entry report, including advisory URL/ID, severity,
range, affected paths, propagation, fix availability and metadata counts.
Changed report fields require review rather than broadening the exception.

The lockfile must contain only Deepmerge 4.3.1 through the inspected Svelte
plugin 7.2.0 dependency; another copy or consumer is refused. Other statuses,
diagnostics, malformed/unsupported/oversized reports, changed lock bindings,
additional findings and expiry fail closed. No environment variable or CLI
argument can extend the exception or inject a clock. It does not omit dev
dependencies or raise the severity threshold. Accepted findings print an
explicit temporary-exception diagnostic rather than claiming a clean scan.
The Rust advisory audit and every other quality gate remain mandatory.

Remove the temporary implementation as soon as npm no longer reports this
advisory associated with the rejected CVE. Zero-finding scans pass after the
deadline; this finding is refused at or after the exact expiry instant.
Renewal is not automatic. This approval does not authorize merging,
publishing a release or changing production.

## Explicit seven-day renewal, 2026-11-09

The original exception expired at 2026-11-09T00:00:00Z and correctly blocked
the audit after the SQLite repair. The owner reviewed the exact date-only
renewal proposal and explicitly approved seven more days, ending at
2026-11-16T00:00:00Z. The advisory, package versions, consumer path, complete
report matching and all other rejection rules remain unchanged.

Rechecking the [current authoritative CVE record](https://raw.githubusercontent.com/CVEProject/cvelistV5/main/cves/2026/93xxx/CVE-2026-93753.json)
still returned REJECTED with the same November 2 rejection timestamp. Live
npm audit still returned the exact two approved entries; primary registry
metadata still offered no patched Deepmerge release, and the latest Svelte
plugin still depended on it. No frontend dependencies changed in this repair.
This renews the inspected build-configuration-only disposition; it does not
claim withdrawal of the GitHub advisory or a zero-finding npm scan.

Existing evaluator regressions continue to accept only the exact report
before expiry and reject it at/after the deadline, along with changed
versions, consumers, finding fields and malformed evidence. Clean scans
continue to pass after expiry. Further renewal is not automatic.
