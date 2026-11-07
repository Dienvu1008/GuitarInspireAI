# ADR-032 Track mixer basics: volume, pan and solo (Phase 6b-3)

Status: Draft (acceptance criteria fixed before any code, 2026-11-07). Becomes Accepted when the user decides the open choices in section 5 and 6b-3 meets section 4.
Date: 2026-11-07
Related documents: [ADR-024](ADR-024-playback-rendering-conventions-and-rt-failures.md), [ADR-025](ADR-025-single-machine-evidence-accepted.md), [ADR-026](ADR-026-bundled-soundfont-trimmed-fluidr3.md), [ADR-028](ADR-028-playback-transport-port-and-session-state.md), [ADR-031](ADR-031-multi-track-tab-display.md), [audio-dsp-architecture.md](../audio-dsp-architecture.md), [project-format.md](../project-format.md), [implementation-phases.md](../../roadmap/implementation-phases.md)

## Context

Phase 6b lists "track mixer basics". The user decided on 2026-11-06: **volume, pan and solo per track**, on top of the existing mute. A per-track SoundFont program is out of scope and needs its own ADR, because it changes both playback and MIDI export.

Findings from the code (not assumptions):

- **Mute has no control in the app.**
  - The core, the FFI and `TransportController.setMuted` work, and `MuteButton` exists in `playback_feature` with its own widget test.
  - No widget of the app uses `MuteButton`, so a user cannot mute a track from the screen.
  - The Phase 4 exit report and the MVP release notes say "per-track mute: met". That holds for the core and the controller, not for the product. 6b-3 must give mute, and the new controls, a place on screen.
- **No volume, pan or controller event exists anywhere.**
  - `PerformanceEvent` is `NoteOff`, `Tempo`, `ProgramChange` and `NoteOn`, and only the program change is emitted per channel at tick 0.
  - `mc-score`, `mc-project` and the MIDI importer and exporter have no notion of volume or pan.
- **Solo was excluded on purpose** by ADR-028 item 3 (Q3: "mute is session state, no solo"). Phase 6b supersedes that; ADR-028 gets an amendment note.
- **The synthesizer supports it.** `rustysynth` 1.3.8 handles CC 7 (volume) and CC 10 (pan) through `process_midi_message`. Voices read the channel volume and pan on every render block, so a change is heard on notes that are already sounding, with no re-trigger. Defaults are volume 100 and pan centre (64), which is what the app sounds like today.
- **Channels.** Tracks map to channels in score order from `[0..=8, 10..=15]`, reused cyclically beyond 15 tracks (ADR-024 item 5). A mix value therefore belongs to a channel, and tracks 16 and up share the channel of tracks 1 and up.
- **Real-time path.**
  - The callback takes commands from a 64-slot ring, at most 8 per callback.
  - A slider drag would flood it.
  - Wait-free atomics that the callback polls are a better fit for a "last value wins" control.
  - A new synthesizer starts with default controllers (`InstallSynth`), and a rebuilt device must keep the mix.
  - FF-16 (no allocation), FF-17 (forbidden constructs in `mc-audio/src/rt`) and FF-18 (10-minute glitch run) apply.

## Decision

### 1. Mix is a value object in `mc-playback`; the rules are in Rust (GR-05)

- `Volume` is 0 to 127, default 100 (MIDI CC 7). `Pan` is -63 to 63, default 0 (CC 10 = 64 + pan). Out-of-range input is rejected with a typed error, never clamped silently.
- 100 is the default and 127 the maximum, so a track can be turned up by at most 27 points. This is what keeps the default sound identical to today's.
- `TrackMix { volume, pan }` and a pure function `assign_channels(score) -> Vec<ChannelAssignment>`, which `compile` also uses, so the application can find a track's channel without compiling.
- Dart shows and sends integers. It does not convert to CC values or decide ranges.

### 2. Mix and solo are session state, like mute (see choice 5.1)

- `TransportState` gains `mix: BTreeMap<TrackId, TrackMix>` and `soloed: BTreeSet<TrackId>`.
- New commands: `TransportCommand::SetTrackMix { track, volume, pan }` and `SetSoloed { track, soloed }`.
- New events: `AppEvent::MixChanged` and `AppEvent::SoloChanged`. They carry no revision bump, are not undoable, and are not saved (Q6).
- State is pruned to existing tracks when applied, as mute is.
- A new document starts with every track at the default mix and no solo.

### 3. Solo is an application rule over the existing mute

- The set of tracks compiled as muted is `muted`, plus every existing track that is not soloed when the soloed set is non-empty (see choice 5.3).
- A change of solo while playing recompiles and swaps the timeline exactly as a mute change does, so held notes are released (Q8). While stopped, nothing is sent to the port.
- Mute still omits the notes and keeps timing and channel (PB-04). Nothing changes in `compile`.
- A soloed track that is deleted leaves the soloed set. If the set becomes empty, everything sounds again.

### 4. Volume and pan reach the sound without recompiling

- Mix is **not** part of the timeline. A change sends the channel, volume and pan to the audio port, with no recompile and no swap.
- `AudioPort` gains `set_channel_mix(channel, volume, pan)`; its implementation writes two atomics per channel into a table that the callback polls.
- **Table:** 16 `AtomicU8` volumes and 16 `AtomicU8` pans, in `mc-audio` outside `rt/`, created with the engine and held by the control side, so it survives a device rebuild.
- **Callback:** once per callback it compares each channel with the value it last applied and calls `process_midi_message` (CC 7, CC 10) only when different. It re-applies all channels after installing a new synthesizer. It allocates nothing, locks nothing, and uses `Relaxed` loads and no new `unsafe`.
- After every `load_timeline` (play, mute or solo swap, edit swap), the session sends the mix of every existing track again. Channel assignment shifts when tracks are added or removed, and this keeps each channel's value right.
- Tracks that share a channel (16 or more tracks) share one mix: the last one set wins. This is a recorded limitation.

### 5. Controls

- A **Mixer panel** in `playback_feature`, opened from the Tools menu like the tuner and the chord finder, lists every track with name, Mute (M), Solo (S), a volume slider and a pan slider. Double-click or a Reset button returns a slider to its default.
- Exactly one place shows mute and solo (see choice 5.4).
- All controls work with the keyboard and have tooltips and semantic labels.
- The fake and the real playback service pass one shared contract.

### 6. What does not change

- No `.giproj` change (with choice 5.1 as proposed): no format version bump, no new fixture, no schema change.
- MIDI export is unchanged: it neither writes CC 7 and CC 10 nor reads them. An exported file does not carry the mix heard in the app. This is a recorded limitation.
- The playback timeline, its golden text and PB-04 are unchanged. No new dependency, no new `unsafe`.

## 4. Acceptance criteria (fixed before implementation)

Measured on the reference machine (single machine, ADR-025). Thresholds are not lowered to match a result.

| Id | Criterion | How it is checked |
|---|---|---|
| X1 | With the default mix, the rendered audio is **bit-identical** to what the engine rendered before this change, over the reference timeline | Offline render compared sample by sample against a render with no mix support |
| X2 | Volume 0 on a channel gives silence on that channel's notes while others are unchanged; a volume change is audible on notes already sounding, in the next callback | Offline render: per-channel energy; change mid-note |
| X3 | Pan hard left puts at most 1% of that channel's energy on the right output; pan centre equals the default; other channels unchanged | Offline render, energy ratio |
| X4 | A mix change takes effect within one callback of being written | Offline engine test |
| X5 | The mix survives a seek, a timeline swap, a synthesizer re-install and a device rebuild | Offline tests, plus the rebuild path of the control thread |
| X6 | After tracks are added or removed during playback, every remaining track's channel has its own mix and not a stale one | Session test with a recording fake port |
| X7 | Values out of range (volume above 127, pan outside -63 to 63) are rejected with a typed error; the state and the port are untouched | Unit and property tests |
| X8 | The solo rule holds for every combination of muted and soloed sets: the compiled muted set equals `muted` plus the non-soloed tracks when the soloed set is non-empty; a deleted track leaves both sets | Property test over random sets and track deletions |
| X9 | Changing solo or mute while playing recompiles and swaps once; changing volume or pan while playing does not recompile or swap; while stopped no command reaches the port | Session tests with the recording fake port |
| X10 | Mix and solo are not saved and not undoable; a new, opened or imported document starts at the defaults; the `.giproj` bytes are unchanged | Core test; byte comparison of a saved file |
| X11 | **FF-16:** 20 s of rendering with random mix changes every few blocks allocates nothing; **FF-17:** the real-time scan passes with no rule changed | `no_alloc` test extended; `xtask` scan |
| X12 | **FF-18:** the 10-minute run on the device reports 0 late callbacks, as in Phase 4, and the callback maximum does not grow by more than 5 µs p99 | Manual procedure, result file committed |
| X13 | The Mixer panel shows every track with M, S, volume and pan; each control sends exactly one command per change and shows the core's state; a rejected value shows the error | Widget tests with the fake, shared contract on fake and core |
| X14 | Mute and solo work from the screen in the built app | Windows integration test; the manual script |
| X15 | No new dependency, no new `unsafe`; frb bindings regenerated by codegen only (FF-12); public API snapshots change by additions only (FF-22); ADR-028 is amended | Fitness functions and CI |

## 5. Choices that need the user's decision

1. **Persistence.**
   - Proposed: **session only**, like mute. It is lost on save and reopen. This needs no format change.
   - Alternative: save volume, pan and solo in the project. This needs a minor format version bump (additive fields, 1.1), validation in the domain (GR-09), a new fixture, a schema snapshot, a rule for whether the mix is undoable, and MIDI export writing CC 7 and CC 10. It is a larger change with its own ADR.
2. **Ranges and default.** Proposed: volume 0 to 127 with default 100, pan -63 to 63 with default 0 (section 1). The alternative is a 0 to 100 percent display, with 100 percent equal to the default and a shorter maximum; the stored value stays the same.
3. **Mute and solo together.** Proposed: **mute always silences**. A track that is muted and soloed stays silent, which is the simplest rule and the one section 3 states. The alternative is that solo overrides mute for the soloed track.
4. **Where mute and solo appear.** Proposed: **only in the Mixer panel**, with the panel open by default so mute is visible. The alternative is also putting compact M and S buttons in the track list, which needs a provider so that `editor_feature` does not import `playback_feature` (FF-05).

## Alternatives

- **Bake volume and pan into the timeline as CC events.** Rejected: every slider move would recompile and swap, and swapping releases held notes (Q8). It would also change the timeline golden for every score.
- **Send mix through the command ring.** Rejected: a slider drag floods a 64-slot ring that the callback drains at 8 per callback, and the intermediate values are worthless. Atomics keep only the last value.
- **Mute by sending volume 0.** Rejected: it would change PB-04 and the existing tests, and the exporter must still export every track.
- **Mix in the score.** Rejected for now: no musical meaning in the domain model, and it would force a format change (choice 5.1).

## Trade-offs

- Session-only mix is lost when the file is closed, which users of a mixer may find surprising. The Mixer panel and the release notes say so.
- Tracks 16 and up share a channel and a mix, a limit of the 15-channel convention.
- A mix heard in the app is not in the exported MIDI file.

## Consequences

- `mc-playback`: `Volume`, `Pan`, `TrackMix`, `assign_channels`.
- `mc-app`: transport state, two commands, two events, the solo rule, `AudioPort::set_channel_mix`.
- `mc-audio`: the mix table and its polling in the callback, the control-side call, and the offline engine helper for tests.
- `mc-ffi`: two engine calls and two event DTOs.
- `core_bridge` and `playback_feature`: service methods (real and fake), contract cases, state, and the Mixer panel.
- `app`: a Tools menu entry and the panel.
- Known limitations and the release notes gain the mixer entries, and the Phase 4 and MVP "per-track mute" claims are corrected to say where the control is.
- ADR-028 item 3 is amended: solo is now in scope.

## Related fitness functions

FF-12, FF-16, FF-17, FF-18, FF-19, FF-22, FF-34.

## Reconsideration conditions

- A persisted mix (choice 5.1), a per-track SoundFont program, or more than 15 tracks that must sound independently would change sections 2 and 4.
- Audio tracks (recording, Phase 8) need their own gain and a master bus, which this design does not provide.
