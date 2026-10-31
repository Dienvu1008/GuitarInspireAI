# Feasibility Assessment

Purpose: Classify every long-term capability by technical feasibility and computational demand, separating achievable engineering from research-intensive work.
Status: Proposed. Classes are reviewed at each phase entry using spike results.
Related documents: [technology-evaluation.md](technology-evaluation.md), [ai-music-architecture.md](../architecture/ai-music-architecture.md), [performance-requirements.md](../architecture/performance-requirements.md), [implementation-phases.md](../roadmap/implementation-phases.md).

## Classes

- D (Deterministic, achievable): Well-known algorithms or engineering. Effort can be estimated.
- M (ML, moderate reliability): Achievable with existing models. Output is a candidate that needs user review. Effort is estimable only after a spike.
- R (Research-intensive): No reliable general solution known [Assumption based on state of MIR practice]. Must be timeboxed. Success is not promised.

Compute classes: L (light, any device), Mo (moderate, real-time on desktop and recent mobile), H (heavy, desktop with GPU preferred, minutes per song on CPU).

## Matrix

| Capability | Class | Compute | Phase | Notes |
|---|---|---|---|---|
| TAB editor, multi-track model | D | L | 1, 3 | Largest effort is UI, not algorithms |
| Standard notation display | D, high effort | L | 6c | Engraving quality is hard. S11 |
| PDF export | D | L | 6c | Depends on layout engine |
| MIDI import and export | D | L | 4 | MIDI import to TAB requires string assignment (see below) |
| MusicXML export | D | L | 6b | Import is harder (broad spec) |
| Guitar Pro import | D with format risk | L | 6d | Proprietary format, reverse-engineered knowledge. S10 |
| SoundFont playback with tempo | D | Mo | 4 | S2 |
| Tuner (monophonic pitch) | D | Mo | 5 | YIN/MPM. S3 |
| Chord finder, scale finder, fretboard | D | L | 2 | Pure theory |
| Voicing exploration, CAGED | D | L | 2, 6 | |
| Fingering suggestion for a given note sequence | D (optimization) | L to Mo | 6+ | Dynamic programming over positions with a cost model. Quality depends on cost model tuning |
| Pattern engine to TAB and MIDI | D | L | 6a | Blocked by DSL semantics questions |
| Vocal pitch tracking, range estimation | D | Mo | 8 | pYIN class algorithms |
| Key estimation from notes | D | L | 8 | Profile matching (Krumhansl-type) |
| Key estimation from audio | D / M | Mo | 9 | Chroma-based, moderate reliability |
| Onset detection, tempo estimation | D | Mo | 8 | Reliable on clear material, weaker on rubato |
| Melody to MIDI (monophonic vocal) | D / M | Mo | 8 | Note segmentation is the hard part |
| Transposition and key suggestion for singer range | D | L | 8 | Given range estimate |
| Chord recognition from audio | M | Mo | 9 | Moderate accuracy on pop material, weaker on complex harmony |
| Polyphonic transcription to MIDI (piano-roll) | M | Mo to H | 9 | Basic Pitch class models |
| Source separation (vocals, drums, bass, other) | M | H | 9 | Desktop only initially. S9 |
| Instrument recognition | M | Mo | 9+ | Track-level tags are feasible; note-level attribution is R |
| Bass transcription from separated stem | M | H | 9 | Errors compound across separation and transcription |
| Drum transcription, sparse mixes | M | Mo | 9+ | |
| Drum transcription, dense mixes | R | H | 10+ | |
| Musical structure detection (verse, chorus) | M / R | Mo | 9+ | Boundaries feasible, labels unreliable |
| Audio to accurate guitar TAB (polyphonic, arbitrary audio) | R | H | 10+ | See below |
| Chord progression inference from vocal melody | M | L | 10 | Many valid answers, needs user choice |
| Accompaniment and arrangement generation | R (quality) | Mo to H | 10 | Rule-based baseline is D; good musical quality is R |
| On-device mobile source separation | R | H | Deferred | Memory and battery limits |
| Real-time collaboration | D (engineering heavy) | L | 11 | Requires CRDT model change (see score-model-and-undo) |

## Why audio to guitar TAB is research-intensive

1. Separation errors: guitar is rarely isolated; separation leaks other instruments.
2. Polyphonic pitch errors: octave errors, missed notes in strummed chords.
3. String and fret ambiguity: one pitch maps to up to 5 or 6 positions on a guitar. The audio rarely encodes the chosen string reliably (timbre cues are subtle).
4. Technique detection (bends, slides, hammer-ons, palm muting) is an open problem at note level.
5. Errors compound across these stages.

Feasible approximation: transcribe to pitch and timing candidates (M), then run a deterministic playability optimizer (D) to propose a TAB that the user edits. Output is always a candidate (ADR-013).

## MVP feasibility check

All MVP features are class D. The main risks are engineering, not research: TAB editor UI effort, real-time audio correctness on Windows, and interop ergonomics. These are covered by spikes S1 to S5 in Phase 0.
