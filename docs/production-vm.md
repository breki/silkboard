# Production VM evidence

Issue [#5](https://github.com/breki/silkboard/issues/5), verified 2026-10-17.
The [bootstrap procedure](../deploy/vm/README.md) and scripts reproduce the
fresh-guest setup. Automatic updates and upgrade recovery remain issue #6.

## Verified configuration

Created a separate `silkboard` guest on malina using the pinned Debian 13
ARM64 image and existing libvirt/QEMU/KVM installation. The trial guest
remains stopped and unchanged. No host bridge, forwarding, DNS, or existing
application-service configuration was changed.

| Setting | Observed result |
| --- | --- |
| Domain UUID | `c58a0183-ee30-4364-a1f5-5521d782dbef` |
| CPU | Two shared host-passthrough vCPUs |
| RAM | Fixed 2 GiB; balloon model `none` |
| Disk | Independent 40 GiB qcow2 disk/backing image |
| Swap | 4 GiB guest swapfile |
| OS | Debian 13 ARM64 |
| Network | virtio with passt; no host port forwarding |
| Administration | Operator SSH public key; operator-only guest agent |
| Libvirt autostart | Enabled |
| Toolchains | Rust 1.93.1, Node 24.14.1 |
| Users | Separate `silkadmin`, `silkbuild`, `silkboard` identities |

Host creation passed read-only preflight. An invalid image checksum was
rejected before mutation. Repeating creation after success refused to
replace the existing domain/disk, preserving the original UUID.

The guest-agent transport has tests for argument serialization, guest exit
codes, remote errors, and upload-path traversal. Live file uploads matched
SHA-256. Initial-activation tests cover startup failure, failed health,
failed service cleanup, and success. These checks do not demonstrate an
upgrade, rollback, or automatic retry.

## Release and live verification

Initial candidate: signed `v0.1.1`, commit
`c68a53902d8e9552abee2d3488645800f856986e`. The
[full release suite](https://github.com/breki/silkboard/actions/runs/39003125429)
passed for that exact tag/commit and registered workflow ID `387356305`.
The owner-approved public signer is independently installed in the guest.
A source-only Git bundle transferred without GitHub credentials. Guest
signature verification and release-check passed before application compilation.

The unprivileged in-guest build and first activation passed. The running
status API reports health `ok`, version `0.1.1`, tag `v0.1.1`, and the exact
commit above. The sandboxed service is enabled and serves loopback port 3000.
The runtime account cannot read build sources or SSH host private keys,
write the installed binary, or access a libvirt control socket. A repeated
activation refused to replace the healthy current release.

Private OpenSSH access as `silkadmin` passed with the guest's ED25519 host
key obtained through the trusted guest-agent channel and strict host-key
checking. The owner authenticated Tailscale; its state is `Running` at
an operator-selected private DNS name, retained outside this repository.
Tailscale Serve proxies only tailnet HTTPS to
loopback. Certificate-validated `/health` and `/api/status` passed. Chromium
rendered the mobile page with exact identity, no overflow, and no JavaScript
errors. Initial certificate issuance caused one HTTPS timeout; subsequent
requests passed without disabling certificate checks. The guest reports
blocked UDP and uses Tailscale relaying; direct connectivity is not claimed.

A final guest-only reboot changed boot ID from
`9f5ca02c-c0ae-4ef0-8a11-e23241c5e922` to
`d05fcd93-84cb-49eb-8a34-8b1fd6629623`. Strict SSH, certificate-validated
HTTPS, app/tailscaled active state, and exact release identity all recovered
within 36.8 seconds. App and tailscaled enabled state, Serve configuration,
and 4 GiB swap persisted. Libvirt autostart is enabled; a host reboot was
not tested.

| Measurement | Observed result |
| --- | --- |
| Initial source build, frontend and backend | 5m 41.97s |
| Peak build process RSS | 482,912 KiB (about 472 MiB) |
| Guest idle after reboot | 203 MiB used; 1,761 MiB available |
| Guest swap after reboot | 4 GiB configured; zero used |
| Guest OOM kills | Zero during build and after reboot |
| Guest root filesystem after build | 6.9 GiB used; 31 GiB available |
| Installed binary and assets | 2.5 MiB |
| Retained build/toolchain state | 957 MiB |
| Sampled QEMU peak RSS | 2,299,568 KiB, including host overhead |

During bootstrap/build observations, all seven existing host service units
were active and all four existing application health endpoints returned 200.
Across 47 complete samples from 08:05:48 to 08:17:25 UTC, host available RAM
stayed at or above 4,864,944 KiB (about 4.6 GiB). The slowest sampled existing
application health response was 4.37 ms. The host used about 2 MiB swap and
had 77 GiB disk available after provisioning. Process RSS is not total guest
or host memory; samples do not establish worst-case future dashboard load.

## Evidence and remaining work

Durable guest logs are `/var/log/silkboard-bootstrap.log` and
`/var/log/silkboard-initial-build.log`. Operator logs are local to this
session; no credentials or temporary Tailscale login links belong here.

Issue #6 must add the separate release downloader/updater, polling, eligibility
verification, staged upgrades, rollback, failed-tag handling, and automatic
upgrade/recovery demonstrations. No GitHub credentials are installed yet.
The host's existing libvirt security model does not establish production
host confinement; the app/build accounts receive no host-control channel.
