# Architecture Summary

Purpose: Executive summary of the architecture, plus the independent review findings and the revisions they caused.
Status: Accepted pre-implementation baseline (2026-10-31, after the final consistency pass). Items marked Proposed depend on Phase 0 spikes. M0 results are in [m0-exit-report.md](../roadmap/m0-exit-report.md).
Related documents: [docs/README.md](../README.md), [architecture-overview.md](architecture-overview.md), [domain-map.md](domain-map.md), [decisions/README.md](decisions/README.md), [roadmap/README.md](../roadmap/README.md), [open-questions.md](../research/open-questions.md).

## 1. Recommended architecture

A modular monolith: a Rust Cargo workspace (Music Core) and a Flutter app, in one process. Rust is the single source of truth for music data; Dart holds opaque handles, UI-only state and cached projections. Inside Rust: pure domain crates, an application crate `mc-app` with ports, and adapter crates (`mc-audio`, `mc-ffi`, `mc-cli`). Crates are introduced phase by phase (11 by MVP). Details: [architecture-overview.md](architecture-overview.md).

Alternatives considered: all-Dart, Flutter + C++/JUCE, Tauri, Kotlin Multiplatform, Rust-only UI, microservices and sidecar processes ([alternative-architectures.md](../research/alternative-architectures.md)). Rust + Flutter won on a reusable, testable core, one UI codebase, and real-time audio capability.

## 2. Domain map

Music Theory (shared kernel, including fretted-instrument value objects), Composition (core, one Score aggregate), Guitar, Pattern Engine (Phase 6a), Project, Playback, Notation Layout (projection), DSP, Analysis (Phase 8). Notation, Audio and Workspace were rejected as bounded contexts. `mc-score` does not depend on `mc-guitar` ([ADR-019](decisions/ADR-019-fretted-instrument-shared-kernel.md)). Details: [domain-map.md](domain-map.md), [bounded-contexts.md](bounded-contexts.md).

## 3. Most important decisions

| Decision | ADR | Alternatives rejected |
|---|---|---|
| Modular monolith | ADR-001 | Microservices, sidecar |
| Rust is source of truth | ADR-002 | Duplicate Dart model |
| frb typed commands, queries and streams; no Protobuf | ADR-005 | Polling, Protobuf over FFI, gRPC IPC |
| Invalidation events per document, no global bus | ADR-006 | Global bus, content events |
| Control and data plane split for audio | ADR-007 | Shared state with locks |
| ZIP + JSON with versioned DTOs | ADR-008 | Directory, SQLite, interchange formats |
| Rational time | ADR-011 | Floats, fixed ticks |
| Structural-sharing undo snapshots | ADR-016 | Full clones, inverse commands, event sourcing |
| Fretted value objects in shared kernel | ADR-019 | Score to Guitar edge, pitch-only score, new crate now |
| Four-category invariant model | ADR-020 | Everything hard, strict bar fill |

## 4. Critical invariants

Positive rational durations (T-01); unique ids (H-01); track bars aligned with master bars as a v1 model limit (H-02); string and fret ranges (H-03, H-04, H-10); one note per string per beat per voice (H-05); valid ties (H-06); validated loads and imports (P-05, IM-01); candidates only through `AcceptCandidate` (AI-02). Overfull and underfull bars are diagnostics (D-01, Accepted), with playback using the effective master bar duration across all tracks and voices so bars stay globally synchronized. MIDI range is a playback concern (D-05). Details: [domain-invariants.md](domain-invariants.md).

## 5. Fitness functions

33 checks (FF-01 to FF-33), activated per phase. Critical examples: crate dependency matrix, cargo-deny bans and licenses, domain purity lints, WASM compile check, Dart import boundaries, format round-trip suite, schema snapshot, invariant and undo property tests, bindings in sync, RT no-alloc and RT lint, protected-path ADR trailer, fitness inventory that cannot shrink. Performance thresholds apply only after baselines. Details: [fitness-functions.md](fitness-functions.md), rules GR-01 to GR-14 in [dependency-rules.md](dependency-rules.md).

## 6. Storage and project format

Native single-file ZIP package with manifest, major and minor versions, required and optional features, SHA-256 per entry, deterministic JSON, versioned DTOs converted through validating builders, forward migration chain, unknown-field capture on every DTO with forced Save As when unknown content exists, atomic temp-and-replace save with backup, autosave and crash recovery, strict ZIP limits. SQLite is deferred; interchange formats are adapters with reports. Details: [project-format.md](project-format.md), [data-architecture.md](data-architecture.md).

## 7. Rust and Flutter integration

flutter_rust_bridge v2 (MIT, stable 2.13.0 [Verified: crates.io API, 2026-10]) in `mc-ffi` only. `execute(CommandDto)` returns an outcome with revision and events; an exhaustive `match` dispatcher routes to per-context handlers; a per-document event stream with seq, revision and correlation ids; resync on gaps; separate latest-value streams for playback position and tuner; no audio samples across FFI. Details: [interop-architecture.md](interop-architecture.md).

## 8. Cross-platform strategy

Windows first (WASAPI, Flutter desktop). macOS and Linux kept building weekly and ported after MVP. Android and iOS in Phase 7 after S12. Web kept possible by making domain crates WASM-compatible (FF-04); known blockers recorded in [ADR-014](decisions/ADR-014-wasm-compatible-domain-crates.md). Adaptive layouts by window size class ([cross-platform-ui.md](cross-platform-ui.md)).

## 9. MVP

Windows desktop: TAB editor with tracks, notes, durations, ties, undo and diagnostics; playback with tempo factor and mute; native save, autosave and recovery; MIDI import and export; tuner; chord finder and fretboard. No AI. Details: [roadmap/README.md](../roadmap/README.md).

## 10. Roadmap

Phase 0 foundation and spikes (2 to 3 FTE weeks), Phase 1 core and format (4 to 6), Phase 2 guitar (2 to 3), Phase 3 editor (6 to 10), Phase 4 playback and MIDI (3 to 5), Phase 5 tools and MVP (2 to 4). Phases 1 to 5: 17 to 28 FTE weeks; Phases 0 to 5: 19 to 31. Phases 6 to 11 follow, with research phases timeboxed. Details: [implementation-phases.md](../roadmap/implementation-phases.md), [milestones.md](../roadmap/milestones.md).

## 11. Major risks

TAB editor effort (R-04), solo capacity (R-17), RT audio safety (R-02), frb dependency (R-01), engraving effort (R-13), license issues in bundled data (R-16), AI quality (R-15). Details: [risk-register.md](../roadmap/risk-register.md).

## 12. Unresolved questions

Project license (Q-10), file extension (Q-08), pattern DSL semantics (Q-01 to Q-06), state management library (Q-12), SoundFont (Q-11), minimum Windows and macOS versions (Q-13), Web priority (Q-14). Resolved on 2026-10-31: string numbering (Q-07), bar diagnostics (Q-09), single active TAB track in MVP (Q-17). Full list: [open-questions.md](../research/open-questions.md).

## 13. Recommended next actions

1. Done: user review; Q-07, Q-09 and Q-17 answered. Still needed: Q-08 before Phase 1 exit.
2. Start M0: repository and CI skeleton with FF-09, FF-10, FF-19, FF-20, FF-33.
3. Run S1, S2, S4, S5 first, then S7. Update ADR-005, ADR-007, ADR-008 and ADR-018 with results.
4. Begin Phase 1 only after Phase 0 exit criteria are met.

## 14. What the architecture accommodates and what it does not

Designed to accommodate without structural change: new commands and projections, new import and export adapters, additional instrument kinds as new variants, analysis features as candidates, replacing frb or the synth, a CLI and SDK, WASM builds of domain crates, additive format changes.

Requires architectural reconsideration: real-time collaboration (operation-based history, possibly aggregate split), polymeter or free-time tracks (major format version), audio track editing, plugin hosting, out-of-process ML runtimes, very large audio projects (directory or hybrid format). This architecture is not future-proof; these are the known limits.

## 15. Independent review findings and revisions

Review lenses: overengineering, unnecessary abstraction, premature optimization, excessive modularization, hidden coupling, inconsistent boundaries, unclear ownership, migration risks, cross-platform gaps, unrealistic AI assumptions, testability gaps, unenforceable rules.

### Revised during review

| Finding | Revision |
|---|---|
| The dependency matrix allowed `mc-ffi` and `mc-cli` to depend on every domain crate, contradicting the DAG and letting the bridge bypass `mc-app` (hidden coupling, unenforceable rule) | `mc-ffi` may depend only on `mc-app` and `mc-audio`; `mc-app` re-exports needed types; `mc-cli` limited to `mc-app`, `mc-project`, optional `mc-audio`. FF-01 now enforces the boundary ([dependency-rules.md](dependency-rules.md)) |
| No owner for the file-system project store (atomic save, recovery); `mc-project` is bytes-only and banned from `std::fs` (unclear ownership) | Placed in `mc-app::infra::fs_store` behind feature `native-fs` ([architecture-overview.md](architecture-overview.md), [bounded-contexts.md](bounded-contexts.md), Phase 1 scope) |
| Full unknown-field side-map preservation before any 1.1 format exists (premature complexity) | MVP: U1 detection and U2 Save As protection; U3 full preservation built with the first 1.1 change ([project-format.md](project-format.md) section 9) |
| Spike timeboxes (earlier 15+ days) exceeded the Phase 0 estimate (planning inconsistency) | Timeboxes reduced and treated as maxima; S3 moved into the Phase 5 budget; S6 optional; reconciliation in [technical-spikes.md](../research/technical-spikes.md) |
| `mc-score -> mc-guitar` edge (core depends on supporting context) | Fretted value objects moved to `mc-theory` ([ADR-019](decisions/ADR-019-fretted-instrument-shared-kernel.md)) |
| Overfull bars, cross-voice string collisions and MIDI range as hard invariants (over-restrictive) | Reclassified as diagnostics or playback constraints ([ADR-020](decisions/ADR-020-invariants-vs-diagnostics.md)) |
| Event `causation` field unused in MVP (overengineering) | Kept as an optional field, always `None` in MVP; no logic depends on it |
| Pattern DSL interpretation had been assumed | Grammar left open with candidate interpretations I1 to I3 ([guitar-pattern-engine.md](guitar-pattern-engine.md)) |

### Accepted with known trade-offs

| Topic | Assessment |
|---|---|
| 11 crates for a solo developer | Acceptable because crates arrive per phase and share one workspace config. At M5, crates under about 300 lines are reviewed for merging (likely candidates: `mc-dsp`, `mc-notation`) |
| `mc-cli` | Justified: Phase 1 has no UI, and the CLI is the test and debug harness for the core and the base of future CLI tools. Kept minimal |
| `mc-audio -> mc-app` for the `AudioPort` trait | Acceptable; extraction to a small `mc-ports` crate is recorded as an option if the edge causes trouble |
| `Arc` structural sharing | Small read indirection cost for generic, correct undo and cheap background snapshots. Benchmarked in Phase 1 |
| Persistence DTOs duplicate domain shapes | Justified by schema evolution; transport DTOs are avoided through mirroring and re-exports |
| Diagnostics model complicates playback, layout and export | Contained: playback uses max(nominal, longest voice); exporters report |
| Rust layout depends on Dart text metrics | Explicit `LayoutMetrics` contract; risk R-05; S4 |
| Lock-freedom is only partly enforceable | Allocation is checked (FF-16); locks and I/O by lint (FF-17) and review. Stated, not overstated |
| frb Web support unverified | Not an MVP target; S6 optional; FF-04 keeps domain crates WASM-ready |
| Shared kernel may grow | Kernel rule and review (GR-13); `mc-instrument` extraction trigger documented |
| macOS CI cost | Weekly macOS builds until the port phase |
| 33 fitness functions | Activated per phase; Phase 0 activates only five; slow checks run nightly or weekly |
| AI realism | All ML features produce candidates; audio to guitar TAB classified as research; Phase 9 and 10 timeboxed with agreed quality thresholds |

### Validation performed on this package

Validation run on 2026-10-31 (documentation only):

- Relative links: 52 Markdown files checked, 0 broken.
- Mermaid: 11 diagrams parsed with the mermaid 11 parser under jsdom, 0 failures. A deliberately broken diagram was rejected, confirming the check works. Rendering (layout appearance) was not checked.
- Consistency: spike ids only S1 to S12, ADR ids only 001 to 020, GR ids only 01 to 14, FF ids only 01 to 33, no `score --> guitar` edge, no em dashes or exclamation points.
- Estimates: Phases 1 to 5 = 4+2+6+3+2 = 17 to 6+3+10+5+4 = 28 FTE weeks; with Phase 0 (2 to 3) = 19 to 31.
- Not validated: any technical claim marked [Assumption] or [To verify]; these need the Phase 0 spikes and the S7 license audit.
