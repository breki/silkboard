# Bombyx development VM handoff (#13)

Goal: prepare native Ubuntu 24.04 agent-user provisioning, private publisher
secret refresh and the separate VM release signer, following the operator's
2026-10-22 handoff. No container layer or migration of the personal owner key.

Decisions: VM commits use Silkboard VM <silkboard-vm@agents.invalid>. Reuse
project silkboard and the endpoint in external operator configuration.
The operator supplied
and registered the separate write-enabled deploy key (fingerprint
SHA256:s/bOUAzymE3qEqSllzx4fLR+TNtaWMhpvLR1FrbSc9I). Keep the owner signer.
The operator handles production trust installation, VM sizing/config and VM
creation; this change does not perform those operations or read the vault.

Implementation plan:
1. Test two independently trusted fixture signers, unlisted signer refusal and
   exact bootstrap trust-set validation; update trust documentation/revocation.
2. Provide rerunnable agent-user provisioning for pinned Rust/Node/gates,
   native Codex/Claude tools, local Git signing and private publisher config.
3. Parse the Bombyx env file as data, never source it. Atomically refresh the
   private token in the bare hook environment, without revealing secrets.
4. Test privacy, malformed/untrusted paths, reruns and missing/rotated secrets;
   wire the bootstrap tests into validation and document operator steps.
5. Run appropriate full checks and sequential independent reviews; prepare a
   signed commit and PR separately. Do not merge unreviewed changes.

The owner has now created the Ubuntu 24.04 guest on Fusion using 2 CPUs,
4 GiB RAM and a 64 GiB sparse disk. Tool startup, local Git signing settings
and private publisher file permissions were checked at merged revision
940461a. Codex reports an authenticated login; Claude login is still pending.
This is not evidence of authenticated work in both tools. The initial private
endpoint DNS check failed because the guest had no Tailscale connection.
Production trust installation remains an operator step.

Validation discovered new publisher TLS advisories RUSTSEC-2026-0381/-0382.
The minimum two transitive patch fixes and exact date-bound security cooldown
exceptions are included so the required audit can pass. This does not change
production trust installation or grant new publisher/agent permissions.

The owner approved a separate Tailscale connection for this VM and requested
Tailscale plus GitHub CLI installation through provisioning. Follow-up checks
cover reruns and installation without automatic account/device authentication.
The guest full validator and the follow-up change's full validator must each
be recorded independently; a workstation pass does not prove the VM works.
