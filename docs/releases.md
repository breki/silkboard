# Release preparation and evidence

Silkboard releases use trusted signed stable tags plus a passing full GitHub
release suite for the exact tagged commit. The production updater is still
pending; these checks alone do not deploy an application.

## Identity and trust

The owner selected the SSH signing key with ED25519 fingerprint
`SHA256:D+XaRkaUyW9uBQ8dMDp+XmZV6QH255p9g58auXRQwf8` on 2026-10-17.
[The allowed signers file](../.github/release-signers) contains only its
public key. CI and the dashboard receive no private signing material.
`git verify-tag` with this file verifies an SSH-signed annotated tag;
lightweight, unsigned, wrong-key, malformed-version, wrong-commit, and
renamed signed tags are rejected by `cargo xtask release-check TAG`.

The agreed workflow is `.github/workflows/release.yml` in `breki/silkboard`,
triggered by a release-tag push. It verifies admission, calls the complete
CI suite from the same revision, then builds the release with the exact
checkout commit and tag embedded in the backend. A failed or skipped required
job prevents a successful release run. CI's normal PR run is useful feedback;
release eligibility requires the tag-triggered release workflow.

The future updater must use an independently installed public trust file,
not a trust file fetched from an unverified candidate. GitHub's workflow API
registered the active release workflow as ID `387356305`, read back on
2026-10-17 after PR #8 merged. Bind evidence to repository, registered workflow
ID and path, push event, release tag, exact peeled commit, and completed success.
Do not accept matching display names or checks on a different revision.

## Verified initial release

On 2026-10-17, signed tag `v0.1.1` at commit
`c68a53902d8e9552abee2d3488645800f856986e` passed
[release run 39003125429](https://github.com/breki/silkboard/actions/runs/39003125429).
The API read-back matched workflow ID `387356305`, the agreed path, push
event, tag, and exact commit, with completed success. Admission, the full
validation suite including ARM64 linting, and release builds all passed.
The remote annotated tag object is
`407232ca5daf02a96865a7abf202be0905210d3c`; its trusted SSH signature was
also verified locally. This establishes release eligibility, not deployment.

The earlier `v0.1.0` tag remains unchanged and ineligible: its
[release run failed admission](https://github.com/breki/silkboard/actions/runs/39001755923)
because checkout replaced the runner's signed tag ref with the peeled commit.
PR #9 fixed checkout selection before `v0.1.1` was published.

## Local preparation

Use the separate [release workflow](ai-agents/workflows/release.md) only when
a release is requested. It requires the owner's version choice, a clean
worktree, and a nonempty Unreleased changelog; it never publishes implicitly.
No release has been requested just by implementing these gates.

1. Inspect `crates/silkboard-web/Cargo.toml` as the application version source.
   `xtask` has an independent tooling version; the frontend displays backend
   identity and has no separately shipped application version constant.
2. Update the application manifest, its exact `silkboard-web` entry in
   `Cargo.lock`, and promote Unreleased changes to a dated changelog section.
   No version/changelog helper exists yet. Preserve other lockfile entries;
   broad dependency updates are not release bookkeeping.
3. Run `cargo xtask validate --check`, then the supported ARM64 Clippy pass
   (`rustup target add aarch64-unknown-linux-gnu` first) and the standalone
   `cargo xtask audit`. Required failures or unavailable prerequisites block
   the release commit and tag. Inspect the final bookkeeping diff.
4. Commit only bookkeeping with `release: vX.Y.Z`, preserving signing, and
   create the signed annotated tag with `git tag -s vX.Y.Z -m 'vX.Y.Z'`.
   Confirm `cargo xtask release-check vX.Y.Z` succeeds on the clean checkout.
5. Report the local commit/tag. Push them only when separately authorized,
   accounting for the future updater's ability to activate eligible releases.
   Pushing a tag starts the release suite; publication is not proof it passed.

The release-check command verifies stable version syntax, a clean tree, tag
direct commit target matching HEAD, the signed tag name matching the
requested release, the trusted SSH signature, and application version.
It does not call GitHub or certify a release suite. No command creates a
GitHub Release object or controls the Pi as an implicit step.

## CI and local validation

[CI](../.github/workflows/ci.yml) uses Linux because the existing hook tests
require Unix. Main, code/mixed/uncertain PR changes and release calls install
pinned Rust/Node and gate tools, run the complete read-only validator, and
cross-lint ARM64. A conservative documentation-only PR path reports canon/diff
checks instead; it is not release evidence. See
[PR scope rules](ai-agents/tooling.md#documentation-only-pr-checks).
CI does not claim native ARM64
runtime or VM deployment coverage. Pinned gate executables use a dedicated,
version-checked cache; npm caches downloads and still runs `npm ci`. See
[CI cache boundaries](ai-agents/tooling.md#ci-cache-boundaries). Tokens have
read-only contents permission and checkout does not persist credentials.

[The release suite](../.github/workflows/release.yml) reuses these exact gates
and builds a tagged release. It uploads no build artifacts: the production
model remains unprivileged in-guest builds. See
[tooling prerequisites](ai-agents/tooling.md) and
[development instructions](development.md). Test fixture tags and keys are
created only in disposable repositories; tests never use the owner's private
key or create Silkboard release tags.
