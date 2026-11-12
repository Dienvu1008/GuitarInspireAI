# ADR-036 Guitar Pro import and export adapter `mc-guitarpro` (Phase 6d)

Status: Accepted (2026-11-11); results in section 8 (2026-11-11). On 2026-11-11 the user confirmed the eight choices of section 7 and the reading of G1 in clarification 1 (section 9).
- The acceptance criteria in section 6 were fixed before any code.
- On 2026-11-11 the user asked for this ADR to be drafted and the work continued ("soạn luôn và tiếp tục"). The choices in section 7 are therefore the agent's recommendations, applied so the work can go on. The user may change any of them at review; a change is recorded in section 7 and the code follows it.
Date: 2026-11-11
Related documents: [ADR-012](ADR-012-interchange-formats-as-adapters.md), [ADR-015](ADR-015-dependency-license-policy.md), [ADR-020](ADR-020-invariants-vs-diagnostics.md), [ADR-023](ADR-023-midi-adapter-crate.md), [ADR-025](ADR-025-single-machine-evidence-accepted.md), [ADR-033](ADR-033-musicxml-export-adapter.md), [ADR-035](ADR-035-playing-techniques-in-the-score.md), [S10 spike](../../../spikes/s10-guitar-pro/README.md), [technical-spikes.md](../../research/technical-spikes.md), [risk-register.md](../../roadmap/risk-register.md) (R-14), [data-architecture.md](../data-architecture.md), [dependency-rules.md](../dependency-rules.md)

## Context

Phase 6d is "Guitar Pro import" with spike S10 as its entry gate (implementation-phases, 6d). The user chose 6d before 6c and set these conditions (2026-11-11):

- **Own implementation, clean-room.** The `guitarpro` crate (MIT label, about 32 000 lines) is not used. Its code may derive from copyleft readers: PyGuitarPro is LGPL-3.0 and a port of alphaTab (MPL-2.0), itself a port of TuxGuitar (LGPL-2.1).
- **Scope:** GP5 read and write, then GP3 and GP4 read and write. Export was added by the user ("thêm cả xuất nếu bạn làm được"); the roadmap line says import only. GP7/8 (`.gp`) and GPX (GP6) are deferred until lawful sample files exist.
- **No Guitar Pro on the machine,** only TuxGuitar 2.0.1, which writes GP3 to GP5 (GP5 as version 5.00 only).

S10 results, recorded in the [spike README](../../../spikes/s10-guitar-pro/README.md), section 6:

- **Fixtures.** 54 files, f01 to f18 each as `.gp3`, `.gp4` and `.gp5`. They were written by TuxGuitar's own writers, called through its public API by a generator, with the user's consent. Licence CC0-1.0.
- **Reader.** A spike reader for GP3, GP4 and GP5 5.00 reads all 54 to the last byte. Its 103 voice lines equal what TuxGuitar's own readers read.
- **Writer.** A spike GP5 writer round-trips all 18 GP5 files. TuxGuitar reads our files with a view identical to the originals (79 of 79 lines).
- **Robustness.** 144 000 mutated inputs gave 0 panics.
- **Format facts.** Byte analysis corrected the public format description in eight places (spike section 6.2) and found version rules (section 6.3):
  - Time signature parts are written only on a change.
  - GP3 and GP4 store repeats; GP5 stores plays.
  - The alternate-ending byte is a bitmask of passes.
  - The pick stroke is 2 for down and 1 for up.
  - GP3 and GP4 have one voice.
- **Open points** (spike section 7):
  - No file written by Guitar Pro itself.
  - No GP5 5.10 file.
  - The meaning of the brush direction bytes.
  - The text encoding of Guitar Pro's own files.
  - The old chord layout, and the GP3 new chord layout.

Findings from the code (not assumptions):

- **No `ScoreBuilder` type exists.** The validating path is `Score::from_parts(ScoreParts)` in `mc-score`, which returns every violation (ADR-023, clarifications). "Through `ScoreBuilder`" (GR-09) means through `from_parts`.
- **The score holds:**
  - Per master bar: time signature, key (fifths, major or minor), pickup, repeat start, repeat end as total plays 2..=16, alternate endings as passes 1..=16, and a marker.
  - A tempo map.
  - Tracks with a name and a fretted instrument (tuning, fret count, capo); bars with voices of beats.
  - Beats: a value with base, dots and a tuplet (actual 2..=13, normal 1..=12), an optional stroke (down, up).
  - Notes: string and fret relative to the capo (H-10: capo + fret ≤ fret count), a tie on the destination note (H-06: the previous beat of the same voice has an equal note), and the effects let ring, dead, accent, palm mute and string-percussive (ADR-035).
  - There are no song metadata, lyrics, chord diagrams, beat texts, dynamics or other techniques.
- **File bytes are read and written by the Dart composition root** (`FileDialogs`). `mc-ffi` takes and returns `Vec<u8>`; only `mc-cli` uses `std::fs` (ADR-023).
- **Fitness functions enumerate no crate names.** FF-01 reads `architecture/allowed-deps.toml`, so a new crate needs a row there and edges from its users.

## Decision

### 1. A new adapter crate `mc-guitarpro`

- Adapter layer, phase 6. Internal dependencies: `mc-theory` and `mc-score`. **No external dependency.** Dev dependency: `proptest`.
- Called by `mc-ffi` and `mc-cli` only, like `mc-midi` and `mc-musicxml`. `allowed-deps.toml` and the dependency-rules matrix gain the row and the two edges.
- **One crate per format,** not a shared `mc-interchange`. The ADR-023 reconsideration condition is checked and not met: Guitar Pro shares no code with SMF (a performance format) or MusicXML (text). Only the report pattern is shared, and that is a convention.
- **Production code is written anew from the spike's findings, not copied from the spike** (technical-spikes rule: spike code is throwaway). The fixtures move into the crate's tests with their provenance README.

### 2. Clean-room and trademark rules (R-14)

- Format knowledge comes only from the sources listed in spike section 3, and from byte analysis of our own fixtures:
  - the PyGuitarPro documentation's format page: facts only, no code or text;
  - TuxGuitar as a black box through its public API, with no method bodies.
- No copyleft source is read while writing the crate. Guitar Pro is never decompiled. A new source is listed in the spike README (or in this ADR) before it is used.
- Test files are only files we generated (CC0-1.0) or files with a licence that allows it. No song files from the internet.
- "Guitar Pro" is named only to describe compatibility. The product shows no logo and states that it is not affiliated with Arobas Music.
- This is not legal advice. A legal review is advised before a commercial release (R-14 stays open until then).

### 3. Import

- `import_gp(bytes) -> Result<(Score, GpImportReport), GpImportError>`.
- **Versions.** The importer reads `FICHIER GUITAR PRO v3.00`, `v4.00`, `v4.06`, `v5.00` and the other v3.x and v4.x strings. `v5.10` and every other version are refused with a typed error naming the version (choice 7.8). The file type is recognised by its content, not its extension.
- **Limits** (checked before allocating):
  - file 16 MiB;
  - 10 000 bars;
  - 127 tracks;
  - 7 strings;
  - 1 000 beats per voice per bar;
  - 100 bend points;
  - strings up to 1 MiB.
- **Build.** The score is built only through `Score::from_parts` (GR-09). A violation that remains after the repairs below is `GpImportError::Invalid`, which means a bug in the importer.
- **Repairs.** Every repair is counted in the report, never done silently:
  - A tie without an equal preceding note loses its tie flag.
  - The fret of a tied note is the fret of the note it continues.
  - A note above the fret count, or on a string the track lacks, is dropped.
  - A repeat structure that breaks H-12 has the repeat marks of that section removed (choice 7.4).
  - A tempo outside the score's range is clamped.
  - A fret count outside 1..=36 (C-01) is clamped into that range; a capo above the fret count is clamped to it.
- **Version rules** from spike section 6.3:
  - The time signature is carried forward, starting from 4/4.
  - The key is carried forward from the song key.
  - The GP3 and GP4 repeat count is incremented by 1.
  - The endings bitmask becomes passes.
  - GP3 and GP4 have one voice.
  - An empty voice 2 is dropped, and a status-0 (empty) beat is skipped.

Mapping (spike section 6.5):

| Guitar Pro | Score | Import |
|---|---|---|
| Guitar track: name, strings 1..=7, tuning (MIDI numbers), fret count, capo | `TrackData`, `FrettedInstrument`; open pitches spelled with sharps (`SpelledPitch::from_semitone_sharp`, mc-theory) | Kept |
| Percussion track | None (no non-fretted instrument) | Skipped, counted (choice 7.5) |
| Time signature, key (fifths and minor flag), repeat start, repeat end, marker name | `MasterBar` | Kept |
| Alternate endings bitmask | `alternate_endings` (passes, increasing) | Kept |
| Marker colour, double bar, triplet feel | None | Counted |
| Song tempo, tempo change in a beat's mix table | `TempoChange` at the beat's offset in the bar | Kept; a change at an offset outside the bar's nominal length moves to the start of the next bar, counted |
| Duration (whole to 64th), dot, tuplet "enters" | `NoteValue`; the tuplet's normal count from the format's fixed table (3:2, 5:4, 6:4, 7:4, 9:8, 10:8, 11:8, 12:8, 13:8) | Kept; another enters value is counted and the beat is written without a tuplet |
| Rest; empty beat (status 0) | Beat without notes; skipped | Kept |
| Two voices (GP5) | `BarContent.voices` | Kept |
| Note string and fret, tie, dead, accent, palm mute, let ring | `Note`, `NoteEffects` | Kept |
| Heavy accent | `accent` | Approximated, counted |
| Ghost note, staccato, velocity (dynamics) | None | Counted (velocity only when it is not the default) |
| Pick stroke (2 down, 1 up) | `Stroke` | Kept |
| Brush (strum with a speed) | `Stroke` | Approximated: direction as TuxGuitar reads it, speed dropped, counted (choice 7.1) |
| Chord diagram, beat text, lyrics | None | Counted |
| Bend, slide, hammer-on and pull-off, harmonics, vibrato, wide vibrato, grace note, tremolo picking, trill, tap, slap, pop, tremolo bar, fade in, rasgueado, fingering, time-independent duration | None | Counted per kind |
| Mix table (instrument, volume, pan, chorus, reverb, phaser, tremolo) | None | Counted |
| Song information (title, artist and so on), notice | None in the score | Counted, not kept (choice 7.2) |
| RSE, page setup, MIDI channels, track colour and display data | None | Skipped exactly |

The report holds:
- the version string;
- the counts of tracks, bars and notes imported;
- `notes_dropped`;
- a list of issues, each with a kind, a count and the first bar where it occurs, in a deterministic order.

The invariant is: **notes in the file = notes imported + notes dropped**. A note on a skipped track counts as dropped.

### 4. Export

- `export_gp(score, version) -> Result<(Vec<u8>, GpExportReport), GpExportError>`, for `version` in GP5 (5.00), GP4 (4.06) and GP3 (3.00). GP5 is the default in the app.
- **It reads the written score, not the performance.** Repeats stay repeats, and every track is exported whatever is muted (as ADR-033).
- **Sections the score does not hold** are written with fixed, documented values: page setup, MIDI channels (one channel per track; channel 10 is never used), RSE and track display data.
- **Text is written as UTF-8** (choice 7.3). Names are cut to the format's buffer, and every cut is counted.

Approximations, each counted in the report:
- a beat with two or more dots is written with one dot;
- a value shorter than a 64th is written as a 64th;
- a tuplet whose ratio is not in the fixed table is written with its "enters" only;
- voices beyond the format's count (2 in GP5, 1 in GP3 and GP4) are dropped, and their notes are counted;
- string-percussive, which has no Guitar Pro note equivalent;
- in GP3, accent, palm mute and pick stroke (choice 7.6);
- tempo values that are not whole BPM, which are rounded;
- endings above pass 8 (the bitmask has 8 bits);
- a pickup bar, which is written as an underfull bar;
- a mid-bar tempo change that falls between beats of voice 1, which is moved to the next beat.

Errors cover only what cannot be written:
- a track with more than 7 strings;
- more than 127 tracks or 10 000 bars, the import limits, so that every exported file can be read back.

The invariant is: **notes written + notes counted as dropped = `Score::note_count`.**

### 5. App and CLI

- The File menu gains:
  - **"Import Guitar Pro..."** (`.gp3`, `.gp4`, `.gp5`). It asks before replacing the document (ADR-030) and shows the report summary.
  - **"Export Guitar Pro..."**, with a save dialog offering `.gp5`, `.gp4` and `.gp3`. The chosen extension selects the version.
- A failure shows "Import failed: rule: message" or "Export failed: rule: message", as for MIDI.
- `GuitarProService` has a real and a fake implementation with one shared contract. Dart decides nothing about the content: the version comes from the extension, by a Rust function.
- `mc-cli` gains `gp:<file>` as a source, and `export-gp <project|source> <out.gp5|.gp4|.gp3>`.
- `mc-ffi` gains `import_guitar_pro(bytes)` and `export_guitar_pro(document, extension)`, with report DTOs named apart from the MIDI ones. The bindings are regenerated by codegen only (FF-12).
- FF-15: a fuzz target `import_gp` in `fuzz/`, run by the weekly fuzz job.

## 6. Acceptance criteria (fixed before implementation)

Measured on the reference machine (single machine, ADR-025). Thresholds are not lowered to match a result.

| Id | Criterion | How it is checked |
|---|---|---|
| G1 | All 54 S10 fixtures import without error. For each, the tracks, strings, tuning, capo, bars, time signatures, keys, tempo map, repeats, endings, markers and every note (string, fret, value, tie, effects, stroke, voice) equal the generator's content, except the differences the spike explains (TuxGuitar's writer losses, GP3/GP4 one voice) | Tests per fixture, embedded with `include_bytes!` |
| G2 | Every Guitar Pro feature not kept is counted in the import report by kind, with the first bar. **Notes in the file = imported + dropped**, on every fixture | Tests, including f12 (unsupported effects), f14 (percussion), f15 to f18 |
| G3 | The version rules of spike section 6.3 each have a test: time signature carried forward, repeat count per version, endings bitmask, pick stroke values, one voice in GP3/GP4, heavy accent approximated | Tests named after the rule |
| G4 | Every repair in section 3 has a test with a hand-built file, and the repaired score passes `Score::from_parts` | Tests |
| G5 | Never a panic or unbounded allocation: arbitrary bytes, and mutated fixtures (at least 100 000 mutations over the 54 fixtures), give either a score or a typed error. Each limit in section 3 has a test. A `cargo-fuzz` target `import_gp` is in the weekly fuzz job | Property tests; limits test; fuzz target and workflow |
| G6 | `v5.10`, GPX, `.gp` (a ZIP), other versions and empty or short input give a typed error naming the cause | Tests |
| G7 | Export to GP5, GP4 and GP3, then our import, keeps every fact the target version holds: tracks, tuning, capo, fret count, bars, time signatures, keys, repeats, endings, markers, tempo map, and every note (string, fret, value, tie, effects, stroke, voice). This holds over `feature_rich`, `techniques`, `reference(TYPICAL)`, and 200 random valid scores (seeded, as in ADR-033) | Property test and fixture tests |
| G8 | Nothing is lost silently on export: **written + dropped = `note_count`**; every approximation of section 4 is counted; the random scores of G7 cover each counted case at least once | Tests, with a coverage test |
| G9 | **TuxGuitar 2.0.1 opens every export** of `feature_rich`, `techniques` and `reference(TYPICAL)` in GP5, GP4 and GP3, through its own readers. Its view equals our re-import, voice line by voice line, with tracks, strings and capo. This is checked through the S10 black-box program `ReadSummary` | Local procedure; result file `benchmarks/tuxguitar-guitarpro.md` committed |
| G10 | Deterministic: the same score gives byte-identical files; golden GP5 files for `feature_rich` and one small score are committed and re-read in tests | Golden tests |
| G11 | Release mode: import and export of `reference(TYPICAL)` each ≤ 100 ms; of `reference(LARGE)` each ≤ 1 s | Benchmark on the reference machine |
| G12 | No music rule in the adapter beyond decoding the format. Spelling comes from `mc-theory` and the score rules from `mc-score` (GR-05); Dart contains no format or music rule | Code review, FF-05 |
| G13 | The two menu items work; cancel and failure paths are handled; fake and core pass one contract. The Windows integration test imports a fixture through the core and exports it again | Widget tests, contract, integration test |
| G14 | FF-01 passes with the new row and edges only; cargo-deny passes with no new external dependency; no `unsafe` (FF-34); bindings by codegen only (FF-12); `.giproj` unchanged | Fitness functions and CI |
| G15 | Known limitations and release notes state the scope and the evidence: GP3 to GP5 5.00; checked only against files written by TuxGuitar 2.0.1 and against TuxGuitar's reader; files written by Guitar Pro not checked; 5.10, GPX and GP7/8 unsupported; effects reported; UTF-8 text. The non-affiliation statement is in the docs and the About text | Docs review |

## 7. Choices (agent recommendations, 2026-11-11; the user may change them)

1. **Brush: imported as a stroke in the direction TuxGuitar reads, speed dropped, counted.**
   - The alternative is to drop the brush and only report it. That is safe, but it loses the most common strum mark.
   - **Evidence gap, recorded in advance:** without a file written by Guitar Pro, the direction is confirmed only against TuxGuitar.
   - Export writes our stroke as a pick stroke (2 down, 1 up), which TuxGuitar reads unambiguously.
2. **Song information is not kept.** The score has no metadata, and adding it is a model and project-format change (GR-08) outside 6d. It is counted, so the user sees that a title was not kept. A later ADR can add song metadata.
3. **Text: read as UTF-8 when valid, else as Windows-1252; written as UTF-8.**
   - TuxGuitar writes UTF-8 by default.
   - Guitar Pro is believed to use a Windows code page, so non-ASCII names written by us may show garbled there [evidence gap].
   - Windows-1252 decoding is a 32-entry table in the adapter.
4. **A repeat structure that breaks H-12** (nesting, a count above 16, an ending outside a section) **is repaired, not refused.**
   - A count above 16 is clamped to 16.
   - Otherwise the marks of the offending section are removed. The notes stay, and the repair is counted.
   - Refusing a whole file for a repeat mark would block most real files with an odd structure.
5. **Percussion tracks are skipped and counted.** A file with only percussion tracks is refused with `NoGuitarTracks`.
6. **GP3 export does not write accent, palm mute or pick stroke.**
   - TuxGuitar's GP3 writer drops them, so no fixture shows a GP3 layout for them.
   - Writing them on the strength of the description alone would be unverified. They are counted.
7. **Chord diagrams in an unverified layout** (the GP3 new layout) make the import fail with a typed error naming the cause. Their length is not known, so skipping them would misread the rest of the file. The old layout is read as described and its test is a hand-built file, marked as unverified against a real file.
8. **GP5 5.10 is refused** until a lawful 5.10 sample exists. A reader for it written from the description alone could not be checked.

## 8. Results (2026-11-11)

Implemented in:
- `9e47f04` (6d-1): the crate;
- `7010a1c` (6d-2): CLI, FFI and the Dart service;
- `e4edfd7` (6d-3): the app;
- `3b4dcb9` (6d-4): the fuzz target;
- the 6d-5 commit: docs and the TuxGuitar procedure.

Single machine (ADR-025). The TuxGuitar procedure and the timing numbers are in [benchmarks/tuxguitar-guitarpro.md](../../../benchmarks/tuxguitar-guitarpro.md).

| Id | Result | Evidence |
|---|---|---|
| G1 | **Met, read with choice 7.5 (see clarification 1).** 51 of the 54 fixtures import. The three `f14-percussion` files hold only a percussion track and are refused with `NoGuitarTracks`, as choice 7.5 specifies. The import of all 54 was checked by hand against the generator, then frozen as a reviewed golden. The main facts of each fixture are also asserted directly. | `g1_every_fixture_imports_as_reviewed`, `g1_fixture_facts`, `tests/golden/import.txt` |
| G2 | Met. Notes in the file = imported + dropped on every fixture. Each feature of f12, f11 and f15 to f18 is reported with its count and first bar. A percussion track next to a guitar track is skipped and its 2 notes counted. | `g2_*` |
| G3 | Met. One test per version rule. | `g3_*` |
| G4 | Met. 11 unit tests on hand-built files: ties, notes outside the track, notes in rests, H-12 repairs, repeat counts, tempo, fret count and capo, time signature and key, tuplets, and both chord layouts. The old chord layout is checked only on a hand-built block. | `src/import.rs` tests |
| G5 | **Met except the first fuzz run.** 102 600 mutated fixtures and 2 000 arbitrary inputs: 0 panics, 0 invalid scores. Each limit has a test. The `import_gp` target compiles and is in the weekly job, but it **has not run yet**: cargo-fuzz runs on the Linux weekly job (Mondays, or by hand from the Actions tab). | `g5_*`, `fuzz/fuzz_targets/import_gp.rs` |
| G6 | Met. Each unsupported or short input gives a typed error that names the cause: 5.10, 2.21, `CLIPBOARD`, ZIP, GPX, empty, short, truncated. | `g6_unsupported_files_are_named` |
| G7 | Met. These round-trip through GP5, GP4 and GP3, and every fact the version holds is unchanged: `feature_rich`, `techniques`, `reference(TYPICAL)`, the 51 imported fixtures, and 200 random scores. | `g7_*` |
| G8 | Met. Written + dropped = `note_count` every time. The random scores reach all 15 counted export cases. Three deliberate breakages are each caught by the tests: let ring dropped on export; the GP3/GP4 repeat rule removed; the brush direction swapped. | `g8_random_scores_cover_every_counted_case` |
| G9 | **Met, with one TuxGuitar limit.** TuxGuitar opens all 9 files, and 1 847 of its 1 849 lines equal our re-import. The 2 others are a note with both palm mute and let ring, which TuxGuitar's note model cannot hold (verified on its API: each setter clears the other). | benchmarks file |
| G10 | Met | `g10_output_is_deterministic_and_matches_the_goldens` |
| G11 | Met. Median times, release mode: TYPICAL export 1.2 ms and import 3.4 ms; LARGE export 26.1 ms and import 73.1 ms. | `g11_timing`, benchmarks file |
| G12 | Met. Open-string spelling comes from `SpelledPitch::from_semitone_sharp` (mc-theory). The H-12 repair asks `Score::from_parts` which bars break the rule; the rule is not copied. The tuplet table only decodes the format, which stores the "enters" count alone. Dart passes the file extension and the core decides the version. | code review |
| G13 | Met. Widget tests cover the menu items, cancel and failure paths (8). Fake and core pass one contract (9 tests). The Windows integration test imports f07 from the File menu, exports `.gp4` byte-equal to the core's output, and re-imports it with the same tracks and notes; 7 of 7 integration tests pass. | `guitar_pro_menu_test.dart`, contract, `acceptance_test.dart` |
| G14 | Met. FF-01 passes with the new row and edges. cargo-deny passes, with no new external dependency. FF-34 passes. FF-12: a second codegen run is identical. `.giproj` is unchanged. | fitness functions, cargo-deny |
| G15 | **Met, with clarification 6.** Known limitations, release notes, data-architecture and the crate documentation state the scope, the evidence and the non-affiliation statement. | docs |

### Clarifications from implementation

1. **G1 and choice 7.5 contradict each other** for a file with only percussion tracks: G1 says every fixture imports, while 7.5 says such a file is refused. The implementation follows 7.5, and G1 is read as "imports as specified". The user may choose instead to import such a file as a score with no notes.
2. **Where an export puts a tempo change** (refines section 4, "moved to the next beat"):
   1. The beat that starts exactly at the change's position, in any track. This counts as not moved.
   2. Else the next beat of the first track that starts inside the bar's nominal length. A beat beyond it, in an overfull bar, would be read back as part of the next bar.
   3. Else the first beat of the bar.
   4. Else, in GP5, an empty beat at the start of the bar. GP3 and GP4 instead use a beat of another track, or drop the change and count it, because TuxGuitar reads an empty beat as a rest there (G9).
   
   Two changes that end up on one beat keep the later one, and the replacement is counted. The importer never reads two changes at one position.
3. **Marker colour** is skipped, not counted (section 3 said "counted"). Every marker has a colour, so the count would always equal the number of markers and tell the user nothing.
4. **Dynamics** are counted only when a note's velocity differs from TuxGuitar's default byte, 6. Export writes 6 on every note, as TuxGuitar does.
5. **Notes within a beat** are written and compared in string order. Guitar Pro stores them that way, so their order in our score is not a fact the format holds.
6. **G15's "About text"**: the app has no About screen. The statement is in the docs and the crate documentation, and it goes into the About text when one is added.
7. **The FFI also exposes `guitar_pro_export_extension_supported`**. Dart uses it to ask the core, not to decide itself.
8. **An empty track name** is imported as "Track N", as the MIDI importer does.
9. **An empty (status 0) beat in a GP3 or GP4 file** is skipped on import, as the format description says. TuxGuitar reads one as a rest. No fixture contains one, because TuxGuitar writes an empty bar of those versions with no beats.

## 9. User decisions after the results (2026-11-11)

1. **A file with only percussion tracks stays refused** with `NoGuitarTracks` (choice 7.5). G1 is read as "imports as specified" (clarification 1). Importing such a file as a score with no notes was offered and not chosen.
2. **The eight choices of section 7 are confirmed as written.** They are no longer only the agent's recommendations.
3. **Phase 6d is merged into `main` now.** The first weekly fuzz run (G5) is not waited for and stays a condition of acceptance ([phase-6d-exit-report.md](../../roadmap/phase-6d-exit-report.md)).

G5 after the merge (2026-11-11): a local cargo-fuzz run of `import_gp` (300 s, 1 801 744 inputs, seeded with the 54 fixtures) found no crash. The CI step had not been reached: the weekly job's MIDI step, which runs first, had failed since 2026-11-09 (a midly panic, now fixed; exit report section 7). G5 stays open until a green weekly run on Linux.

## Alternatives

- **The `guitarpro` crate (MIT).** Rejected by the user: its provenance may derive from copyleft readers, and auditing 32 000 lines is not possible.
- **A port or binding of TuxGuitar, alphaTab or PyGuitarPro.** Rejected: their copyleft licences (LGPL-2.1, MPL-2.0, LGPL-3.0) are outside ADR-015, and a port would not be clean-room.
- **Import through MusicXML** (convert with an external tool). Rejected: it needs a tool the user may not have, and loses string and fret.
- **Refusing files with any unsupported feature.** Rejected: almost every real file has a bend or a slide. Reporting them follows ADR-012.
- **One `mc-interchange` crate.** Rejected; see section 1.

## Trade-offs

- Compatibility is proven only against TuxGuitar's writer and reader. Guitar Pro's own files may differ in places the fixtures cannot show. The importer refuses rather than guesses where the layout is unknown, so a real file may fail where a guess might have worked.
- Many Guitar Pro techniques are counted and dropped until the score model has them (ADR-035 added four). The import report makes the loss visible.
- UTF-8 output suits TuxGuitar and may not suit Guitar Pro.

## Consequences

- New crate `mc-guitarpro`, a new `allowed-deps.toml` row and edges from `mc-ffi` and `mc-cli`, a dependency-rules row and column.
- `mc-ffi`: two functions and report DTOs; bindings regenerated.
- `mc-cli`: the `gp:` source and `export-gp`.
- Dart: the service (real and fake) and its contract, two menu items, and two `FileDialogs` methods.
- `fuzz/`: the `import_gp` target; the weekly workflow gains a step (a protected path, trailer `Architecture-Change: ADR-036`).
- Docs:
  - data-architecture section 4 (Guitar Pro: import and export, 6d);
  - known limitations and release notes;
  - the S10 result in technical-spikes;
  - R-14 updated.

## Related fitness functions

FF-01, FF-05, FF-11 (fixtures embedded), FF-12, FF-15, FF-19, FF-34, cargo-deny.

## Reconsideration conditions

- A file written by Guitar Pro itself, or a user report, that the importer refuses or misreads. Choices 7.1, 7.3 and 7.7 are revisited first.
- A lawful GP5 5.10, GPX or GP7/8 sample (choice 7.8; deferred formats).
- The score model gains song metadata, lyrics, chord diagrams or more techniques (choice 7.2 and the counted rows of section 3).
- A legal review that changes the clean-room or trademark rules.
