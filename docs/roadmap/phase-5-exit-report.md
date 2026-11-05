# Phase 5 Exit Report (MVP, M5)

Purpose: Evidence against the Phase 5 acceptance criteria (tuner, chord finder, fretboard; MVP close), the single-machine and synthetic-only scope of the tuner evidence, the MVP feature checklist, the crate-count and Web-priority (Q-14) review the roadmap asks for at M5, and open points.
Status: Proposed for user acceptance. CI status in section 8.
Related documents: [implementation-phases.md](implementation-phases.md), [milestones.md](milestones.md), [README.md](README.md), [mvp-known-limitations.md](mvp-known-limitations.md), [mvp-release-notes.md](mvp-release-notes.md), [phase-4-exit-report.md](phase-4-exit-report.md), [ADR-029](../architecture/decisions/ADR-029-tuner-pitch-detection.md), [ADR-025](../architecture/decisions/ADR-025-single-machine-evidence-accepted.md).

## 1. Outcome

Phase 5 delivered the MVP tools on branch `phase-5-mvp` in six slices: the S3 tuner spike (5a), the `mc-dsp` pitch detector (5b), the `mc-audio` tuner engine (5c), the tuner FFI and the `tools_feature` tuner UI (5d), the chord finder and fretboard with insert-at-cursor (5e), and this MVP close (5f). No standard notation, MusicXML, Guitar Pro, mixer, vocal analysis or AI was added.

| Area | Where |
|---|---|
| Pitch detector (MPM, allocation-free, preallocated FFT) | `mc-dsp` (new domain crate) |
| Microphone tuner (input ring, non-RT detector thread, MMCSS reused) | `mc-audio` (`TunerHandle`) |
| Reference pitch (nearest string + cents from tuning + capo) | `mc-guitar` (`tuner_targets`, `nearest`) |
| Chord voicings for a track | `mc-guitar` search (Phase 2) via `mc-app::guitar::track_chord_voicings` |
| Tuner / chord FFI (opaque handles, typed errors, DTOs) | `mc-ffi` |
| Tuner, chord finder, fretboard UI | `packages/tools_feature` |
| App Tools menu, tuner/chord panels, insert-at-cursor wiring | `app/` (`tools_wiring`, `playback_wiring`, `shell`) |

## 2. Acceptance criteria

| Criterion ([implementation-phases.md](implementation-phases.md) Phase 5; M5) | Evidence | Result |
|---|---|---|
| A8 latency ≤ ~100 ms | S3: MPM 24–35 ms to a stable reading (ADR-029 §5) | Met (single machine) |
| A9 accuracy ±2 cents steady state, stable low E | Synthetic: MPM ≤ 0.2 cents clean across E2..E6, 0% octave errors; `mc-dsp` tests enforce it. **Real guitar not measured** (§4) | Met on synthetic; real-guitar pending |
| A10 mic permission / device errors are typed, not crashes | `mc-audio` typed `NoInputDevice`/`MicPermissionDenied`/`Stream`; tuner UI shows distinct messages and the rest of the app keeps working (T2); device test | Met (automated path; privacy/busy in the manual script) |
| Chord finder results match the Phase 2 golden set in the UI | The UI calls the same `mc-guitar` search; the real-core test shows open C = `x32010` first with the Phase 2 ordering | Met |
| MVP feature list complete | Section 3 | Met, with the documented limitations |

Milestone M5 demo ("tune a guitar, find a chord, insert it, play it back, save"): tuning via the Tools > Tuner panel; chord finder inserts a voicing at the cursor; playback plays it; save/open/MIDI work from Phase 4. Each step is covered by widget/integration tests; the tuner's live microphone and the privacy cases are in the manual script (section 7).

## 3. MVP feature checklist ([README.md](README.md) section 1)

| Capability | MVP scope | Status |
|---|---|---|
| TAB editor | multi-track management, one active track, keyboard/mouse entry, durations+dots+triplets, ties, time-signature bars, undo/redo, diagnostics | Done (Phase 3; copy/paste within a track is the one editor item not yet wired — see O-4) |
| Playback | all tracks together, play/stop/seek, tempo factor, per-track mute, SoundFont synth | Done (Phase 4). Loop selection was Proposed; deferred (O-2) |
| Project | new/open/save/save as, autosave, crash recovery, MIDI import/export | Done (Phases 3–4) |
| Tuner | microphone tuner for the track tuning or presets | Done (Phase 5), synthetic-evidence caveat (§4) |
| Chord finder and fretboard | symbol → voicings, fretboard view, insert into TAB | Done (Phase 5). Voicing → chord name exists in the core (`identify`) but is not surfaced as a separate UI panel in the MVP (O-3) |

## 4. Tuner evidence and its scope (single machine + synthetic, ADR-025 / ADR-029)

- **A9/A8 measured on synthetic signals** (sine and a harmonic pluck, ± 20 dB noise): MPM is within ~0.02 cents across E2..E6 clean, 0% octave errors, allocation-free per window; latency 24–35 ms. Numbers and thresholds are fixed in [ADR-029](../architecture/decisions/ADR-029-tuner-pitch-detection.md) before the run.
- **Real-guitar accuracy is not measured.** The development environment cannot capture a physical instrument; a web virtual guitar was considered and rejected as not representative of microphone input. ADR-029 stays **Draft**; the MPM-vs-YIN choice is provisional (MPM). The capture harness and procedure are committed (`spikes/s3-tuner/`); the developer records the set and fills ADR-029 §5 to close it. **No real-guitar number is invented.**
- Single machine only (ADR-025, R-21 open). The exit report does not claim coverage beyond the reference machine.

## 5. Defects and decisions

- A spike MPM bug (missing initial-lobe skip → E2 read as ~427 kHz) was fixed before measuring; a second, high-note octave fold (short period, `min_lag` inside the first lobe) was caught by `mc-dsp`'s chromatic sine-sweep property test and fixed (skip from lag 1, apply `min_lag` as a candidate floor). Both are recorded in ADR-029 §5; the spike is throwaway and did not seed the production crate.
- Reference-pitch and chord search are Guitar-context rules (GR-05): they live in `mc-guitar`, not in the audio adapter or Dart. The tuner engine reports only a frequency; the app maps it against the track's tuning and capo (T8) and renders voicings (T6).
- No new `unsafe`: the tuner thread reuses the MMCSS call (ADR-027); FF-34 still sees exactly the two reviewed exceptions.

## 6. Fitness functions, crate count and Q-14 review

- 33 of 34 checks active (unchanged; only FF-31, Phase 8, is planned). `current_phase = 5`. `mc-dsp` joins the FF-01 matrix, FF-03 purity (its `clippy.toml`), FF-04 wasm and the FF-22 public-API snapshots.
- **Crate-count review (roadmap asks at M5):** 12 production Rust crates (`mc-theory`, `mc-score`, `mc-guitar`, `mc-notation`, `mc-playback`, `mc-project`, `mc-dsp`, `mc-audio`, `mc-midi`, `mc-app`, `mc-ffi`, `mc-cli`) plus `xtask`; 7 Dart packages (`core_bridge`, `editor_feature`, `playback_feature`, `tools_feature`, `app_platform`, `ui_kit`, `mc_ffi`). Each crate maps to one bounded context or adapter; no empty crates. The count is proportionate; no consolidation is proposed.
- **Web priority (Q-14):** recommend keeping Web at Phase 11 (unchanged). The domain crates compile for wasm32 (FF-04) including `mc-dsp`, so the option stays cheap, but audio (cpal AudioWorklet needs nightly + cross-origin isolation) and the frb Web path are not MVP-justified. Decision is the user's at this review.
- FF-29 bundle: 34.38 MiB with the SoundFont (`benchmarks/bundle-size.md`), +1.87 MiB over Phase 4 for the tuner/chord code and the FFT dependency.

## 7. Manual procedures (reference machine)

- Tuner pitch (synthetic): `cargo test -p mc-dsp --test pitch`.
- Tuner device + A10: `cargo test -p mc-audio --test tuner_device -- --ignored --nocapture`.
- S3 spike: `cargo run --release -- synthetic | latency | input` in `spikes/s3-tuner/`.
- MVP manual script: build the Windows app, then: open the Tuner panel and pluck each
  string (watch the needle and string label); deny the microphone in Windows privacy
  settings and confirm a clear message with the rest of the app still working; hold the
  mic in another app and confirm the busy case; open the Chord finder, type `C`, insert a
  voicing at the cursor, and play it back; tune, find, insert, play, save end to end.

## 8. CI status

Commits `8e2493e` (5a), `5925644` (5b), `d0a7379` (5c), `689b59d`/`c617342` (5d),
`f098699` (5e) all passed GitHub Actions (governance, cargo-deny, bridge, including the
new `tools_feature` step and the Windows build/integration tests). The 5f commit adds the
MVP docs, the FF-29 update, `current_phase = 5` and this report; local runs of every CI
step passed before push.

## 9. Closure

Phase 5 closes with a merge of `phase-5-mvp` into `main` with CI passing. The MVP (M5) is
reached with the documented limitations ([mvp-known-limitations.md](mvp-known-limitations.md)).
ADR-029 remains Draft until the real-guitar tuner set is recorded. Next: Phase 6 (editor
depth, post-MVP), sub-phases reorderable by user priority.
