# Data Architecture

Purpose: Define where each kind of data lives, who owns it, and how it is stored, cached and (later) synchronized.
Status: Accepted for MVP. SQLite and cloud are Deferred.
Related documents: [project-format.md](project-format.md), [ai-music-architecture.md](ai-music-architecture.md), [security-and-privacy.md](security-and-privacy.md), [ADR-008](decisions/ADR-008-native-project-format.md), [ADR-009](decisions/ADR-009-defer-sqlite.md), [ADR-010](decisions/ADR-010-local-first.md), [ADR-012](decisions/ADR-012-interchange-formats-as-adapters.md).

## 1. Data classes

| Data | Canonical store | Owner | MVP | Regenerable |
|---|---|---|---|---|
| Score, tracks, patterns | Native project package | `mc-project` | Yes | No |
| Project audio assets (recordings) | Inside package (stored, uncompressed) or referenced external file (Proposed for large audio) | `mc-project` | No | No |
| Undo history | Memory only | `mc-app` session | Yes | n/a |
| Autosave and recovery snapshots | App data `recovery/<project-id>/` | `mc-app` + adapter | Yes | No (until saved) |
| Preferences, recent files, workspace layout | App data JSON files | Flutter (`platform` package) for UI; Rust for core settings | Yes | Partly |
| SoundFonts | App data `soundfonts/` or bundled | `mc-audio` | Yes | Re-download |
| Project catalogue and library index | SQLite (Deferred) | future `mc-library` adapter | No | Yes (rebuild by scan) |
| DSP and AI analysis cache | SQLite plus blob files (Deferred) | future analysis adapter | No | Yes |
| Accepted analysis results | Native project package `analysis/` with provenance | `mc-project` | No | No |
| Cloud copies | Remote (Deferred) | sync adapter | No | n/a |

App data location: platform standard (`%APPDATA%\GuitarInspireAI` on Windows [Assumption], equivalents on macOS and Linux via a directories crate or Flutter path_provider). Resolved in the `platform` adapters only.

## 2. Why a native format (and not an interchange format)

Interchange formats cannot represent the full domain without loss:

| Format | Limitation as canonical model |
|---|---|
| MIDI | No string or fret, no spelling, no notation semantics, no diagnostics |
| MusicXML | Verbose, notation-centric, ambiguous for TAB technique details, large schema |
| Guitar Pro | Proprietary, versioned by a third party, partial public knowledge |

Therefore the native format holds the canonical model, and all others are adapters with reports ([ADR-012](decisions/ADR-012-interchange-formats-as-adapters.md)).

## 3. Interchange adapters

| Format | Direction | Phase | Lossiness notes |
|---|---|---|---|
| MIDI (SMF type 0 and 1) | Import and export | 4 | Import needs string assignment (deterministic lowest-fret or playability heuristic) and quantization; reported |
| MusicXML | Export (6b), import later | 6b | Export of TAB technique subset |
| Guitar Pro GP3 to GP5, GPX, GP7+ | Import | 6d | Unsupported effects reported; S10 |
| WAV | Import and export (rendered audio) | 4 (export), 8 (import) | |
| FLAC | Import (8), export later | 8 | Decoder license check [To verify] |
| MP3 | Import | 8 | Decoder via `symphonia` (MPL-2.0 [To verify]) evaluation in S7; patents expired [To verify] |
| PDF | Export | 6c | Depends on engraving |

Every import produces `ImportReport { dropped, approximated, warnings }`, shown to the user. Every export produces `ExportReport`. Imports go through `ScoreBuilder` (GR-09).

## 4. SQLite decision

Not in MVP. Preferences and recent files are small JSON files. Reasons to defer: no query needs, migration overhead, and a native dependency. Introduce (Rust side, `rusqlite` with bundled SQLite) when one of these arrives: library index over many projects, analysis cache with lookups by content hash, or practice history. SQLite is never the canonical store for score content ([ADR-009](decisions/ADR-009-defer-sqlite.md)).

## 5. Caches

- Projections (layout) cache: memory, keyed by (track, bar, revision, metrics hash).
- Compiled PerformanceTimeline: memory, rebuilt on change.
- Future analysis cache: keyed by (source content hash, algorithm id, version, parameters hash). Deletable at any time.

## 6. Synchronization (Deferred, Phase 11)

The local-first design keeps the project package as the unit of sync. A first version can be whole-file sync with conflict copies. Fine-grained merge or collaboration requires a CRDT or OT model, which is a reconsideration trigger for the Score aggregate ([score-model-and-undo.md](score-model-and-undo.md)).
