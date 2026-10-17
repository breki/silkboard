# Quality and verification

These policies apply from the first application code. The shared validator
and CI/release workflows enforce the gates; see
[tooling status](tooling.md) for prerequisites and pending interfaces.

## Code and tests

Use Rust edition 2024, denied warnings, forbidden unsafe production code,
and Clippy pedantic where practical. Public APIs need documentation. Use
validated domain types when they carry meaningful invariants; avoid needless
wrappers around values with no rules. Prefer typed library errors and useful
context at application boundaries. Keep entry points thin and test real logic.

For substantive behavior changes, write a failing behavioral test, verify the
failure, implement the change, and refactor with tests passing. Pure data or
simple structural changes can land with their checks as a unit. Documentation
and low-impact edits need appropriate checks, not artificial red/green tests.

Test observable contracts and composition: route authorization, revision and
release selection, update failure, and recovery. Include negative cases and
families of invalid input. Deployment artifacts are legitimate test subjects
when they carry an executable contract. Avoid tests that mirror source text,
heading counts, or implementation details without a behavioral requirement.

## Baseline gates

| Gate | Policy |
| --- | --- |
| Rust formatting | rustfmt, 80-column configuration |
| Rust linting | Zero warnings; Clippy pedantic where practical |
| Coverage | At least 90% overall and 85% per module |
| Duplication | At most 6% of production code |
| Unsafe code | Forbidden in production crates |
| Frontend | Formatting, types, unit tests, relevant browser tests |
| Dependencies | License/source checks, cooldown, security audit |
| Documentation | Valid links and accurate canon references |

Document narrow, justified coverage or tooling exceptions beside their
configuration. Keep unmockable I/O leaves small and exercise the orchestration;
do not exclude business logic to meet a threshold. Do not weaken a gate merely
to make an unrelated change pass.

Use configured xtask entry points once installed. Run targeted checks while
iterating; after a failed gate, rerun that gate before the full pipeline.
Use the full read-only validation form at completion of code changes and
before release, once implemented. Documentation-only changes use link and
consistency checks plus `git diff --check`.

## Dependencies

Keep lockfiles committed once they exist. Prefer scoped updates. Do not adopt
a dependency version less than 14 days old without a stated justification;
urgent security fixes can justify an exception. Check changed transitive
versions too. Review resolved versions before compiling newly adopted code.
License/source checks and advisory audits are different gates. A release must
fail if its required audit cannot run; warnings about unavailable tools are
not a clean audit. Do not introduce broad reusable bypasses.

## Deployment evidence

Local tests and dry runs cannot prove a Pi VM deployment works. Changes to
provisioning, upgrade activation, or recovery need live verification against
an authorized target before claiming operational success. Record the revision,
checks, health result, and any unverified steps. Demonstrate failed-upgrade
recovery and reboot persistence for milestone 1. Do not touch unrelated host
services. Consult the roadmap before choosing the open deployment policies.
