# Implementation Phases

Purpose: Define each phase with objective, scope, dependencies, architectural prerequisites, deliverables, risks, acceptance criteria, testing strategy, complexity and exit criteria.
Status: Proposed. Phases 0 to 5 are Accepted as the MVP path; later phases are reviewed at each entry.
Related documents: [roadmap/README.md](README.md), [milestones.md](milestones.md), [dependency-map.md](dependency-map.md), [risk-register.md](risk-register.md), [technical-spikes.md](../research/technical-spikes.md), [fitness-functions.md](../architecture/fitness-functions.md).

All estimates are FTE weeks for one developer. Research items are timeboxed and marked as such. None of these activities were executed during the documentation assignment.

## Summary

| Phase | Name | Estimate (FTE weeks) | Kind |
|---|---|---|---|
| 0 | Foundation and risk burn-down | 2 to 3 | Engineering + spikes |
| 1 | Theory, Score, Project format, application layer, CLI harness | 4 to 6 | Engineering |
| 2 | Guitar core | 2 to 3 | Engineering |
| 3 | Flutter shell, bridge, desktop TAB editor | 6 to 10 | Engineering (highest risk) |
| 4 | Playback and MIDI | 3 to 5 | Engineering |
| 5 | Tuner, chord finder and fretboard UI (MVP) | 2 to 4 | Engineering |
| Total 1 to 5 | | 17 to 28 | |
| Total 0 to 5 | | 19 to 31 | |
| 6a | Pattern engine | 3 to 5 after Q-01 to Q-06 | Engineering, blocked by decisions |
| 6b | Multi-track polish, MusicXML export | 3 to 5 | Engineering |
| 6c | Standard notation and PDF | 8 to 16 [high uncertainty] after S11 | Engineering with research |
| 6d | Guitar Pro import | 3 to 6 after S10 | Engineering with format risk |
| 7 | Android, then iOS | 4 to 8 after S12 | Engineering |
| 8 | Vocal analysis (deterministic DSP) | 4 to 6 | Engineering |
| 9 | MIR and ML candidates | Timeboxed research, 2 to 4 per feature before commit | Research |
| 10 | AI arrangement | Timeboxed research | Research |
| 11 | Cloud sync, SDK and CLI publication, Web, plugins | Not estimated | Per-feature ADRs |

## Phase 0: Foundation and risk burn-down

- Objective: Remove the highest architectural uncertainties before production code.
- Scope: Repository and CI skeleton (Windows runner), cargo-deny configuration, protected-path trailer check (FF-19), fitness inventory (FF-20), docs link and Mermaid check (FF-33), steering file, reference machine record; spikes S1, S2, S4, S5, S7. No production crates.
- Dependencies: This documentation package.
- Architectural prerequisites: ADR-001 to ADR-020 reviewed by the user; Q-07, Q-09 and Q-17 answered (done).
- Deliverables: CI pipeline running governance checks; spike result notes; ADR-005, ADR-007 (partly), ADR-008 and ADR-018 moved to Accepted or revised; SoundFont choice.
- Technical risks: frb build issues on Windows (R-01), RT safety of synth (R-02), canvas performance (R-03).
- Acceptance criteria: Each spike has a recorded result against its success criteria; CI fails on a deliberately added forbidden dependency and on a protected-path change without trailer.
- Testing strategy: Spike measurements; CI self-tests with intentional violations on a throwaway branch.
- Estimated complexity: Medium.
- Exit criteria: No spike result invalidates the architecture, or the affected ADRs are revised and accepted.

## Phase 1: Theory, Score, Project format, application layer, CLI

- Objective: A correct, tested music core usable without UI.
- Scope: `mc-theory` (including `fretted`), `mc-score` (aggregate, builder, diagnostics), `mc-project` (v1 DTOs, ZIP read and write on byte streams, migration framework), `mc-app` (session, dispatcher, handlers, undo, events, ports, `native-fs` store with atomic save and recovery), `mc-cli` (create, edit via scripted commands, validate, save, load, dump).
- Dependencies: Phase 0.
- Architectural prerequisites: ADR-008, ADR-011, ADR-016, ADR-019, ADR-020; FF-01 to FF-04, FF-11, FF-13, FF-14, FF-32 activated.
- Deliverables: Crates above; fixture corpus v1.0; criterion benchmarks with baselines; reference score generator.
- Technical risks: Unknown content handling (R-06); rational arithmetic overflow; undo memory (R-07).
- Acceptance criteria: All H rules covered by tests (FF-24 report clean for H rules); PB-01 to PB-08 (effective bar duration) pass; round-trip suite items 1 to 8 pass, with item 4 at levels 4a (U1 detection) and 4b (U2 Save As protection) only (4c and U3 come with format 1.1); CLI can build the large reference score and round-trip it; benchmarks recorded.
- Testing strategy: Unit, proptest command sequences, golden files, malicious ZIP fixtures.
- Estimated complexity: Medium.
- Exit criteria: Milestone M1.

## Phase 2: Guitar core

- Objective: Fretboard and chord intelligence as pure services.
- Scope: `mc-guitar`: tuning presets, fretboard position queries, chord finder both directions, voicing constraints, scale positions, retune planning service; app queries and the retune command.
- Dependencies: Phase 1 (`mc-theory`).
- Architectural prerequisites: ADR-019.
- Deliverables: `mc-guitar` with golden tests for common chords in standard and alternate tunings; CLI commands for chord lookup.
- Technical risks: Chord naming ambiguity (slash chords, omissions) (R-09).
- Acceptance criteria: Golden set of at least 100 chord and voicing cases agreed with the user passes; voicing search under 50 ms for one chord [Assumption, interactive feel].
- Testing strategy: Golden tests, property tests (voicings satisfy constraints, G-01, G-02).
- Estimated complexity: Low to medium.
- Exit criteria: Milestone M2.

## Phase 3: Flutter shell, bridge and desktop TAB editor

- Objective: An editable, saveable TAB document in a Windows desktop app.
- Scope: `mc-notation` (TAB layout), `mc-ffi` (document API, events), Flutter app and packages (`core_bridge`, `editor_feature`, `app_platform`, `ui_kit`), desktop layout, keyboard and mouse editing, track list, inspector, file dialogs, autosave and recovery UI.
- Dependencies: Phases 1 and 2 (chord insert can come in Phase 5).
- Architectural prerequisites: ADR-002, ADR-004, ADR-005, ADR-006, ADR-018, ADR-021, ADR-022 accepted; Q-12, Q-13, Q-15 decided; FF-05, FF-06, FF-08, FF-12, FF-34 activated.
- Deliverables: Windows app build; Dart contract tests against fake and real implementations; layout golden tests.
- Technical risks: Editor UX effort underestimated (R-04), layout metrics coupling (R-05), frb build complexity (R-01).
- Acceptance criteria: User can create a 2-track song, switch the active TAB track from the track list, enter notes by keyboard, undo and redo, save, reopen identically, recover after a forced kill; edit feedback within budget on the typical score.
- Testing strategy: Rust layout goldens, Dart widget tests with fakes, integration tests on Windows, manual exploratory test script.
- Estimated complexity: High.
- Exit criteria: Milestone M3.

## Phase 4: Playback and MIDI

- Objective: Hear the score with tempo control; MIDI interchange.
- Scope: `mc-playback` (timeline compile, tempo map, MIDI mapping), `mc-audio` (cpal output, rustysynth, rings, control thread), transport UI and position stream, MIDI import (with string assignment and report) and export.
- Dependencies: Phase 3 (UI), S2 results.
- Architectural prerequisites: ADR-007 accepted; FF-15 to FF-18 activated.
- Deliverables: Playback in the app; MIDI adapters with fixtures; glitch test procedure based on callback timing; Windows MMCSS thread promotion in `mc-audio`; bundled FluidR3 GM with its original license text and a provenance record (source, version, SHA-256) (ADR-015).
- Technical risks: RT glitches (R-02), device handling (R-08), MIDI string assignment quality (R-10).
- Acceptance criteria: A1 to A7 hold in the product build on the reference machine (single-machine evidence, R-21 open, [ADR-025](../architecture/decisions/ADR-025-single-machine-evidence-accepted.md)); all tracks play simultaneously regardless of the displayed track; play, stop, seek, tempo factor and per-track mute work; PB-09 and timeline variants of PB-02, PB-04 and PB-07 pass; MIDI round trip preserves pitches and rhythm within reported quantization.
- Testing strategy: Timeline unit tests, assert_no_alloc tests, fuzzing of MIDI import, manual glitch test.
- Estimated complexity: Medium to high.
- Exit criteria: Milestone M4.

## Phase 5: Tuner, chord finder and fretboard UI (MVP)

- Objective: Complete the MVP tools.
- Scope: S3 (if not done), `mc-dsp` (YIN or MPM), tuner thread in `mc-audio`, tuner UI, chord finder UI, fretboard widget, insert chord voicing into TAB.
- Dependencies: Phases 2 and 4.
- Architectural prerequisites: ADR-007; GR-03 checks for `mc-dsp`.
- Deliverables: MVP build for Windows; release notes; known limitations list.
- Technical risks: Tuner accuracy on low strings (R-11), microphone permission handling (R-08).
- Acceptance criteria: A8 to A10 hold; chord finder results match the Phase 2 golden set in the UI; MVP feature list in [roadmap/README.md](README.md) complete.
- Testing strategy: Recorded-audio DSP tests, UI tests, MVP acceptance checklist.
- Estimated complexity: Medium.
- Exit criteria: Milestone M5 (MVP). Review crate count, Web priority (Q-14) and next phase order.

## Phase 6: Editor depth (post-MVP)

Sub-phases can be reordered by user priority.

- 6a Pattern engine: Objective: reusable patterns bound to chords. Prerequisites: Q-01 to Q-06 answered (done 2026-11-09), example corpus confirmed by the user. Design: [ADR-034](../architecture/decisions/ADR-034-pattern-engine-ir-first.md) (the Pattern IR is the source of truth; grid editor, gesture palette and text DSL all edit it) and [ADR-035](../architecture/decisions/ADR-035-playing-techniques-in-the-score.md) (accent, palm mute, string-percussive and stroke direction in the score, format 1.1). Both accepted on 2026-11-09. Slices: 6a-1 techniques in the score (format 1.1); 6a-2 IR and validation; 6a-3 binding, placement and ApplyPattern (voices 1 and 2, overwrite safeguard, explicit fit); 6a-4 text DSL and the confirmed corpus; 6a-5 grid editor, palette and library storage (format 1.2). Later, each with its own ADR: figures, tap-to-create, StrokeOnly, more lanes, a chord track, pickup support, muted-string or body percussion, strum spread. Acceptance: the criteria of both ADRs. Complexity: Medium to high. Accepted on 2026-11-11 with five evidence gaps as conditions ([phase-6a-exit-report.md](phase-6a-exit-report.md)).
- 6b Multi-track polish and MusicXML export: simultaneous multi-track notation display (deferred from MVP by Q-17), track mixer basics, MusicXML export with ExportReport. Complexity: Medium. The scope of the MusicXML export is set by [ADR-033](../architecture/decisions/ADR-033-musicxml-export-adapter.md): TAB staff only, uncompressed `.musicxml`. Recorded for later, not scheduled: compressed `.mxl` export, and truncating overfull bars as an option that is never the default.
- 6c Standard notation and PDF: entry gate S11. High uncertainty; engraving quality is a long tail. Complexity: High.
- 6d Guitar Pro import: entry gate S10. Acceptance: fixture files of each supported version import with reports. Complexity: Medium to high with format risk.

Common testing: golden files, fuzzing for importers, format fixtures appended (format minor bump for patterns).

## Phase 7: Mobile (Android, then iOS)

- Objective: Mobile tools (tuner, chord finder, fretboard, TAB viewing and quick edit).
- Entry gate: S12. Prerequisites: compact layouts designed; touch input. Risks: audio latency and packaging (R-12). Complexity: Medium to high.

## Phase 8: Vocal analysis (deterministic)

- Objective: Record vocals, pitch tracking, range estimate, key estimate from notes, transposition suggestions, melody to MIDI candidates.
- Prerequisites: ADR-013 implementation, `mc-analysis` crate, recording support in `mc-audio`, audio assets in format (minor bump), Q-16. Complexity: Medium to high.

## Phase 9: MIR and ML candidates (research, timeboxed)

- Objective: Chord recognition, polyphonic transcription, source separation on desktop.
- Entry gates: S8, S9. Each feature gets a 2 to 4 week timebox; continue only if quality on a test set meets a threshold agreed beforehand with the user.

## Phase 10: AI arrangement (research, timeboxed)

- Objective: Accompaniment from melody plus chords; start rule-based using the pattern engine; ML only after evaluation.

## Phase 11: Ecosystem

- Cloud sync (whole-file first), SDK and CLI publication (semver checks), Web build (ADR-014 blockers), plugins (ADR-017 conditions). Each needs its own ADR and estimate.
