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
| [ADR-015](ADR-015-dependency-license-policy.md) | Dependency license policy | Accepted; MPL-2.0 not allowed (own MMCSS call instead); FluidR3 GM default SoundFont (2026-10-31) |
| [ADR-016](ADR-016-undo-structural-sharing-snapshots.md) | Undo via structural-sharing snapshots | Accepted (budgets provisional) |
| [ADR-017](ADR-017-plugin-architecture-deferred.md) | Plugin architecture deferred | Deferred |
| [ADR-018](ADR-018-notation-layout-in-rust.md) | Notation layout in Rust, painting in Flutter | Accepted (S4) |
| [ADR-019](ADR-019-fretted-instrument-shared-kernel.md) | Fretted-instrument value objects in the shared kernel | Accepted |
| [ADR-020](ADR-020-invariants-vs-diagnostics.md) | Invariants vs diagnostics model | Accepted |
| [ADR-021](ADR-021-bundled-font-ofl.md) | Bundled TAB font under SIL OFL 1.1 (Noto Sans Mono) | Accepted (2026-10-31) |
| [ADR-022](ADR-022-unsafe-confined-to-generated-ffi.md) | Unsafe code confined to the generated FFI module | Accepted (2026-10-31) |
| [ADR-023](ADR-023-midi-adapter-crate.md) | MIDI interchange in its own adapter crate `mc-midi` | Accepted (2026-10-31) |
| [ADR-024](ADR-024-playback-rendering-conventions-and-rt-failures.md) | Playback rendering conventions and real-time failure handling | Accepted (2026-10-31) |
| [ADR-025](ADR-025-single-machine-evidence-accepted.md) | Single-machine performance and glitch evidence accepted (R-21 deferred) | Accepted (2026-10-31) |
| [ADR-026](ADR-026-bundled-soundfont-trimmed-fluidr3.md) | Bundled SoundFont: trimmed FluidR3 GM (guitar preset only) | Accepted (2026-10-31) |
| [ADR-027](ADR-027-unsafe-exception-mmcss-module.md) | Unsafe exception for the Windows MMCSS call in mc-audio (extends ADR-022) | Accepted (2026-10-31) |
| [ADR-028](ADR-028-playback-transport-port-and-session-state.md) | Playback transport: application-owned AudioPort, session transport state, swap-on-edit | Accepted (2026-10-31) |
| [ADR-029](ADR-029-tuner-pitch-detection.md) | Tuner pitch detection: owned MPM/YIN, S3 thresholds and results | Draft (thresholds fixed; S3 pending) |
| [ADR-030](ADR-030-editor-debt-clipboard-and-session-marker.md) | Editor debt (Phase 6b-1): beat clipboard, close prompt, session marker | Accepted (2026-11-06) |
| [ADR-031](ADR-031-multi-track-tab-display.md) | Simultaneous multi-track TAB display (Phase 6b-2) | Accepted (2026-11-06) |
| [ADR-032](ADR-032-track-mixer-volume-pan-solo.md) | Track mixer basics: volume, pan, solo (Phase 6b-3) | Accepted (2026-11-07) |
| [ADR-033](ADR-033-musicxml-export-adapter.md) | MusicXML export adapter `mc-musicxml` (Phase 6b-4) | Accepted (2026-11-07) |
| [ADR-034](ADR-034-pattern-engine-ir-first.md) | Pattern engine: the Pattern IR is the source of truth (Phase 6a) | Draft rev. 2 (user decisions recorded; awaiting approval) |
| [ADR-035](ADR-035-playing-techniques-in-the-score.md) | Playing techniques in the score: accent, palm mute, string-percussive, stroke (Phase 6a-1) | Draft rev. 2 (user decisions recorded; awaiting approval) |
