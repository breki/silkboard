# Exact dependency security exceptions

The [quality policy](ai-agents/quality.md#dependencies) permits a stated
exception to the 14-day cooldown for an urgent security fix. This record
does not authorize skipping registry, source, license, checksum or audit gates.
There is no environment flag or configurable general exemption.

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
