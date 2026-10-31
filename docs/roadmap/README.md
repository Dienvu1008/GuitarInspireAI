# Roadmap Overview and MVP Definition

Purpose: Define the MVP, separate the minimum viable implementation from the long-term direction, and point to phase, milestone, risk and dependency documents.
Status: Accepted (MVP scope). Estimates are FTE weeks and are estimates, not commitments.
Related documents: [implementation-phases.md](implementation-phases.md), [milestones.md](milestones.md), [risk-register.md](risk-register.md), [dependency-map.md](dependency-map.md), [technical-spikes.md](../research/technical-spikes.md), [feasibility-assessment.md](../research/feasibility-assessment.md).

This roadmap describes future work. Nothing in it, including Phase 0 and spikes, was executed during the documentation assignment.

## 1. MVP (milestone M5, end of Phase 5)

A Windows desktop app that validates the architecture end to end and is useful without AI:

| Capability | MVP scope | Not in MVP |
|---|---|---|
| TAB editor | Multiple fretted tracks managed in the track list (add, remove, rename, reorder, switch); one active TAB track displayed and edited at a time (Q-17); edit tuning; insert, delete, change notes by keyboard and mouse; rests; durations with dots and triplets; ties; bars with time signature changes; copy and paste within a track; undo and redo; diagnostics shown | Standard notation, effects beyond tie, let ring and dead note, simultaneous multi-track notation display (deferred to Phase 6b), lyrics, repeats editing UI beyond simple start and end |
| Playback | All tracks play simultaneously, independent of which track is displayed; play, stop, seek, loop selection [Proposed], tempo factor, per-track mute; SoundFont synth | Mixer, effects, audio tracks, count-in [Proposed later] |
| Project | New, open, save, save as, autosave and crash recovery; MIDI import and export | MusicXML, Guitar Pro, PDF |
| Tuner | Microphone tuner for the track tuning or standard presets | Polyphonic tuning, calibration beyond A4 reference |
| Chord finder and fretboard | Chord symbol to voicings, voicing to chord name, fretboard view with scale and chord tones; insert chord into TAB | CAGED explorer, fingering optimization |

Effort to MVP: Phases 0 to 5, 19 to 31 FTE weeks (Phases 1 to 5: 17 to 28).

## 2. Minimum vs long-term

| Area | Minimum viable implementation | Long-term direction |
|---|---|---|
| Score model | Fretted tracks, master bars, voices, rational time | Pitched and percussion tracks, polymeter (major format), audio tracks |
| UI | Fixed desktop layout | Adaptive mobile, tablet, desktop with docking and workspaces |
| Notation | TAB only | Standard notation, PDF, engraving quality |
| Interop | frb typed API | Same; IPC only if sidecar or plugins arrive |
| Storage | ZIP + JSON, JSON preferences | SQLite library index and analysis cache, sync |
| Audio | WASAPI playback and tuner | Recording, low-latency exclusive mode, mobile audio |
| Intelligence | None | Vocal analysis (deterministic), MIR and ML candidates, arrangement |
| Ecosystem | None | SDK, CLI tools, Web, plugins |

## 3. Estimated vs research work

- Phases 0 to 5 and 6a, 6b, 7, 8 are estimable engineering (estimates in [implementation-phases.md](implementation-phases.md)).
- Phase 6c (engraving), 6d (Guitar Pro), 9 (MIR and ML), 10 (arrangement) contain research. They have timeboxed spikes and no completion promise.
