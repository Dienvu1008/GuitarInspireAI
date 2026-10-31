# Bounded Contexts

Purpose: Specify each context: responsibilities, entities, value objects, aggregates, services, events, public interface, dependencies, invariants and relationships. Records the Guitar and Score dependency decision.
Status: Accepted for MVP contexts. Pattern and Analysis are Proposed.
Related documents: [domain-map.md](domain-map.md), [domain-invariants.md](domain-invariants.md), [score-model-and-undo.md](score-model-and-undo.md), [guitar-pattern-engine.md](guitar-pattern-engine.md), [ADR-019](decisions/ADR-019-fretted-instrument-shared-kernel.md).

Type names are illustrative, not final API.

## 1. Music Theory (`mc-theory`, shared kernel)

1. Responsibilities: Exact musical value types and pure arithmetic on them.
2. Entities: None.
3. Value objects: `Letter`, `Alter` (-2..=2 in v1), `SpelledPitch` (letter, alter, octave), `PitchClass`, `Interval` (spelled: number and quality), `ScaleFormula`, `ChordFormula`, `ChordSymbol`, `Key`, `Rational`, `NoteValue` duration primitives (base value, dots, tuplet ratio), `TimeSignature`, `Tempo` (positive), and module `fretted`: `StringIndex`, `Fret`, `Capo`, `Tuning`, `FretPosition`, function `pitch_at(tuning, capo, position)`.
4. Aggregates: None.
5. Domain services: Pure functions only: transpose, interval between pitches, enharmonic respelling, chord formula to pitch classes.
6. Domain events: None.
7. Public interface: Value types with smart constructors and pure functions.
8. Dependencies: `std` only (plus `serde` derive behind a feature flag if mirroring requires it; to be decided by S1).
9. Invariants: Valid alteration range, positive rationals, tuning has at least one string.
10. Relationships: Shared kernel for Composition, Guitar, Pattern, Analysis.

Kernel bloat rule (enforced by review, GR-13): only value objects and pure arithmetic. Any search, optimization or catalogue (voicing search, fingering cost, instrument technique lists) belongs to its owning context.

String index convention (Accepted, Q-07): `StringIndex(1)` is the highest-pitched string of the standard layout, matching conventional guitar and TAB numbering. Stored as `u8`, 1-based. String numbers are physical positions: they stay stable when the tuning changes and are never renumbered by pitch (for example, in an unusual tuning where string 2 is tuned above string 1, numbering does not change).

## 2. Composition (`mc-score`, core domain)

1. Responsibilities: Represent the musical content of a song and enforce its structural invariants.
2. Entities: `Track` (id), `Beat` (id), `Note` (id). Master bars are identified by index.
3. Value objects: `MasterBar` (time signature, key, repeat marks, markers, tempo change), `InstrumentSpec` (`Fretted { tuning, fret_count, capo }`, future variants), `NoteValue` (`Fretted { string, fret }`, `Pitched(SpelledPitch)`, later `Percussion`), `NoteEffects` (tie, bend, slide, hammer, palm mute; MVP subset: tie, let ring, dead note [Proposed]), `Diagnostic`.
4. Aggregates: `Score` (root). One aggregate; reasoning in [score-model-and-undo.md](score-model-and-undo.md).
5. Domain services: `ScoreDiagnostics::evaluate(score, bar_range)`, `TimeMap` (rational position to ticks).
6. Domain events: `TrackAdded`, `TrackRemoved`, `TrackPropertiesChanged`, `MasterBarsInserted`, `MasterBarsRemoved`, `MasterBarChanged`, `BarContentChanged { track, bar_range }`, `TempoMapChanged`.
7. Public interface: Aggregate methods taking value objects and returning `Result<Vec<DomainEvent>, DomainError>`. Read accessors. Validating `ScoreBuilder` used by importers and the project loader.
8. Dependencies: `mc-theory` only.
9. Invariants: See [domain-invariants.md](domain-invariants.md) H-01 to H-12.
10. Relationships: Upstream of Project, Playback, Notation, Pattern. Never depends on Guitar.

### Guitar and Score dependency evaluation

The earlier proposal had `mc-score -> mc-guitar` to reuse Tuning, StringNumber and FretPosition. Evaluated alternatives:

| Option | Description | Assessment |
|---|---|---|
| A | `mc-score` depends on `mc-guitar` | Rejected. The core depends on a supporting context. Voicing and algorithm churn ripples into the core. Future instruments would be routed through "guitar" |
| B | Score stores pitch only; Guitar derives string and fret | Rejected. String choice is user-authored musical data (the same pitch on a different string is a deliberate choice). It cannot be derived reliably. A Guitar-owned side table keyed by note id creates synchronization and integrity problems |
| C | New `mc-instrument` crate shared by both | Not now. Adds a crate for about five value types. It is the documented extraction target |
| D | Fretted-instrument value objects in `mc-theory` (module `fretted`) | Recommended. No new crate, no core-to-supporting edge, no cycle. Both `mc-score` and `mc-guitar` depend only on `mc-theory` |

Note model under D:

- For fretted tracks, `NoteValue::Fretted { string, fret }` is authoritative. Sounding pitch is derived from the track tuning and capo with `pitch_at`. Display spelling is derived from key context, with an optional per-note spelling override.
- `NoteValue::Pitched(SpelledPitch)` exists for future non-fretted tracks. Not used in MVP.
- Retuning semantics (keep frets or keep pitches) is a Guitar domain service that computes a list of edits. The application layer applies them as one command.

Reconsideration triggers for extracting `mc-instrument`: instrument ranges and techniques catalogue, drum kits, transposing instruments, or a third context needing instrument data.

## 3. Guitar (`mc-guitar`, supporting)

1. Responsibilities: Fretboard queries and guitar-specific musical knowledge.
2. Entities: None in MVP. Later `CustomTuningPreset` (id) if user presets are stored.
3. Value objects: `Voicing` (list of `Option<FretPosition>` per string), `VoicingConstraints` (max stretch, max fingers, allow open strings, fret range), `ScalePosition`, `CagedShape`, `TuningPreset`.
4. Aggregates: None (stateless services).
5. Domain services: `Fretboard::positions_of(pitch_class, tuning, fret_range)`, `ChordFinder::voicings(chord, tuning, constraints)`, `ChordFinder::identify(voicing, tuning) -> Vec<ChordSymbol>`, `ScaleMap`, `Retune::plan(track_notes, from, to, mode)`, later `FingeringOptimizer`.
6. Domain events: None.
7. Public interface: Pure functions and query structs.
8. Dependencies: `mc-theory`.
9. Invariants: A voicing has at most one position per string; positions are within the fret range of the tuning used.
10. Relationships: Customer of the shared kernel. Upstream of Pattern. Used by `mc-app` for chord finder queries and retune commands.

## 4. Pattern Engine (`mc-pattern`, Phase 6a, Proposed)

1. Responsibilities: Reusable picking and strumming templates, validation, binding to voicings to produce score fragments.
2. Entities: `PatternDefinition` (id, name).
3. Value objects: `Pattern`, `Step`, `StringSelector`, `Direction`, `Articulation`.
4. Aggregates: `PatternDefinition` (small aggregate, stored in the project package).
5. Domain services: `parse`, `validate`, `bind(pattern, voicing, tuning) -> ScoreFragment`.
6. Domain events: `PatternDefined`, `PatternChanged`.
7. Public interface: Parse and bind functions; IR types.
8. Dependencies: `mc-guitar`, `mc-score` (fragment types), `mc-theory`.
9. Invariants: See [guitar-pattern-engine.md](guitar-pattern-engine.md).
10. Relationships: Emits fragments; the application layer applies them as score commands. Grammar not finalized.

## 5. Project (`mc-project`, supporting)

1. Responsibilities: Package format, persistence DTOs, migration, integrity, asset references.
2. Entities: `ProjectManifest` (project id), `AssetRef` (asset id).
3. Value objects: `FormatVersion`, `FeatureFlag`, `ContentHash`, `ImportReport`, `LoadWarning`.
4. Aggregates: `ProjectPackage` (manifest + entries) as a loading and saving unit, not a mutable domain aggregate.
5. Domain services: `load(bytes) -> Result<LoadedProject, LoadError>`, `save(snapshot) -> bytes`, `migrate(dto, from, to)`.
6. Domain events: Application-level only (`DocumentSaved`, `DocumentLoaded`).
7. Public interface: Load and save functions operating on byte streams; file system access is done through the `ProjectStore` port in `mc-app` adapters.
8. Dependencies: `mc-score`, `mc-theory`, `serde`, `serde_json`, `zip`. From Phase 6 also `mc-pattern`.
9. Invariants: P-01 to P-06 in [domain-invariants.md](domain-invariants.md).
10. Relationships: Conformist to Composition. Decoupled from it by versioned DTOs.

Open design point: whether `mc-project` reads bytes only (pure, WASM-friendly) or also files. Decision: bytes and `Read + Seek` streams only; files are opened by the `ProjectStore` adapter in `mc-app` (`infra::fs_store`, feature `native-fs`), which also implements atomic save and recovery.

## 6. Playback (`mc-playback`, supporting, pure)

1. Responsibilities: Convert a Score snapshot into a `PerformanceTimeline` of tick-stamped events, apply repeats, tempo map and tuning; map to MIDI.
2. Entities: None.
3. Value objects: `PerformanceTimeline`, `PerformanceEvent` (note on, note off, program, tempo), `MidiNote` (0..=127), `TickRate`.
4. Aggregates: None.
5. Domain services: `compile(score, options) -> (PerformanceTimeline, Vec<PlaybackWarning>)`, `to_smf(timeline)`.
6. Domain events: None.
7. Public interface: Pure compile functions.
8. Dependencies: `mc-score`, `mc-theory`. MIDI file writing via `midly` stays in an adapter module.
9. Invariants: Timeline events sorted by tick; every note on has a matching note off; pitches outside MIDI range are reported (D-05), not silently clamped.
10. Relationships: Downstream of Composition; upstream of `mc-audio`.

## 7. Notation Layout (`mc-notation`, projection)

1. Responsibilities: Compute TAB (later standard notation) layout as a display list in abstract units.
2. Entities: None. 3. Value objects: `TabLayout`, `SystemLayout`, `GlyphRun`, `HitRegion`.
4. Aggregates: None. 5. Services: `layout_tab(score, track, bar_range, metrics)`. 6. Events: None.
7. Public interface: Layout functions. 8. Dependencies: `mc-score`, `mc-theory`.
9. Invariants: Layout is deterministic for identical inputs. 10. Relationships: Projection of Composition. See [ADR-018](decisions/ADR-018-notation-layout-in-rust.md).

## 8. DSP (`mc-dsp`, generic)

1. Responsibilities: Allocation-free signal algorithms after initialization: YIN/MPM pitch, windowing, FFT wrapper, later onset and tempo.
2. Entities: None. 3. Value objects: `PitchEstimate { hz, clarity }`, `AnalysisConfig`.
4. Aggregates: None. 5. Services: `PitchDetector::process(&mut self, frame) -> Option<PitchEstimate>`. 6. Events: None.
7. Public interface: Stateful processors with preallocated buffers. 8. Dependencies: `rustfft` or `realfft` only.
9. Invariants: Sample rate > 0, frame size matches configuration. 10. Relationships: Used by `mc-audio` (tuner thread) and later Analysis.

## 9. Analysis (Music Intelligence, Phase 8+, Proposed)

Created only when vocal analysis begins. Responsibilities: produce `Candidate<T>` values (melody, key, chords, transcription) with provenance; never mutate Score. Details in [ai-music-architecture.md](ai-music-architecture.md).
