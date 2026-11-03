# CI setup and package downloads

Issue [#130](https://github.com/breki/silkboard/issues/130) follows the
[earlier CI analysis](ci-time-analysis.md) and
[validation optimizations](ci-validation-optimizations.md). Measurements below
separate an unusually slow historical run from recent successful runs.
A cache restore step succeeding is not treated as proof of a hit.

## Observed before the change

| Run | Exact source revision | Event / result |
| --- | --- | --- |
| [40298305640](https://github.com/breki/silkboard/actions/runs/40298305640) | `a8bb2a5c74d34cacc6923f4250a48558df503236` | PR; cancelled during ARM64 package download |
| [41544524293](https://github.com/breki/silkboard/actions/runs/41544524293) | `022adb6dbc2b01167e329447fdcf0549a44f0d95` | v0.13.0 tag; full release suite passed |
| [41578009028](https://github.com/breki/silkboard/actions/runs/41578009028) | `5a7d3fb835adff89dba1ac9ed56d8245fc449906` | main; all validation passed; implementation base |

| Selected validation job phase, seconds | Slow PR | v0.13.0 | Base main |
| --- | ---: | ---: | ---: |
| Run creation to selected job start | 3 | 34 | 3 |
| Whole selected job | 2422 | 767 | 305 |
| Gate-tool restore lookups | 0 | 1 | 2 |
| Gate-tool installation | 628 | skipped | skipped |
| Rust build archive restores | 12 | 1 | 8 |
| Rust publication-date restores | 0 | 0 | 1 |
| Rust build archive saves | 0 | 16 | 0 |
| Frontend + browser + browser system dependencies | 694 | 28 | 23 |
| Full validation | 350 | 624 | 221 |
| ARM64 packages + cross-target lint | 689, cancelled | 51 | 16 |

Times come from completed Actions job/step timestamps (whole-second
precision). The release job's 34-second pre-start interval includes admission
and dependency scheduling, not just runner queue time. The two direct CI
runs have no prerequisite job; their creation-to-start intervals combine
queue/dispatch overhead. Internal parallel validation gate durations overlap;
use the full validation step's wall time rather than adding gate durations.
A zero cache duration can mean a skipped save or a subsecond lookup; the
cache-outcome discussion distinguishes those cases. Whole-job time includes
setup, restore/save and cleanup; these selected rows
are not a complete partition of it. The timeout run took 40m22s including
runner cleanup after the 40-minute execution budget.

### Cache outcomes and cold setup

The slow PR restored all three trusted Rust dependency archives and trusted
publication-date evidence. Its npm download cache also explicitly hit. Those
caches were functioning; none caches Ubuntu packages or installed Chromium.
`npm ci` installed 247 packages in three seconds. Playwright's APT download
fetched 21.3 MB in 11m01s at 32.2 kB/s from Ubuntu's Azure mirror; browser
binary downloads began afterwards and completed in about 12 seconds. The
frontend delay was primarily OS packages, not npm or Chromium download.

Gate-tool lookups explicitly missed both trusted and PR keys ending in
`bc7eaaa1a8825009589e7b3bbd50862090f5de9329110e78f2a21d5fede51128`.
The preceding successful v0.11.0
[run 40261444811](https://github.com/breki/silkboard/actions/runs/40261444811)
hit the trusted key ending in
`bc62848e44afdf277a147816e44d59f88a8c072619b92ef6b1bd7a5d96e6c7f9`.
The old key hashed the entire workflow: an unrelated workflow edit changed
the namespace despite unchanged pinned tool versions. The slow PR saved its
new PR entry before later work; it could not populate main's trusted entry.
These observations establish a changed key and exact misses, not eviction.

The v0.13.0 job explicitly hit the trusted gate-tool and npm caches, but
missed all three Rust build keys and date evidence. The base main job hit
all of those trusted caches. Runner image/configuration namespaces must
remain bound for Rust artifacts; this change does not broaden those keys.
The existing [Rust cache procedure](ci-rust-build-cache.md) still owns their
compatibility, workspace pruning and save rules.

ARM64 installation requested 22 packages, 45.2 MB. In the slow PR, the
10.8 MB compiler preprocessor download occupied roughly four minutes and
the job was eventually cancelled before lint began. On v0.13.0, APT fetched
45.2 MB in six seconds; on base main, in three seconds. The same Azure mirror
can be fast. These are observational samples on different runners/revisions,
not evidence that every Azure download is slow or an isolated compiler-time
regression. No cancelled lint is counted as passing.

## Selected setup changes

The Ubuntu 24.04 x64 job rewrites only the Azure Ubuntu archive URI in its
Deb822 source file to `https://archive.ubuntu.com/ubuntu`. Source suites,
components, signing-key configuration, third-party repositories and ordinary
APT verification remain intact. This targets both measured APT consumers.
APT uses three retries and 30-second HTTP/HTTPS connection/data timeouts;
these are not a total step deadline or a minimum-throughput guarantee.
The job retains its 40-minute limit and fails normally on installation errors.
Ubuntu documents its [primary archive and mirrors](https://ubuntu.com/project/docs/how-ubuntu-is-made/concepts/package-archive/).
This source-file edit is specific to the selected runner image, not a general
production/host setup command. A missing expected file fails setup.

ARM64 OS-toolchain installation now precedes validation in its own step.
The later cross-target lint still runs, using its separate existing Rust
output directory. npm installation and Chromium/system-dependency setup
also have separate steps. This gives new runs distinct download/setup and
compilation timing without removing any work. Playwright retains its normal
system-dependency installation and package-manager checks.

Gate-tool cache version 2 hashes the
[dedicated pinned installer](../dev/ci-install-gate-tools), alongside the
existing OS/architecture/Ubuntu/Rust identities. Unrelated workflow changes
no longer invalidate these four binaries. The initial migration deliberately
misses version-1 entries. PR/trusted key separation, exact lookups, version
checks, locked installation outside application targets, and early explicit
save remain unchanged. Only `bin/` is cached; no application result is cached.

## Alternatives considered

| Option | Decision and tradeoff |
| --- | --- |
| Official Ubuntu archive + network bounds | Selected as a small change addressing both observed APT consumers; hosted measurements must establish its actual behavior. It cannot guarantee performance on every runner. |
| Cache downloaded `.deb` packages | Deferred: recent downloads take 3–6 seconds; a new archive would add restore/save cost, package-version churn and another trusted/PR boundary. It would still require metadata verification and installation. No unsafe installed-system snapshot or PR-to-trusted promotion is introduced. |
| Preinstalled cross-toolchain/container image | Deferred: image build/update/security maintenance and pulling an image add a new delivery surface. Current successful package setup does not justify that scope. |
| Cache Chromium binaries | Not selected: the slow sample's browser download took seconds; its OS dependencies caused the delay. [Playwright also advises against browser caching](https://playwright.dev/docs/ci#caching-browsers) because restore can cost as much as download and OS dependencies still need installation. |

## Verification and hosted comparison

Local syntax and controlled bootstrap-execution checks verify parsing,
application-target isolation and installation failure propagation. They do
not establish hosted mirror performance. The required local full validation,
review and hosted comparison are recorded below when observed; no speedup
is inferred from local package downloads or cache-step status alone.

The initial implementation passed local full read-only validation in
415.220 s (including the existing narrowly scoped, time-limited npm advisory
exception); separate ARM64 lint passed with Cargo reporting 10.39 s.
Controlled bootstrap execution verified external working/output directories,
locked installs and failure propagation. HTTP/HTTPS mirror fixtures preserved
source suites, signing-key declarations and unrelated sources. Bash syntax
and workflow parsing with the locked Prettier YAML parser passed. Independent
artisan, red-team and fresh-reader reviews found no actionable issues.
Hosted after-change results are pending; these local checks are correctness
evidence, not a download-performance result.
