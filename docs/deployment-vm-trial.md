# Deployment VM resource trial

Completed on malina, 2026-10-16. This validates a representative rustbase
guest, not an implemented Silkboard release pipeline.

## Configuration and results

The dedicated `silkboard-trial` guest used QEMU/KVM with libvirt, Debian 13
ARM64, two shared vCPUs, a 2 GiB RAM ceiling, and a 40 GiB virtual disk with
4 GiB guest swap. It uses passt networking and private Tailscale HTTPS.
The workload was rustbase revision
`6528907cea4e1e2e369fe260ccd0432b29c557b3`, built unprivileged with unchanged
Cargo and npm lockfiles. Cargo used one build job; frontend stages followed
the backend sequentially.

| Check | Observed result |
| --- | --- |
| Cold release backend | Passed in 7m 05.51s; peak process RSS 482.3 MiB |
| Cold frontend dependency install | Passed in 26.81s |
| Cold frontend build | Passed in 4.62s |
| Warm backend, unchanged source | Passed in 0.41s; fully cached |
| Warm frontend build | Passed in 4.43s |
| 1 GiB idle | 600 seconds; all 21 SSH/HTTPS probe pairs passed |
| 512 MiB idle | 600 seconds; all 21 SSH/HTTPS probe pairs passed |
| Physical memory reclamation | QEMU RSS: 2,236 MiB build peak to 702 MiB idle median |
| Guest-only reboot at 512 MiB | SSH, app, and private HTTPS recovered in 40.78s |
| OOM and host impact | No OOM kills, host swap use, or monitored service failures |

The 1 GiB idle QEMU RSS median was 1,208 MiB. At 512 MiB, the guest had
164-194 MiB available during probes; guest swap usage peaked at 49.7 MiB
after resizing. Memory targets exclude QEMU's host overhead. No cache-drop
commands were used. Both reductions were explicit host-side balloon requests.

The guest's minimal app is not a benchmark for future dashboard features or
data-heavy applications. No multi-VM load test, automatic memory controller,
production security-hardening validation, or host-reboot test was performed.
Guest Tailscale reported blocked UDP and used relaying; private HTTPS worked.

## Cleanup completed

On 2026-10-17, [issue #2](https://github.com/breki/silkboard/issues/2) removed
the completed trial. Before deletion, the raw evidence confirmed 154 host
samples with all seven monitored services active and all four HTTP checks
returning 200, plus 21 probe pairs in each idle interval. Useful results
remain in this report and the sibling homelab runbook,
`docs/runbooks/malina-silkboard-vm-trial.md`; raw trial evidence is no longer
retained on malina.

Removed the shut-off domain `0a513704-ae78-4308-8457-2461eef85100`, its
autostarted directory storage pool, qcow2 base/overlay, seed images, NVRAM,
QEMU log, and `/home/igor/silkboard-trial/` workspace, including monitoring
scripts, source archives and raw logs. No snapshots, active trial monitor,
or trial systemd/cron configuration remained. Other VM disk backing chains
were checked before deleting storage; production uses its own base image.

The owner removed the offline Tailscale device `silkboard-trial`
from the tailnet. A subsequent host tailnet
readback confirmed its absence and preserved malina and production Silkboard.
Deleting the guest disk also removed its local Serve configuration. No
separate trial configuration was found on the host; malina's existing Serve
routes were unchanged.

Final checks at 15:19 UTC found no trial-named artifacts under the inspected
libvirt configuration/storage/runtime/log/cache, systemd/cron, host temporary
or home paths. Only production `silkboard` remained in the domain and pool
lists. All seven existing host services, four HTTP health checks and DNS
passed. Production private HTTPS reported healthy `v0.2.0`, commit
`6a24145a6f5404e210f7597001d02e166b023889`; both health endpoints passed.

Shared virtualization/provisioning packages and firmware remain for the
production VM and future provisioning. Shared host journals retain historical
service events; they were not purged to erase trial references. No host or
production reboot, package removal, or resource-policy change was performed.

After the trial, the owner chose to retain a fixed 2 GiB allocation and
rejected host-directed resizing because of setup and maintenance overhead.
The tested 512 MiB idle / 2 GiB build range is not adopted. Automatic memory
sharing is a [deferred investigation](https://github.com/breki/silkboard/issues/1);
the trial did not verify that behavior. The [roadmap](roadmap.md) records
the current resource policy.
