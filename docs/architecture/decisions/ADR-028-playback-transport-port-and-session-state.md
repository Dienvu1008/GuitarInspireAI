# ADR-028 Playback transport: an application-owned `AudioPort`, session transport state, and swap-on-edit
Status: Accepted (2026-10-31, user decision at slice 4d; decisions Q2, Q3, Q6, Q7, Q8)
Date: 2026-10-31
Related documents: [ADR-003](ADR-003-context-map-and-crate-dag.md), [ADR-005](ADR-005-dart-rust-interop-frb.md), [ADR-007](ADR-007-audio-thread-isolation.md), [ADR-011](ADR-011-rational-time-model.md), [ADR-024](ADR-024-playback-rendering-conventions-and-rt-failures.md), [audio-dsp-architecture.md](../audio-dsp-architecture.md), [dependency-rules.md](../dependency-rules.md), [architecture-overview.md](../architecture-overview.md)

## Context

Slice 4d connects the pure timeline compiler (`mc-playback`, slice 4a) and the real-time engine (`mc-audio`, slice 4b) to the application and the UI. Something must decide *what* to compile, *when* to recompile, where playback starts and how user settings (mute, tempo factor) map to the engine. Those are application decisions (GR-05: adapters and Dart own no musical or application rules), but producing sound is an adapter responsibility.

Two concrete questions had to be answered:

1. Where does the port abstraction between the application and the audio engine live, and which crate depends on which?
2. Are mute and the user tempo factor document content (saved, undoable) or session state?

`mc-audio` already depends on `mc-app` (it reads a snapshot to compile; see the allowed dependency matrix `architecture/allowed-deps.toml` and [dependency-rules.md](../dependency-rules.md) section 1). If `mc-app` also depended on `mc-audio` to call it, that would be a crate cycle, which Cargo rejects and GR-07 forbids.

## Decision

1. **The port is defined in `mc-app`, implemented in the adapter (dependency inversion).** `mc-app` owns a trait `AudioPort` (in `mc_app::transport`) that takes a fully compiled `mc_playback::PerformanceTimeline` plus simple transport calls (`play`, `stop`, `seek`, `set_tempo_factor`) and exposes engine state and position. The application depends on the abstraction; the audio adapter depends on the application and implements the trait. The concrete implementation that wraps `mc_audio::AudioEngine` is written in `mc-ffi` (the composition root, ADR-005), not in `mc-app` and not in `mc-audio`, so no new crate edge is introduced and no cycle is possible. `mc-audio` keeps its existing one-way dependency on `mc-app`.

2. **`mc-app` depends on `mc-playback`.** This edge is already in the allowed-deps matrix (phase 4, decision Q2). `mc-app` compiles the timeline itself from `DocumentSession::snapshot()`, applying the muted set, and hands the finished timeline to the port. It never touches `cpal`, `rustysynth`, SoundFonts or the real-time thread.

3. **Mute and the user tempo factor are session state, not document content (Q3, Q6).** They live in a `TransportState` held by `DocumentSession`, next to the score but outside it. Changing them:
   - does not create a command, does not bump the revision, and is not part of undo/redo (they are not score edits);
   - is not written to the project file (`mc-project` never sees them);
   - is reported to the UI through new session events `MuteChanged`, `TempoFactorChanged` and `PlaybackStateChanged`, carried like other `AppEvent`s but with revision unchanged.
   There is no solo in the MVP (Q3); mute is a per-track flag. The tempo factor range and step are fixed by `mc_playback::TempoFactor` (0.25x..=2.0x in 0.05 steps, Q6); it is a render scalar, never part of the timeline, so changing it never recompiles.

4. **The muted set omits note events only (PB-04).** Compilation drops a muted track's notes but keeps all bar timing and still assigns the track a channel, so timing is identical whether or not a track is muted. A muted track that is later removed by an ordinary edit is pruned from the set at the next compile, so a stale id never reaches the compiler.

5. **Edits during playback recompile and swap (Q8).** When the transport is playing and the score changes (or the muted set changes), `mc-app` recompiles and calls `AudioPort::load_timeline`. The engine swaps the timeline and releases all held notes (all-notes-off), per [audio-dsp-architecture.md](../audio-dsp-architecture.md) section 3 and [ADR-024](ADR-024-playback-rendering-conventions-and-rt-failures.md). Playback is not restarted; the cursor is relocated by the engine. When stopped, an edit does nothing to the engine.

6. **Play starts at the bar containing the cursor (Q7).** `TransportCommand::Play { from_master_bar }` names the master bar the user's cursor is in (a UI concept passed as an index). `mc-app` maps it to a start tick through `PerformanceTimeline::first_played_start_of_master_bar`; a bar past the end of the expanded playback order is rejected with `BarOutOfRange` and nothing starts. `from_master_bar = None` resumes from the engine's current position. Click-to-seek is `TransportCommand::Seek { tick }`. Loop, metronome and count-in are deferred to Phase 5 (Q7).

7. **Positions crossing FFI are `i64` (ADR-005 amendment 2).** The 4d position stream and every tick/id DTO use types that map to Dart `int`, never `u64`/`BigInt`. The engine handle and the 60 Hz position stream live in `mc-ffi` as `EngineHandle` and a typed `StreamSink`, with Rust owning the stream lifecycle (ADR-005 amendment 1).

## Alternatives

- **Define `AudioPort` in `mc-audio` (slice 4b).** Rejected during 4b: it would either pull application concerns into the adapter or force `mc-app` to depend on `mc-audio`, creating a cycle with the existing `mc-audio -> mc-app` edge.
- **Let `mc-ffi` own all transport logic and call `mc-audio` directly, with no port in `mc-app`.** Rejected: the start-bar mapping, muted-set pruning and swap-on-edit are application decisions (GR-05); keeping them testable without audio hardware requires them in `mc-app` behind a port that a fake can implement. The 4d transport tests use exactly such a fake.
- **Mute and tempo factor as document content (saved, undoable).** Rejected (Q3, Q6): they are per-session listening preferences, not part of the musical document; saving them would churn the file and surprise undo.
- **Apply mute by zeroing velocity or muting a channel at render time instead of omitting events.** Rejected: omitting events at compile time keeps the real-time path simpler and matches PB-04 (timing unchanged, notes absent), and it makes the muted timeline a normal deterministic artifact (PB-09).

## Trade-offs

Dependency inversion adds one trait and a small amount of wiring in the composition root, in exchange for no crate cycle, an application layer that is unit-testable without audio hardware, and an adapter that owns no application rules. Session state outside the score means the UI must subscribe to three more `AppEvent` variants, but keeps the document format and undo unchanged.

## Consequences

- `mc-app` gains a `transport` module (`AudioPort`, `TransportCommand`, `TransportState`, `PlaybackState`, `TransportError`, `PositionUpdate`), three new `AppEvent` variants, and `DocumentSession::transport`, `refresh_playback_after_edit` and `transport_state`. `architecture/public-api/mc-app.txt` grows (additions only); this is a protected path, so the commit carries the `Architecture-Change: ADR-028` trailer.
- `mc-ffi` gains `EngineHandle` (wrapping `mc_audio::AudioEngine` as the `AudioPort` implementation), a 60 Hz `i64` position stream, a `PlaybackStateChanged` DTO plus `EventKindDto` variants for the three new events, and (when `mc-midi` is wired) MIDI import/export entry points. The generated bridge files are regenerated, never hand-edited (FF-12).
- `mc-ffi` declares `mc-audio` (and later `mc-midi`) as dependencies; both edges are already in the allowed-deps matrix.
- No change to `mc-audio`'s dependencies, to `ADR-007`, or to the real-time rules.

## Related fitness functions

FF-01 (dependency matrix: `mc-app -> mc-playback`, `mc-ffi -> mc-audio`), FF-07 (platform code stays in adapters), FF-12 (generated bindings in sync), FF-16/FF-17 (real-time rules, unchanged), FF-22 (public API diff of `mc-app`).

## Reconsideration conditions

A second audio backend or an out-of-process engine is added (then the port may need to move to its own crate); solo, loop or metronome are implemented and need more transport state; or a non-Dart binding needs the same transport API.
