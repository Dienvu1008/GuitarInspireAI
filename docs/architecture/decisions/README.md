# Architectural Decision Records

Purpose: Define the ADR process, status lifecycle and template, and index all ADRs.
Status: Accepted.
Related documents: [dependency-rules.md](../dependency-rules.md), [fitness-functions.md](../fitness-functions.md), [architecture/README.md](../README.md).

## 1. When an ADR is required

- Any change to a governance rule (GR-01 to GR-14) or a Critical fitness function.
- Adding or removing a crate, a bounded context, or an edge in the allowed dependency matrix.
- Adding a dependency outside the license allow-list, or any native dependency.
- Any project format major version, or a new `required_features` entry.
- Changing interop transport, threading model, or audio thread rules.
- Reversing or superseding an existing ADR.

Small, local design choices do not need an ADR.

## 2. Status lifecycle

Proposed (awaiting a spike or confirmation) -> Accepted -> Superseded by ADR-NNN, or Deprecated. Deferred means intentionally postponed with a trigger. ADRs are append-only: a decision is changed by a new ADR that supersedes it, and the old one gets a "Superseded by" line.

Commits that change protected paths reference an ADR with a trailer `Architecture-Change: ADR-NNN` (FF-19).

## 3. Template

```markdown
# ADR-NNN Title

Status: Proposed | Accepted | Deferred | Superseded by ADR-NNN
Date: YYYY-MM-DD
Related documents: links

## Context
## Decision
## Alternatives
## Trade-offs
## Consequences
## Related fitness functions
## Reconsideration conditions
```

## 4. Index

| ADR | Title | Status |
|---|---|---|
| [ADR-001](ADR-001-modular-monolith.md) | Modular monolith | Accepted |
| [ADR-002](ADR-002-rust-core-source-of-truth.md) | Rust Music Core is the source of truth | Accepted |
| [ADR-003](ADR-003-context-map-and-crate-dag.md) | Context map and crate DAG | Accepted |
| [ADR-004](ADR-004-flutter-desktop-first.md) | Flutter presentation, desktop first on Windows | Accepted |
| [ADR-005](ADR-005-dart-rust-interop-frb.md) | Dart and Rust interop via flutter_rust_bridge, no Protobuf | Accepted with amendments (S1) |
| [ADR-006](ADR-006-event-model.md) | Event model: invalidation events, no global bus | Accepted |
| [ADR-007](ADR-007-audio-thread-isolation.md) | Audio thread isolation | Accepted with amendments (S2); tuner part pending S3 |
| [ADR-008](ADR-008-native-project-format.md) | Native project format: ZIP + JSON with versioned DTOs | Accepted with amendments (S5) |
| [ADR-009](ADR-009-defer-sqlite.md) | Defer SQLite | Accepted |
| [ADR-010](ADR-010-local-first.md) | Local-first, no network in MVP | Accepted |
| [ADR-011](ADR-011-rational-time-model.md) | Rational time model | Accepted |
| [ADR-012](ADR-012-interchange-formats-as-adapters.md) | Interchange formats are adapters | Accepted |
| [ADR-013](ADR-013-ai-provenance-candidate-layer.md) | AI provenance and candidate layer | Accepted (principle), Deferred (implementation) |
| [ADR-014](ADR-014-wasm-compatible-domain-crates.md) | WASM-compatible domain crates | Accepted |
| [ADR-015](ADR-015-dependency-license-policy.md) | Dependency license policy | Accepted; MPL-2.0 decision for `audio_thread_priority` pending user |
| [ADR-016](ADR-016-undo-structural-sharing-snapshots.md) | Undo via structural-sharing snapshots | Accepted (budgets provisional) |
| [ADR-017](ADR-017-plugin-architecture-deferred.md) | Plugin architecture deferred | Deferred |
| [ADR-018](ADR-018-notation-layout-in-rust.md) | Notation layout in Rust, painting in Flutter | Accepted (S4) |
| [ADR-019](ADR-019-fretted-instrument-shared-kernel.md) | Fretted-instrument value objects in the shared kernel | Accepted |
| [ADR-020](ADR-020-invariants-vs-diagnostics.md) | Invariants vs diagnostics model | Accepted |
