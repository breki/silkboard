# Silkboard Roadmap

Status: Milestone 1 defined; implementation pending. Later milestones are open.

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
- Verify eligibility before building or activating a revision. The precise
  policy for trusted signatures and passing release checks remains to be set.
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
   can become eligible after those checks finish, if checks are required.
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
setup. Silkboard's VM provisioning and networking still need to be established.

### Decisions still needed

- What constitutes a verified release: trusted commit/tag signatures, passing
  GitHub release checks, or both.
- VM platform, guest OS, resource allocation, and private access address.
- Build inside the VM or download an ARM64 artifact produced by CI.
- Release ordering, polling interval, and rollback pinning/retry policy.

## Later milestones

To be defined with the owner after the deployment milestone. Project overview,
agent handoffs, and task coordination remain product ideas, not an agreed
delivery sequence.
