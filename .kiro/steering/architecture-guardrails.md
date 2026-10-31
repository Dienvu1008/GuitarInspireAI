---
inclusion: always
---

# Architecture Guardrails for Coding Agents

These rules apply to every change in this repository. Full details: `docs/architecture/dependency-rules.md`, `docs/architecture/fitness-functions.md`, `docs/architecture/decisions/README.md`.

## Governance rules (summary)

- GR-01 Domain crates must not depend on Flutter or flutter_rust_bridge (frb only in `mc-ffi`).
- GR-02 Domain crates must not depend on SQLite.
- GR-03 `mc-dsp` must not depend on presentation or other internal crates.
- GR-04 AI and analysis code must not bypass domain validation; candidates become score content only through the `AcceptCandidate` command.
- GR-05 Adapters and Dart code must not own domain decisions.
- GR-06 Contexts must not access each other's internals (Rust visibility, Dart `lib/src`).
- GR-07 No circular dependencies between crates or Dart packages.
- GR-08 Project format changes follow the versioning and migration policy (`docs/architecture/project-format.md`).
- GR-09 All imported data passes validation through `ScoreBuilder`.
- GR-10 Real-time audio code in `mc-audio/src/rt/` must not allocate, lock, log, perform I/O or call FFI.
- GR-11 Platform-specific code stays behind adapters.
- GR-12 Fitness functions are version-controlled in this repository.
- GR-13 Changes to protected paths require an ADR reference.
- GR-14 Agents must not weaken or delete fitness functions to make tests pass.

## Protected paths

`architecture/`, `deny.toml`, `clippy.toml`, `xtask/src/fitness/`, CI workflow files, `docs/architecture/dependency-rules.md`, `docs/architecture/fitness-functions.md`, `docs/architecture/decisions/`, and this file.

## Mandatory agent behaviour

1. Never delete, skip, disable, loosen or rename an architecture test, fitness function, invariant test, lint rule, cargo-deny rule or allowed-dependency entry to make a build or test pass.
2. If a change appears to require weakening a rule, stop and explain the conflict to the user. Propose either a code change that satisfies the rule or a new ADR for the user to approve.
3. Any commit that touches a protected path must include the trailer `Architecture-Change: ADR-NNN` referencing an existing or newly written ADR approved by the user.
4. Do not add dependencies outside the license allow-list in ADR-015 without an ADR.
5. Do not put music rules in Dart or in adapters; add them to the owning Rust context.
6. Do not create empty crates or packages ahead of the phase that needs them.
7. Do not modify fixture files of released project format versions; they are append-only.
