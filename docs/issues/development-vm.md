# Bombyx development VM

Issue [#13](https://github.com/breki/silkboard/issues/13). In progress.

## Goal and agreed approach

Provision Silkboard development directly inside a Bombyx-managed VM, with
Codex and Claude Code. The owner explicitly replaced the container approach
with native VM provisioning. Preserve separation from production and the
owner's workstation credentials. Use Bombyx's existing agent-user script
contract, not a new backend or a nested runtime.

## Acceptance and progress

- Native prerequisite/tool/project setup scripts and external registry example.
- Pinned/checksummed Node/bootstrap downloads and locked agent installation.
- Behavioral tests for setup order, package/tool failure propagation, and
  corrupt downloads failed before implementation and now pass.
- Document migration, login, credential boundaries, and preserved agent
  approval/sandbox settings in [the operator procedure](../development-vm.md).
- Full native read-only validation, ShellCheck, formatting, canon links, and
  diff checks passed. Seven behavioral tests are invoked by the Cargo suite.
- Sequential independent artisan, red-team, and fresh-reader reviews of the
  native implementation found no actionable findings.
- Full guest provisioning, repeat provisioning, ARM64 installation, sandbox
  execution, and authenticated use are still unverified. No development
  migration or production change has occurred.
