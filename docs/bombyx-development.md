# Silkboard development in a Bombyx VM

This is native Ubuntu 24.04 guest provisioning for
[#13](https://github.com/breki/silkboard/issues/13), without containers. Bombyx
clones `main` into `/home/agent/silkboard` and runs `.bombyx/provision.sh` as
`agent`, with passwordless guest sudo. These files must be merged to `main`
before the operator creates the VM. They do not create a VM, open network
access, install production trust or authenticate either AI service.

## Provision and enter

Use the operator's existing external Bombyx project configuration and KeePassXC
vault. Keep secrets and VM sizing outside this repository. The source repository
is `git@github.com:breki/silkboard.git`, project `silkboard`, ref `main`, and
provisioning script `.bombyx/provision.sh`. Set the project's `secrets_refreshed`
hook to `.bombyx/refresh-secrets.sh`. The vault supplies the deploy-key attachment
and `SILKBOARD_PUBLISH_TOKEN`; do not copy vault contents into this checkout.

The operator reviews CPU/RAM/disk choices, checks `bombyx list --offline`, then
runs `bombyx up silkboard`. Use `bombyx shell silkboard` to enter. To rerun the
bootstrap on an existing VM use `bombyx provision silkboard`; another `up` does
not rerun provisioning. Follow the installed Bombyx help for its configuration
path; this repository does not invent a second registry.

Provisioning installs system build/browser/cross-compiler prerequisites and:

- Rust 1.93.1, rustfmt, Clippy, llvm-tools-preview and ARM64 target support.
- Node 24.14.1 and gate tools pinned by the existing CI/tooling guide.
- Codex 0.162.1, retaining the version used to verify native hook reporting.
- Kozmotic 2.3.0 at `~/.claude/bin/kozmotic`, required by the repository's
  Git-status hook. Fixed release archive and binary checksums cover x86_64 and
  ARM64; reruns retain the exact installed binary and refuse conflicting files.
- Claude Code 2.1.293, published 2026-10-07 and older than fourteen days when
  selected on 2026-10-22. Installing Claude does not enable Claude reporting;
  that integration remains #43.
- GitHub CLI (`gh`) from GitHub's signed package repository, avoiding Ubuntu's
  older community package with deprecated API behavior.
- Tailscale from its signed stable Ubuntu 24.04 repository, with `tailscaled`
  enabled. Provisioning does not log in, advertise routes, enable Tailscale SSH
  or change the VM host's firewall. Repeated provisioning retains its guest
  device identity and authentication state.
- Locked frontend dependencies/Chromium and a release-built standalone publisher.

Node/rustup archives use fixed official SHA256 values for x86_64 and ARM64.
Agent npm packages use a user-owned prefix, never `sudo npm install`. Repeating
provisioning retains compatible installed tools, configuration and counters;
it refuses conflicting publisher configuration and unrelated command files.
No personal agent configuration, authentication cache or hook trust is copied.
Run provisioning while agent sessions are idle, since rebuilding/installing
programs can replace the executables they use.

## Commit author and signing

The owner selected `Silkboard VM <silkboard-vm@agents.invalid>`. The `.invalid`
address has no mailbox and does not attribute commits to a GitHub account.
Provisioning sets this identity and SSH commit/tag signing only in the clone's
local Git configuration. It preserves Bombyx's `core.sshCommand` and signs with
`~/.ssh/bombyx-deploy-key`. It never transfers the owner's personal key.

The separate VM key has both repository write and release-signing authority
once independently installed production trust permits it. Guest code can read
that key; the VM's passwordless sudo does not isolate it from its agents. See
[release trust and revocation](releases.md#vm-signer-and-revocation) before
installing production trust. A GitHub Verified badge is not the trust decision.

## Private publisher and refresh

The project and commit author are in `.bombyx/settings.json`. Supply the real
endpoint through your external Bombyx registry; there is no endpoint default:

```toml
[projects.silkboard.env]
SILKBOARD_PUBLISH_ENDPOINT = "https://silkboard.example.test"
```

Replace this reserved example origin with your own lowercase HTTPS origin,
without a trailing slash, path, query, credentials or fragment. Missing or
invalid input fails configuration; it never selects another deployment. Keep
your real address outside Git. Existing differing publisher configuration is
refused rather than silently redirected. Provisioning creates:

- `~/.local/bin/silkboard-publish` and user-visible tool links.
- `~/.config/silkboard-publish/config.json`, mode 0600, with the actual checkout,
  guest hostname and optional Codex reporting configuration.
- `~/.config/silkboard-publish/publisher.token`, mode 0600.
- `~/.local/state/silkboard-publish/`, mode 0700; counters are retained on reruns.

Configuration and token directories are private. The refresh hook uses the
installed Node runtime by absolute path, so Bombyx's bare environment works.
It parses `$BOMBYX_ENV_FILE` as data, never sources shell code, accepts the
64-lowercase-hex publisher token and atomically replaces the destination.
It supports CRLF, `export`, quoting, comments and last-assignment-wins for the
one selected variable. As specified by Bombyx, a quoted value ends at its
closing quote; trailing bytes are ignored, not executed. An unterminated quote
is refused. Secret input is capped at 64 KiB and must be a regular,
private owner-controlled file. Unsafe paths, symlinks and hardlinks are refused.
Missing, malformed or removed input returns failure and removes any safely
identified old destination token, preventing stale publishing after removal.
An unsafe existing destination is left untouched for operator inspection.
No token, raw input or exception details are printed, including on failure.

Bombyx rewrites its private `~/.bombyx-env` copy and runs the configured refresh
hook on provisioning and `bombyx shell silkboard --refresh-secrets`. The hook
is safe to run twice and needs no login shell or input. Bombyx does not run the
hook for scratch VMs. Rotation does not reset publisher counters or change the
server's credential policy. Revoke retired credentials using the owner UI.

## Authentication and live verification

Provisioning installs programs, not authenticated accounts. Inside the guest:

1. Confirm `rustc --version`, `node --version`, `codex --version`,
   `claude --version`, `gh --version`, `tailscale version` and
   `silkboard-publish --version`.
2. Use `codex login --device-auth` and follow its private browser instructions.
   Start Claude and complete its own `/login` flow. Do not paste credentials,
   one-time codes or login links into repository records or chat.
3. Review the repository's Codex handlers in native `/hooks` before enabling
   reporting. Preserve validation/Git-status hooks and normal agent permission
   settings; provisioning does not enable bypass flags or edit trust records.
   Provisioning installs the informational Git-status handler's Kozmotic
   prerequisite; it does not copy the workstation's binary. Follow the
   [Git-status setup guide](ai-agents/git-status-hook.md#enable) to verify the
   next actual turn. This informational hook does not replace release gates.
4. Register this guest as its own private tailnet device: run
   `sudo tailscale up --hostname=silkboard-dev` in the guest and approve the
   login on your phone. Confirm `tailscale status` reports it connected before
   checking the endpoint. No auth key, login URL or tailnet address belongs in
   the repository. Tailscale account approval remains an operator action.
   Check `curl --fail "$SILKBOARD_PUBLISH_ENDPOINT/api/status"` during
   provisioning, or use the endpoint from your private publisher config after
   entering the guest (Bombyx provisioner environment variables do not persist
   in interactive shells). The private
   endpoint needs an approved guest route/DNS/Tailscale arrangement. The VM host's
   guest-to-host filtering must not be assumed to provide access. Choose/fix
   connectivity explicitly rather than weakening the publisher's HTTPS policy.
5. Run `cargo xtask validate --check` and ARM64 Clippy from the guest. Complete
   actual authenticated work in each AI tool and verify ordinary Codex reports
   appear with the guest hostname/native identity in the owner's phone feed.

GitHub CLI installation does not provide GitHub API access. The existing
repository deploy key remains the Git SSH credential; it is not a `gh` token.
Use a separately approved credential for private GitHub API operations. No
workstation `gh` authentication/configuration is copied by provisioning.

The scripts and isolated tests do not establish a live Ubuntu VM, endpoint
reachability, authenticated agent use or a working release signed by the actual
VM key. Those operator-dependent observations remain pending. Production updater
credentials and dashboard execution authority are unchanged.

Sources: [Bombyx tutorial](https://github.com/breki/bombyx/blob/main/docs/tutorial.md),
[Bombyx configuration sample](https://github.com/breki/bombyx/blob/main/config.toml.sample),
[Codex authentication](https://learn.chatgpt.com/docs/auth),
[Claude installation](https://code.claude.com/docs/en/setup),
[Tailscale Ubuntu packages](https://pkgs.tailscale.com/stable/#ubuntu-2404),
[GitHub CLI installation](https://github.com/cli/cli/blob/trunk/docs/install_linux.md).
