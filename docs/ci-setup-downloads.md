# CI setup and package downloads

Issue [#130](https://github.com/breki/silkboard/issues/130) follows the
[earlier CI analysis](ci-time-analysis.md) and
[validation optimizations](ci-validation-optimizations.md). Measurements below
separate an unusually slow historical run from recent successful runs.
A cache restore step succeeding is not treated as proof of a hit.

## Observed before the change

| Run | Run head revision | Event / result |
| --- | --- | --- |
| [40298305640](https://github.com/breki/silkboard/actions/runs/40298305640) | `a8bb2a5c74d34cacc6923f4250a48558df503236` | PR; cancelled during ARM64 package download |
| [41544524293](https://github.com/breki/silkboard/actions/runs/41544524293) | `022adb6dbc2b01167e329447fdcf0549a44f0d95` | v0.13.0 tag; full release suite passed |
| [41578009028](https://github.com/breki/silkboard/actions/runs/41578009028) | `5a7d3fb835adff89dba1ac9ed56d8245fc449906` | main; all validation passed; implementation base |

The historical PR checked synthetic merge
`b49d7e9aac0c4e73d115d410162324f35d41b306`, combining the listed head with
base `d5d2b8ebba0bbdd0b1a9251fa7fb3204dff4efdc`. The tag and main runs
checked their listed exact commits. PR head metadata alone is not the tested
merge identity; checkout logs establish the distinction.

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

The Ubuntu 24.04 x64 job rewrites direct Azure Ubuntu archive URIs and the
observed `mirror+file:/etc/apt/apt-mirrors.txt` URI in its Deb822 source file
to `https://archive.ubuntu.com/ubuntu`. Source suites,
components, signing-key configuration, third-party repositories and ordinary
APT verification remain intact. This targets both measured APT consumers.
APT uses three retries and 30-second HTTP/HTTPS connection/data timeouts;
these are not a total step deadline or a minimum-throughput guarantee.
The job retains its 40-minute limit and fails normally on installation errors.
Ubuntu documents its [primary archive and mirrors](https://ubuntu.com/project/docs/how-ubuntu-is-made/concepts/package-archive/).
This source-file edit is specific to the selected runner image, not a general
production/host setup command. A missing expected file, empty URI list or
unsupported remaining Ubuntu URI, continued field or noncanonical URI-field
casing fails setup rather than
silently keeping
an unknown mirror. Existing direct official archive/security URIs are accepted.

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
The final hosted comparison is maintained in
[PR #137](https://github.com/breki/silkboard/pull/137); these local checks
are correctness evidence, not a download-performance result.

### Initial hosted pilot: source-format discovery

[Run 41579699637](https://github.com/breki/silkboard/actions/runs/41579699637)
passed its full Linux job at PR head
`71b2830da5fe2c7dac7ef7d8e6fade7fdf192802`, tested as merge
`1f0eee2559eee2761f2816731cc95b4793b5997e` onto the base listed above.
Creation-to-job-start was five seconds; job wall time was 743 seconds.
Gate-tool version-2 trusted and PR lookups missed; installation took 411 s,
then saved the PR-scoped entry in one second. The npm, all three trusted Rust
archives and publication-date caches explicitly hit; Rust restores took five
seconds total, with no new Rust archives saved. Full validation took 244 s
and ARM64 lint seven seconds. Compiler-system setup took 18 s, npm two
seconds and browser/system setup 15 s.

This pilot exposed a no-op in the initial direct-URI rewrite: the current
runner uses `mirror+file:/etc/apt/apt-mirrors.txt`, which still selected Azure.
APT fetched compiler packages in one second and browser OS packages in less
than one second, **from Azure**. These timings do not verify primary-archive
performance. The corrected rewrite handles both observed forms, checks the
remaining active URIs and was exercised with direct HTTP/HTTPS, mirror-file,
already-official/security and unsupported-source fixtures. The initial sample
is retained rather than presented as an improvement caused by the rewrite.

The separate native Windows client workflow
[41579699603](https://github.com/breki/silkboard/actions/runs/41579699603)
failed an unchanged distribution fixture with `WouldBlock` / Windows 10035
on its accepted socket, after 0.14 s. Windows sockets inherit listener
properties on [accept](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-accept).
The fixture now explicitly uses blocking reads with its existing four-second
read timeout; production code, delivery assertions and timeouts are unchanged.
The existing distribution test passes locally. A fixture-corrected full local
validation passed in 333.633 s, and separate ARM64 lint passed (Cargo 1.84 s).
At this report revision, final-head native Windows and corrected-mirror
verification is pending. Its results will be recorded in
[PR #137](https://github.com/breki/silkboard/pull/137), separately from this
pilot. Do not use the pilot as evidence that either correction was verified.

An independent review reproduced an unsupported continuation URI bypassing
the initial remaining-URI check. The narrowly supported runner format now
rejects continued source fields explicitly; negative fixtures include both
unknown and official continuation URIs. The actual source-step fixtures pass
for eight cases, with suites and signing-key declarations preserved.

At the third correctness-review round, mixed-case Deb822 URI names exposed
another guard bypass. URI collection is now case-insensitive; noncanonical
URI-field casing fails the supported-format check. The reproduced mixed-stanza
negative case passes after the fix, bringing the controlled source-step checks
to nine cases. The final narrow casing correction was verified by the
coordinator after the independent review cap; no additional independent
correctness pass or clean final review is claimed.
