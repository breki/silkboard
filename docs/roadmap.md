# Silkboard Roadmap

Status: Milestone 1 in progress; the minimal application is implemented.
Release gates and initial production deployment are verified; automatic
upgrades and recovery remain pending.
Later milestones are open.

This roadmap replaces the original milestone list in the architecture proposal.
Milestones follow the owner's priorities; later product scope is not yet set.

## 1. Complete deployment pipeline with a minimal status page

Establish the delivery pipeline before building dashboard features. Silkboard
runs inside a dedicated VM on the owner's Raspberry Pi and automatically
upgrades when a new verified commit with a release tag appears on GitHub.
The initial web app can be just a status page.

### Scope

- Base the project on [breki/rustbase](https://github.com/breki/rustbase),
  adapting its Cargo workspace, Rust/Axum backend, Svelte/Vite frontend,
  xtask tooling, and CI/release quality gates.
- Provide repeatable setup for the ARM64 VM and application service, with
  private access to the status page.
- Detect eligible GitHub release tags automatically and deploy the exact
  referenced revision without a development machine driving each upgrade.
- Verify eligibility before building or activating a revision: require a
  trusted signed release tag and a passing full GitHub release suite for the
  exact referenced revision.
- Stage a release, activate it, and check service health. Keep a working
  release available for rollback if an upgrade fails.
- Show the running version, release tag, commit, and application health on the
  status page. Provide deployment logs and documented recovery steps.
- Keep deployment credentials and responsibilities separate from the dashboard
  application's future access to project data.

### Completion criteria

1. A documented setup brings up the status page inside the Pi VM.
2. Publishing an eligible tagged revision causes an automatic upgrade, and
   the status page identifies the deployed revision.
3. An untagged or unverified revision is not deployed. A tag waiting for checks
   can become eligible after its required release checks finish successfully.
4. A failed build or upgrade preserves or restores the working release;
   rollback and retry behavior are demonstrated.
5. The application and automatic update mechanism recover after a VM reboot.

### Existing patterns checked

Repository documentation and source checked on 2026-10-15. This research did
not inspect live services on the Pi.

- [MuTrack deployment](https://github.com/breki/mutrack/blob/main/deploy/malina/README.md):
  documents an end-to-end deployment on malina verified on 2026-09-02. A
  systemd timer polls every ten minutes for the newest version tag, builds
  locally, switches a release symlink, and restarts the service. HTTPS is
  provided through Tailscale. This is the closest existing release-triggered
  upgrade pattern; its root build model is not a requirement for Silkboard.
- [Phren deployment](https://github.com/breki/phren/blob/main/deploy/malina/README.md):
  polls main, verifies commit signatures before checkout/build, builds as an
  unprivileged user, and publishes through an atomic symlink swap. This supplies
  a verification pattern, but deploys commits rather than release tags.
- [Hoard release system](https://github.com/breki/hoard/blob/main/.claude/commands/release.md):
  the existing release command bumps the workspace version, updates the lockfile
  and changelog, runs validation, cross-target Clippy, and a standalone security
  audit, then creates a release commit and annotated version tag. Its deploy
  guard requires a clean tree at a tag matching the workspace version. This is
  useful prior art for preparing and admitting Silkboard releases. The existing
  deploy is driven from the development machine over SSH; the guard does not
  itself verify signatures or prove that release checks ran.
- [Hoard pull-deployment plan](https://github.com/breki/hoard/blob/main/docs/issues/malina-pull-deploy.md):
  proposes signed-tag verification, separate build/activation responsibilities,
  health checks, and rollback. Its current document is marked planning; it is
  not evidence of an implemented updater.
- [Rustbase](https://github.com/breki/rustbase): provides the application and
  tooling foundation. Its release workflow runs CI plus coverage, duplication,
  and security audit gates on version tags. Its current deployment uses
  cargo xtask deploy; automatic pull upgrades need to be added for Silkboard.

These deployments document host services on malina, not a reusable VM deployment
setup. The subsequent [VM resource trial](deployment-vm-trial.md) verified a
representative guest. [Production provisioning](production-vm.md) now verifies
the dedicated guest, initial release, private access, and guest reboot recovery.

### Agreed deployment decisions

Selected with the owner on 2026-10-16. Initial provisioning is verified;
the automatic updater remains pending.

- Target malina with a dedicated QEMU/KVM guest managed by libvirt, running
  minimal Debian 13 ARM64. A live KVM API and VM-creation check succeeded.
- Build the Rust backend and frontend inside the guest as an unprivileged
  user. Downloaded CI build artifacts are not the chosen deployment model.
- Resource budget: two shared vCPUs, fixed 2 GiB RAM, and a 40 GiB disk
  including 4 GiB guest swap. A separate trial tested lower idle memory
  and host-controlled ballooning; see the completed
  [trial results](deployment-vm-trial.md). The owner rejected host-directed
  resizing because of setup and maintenance overhead. Automatic memory
  sharing is deferred for [future investigation](https://github.com/breki/silkboard/issues/1);
  it is not a milestone 1 prerequisite or an adopted configuration.
- Use guest Tailscale Serve for private HTTPS with the hostname `silkboard`,
  subject to name availability.
- Require a trusted signed release tag and a passing full release suite for
  the tagged revision, including the agreed CI and release quality gates.
- Trust the owner's existing Git SSH signing key for release tags: ED25519
  fingerprint `SHA256:D+XaRkaUyW9uBQ8dMDp+XmZV6QH255p9g58auXRQwf8`.
  Selected on 2026-10-17; private signing material stays outside CI and the
  dashboard.
- Require the release-suite workflow at
  `breki/silkboard/.github/workflows/release.yml`, triggered by the release
  tag, to pass all required gates for that tag's exact commit. Selected on
  2026-10-17; GitHub registered workflow ID `387356305`, read back after
  PR #8 merged. Signed `v0.1.1` passed its exact tag-triggered suite; see
  [release evidence](releases.md#verified-initial-release).
- Poll every ten minutes, including reconsidering tags awaiting checks.
  Select the highest newer stable `vMAJOR.MINOR.PATCH` version. Exclude
  prereleases; downgrades require explicit operator action.
- Preserve or restore the working release after failure. Block the failed
  tag until manual retry, while continuing to consider newer eligible tags.
- Use a fine-grained GitHub token restricted to `breki/silkboard`, with
  Contents/read and Actions/read, for the separate downloader. Selected on
  2026-10-17; credential installation and live updater verification are pending.

### Decisions and evidence still needed

- Install the agreed public-key trust configuration and workflow identity
  in the production updater. Registration evidence is in
  [release preparation](releases.md).
- Automatic release polling, eligibility checks, upgrade activation, and
  demonstrated rollback and retry behavior. The initial production deployment
  is operational; the [updater implementation](automatic-upgrades.md) is under
  development and the automatic delivery pipeline is not verified yet.

## Later milestones

To be defined with the owner after the deployment milestone. Project overview,
agent handoffs, and task coordination remain product ideas, not an agreed
delivery sequence.
