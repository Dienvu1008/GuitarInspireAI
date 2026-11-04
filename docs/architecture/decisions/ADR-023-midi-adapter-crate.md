# ADR-023 MIDI interchange in its own adapter crate `mc-midi`
Status: Accepted (2026-10-31, user decision Q1 at Phase 4 entry)
Date: 2026-10-31
Related documents: [ADR-003](ADR-003-context-map-and-crate-dag.md), [ADR-011](ADR-011-rational-time-model.md), [ADR-012](ADR-012-interchange-formats-as-adapters.md), [ADR-014](ADR-014-wasm-compatible-domain-crates.md), [ADR-015](ADR-015-dependency-license-policy.md), [bounded-contexts.md](../bounded-contexts.md), [dependency-rules.md](../dependency-rules.md), [audio-dsp-architecture.md](../audio-dsp-architecture.md)

## Context
Phase 4 adds Standard MIDI File import and export. Earlier documents put the `midly` parser "in an adapter module" of `mc-playback`. But `mc-playback` is a domain crate: it has no external dependencies, must compile for wasm32 (FF-04), follows the domain purity lints (FF-03) and may depend only on `mc-theory` and `mc-score`. MIDI import also needs `ScoreBuilder` (GR-09) and string assignment, which is a Guitar-context rule (`mc-guitar`). A module inside a domain crate cannot satisfy these rules.

## Decision
1. A new adapter crate `mc-midi` (layer `adapter`, phase 4) owns SMF reading and writing. Internal dependencies: `mc-theory`, `mc-score`, `mc-guitar`, `mc-playback`. External dependency: `midly =0.5.3` (Unlicense, on the ADR-015 allow-list), wrapper-banned to `mc-midi` in `deny.toml` when it is added.
2. Users of `mc-midi` are the other adapters (`mc-ffi`, `mc-cli`). `mc-app` and the domain crates do not depend on it.
3. Export reads a snapshot through `mc-playback`: SMF type 1, 960 ticks per quarter note, repeats expanded in playback order, every track exported including muted ones (mute is session state), tempo factor not applied. It produces an `ExportReport` (ADR-012).
4. Import builds the score only through `ScoreBuilder` (GR-09, IM-01) and returns an `ImportReport`. Quantization grid: 1/16 plus triplet 1/12 of a whole note; every changed onset or duration is reported (ADR-011). One TAB track per MIDI track/channel with notes, standard tuning; channel 10 (percussion) is skipped and reported; tempo and time signatures are imported. Parser limits: 16 MB file, 64 tracks, 1 000 000 events.
5. The string assignment heuristic (lowest fret position close to the previous note within the span limit) is a Guitar-context rule and lives in `mc-guitar` as a pure function; `mc-midi` only calls it (GR-05: adapters do not own domain decisions). Quantization is part of converting MIDI ticks to rationals and stays in the adapter, with its grid as an explicit parameter.
6. Round trip criterion (Phase 4 acceptance): after export and re-import, pitches are equal, and onsets and durations are equal after quantization to the import grid, compared on the score with repeats expanded.
7. FF-15 fuzzes the `mc-midi` reader (cargo-fuzz, weekly Linux job); crashes become regression fixtures.

## Alternatives
- `midly` inside `mc-playback`: breaks FF-03, FF-04 and the domain dependency rules.
- A module in `mc-app`: `mc-app` has no external dependencies and is not an adapter; it would mix interchange parsing into the application layer.
- A module in `mc-cli` or `mc-ffi`: the app and the CLI both need MIDI, so the code would be duplicated or one adapter would depend on the other.

## Trade-offs
One more crate (crate-count review at M5) in exchange for keeping the domain crates pure and wasm-compatible.

## Consequences
`architecture/allowed-deps.toml` declares `mc-midi` and lets `mc-ffi` and `mc-cli` depend on it; `dependency-rules.md` section 1 gets a `midi` row; `bounded-contexts.md` and `architecture-overview.md` point to `mc-midi` instead of "an adapter module in mc-playback".

## Related fitness functions
FF-01 (dependency matrix), FF-02 (wrapper ban), FF-11 (round trip fixtures), FF-15 (fuzzing), IM-01.

## Reconsideration conditions
More interchange formats (MusicXML, Guitar Pro) share enough code that one `mc-interchange` crate with modules is simpler than one crate per format.
