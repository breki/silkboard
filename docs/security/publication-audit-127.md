# Repository publication audit

Issue [#127](https://github.com/breki/silkboard/issues/127), audited
2026-10-26. This is a disclosure audit before open sourcing, not a live
penetration test or authorization to change repository visibility.

No credentials or private keys were confirmed in the inspected material.
Publication still needs an owner decision about the privacy findings below.
A clean scanner result is not proof that every possible secret is absent.

## Audited snapshot and coverage

The current-tree reference is `d5d2b8ebba0bbdd0b1a9251fa7fb3204dff4efdc`.
GitHub branches, tags and pull-request heads were fetched into a separate
local audit namespace without switching the owner's checkout. The inventory
included 66 remote branches, 16 tags and 65 pull-request heads, plus existing
local refs: 186 unique reachable commits and 1,389 unique historical blobs,
18,882,939 bytes. Deleted file versions were included. The clone was not
shallow. Unrelated uncommitted owner edits were preserved and excluded.

Gitleaks [8.30.1](https://github.com/gitleaks/gitleaks/releases/tag/v8.30.1)
was downloaded from its official release; its Linux x64 archive matched the
release's SHA-256 digest. The default rules were used with full
redaction, inline suppression disabled and no repository baseline or ignore
file. The Git-diff scan covered 117 commits reported by the scanner. A separate
scan of every extracted reachable blob covered unchanged and deleted content
independently of that diff count. Commit messages, author/committer metadata
and all 16 unique annotated tag objects were also scanned.

Supplemental rules checked Silkboard setup fragments, assigned opaque tokens,
Bearer credentials and HTTP URLs with embedded credentials. Manual review
covered 64-character hexadecimal candidates outside dependency lockfiles,
credential generation code, risky filenames and database signatures. Owner
setup and session tokens use 32 random bytes encoded as 64 hex digits;
checksums with the same shape require context rather than automatic removal.

GitHub coverage at the snapshot:

- 127 issue/PR bodies, 65 PR metadata records, one issue comment and no inline
  review comments. Review-body inventories for all 65 PRs were empty.
- 186 completed workflow runs plus nine retained earlier attempts:
  195 downloaded log archives in total. Extracted text was scanned. The one
  run still in progress at inventory time was excluded.
- No release records or Actions artifacts were returned. Git tags were
  inspected separately. No attachment URLs were found in the retrieved
  issue/PR text. Repository metadata showed no forks, wiki or discussions.
- The GitHub secret-scanning alerts API returned HTTP 403, "Resource not
  accessible by personal access token". Its alert status is unknown.

## Findings

### P127-1: historical deployment network identifiers

Severity: medium privacy concern; no demonstrated access bypass.

The production tailnet DNS suffix occurs in 22 historical blob versions across
10 paths. It is absent from all blobs in the audited current main tree. The
paths are `.bombyx/settings.json`, `.bombyx/tests/secrets.test.mjs`,
`deploy/vm/silkboard-progress.conf`, `docs/application-data.md`,
`docs/bombyx-development.md`, `docs/issues/bombyx-development-vm.md`,
`docs/owner-login.md`, `docs/owner-setup.md`, `docs/production-vm.md` and
`docs/publishing-cli.md`. Values are intentionally omitted here.

For example, production evidence entered in commit `0017951` and development
configuration entered in `4a5b31c` retain the original endpoint in history.
Commit `03b8f10` generalized deployment origins in current files; it did not
remove earlier revisions. An earlier `docs/deployment-vm-trial.md` version
also records the trial device's private Tailscale IP and device identifier.

The description of [PR #10](https://github.com/breki/silkboard/pull/10) still
contains the production endpoint. A Git rewrite would not edit that text.
The production tailnet suffix was absent from all 195 inspected workflow
log archives, including retained earlier attempts. Private-IP literals in owner-handoff tests are
fixtures and should not all be treated as real infrastructure disclosures.

Consequence: publishing the existing repository associates a tailnet and
service identity with the project and owner. Knowing the DNS name does not
by itself grant network access. The repository documents tailnet-only
Tailscale Serve, not public Funnel; live configuration was not inspected.

Tailscale's [HTTPS guidance](https://tailscale.com/docs/how-to/set-up-https-certificates)
explains that certificate issuance publishes machine and tailnet DNS names
in Certificate Transparency logs while access restrictions remain in place.
Given the documented certificate-validated HTTPS deployment, the hostname
may already be discoverable independently of GitHub. No query for the actual
private hostname was sent to an external search or certificate-log service.
Removing it from Git can reduce project/owner correlation but cannot promise
that the hostname becomes secret.

Recommendation: decide whether to accept this disclosure or sanitize the
publication source and GitHub text. Do not rotate unrelated credentials
solely because a DNS name was published. Live access verification is separate
from this repository audit.

### P127-2: operational and personal metadata

Severity: low privacy concern; owner acceptance needed.

Current documentation includes host names, VM UUIDs, resource budgets, service
accounts, filesystem paths and links to other development repositories.
Examples are [production evidence](../production-vm.md),
[the VM trial](../deployment-vm-trial.md),
[development provisioning](../bombyx-development.md) and
[the roadmap](../roadmap.md). The trial document includes a personal home
path. These are operational disclosures, not passwords.

Git history has four distinct author/committer name-and-email pairs. Two use
non-noreply addresses, including the reserved `.invalid` VM identity. The
owner should review the remaining personal attribution before publication.
Changing current Git configuration does not change previous metadata.

Public signing keys and fingerprints appear in release configuration,
documentation and workflow output. These are verification material; the scan
did not establish a private signing-key leak. Keep necessary public trust
material separate from any proposed privacy cleanup.

Recommendation: retain useful deployment instructions with generic examples;
remove or explicitly accept owner-specific evidence and attribution. Review
references to other private projects before exposing their names.

## Candidate disposition

Default Gitleaks scans found no matches in Git diffs, the historical blob
corpus or Git/GitHub text metadata. Workflow logs produced 31 public-signature
fingerprint matches; every match was checked against a successful Git
signature verification line, not classified by its rule name alone.

The supplemental URL rule found 16 historical occurrences, all synthetic
credential-URL fixtures in `xtask/tests/gates.rs`, publisher endpoint tests
and `.bombyx/tests/secrets.test.mjs`. The setup-fragment and opaque-credential
rules found no additional confirmed credentials. The manual hex review found
checksums and synthetic fixtures rather than confirmed live tokens.

No SQLite database signature or tracked private-key/database/archive filename
was found in the historical inventory. Fourteen PNG objects were reviewed
visually as a contact sheet: Silkboard design mockups and third-party product
reference images. No private credential was visible. This was not an OCR,
steganography or image-rights audit.

An initial workflow scan mistakenly processed zero bytes because extracted
files retained archive-like names. That result was discarded. Files were
renamed and the actual extracted log text scanned before drawing conclusions.
All raw material and scanner output stayed in private temporary storage;
secret values were not copied into this report.

## Publication and cleanup options

The owner has not authorized either option below or a visibility change.

1. Publish a sanitized snapshot in a separate public repository. Review its
   files and attribution, create fresh public history, scan the result and
   retain the existing private repository for historical release evidence.
   This avoids rewriting production's existing signed release lineage. Public
   development and the private updater would need an explicit future release
   arrangement; this report does not silently change that architecture.
2. Preserve development history by rewriting a disposable mirror with
   `git-filter-repo`, replacing the agreed DNS names/addresses and any other
   owner-selected private material across branches, tags and relevant messages.
   Scan the rewritten result and review the changed refs before any force push.
   Handle PR text and other GitHub surfaces separately. Coordinate existing
   clones to avoid reintroducing the original history.

A rewrite changes affected commit IDs and invalidates or removes signatures.
Silkboard's updater relies on signed tags and exact-commit checks, so tag
replacement and installed release-state compatibility require a separate
reviewed plan. Do not simply re-sign or force-push release tags during cleanup.
[GitHub's history-removal guidance](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository)
warns that old data can remain in clones, forks, PR refs and cached views.
GitHub Support does not guarantee removal of non-sensitive information such
as a hostname. A force push alone cannot establish complete erasure.

If a real credential is discovered later, revoke or rotate it before treating
history cleanup as remediation. No discovered credential currently requires
that response on the evidence recorded here.

## Limits and readiness

This is a bounded snapshot, not a guarantee of absence. It excludes unreachable
server objects, inaccessible/deleted GitHub content, external clones and
caches, the in-progress workflow run, later repository activity and live
infrastructure. No LFS/submodule configuration was found in the reachable
blob inventory. Ignored local publisher configuration, vaults, private keys
and application state were not opened merely to compare secrets against Git.
GitHub's native secret-scanning status could not be read. No credential was
tested against an external service.

The audit found no confirmed credential blocker, but making the existing
repository public would disclose historical network identifiers, operational
information and personal attribution. Owner acceptance or an agreed cleanup
is still needed. No history rewrite, credential rotation, PR-text edit,
production change or visibility change was performed.
