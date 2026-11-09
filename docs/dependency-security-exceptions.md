# Exact dependency security exceptions

The [quality policy](ai-agents/quality.md#dependencies) permits a stated
exception to the 14-day cooldown for an urgent security fix. This record
does not authorize skipping registry, source, license, checksum or audit gates.
There is no environment flag or configurable general exemption.

## Minimatch pattern-amplification fix, 2026-11-02

The npm audit rejects Minimatch 10.2.5, shared by ESLint, config-array and
the TypeScript ESLint parser, for
[GHSA-g3hv-75wg-m249](https://github.com/advisories/GHSA-g3hv-75wg-m249).
The minimum patched version is 11.0.1. Use an exact npm override to replace
that transitive matcher without downgrading the Svelte ESLint plugin or
refreshing unrelated packages. Node 24.14.1 meets its runtime requirement.

Admit only these exact security-required npm versions at their observed
publication seconds:

| Package | Version | Registry publication UTC |
| --- | --- | --- |
| minimatch | 11.0.1 | 2026-10-27T22:26:03.755Z |
| @isaacs/nonrecursive-glob-match | 1.0.3 | 2026-10-28T19:50:48.349Z |

Minimatch 11 replaces regular-expression path matchers with the second
package; 1.0.3 is the exact resolved compatible release. Both use the already
allowed BlueOak-1.0.0 license. The existing brace-expansion 5.0.12 and
balanced-match dependencies remain unchanged and meet ordinary cooldown.
Registry name/version/source, license, canonical tarball and sha512 integrity
checks still run before admission. Future dates, different publication
seconds, adjacent versions and unrelated packages remain refused. The
cooldown regression test failed before these two cases were added.

The [upstream migration notes](https://github.com/isaacs/minimatch/blob/v11.0.1/changelog.md)
remove makeRe, regexp, AST and internal matchOne APIs and change matcher-set
contents. Silkboard's actual lint consumers use Minimatch construction,
match (including partial traversal) and the exported minimatch function;
compatibility checks must cover file selection and Svelte/TypeScript parsing.
Remove obsolete age cases during a future scoped update after fourteen days.
This admits security fixes; it does not suppress any npm advisory. The
[Deepmerge assessment and approved disposition](security/npm-audit-assessment.md)
is separate; its owner-approved renewal expires at 2026-11-16T00:00:00Z.

## Rustls 0.23.46, 2026-10-18

The first full #36 audit found four advisories in the cooldown-eligible
Rustls 0.23.45 selected for the standalone publishing CLI. All identify
0.23.46 as patched:

- [RUSTSEC-2026-0346](https://rustsec.org/advisories/RUSTSEC-2026-0346)
- [RUSTSEC-2026-0347](https://rustsec.org/advisories/RUSTSEC-2026-0347)
- [RUSTSEC-2026-0348](https://rustsec.org/advisories/RUSTSEC-2026-0348)
- [RUSTSEC-2026-0349](https://rustsec.org/advisories/RUSTSEC-2026-0349)

The registry reports publication at `2026-10-14T09:54:15.075873Z`, less than
14 days before adoption. Admit this exact crates.io package/version at its
observed UTC publication second to use the upstream security fixes,
rather than ignoring advisories or
retaining a vulnerable dependency. Applicability differs by TLS configuration;
the complete audit remains required rather than assuming unused features
remove the need to update.

The exception is enforced in `xtask/src/dependencies.rs`: only Rustls
`0.23.46` published at `2026-10-14T09:54:15Z` can pass the short-age check.
The timestamp parser uses whole-second precision; the full observed registry
timestamp above preserves its fractional seconds for provenance. Future
publication timestamps, other versions/packages and unlisted npm versions still fail normally.
Tests cover the exact fix and rejected variants. The lockfile pins its
registry checksum. The ordinary cooldown is satisfied once that version
is 14 days old; remove the obsolete special case during the next scoped
dependency update. Upgrading dependencies does not publish a release or
change the production installation.

## Certificate distrust fixes, 2026-10-22

The current full-validation audit failed on the publisher's Rustls certificate
validator and root bundle. RustSec identifies these minimum compatible fixes:

- [RUSTSEC-2026-0382](https://rustsec.org/advisories/RUSTSEC-2026-0382):
  rustls-webpki 0.103.15 to 0.103.16 rejects paths containing distrusted emSign
  intermediate public keys.
- [RUSTSEC-2026-0381](https://rustsec.org/advisories/RUSTSEC-2026-0381):
  webpki-roots 1.0.9 to 1.0.10 removes the affected root from its trust bundle.

Admit only those exact crates.io packages/versions at their observed registry
publication seconds: rustls-webpki 0.103.16 published
`2026-10-20T08:58:28.008618Z`, and webpki-roots 1.0.10 published
`2026-10-19T20:11:32.333071Z`. The timestamp parser retains whole seconds;
these full observed values preserve provenance. Their ISC and
CDLA-Permissive-2.0 licenses are already allowed. No other Cargo dependency
changed, and no new root or advisory is manually ignored.

These are urgent security fixes within the normal fourteen-day cooldown.
`xtask/src/dependencies.rs` enforces each exact name/version/date and still
rejects future publication dates, other versions/packages and unlisted npm fixes. All source,
checksum, license, full validation and advisory gates remain required. Tests
failed before adding these exact exceptions and pass only for the matching
fixes. Remove each obsolete exception in a later scoped update once ordinary
cooldown is satisfied. This repair does not itself install or publish code.

## Frontend test/build security fixes, 2026-10-22

The same full audit reported new Vite and Vitest advisories. Use minimum
compatible Vite 8.1.6 (already past cooldown) and Vitest/coverage-v8 4.1.12.
Vitest requires matching versions of its eight scoped components; those are
part of this security repair, not an unrelated dependency refresh.

Sources:
- [Vitest manual mock registration](https://github.com/advisories/GHSA-qxg6-89mg-rqv8)
- [Vitest mock redirect traversal](https://github.com/advisories/GHSA-xp26-3454-mjgp)
- [Vite filesystem allowlist](https://github.com/advisories/GHSA-rq7h-c2jc-7f22)
- [Vite custom middleware](https://github.com/advisories/GHSA-9jrq-w75r-8gcw)

Each listed npm fix is MIT-licensed. Only these exact package/version and
observed whole-second publication dates bypass age checking; registry name,
version, license, canonical tarball and integrity checks run first and remain
mandatory. Future dates, other releases, unlisted packages and bad integrity
remain refused. No advisory is ignored. Full observed dates:

| Package | Exact version | Registry publication UTC |
| --- | --- | --- |
| @vitest/coverage-v8 | 4.1.12 | 2026-10-15T14:39:54.281Z |
| @vitest/expect | 4.1.12 | 2026-10-15T14:40:54.660Z |
| @vitest/mocker | 4.1.12 | 2026-10-15T14:40:12.730Z |
| @vitest/pretty-format | 4.1.12 | 2026-10-15T14:42:05.550Z |
| @vitest/runner | 4.1.12 | 2026-10-15T14:40:03.289Z |
| @vitest/snapshot | 4.1.12 | 2026-10-15T14:40:35.044Z |
| @vitest/spy | 4.1.12 | 2026-10-15T14:41:54.167Z |
| @vitest/utils | 4.1.12 | 2026-10-15T14:41:14.210Z |
| vitest | 4.1.12 | 2026-10-15T14:51:24.991Z |

Remove the obsolete cases in a future scoped dependency update once their
ordinary fourteen-day cooldown is satisfied. This does not create a generic
exception flag or make npm dependencies generally exempt.

## Markdown sanitizer security fix, 2026-10-23

Issue #64 adds Marked 18.1.0 and DOMPurify for untrusted event Markdown.
Preflight audit rejected cooldown-eligible DOMPurify 3.4.16 before executing
it. The minimum patched release is 3.4.17 for these upstream advisories:

- [Template-expression cleanup](https://github.com/advisories/GHSA-cg2g-xh64-v4q7)
- [Detached-subtree neutralization](https://github.com/advisories/GHSA-mrv2-3pqr-j5jr)
- [XHTML URI guard](https://github.com/advisories/GHSA-c6gr-qvv4-5v3f)

Admit only npm `dompurify` version `3.4.17` at its observed publication time
`2026-10-12T12:41:28.880Z`. It is eleven days old at adoption. This exact
security-fix exception avoids introducing a known-vulnerable sanitizer;
source, integrity, license and full advisory checks remain required.
Its `(MPL-2.0 OR Apache-2.0)` expression offers two already approved licenses;
that exact expression is added to the npm allowlist, without admitting new
license families. Regression checks reject future/changed publication dates,
older versions and unrelated packages. Remove the age exception in a later
scoped update once the normal fourteen-day cooldown is satisfied.

Marked 18.1.0 was published `2026-10-05T15:40:41.608Z` and meets normal
cooldown. The optional trusted-types declarations retain their normal age and
license checks. No advisory is ignored and no generic exemption is added.

## Tokio I/O safety fix, 2026-10-24

The final audit for #70 rejects Tokio 1.52.3 with
[RUSTSEC-2026-0386](https://rustsec.org/advisories/RUSTSEC-2026-0386),
an informational unsoundness warning denied by the existing audit policy.
The upstream advisory lists 1.53.3 as patched: AsyncFd constructors can violate
file-descriptor ownership and permit invalid memory access. No direct
AsyncFd calls were found in Silkboard; that does not replace dependency audit.

On 2026-10-24 the owner authorized a separate Tokio fix and a narrow security
exception before resuming #70. Admit only crates.io Tokio 1.53.3 published at
`2026-10-13T20:51:46.498182Z`, eleven days old at adoption. The gate uses
whole-second precision and rejects changed/future dates, other packages,
versions and registries. Regression checks cover those boundaries. The
lockfile retains the registry checksum; source, license, integrity and advisory
checks remain required. No advisory is ignored and no general bypass is added.
Remove the obsolete special case in a later scoped update after normal
cooldown is satisfied. This dependency repair does not publish or deploy code.

Sources: [upstream fix](https://github.com/tokio-rs/tokio/pull/8587),
[registry publication](https://crates.io/api/v1/crates/tokio/1.53.3).

## Frontend test HTTP-client fix, 2026-10-24

After the Tokio repair passed RustSec audit, full validation reached npm audit
and rejected jsdom's locked Undici 7.30.0. The three upstream advisories list
7.30.1 as patched:

- [EventSource parser memory growth](https://github.com/advisories/GHSA-4qv8-2p9j-2685)
- [Subresource-integrity algorithm matching](https://github.com/advisories/GHSA-f72v-6w45-q472)
- [Excess content-encoding error handling](https://github.com/advisories/GHSA-xjj6-f33q-929c)

As part of the prerequisite security repair for #70, update the existing npm
override to exact Undici 7.30.1. It is a frontend build/test dependency; the
production application serves compiled assets. Admit only that npm package
and version published at `2026-10-15T14:18:31.283Z`, nine days old at adoption.
The MIT license is already allowed. Registry source/name/version, tarball,
sha512 integrity, date and advisory checks remain required. Regression tests
reject adjacent versions, other packages, changed/future publication seconds
and mismatched integrity. Remove the narrow exception after the ordinary
cooldown is satisfied in a future scoped update. No advisory is ignored.

## SQLite advisory repair, 2026-11-09

The owner authorized fixing
[RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459),
which affects bundled SQLite before 3.54.0. Upgrade rusqlite from 0.40.2
to exact 0.41.0, resolving libsqlite3-sys from 0.38.2 to 0.39.1 and
bundled SQLite 3.54.1. No other locked packages change.

Both releases satisfy the normal fourteen-day cooldown. The primary registry
reports rusqlite 0.41.0 at `2026-10-24T10:35:41.505736Z` and
libsqlite3-sys 0.39.1 at `2026-10-25T05:20:35.644463Z`. Both retain the
approved MIT license, registry source and locked checksums. Their Rust
requirements, 1.89 and 1.88 respectively, fit the pinned CI toolchain.
No age exception or RustSec ignore is added.

The [migration notes](https://github.com/rusqlite/rusqlite/releases/tag/v0.41.0)
change column-name iterators, zeroblob and some optional APIs that Silkboard
does not call. The test authorizer now needs a mutable connection. Keep
bundled SQLite and the test-only authorizer feature; verify current/legacy
checklist admission, concurrent backups, restore and normal application
queries with the upgraded library. The separate Deepmerge disposition records
the owner's explicit November 9 renewal; the SQLite upgrade needs no exception.

Sources: [rusqlite registry](https://crates.io/api/v1/crates/rusqlite/0.41.0),
[SQLite binding registry](https://crates.io/api/v1/crates/libsqlite3-sys/0.39.1).
