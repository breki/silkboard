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

Live guest provisioning, endpoint reachability, agent authentication and
production trust rotation remain unverified until their operator steps run.
The previously authorized v0.4.0 release remains separately in the primary
worktree; do not include its unfinished version bump in this change.

Validation discovered new publisher TLS advisories RUSTSEC-2026-0381/-0382.
The minimum two transitive patch fixes and exact date-bound security cooldown
exceptions are included so the required audit can pass. This does not change
production trust installation or grant new publisher/agent permissions.
