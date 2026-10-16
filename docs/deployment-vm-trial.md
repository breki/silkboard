# Deployment VM resource trial

Completed on malina, 2026-10-16. This validates a representative rustbase
guest, not an implemented Silkboard release pipeline.

## Configuration and results

The dedicated `silkboard-trial` guest uses QEMU/KVM with libvirt, Debian 13
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

## Retained state and next action

The trial guest is shut down with autostart disabled; its monitor is stopped.
Its disk and logs remain on malina. Existing host services and DNS were
healthy after shutdown. The inactive guest definition retains the original
2 GiB starting allocation; the test does not silently change production
resource policy.

Full provisioning details and measurements are in the sibling homelab repo,
`docs/runbooks/malina-silkboard-vm-trial.md`. Raw host/guest evidence is in
`/home/igor/silkboard-trial/` on malina. Installed virtualization packages and
the guest image directory `/var/lib/libvirt/images/silkboard-trial/` remain.

The next resource decision is whether to adopt the tested 512 MiB idle /
2 GiB build range. Automatic transitions would require an independently
authorized host controller, with no VM-control permissions granted to the
dashboard or its build account. The [roadmap](roadmap.md) retains the agreed
2 GiB starting budget until that decision is made.
