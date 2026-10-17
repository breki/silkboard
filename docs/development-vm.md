# Development in a Bombyx VM

Issue [#13](https://github.com/breki/silkboard/issues/13) provides native
Silkboard development provisioning with Codex and Claude Code. Bombyx creates
a development VM, clones the project inside it, and invokes this repository's
provisioning script as the agent user. Development happens directly in that
checkout. No Docker layer is installed or required.

The development VM is separate from the production dashboard and updater.
This procedure does not import production credentials, signing keys, or
control of other VMs. It does not migrate the current checkout automatically.

## Configure and provision

Install [Bombyx](https://github.com/breki/bombyx) and prepare a libvirt host
using its documentation. Merge the
[example registry](../.bombyx/config.toml.example) into your private Bombyx
configuration outside the repository. Choose your host, available base box,
and development resources. The example sizes are suggestions, independent of
the production VM budget. The guest must be Debian/Ubuntu with Git, Bash,
and the passwordless sudo Bombyx gives its agent user.

The repository is private. Configure a separate repository-limited clone
credential through Bombyx's documented `deploy_key` or `repo_token` settings.
Keep its value outside the registry and checkout. Bombyx documents the
additional curl/jq prerequisites for SSH cloning on the selected base box.
The example uses HTTPS and points at this implementation branch; switch its
ref to `main` after merge. Then run:

```sh
bombyx doctor silkboard-dev
bombyx up silkboard-dev
bombyx shell silkboard-dev
```

Bombyx invokes `.bombyx/provision.sh` from the guest clone as the agent user.
It installs compiler/system prerequisites, Git/GitHub tools and bubblewrap
with sudo. Everything else installs under the agent's home:

- Rust 1.93.1 with rustfmt, Clippy, LLVM tools and the ARM64 lint target.
- Node 24.14.1, verified against a pinned architecture-specific checksum.
- The repository's pinned Cargo validation tools.
- Codex 0.160.0 and Claude Code 2.1.288 from a separate npm lockfile.
- Locked frontend dependencies, Chromium and its required system libraries.

Tool installation, dependency fetches and both CLI version checks must
succeed; any failed step stops provisioning. It does not report full
validation or agent login as passed. Long first installs compile the Cargo
gates from source; keep the provisioning output when diagnosing a failure.

Re-running `bombyx provision silkboard-dev` installs missing tools, checks the
pinned Rust toolchain, repeats locked dependency installation, and preserves
existing agent settings/login. An ordinary `up` does not reprovision an
existing VM. Do not reprovision while an agent session is using the installed
binaries: npm replaces its tool directory during installation.

The scripts support Linux x86_64 and ARM64. The verification section records
which platforms and execution paths have actually been exercised.

## Start development and log in

Open a new `bombyx shell silkboard-dev` after provisioning. The generated
`~/.config/silkboard/dev-env.sh` is sourced from the agent's `.profile` and
`.bashrc`, putting the pinned tools on PATH. For an already open shell:

```sh
source "$HOME/.config/silkboard/dev-env.sh"
codex login --device-auth
claude
```

Codex device login requires the account/workspace to allow it. Open the link
and enter the one-time code in your own browser. Claude's first run provides
a browser login; for a remote terminal, open its link on your own device and
return the login code to that terminal if prompted. These owner steps are
separate from unattended provisioning. Never put credentials or login codes
into chat, GitHub issues, source files, or provisioning arguments. Existing
workstation credentials are not copied into the VM.

Launch `codex` or `claude` from the checkout. Both discover the tracked agent
guidance and workflow wrappers. The provisioner does not change approval
settings, disable agent sandboxes, or make the project automatically trusted.
Bubblewrap is installed for Codex's Linux sandbox; verify compatibility on
the selected guest with `codex sandbox linux -- /usr/bin/true`. If the guest
blocks sandbox creation, diagnose its kernel/AppArmor policy before changing
permissions; do not silently bypass the sandbox.

Check login with `codex login status` and Claude's `/status`. Ask each agent
to identify the repository and summarize `README.md` to verify authenticated
use. CLI version output alone does not demonstrate a successful model request.

The agent account has the sudo access provided by Bombyx. Code in this VM can
read the account's files and credentials and can reach the network; the VM
and Bombyx host firewall are the boundary from the host. Agent approvals and
sandboxes remain additional controls, not a replacement for that boundary.
Keep repository and agent credentials limited to this development use.

## Validate and switch development

From the guest checkout:

```sh
cargo xtask validate --check
cargo xtask clippy --target aarch64-unknown-linux-gnu
```

See [local development](development.md) for running the backend/frontend and
other checks. Servers default to guest loopback. Browser access from your
workstation needs deliberate SSH forwarding or another private access setup;
this provisioner creates no public listener or production HTTPS endpoint.

Bombyx's clone configuration already provides the Git access you selected in
its private registry. Verify fetch/push permissions before switching work.
If you need GitHub CLI functionality, authenticate inside the VM with
`gh auth login --web`; do not overwrite Bombyx's Git configuration blindly.
Configure your Git author identity and signing policy deliberately. The
owner's signing private key is not provisioned by these scripts.

To switch, commit/push current work, provision the selected revision, run
validation and both agent login checks, then resume from the guest checkout.
Keep the original checkout until the replacement is verified. Agent settings,
credentials, toolchains, build caches, and the checkout persist on the VM
disk; the scripts do not create host shares or mount personal directories.

Use Bombyx's snapshot/reset workflow deliberately. Snapshots include guest
disk data and therefore any stored development credentials. Resetting can
lose changes and restore old credentials; revoke/logout credentials before
sharing or exporting a snapshot. Destroying the VM also destroys its checkout
unless separately backed up. No VM deletion is part of this provisioning.

## Tool pins and verification status

The agent npm lockfile is separate from application dependencies. Its package
publication dates, canonical URLs and integrity were checked before adoption;
all entries met the fourteen-day cooldown. Codex is Apache-2.0; Claude Code
retains its upstream proprietary license and is installed as a developer tool.
Automatic Claude updates are disabled so the environment uses the pinned
version; update the lockfile and reprovision deliberately.

Native provisioning behavior is tested against fake package/network command
boundaries, including failed package/tool installation and corrupt bootstrap
downloads. This does not establish a working Bombyx VM. A full fresh-guest
provisioning run, repeat provisioning, ARM64 installation, sandbox execution,
and authenticated model requests remain unverified. Production is unchanged.

## Installation references

Checked on 2026-10-17:

- [Bombyx provisioning contract](https://github.com/breki/bombyx/blob/38fe77286f55a0bcc6e12bd5d4500e72ae4ebb06/docs/tutorial.md).
- [Codex CLI installation](https://learn.chatgpt.com/docs/codex/cli),
  [headless authentication](https://learn.chatgpt.com/docs/auth), and
  [Linux sandbox setup](https://learn.chatgpt.com/docs/sandboxing).
- [Claude Code npm installation](https://code.claude.com/docs/en/setup#install-with-npm)
  and [authentication](https://code.claude.com/docs/en/authentication).
