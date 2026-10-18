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
publication timestamps, other versions/packages and npm still fail normally.
Tests cover the exact fix and rejected variants. The lockfile pins its
registry checksum. The ordinary cooldown is satisfied once that version
is 14 days old; remove the obsolete special case during the next scoped
dependency update. Upgrading dependencies does not publish a release or
change the production installation.
