# ADR-033 MusicXML export adapter (Phase 6b-4)

Status: Accepted (2026-11-07). The acceptance criteria in section 4 were fixed before any code. The user decided the six choices of section 5 on 2026-11-07, with additions recorded there.
Date: 2026-11-07
Related documents: [ADR-012](ADR-012-interchange-formats-as-adapters.md), [ADR-015](ADR-015-dependency-license-policy.md), [ADR-019](ADR-019-fretted-instrument-shared-kernel.md), [ADR-020](ADR-020-invariants-vs-diagnostics.md), [ADR-023](ADR-023-midi-adapter-crate.md), [ADR-025](ADR-025-single-machine-evidence-accepted.md), [data-architecture.md](../data-architecture.md), [dependency-rules.md](../dependency-rules.md), [implementation-phases.md](../../roadmap/implementation-phases.md)

## Context

Phase 6b ends with "MusicXML export with ExportReport" (implementation-phases, 6b). data-architecture section 4 lists MusicXML as "Export (6b), import later" for "the TAB technique subset". The user chose MuseScore as the reader to check against. MuseScore 4.7.5 is installed on the reference machine, and headless conversion works.

Findings from the code and the specification (not assumptions):

- **Precedent: `mc-midi`** (ADR-023).
  - It is an adapter crate called only by `mc-ffi` and `mc-cli`.
  - It has `export_smf(score, timeline) -> (bytes, ExportReport)` with counts, and a typed `ExportError`.
  - The FFI returns bytes and a report DTO; the Dart side writes the file through `FileDialogs`.
  - ADR-023 asks whether more formats should share one `mc-interchange` crate.
- **What the score holds.**
  - Master bars: time signature, key (`fifths`, mode), pickup, repeat start and end with a play count, alternate endings, marker.
  - Tempo changes, which can fall mid-bar.
  - Tracks: name, fretted instrument (tuning, fret count, capo), bars with one or more voices of beats.
  - Beats: a value with base, dots and an optional per-beat tuplet ratio. There is no grouping of tuplets.
  - Notes: string and fret, a tie flag on the destination note (H-06, also across bar lines), let ring, dead.
  - There is no clef, no lyrics and no instrument program in the score.
- **Sounding pitch** is `mc_theory::fretted::pitch_at(tuning, capo, position)`, a semitone. The only spelling function is `SpelledPitch::from_semitone_sharp`. MusicXML requires `<pitch>` (step, alter, octave) on every pitched note, even on a TAB staff, so a spelling rule is needed. It is a music rule (GR-05).
- **Bars may be underfull or overfull** (D-01 is a diagnostic, not an invariant). ADR-020 already says strict exporters "report, and may split or truncate with a report". MusicXML measures carry their own durations.
- **Notation in MusicXML** (W3C MusicXML 4.0):
  - Tablature uses a `TAB` clef with `<staff-details>`: lines, `<staff-tuning>` per string and `<capo>`.
  - Each note carries `<technical><string>/<fret>`.
  - A dead note is `<notehead>x</notehead>`.
  - Let ring has its own form: a single `<tied type="let-ring"/>` on the note, not a start and stop pair. Per the tied-type data type in the 4.0 reference, it indicates the instrument should be undamped. The schema at tag `v4.0` lists `let-ring` in that type. The user pointed this out; the draft had missed it.
  - MuseScore has a known, still open issue: it does not export let ring correctly (reported in 2019). A correct export may therefore not show or round-trip in MuseScore.
  - The specification and schemas are under the W3C Community Final Specification Agreement. The exporter is written from the specification (clean-room); the schema file is not copied into the repository without a licence check (R-16).
- **No XML crate** is in `Cargo.lock`. Writing MusicXML needs only escaping and nesting.
- **Validation tooling.** There is no `xmllint` or Java on the reference machine. The GitHub Ubuntu runners used by the `cargo-deny` job can install `libxml2-utils`.

## Decision (proposed)

### 1. A new adapter crate `mc-musicxml`

- Adapter layer, phase 6. Internal dependencies are `mc-theory` and `mc-score`; there are no external dependencies.
- Dev dependencies are `proptest` and `roxmltree =0.21.2`, used to parse the output in tests only.
  - `roxmltree` is MIT OR Apache-2.0, current on crates.io on 2026-11-07.
  - Its one dependency, `memchr` (MIT OR Unlicense), is already in `Cargo.lock`.
  - Both licences are on the ADR-015 allow-list. `allowed-deps.toml` lists it under `dev_external` with the reason "test-only XML parsing".
- It is called by `mc-ffi` and `mc-cli`, the same as `mc-midi`. `allowed-deps.toml` and the dependency-rules matrix gain the row and the two edges.
- There is one crate per format, not a shared `mc-interchange`. MIDI and MusicXML share no code: one exports a performance and the other exports the written score.
- The XML writer is our own: about 100 lines with escaping and indentation. No XML library is used in the product code.

### 2. The written score, not the performance

- The export reads the `Score` snapshot, not the playback timeline.
- Repeats stay repeats: repeat barlines, `<ending>` with the pass numbers. Bars are not expanded.
- One `<measure>` per master bar, with the same numbering in every part (H-02).
- Every track is exported, whatever is muted, soloed or hidden. The mix and the tempo factor are not exported (ADR-032, Q6).

### 3. Mapping

| Score | MusicXML 4.0 (partwise) |
|---|---|
| Track | `<score-part>` with the track name, and one `<part>` |
| Instrument | TAB clef; `<staff-details>` with `<staff-lines>` = string count, `<staff-tuning>` per string (open pitches without the capo), `<capo>` |
| Master bar | `<measure>`; `<attributes>` with `<divisions>`, `<key>`, `<time>` and the clef at the start and when they change |
| Pickup | `implicit="yes"`, numbered 0 |
| Repeat start and end | `<barline>` with `<repeat direction="forward">` and `<repeat direction="backward" times="n">` |
| Alternate endings | `<ending number="1, 2" type="start/stop">` |
| Marker | `<direction>` with `<rehearsal>` |
| Tempo change | `<direction>` with `<metronome>` and `<sound tempo>`, at its offset in the bar |
| Beat with notes | `<note>` elements: `<chord/>` from the second note on; `<pitch>` = sounding pitch, spelled (choice 5.3); `<duration>`; `<type>`; `<dot/>`; `<time-modification>` for tuplets; `<technical><string>/<fret>` |
| Rest | `<note><rest/>` |
| Voices | `<voice>` numbers, `<backup>` between voices |
| Tie (flag on the destination) | `<tie type="stop">` on it and `<tie type="start">` on the source note, also across bar lines; `<tied>` for display |
| Dead note | `<notehead>x</notehead>` |
| Let ring | a single `<tied type="let-ring"/>` in `<notations>` (choice 5.5) |
| Tuplet brackets | `<tuplet type="start/stop">` around consecutive beats with the same ratio that fill a whole group. An incomplete group keeps its `<time-modification>` without a bracket and is counted in the report. |

`<divisions>` per measure is the least common multiple needed for every beat of every voice in that measure, so all durations are exact integers.

### 4. Report and errors

- `export_musicxml(score) -> Result<(String, MusicXmlReport), MusicXmlError>`.
- The report counts: parts, measures, notes written, and each approximation:
  - underfull and overfull bars (choice 5.4)
  - underfull voices padded with a rest, and overfull measures kept whole (choice 5.4)
  - tuplets without a bracket
  - characters removed from names and markers because XML 1.0 forbids them
- Nothing is dropped without being counted.
- The error type covers only what cannot be written, for example a duration that overflows the divisions.

### 5. App

- The File menu gains "Export MusicXML...". It uses a save dialog for `.musicxml` (choice 5.2) and shows the report summary as the MIDI export does. Errors show as "Export failed: rule: message".
- `MusicXmlService` (or a method next to `MidiService`) has a real and a fake implementation with one shared contract.
- Dart decides nothing about the content.
- `mc-cli export-musicxml <project> <out>` is added for the CI schema check and the MuseScore procedure.

## 4. Acceptance criteria (fixed before implementation)

Measured on the reference machine (single machine, ADR-025). Thresholds are not lowered to match a result.

| Id | Criterion | How it is checked |
|---|---|---|
| E1 | Every export is well-formed XML and **validates against the MusicXML 4.0 partwise XSD**. Inputs: `feature_rich`, `reference(SMALL)`, `reference(TYPICAL)`, a new empty document, and a MIDI-imported score | CI job with `xmllint --schema` on `mc-cli` output (choice 5.6) |
| E2 | Structure: one part per track in score order with its name; one measure per master bar in every part; key, time and clef at the start and at each change; the pickup is implicit; repeats, play counts, endings, markers and tempo changes (including mid-bar ones at their offset) are present | Parse the output in tests (`roxmltree`) and compare with the score |
| E3 | Every note round-trips its facts: string and fret equal; the semitone of `<pitch>` equals `pitch_at`; each voice's durations sum to the voice length; dots, tuplet ratio, rests, chords and voices are as in the score; every tie has a matching start and stop, also across bars; dead notes have the x notehead; every let ring note has exactly one `<tied type="let-ring"/>` and no other note has one | Property test over random edit sequences (the existing `mc-app` command strategy) plus `feature_rich` |
| E4 | TAB staff: TAB clef, line count = string count, one `<staff-tuning>` per string matching the tuning (drop D included), `<capo>` when the capo is not 0 | Tests on `feature_rich` (drop D, capo 2) |
| E5 | Nothing is lost silently: notes in the file plus notes counted as dropped equals `Score::note_count`; every approximation in section 4 is counted | Tests, including on the overfull and underfull bars of `feature_rich` |
| E6 | Deterministic: the same score gives byte-identical output; golden files for `feature_rich` and one small score are reviewed and committed | Golden tests |
| E7 | Names and markers with `< > & " '`, non-ASCII text and control characters give valid XML; forbidden characters are removed and counted | Property test with arbitrary strings, then parsed |
| E8 | **MuseScore 4.7.5 opens every E1 input headless without error.** Its MIDI export of `feature_rich` and `reference(TYPICAL)` has the same notes as our own `export_smf` (pitch, onset and length, with repeats expanded), except notes in bars counted by the report as approximated | Local procedure, result file committed |
| E9 | MuseScore's own MusicXML re-export of our file keeps the string and fret of every note. Let ring after the MuseScore round trip is recorded as observed. Because of the known MuseScore issue, a loss there is an evidence gap, not a pass, and it does not change E3 | Local procedure, compare by parsing |
| E10 | A PNG rendering of `feature_rich` by MuseScore is committed and reviewed: TAB shown, repeats and endings shown, no collisions that hide notes. This is a check that it reads, not a layout-quality judgement | Local procedure, image in the result file |
| E11 | Export time in release mode: `reference(TYPICAL)` ≤ 200 ms and `reference(LARGE)` ≤ 2 s | Benchmark on the reference machine |
| E12 | The spelling rule lives in `mc-theory` with its own tests for the three cases (no accidentals in the key, sharp keys, flat keys) (GR-05); the exporter and Dart contain no music rule | Code review, FF-05, tests |
| E13 | The menu item writes a file and shows the report; a failure shows the error; fake and core pass one contract; the Windows integration test exports and re-reads the file through the core | Widget test, contract, integration test |
| E14 | FF-01 passes with the new rows only; cargo-deny passes; no `unsafe` (FF-34); bindings by codegen only (FF-12); snapshot additions only (FF-22, `mc-theory`); `.giproj` unchanged | Fitness functions and CI |
| E15 | Known limitations and release notes state the scope: export only; TAB subset; `.musicxml` only (no `.mxl`); checked with MuseScore 4.7.5 on one machine; Finale, Dorico and Guitar Pro not checked; layout quality not judged; let ring in MuseScore limited by its known issue | Docs review |

## 5. User decisions (2026-11-07)

1. **Staves: TAB only.** No standard-notation staff in Phase 6. It belongs to 6c, and adding it now would bring decisions on clef, displayed key and beaming that are outside 6b-4.
2. **File type: uncompressed `.musicxml` only.** No `.mxl`, so no `zip` dependency just to compress. The goal of 6b-4 is correct content, not packaging. `.mxl` is recorded for a later roadmap slot.
3. **Spelling: sharps when the key has no accidentals or has sharps, flats when it has flats.** "Always sharps" would give enharmonic spellings that are hard to read. The rule lives in `mc-theory`, tested for the three cases.
4. **Underfull and overfull bars: written as they are, reported.**
   - An underfull voice is padded with a rest, and the padding is counted.
   - An overfull measure keeps all its notes. The report names it as overfull, so the user knows no data was lost.
   - No note is ever cut. Truncation, if ever asked for, becomes a separate option and is never the default.
5. **Let ring: `<tied type="let-ring"/>`**, as in the specification.
   - `<other-technical>let ring</other-technical>` is a fallback only if the MuseScore test shows the specified form does not round-trip at all, and then it needs a recorded decision.
   - Leaving let ring out and only reporting it is the last resort, only if tests prove a correct export impossible. The specification makes it possible, so it is not done.
   - **Evidence gap, recorded in advance:** let ring is exported according to the specification. A MuseScore round trip may not confirm it because of MuseScore's known let-ring export issue. That is a MuseScore limitation; E3 and E9 are not changed to match it.
6. **Schema check: a new CI job on Ubuntu.** It installs `libxml2-utils`, downloads the XSD files, checks their SHA-256 and runs `xmllint --schema` on the `mc-cli` outputs.
   - **The XSD is fetched at the `v4.0` tag of `w3c/musicxml`, commit `799e2defb2ece0ae7bafe08dcbcac25b2c631d53`, path `schema/`, and checked in CI. It is not stored in this repository, because of the licence:** the W3C Community Final Specification Agreement has not been reviewed, and storing the file is an unnecessary legal risk.
   - SHA-256 at that commit:

     | File | Size | SHA-256 |
     |---|---|---|
     | `musicxml.xsd` | 379 505 | `BFE37ED25A9EC00E6F2591D53DF260B84EFE12AED209BA3AC0A76F9287665A99` |
     | `xml.xsd` | 5 695 | `616A3077DF5CFC954AC74A75ABE9697B95EEF7A85DBE09367D995A483E840EB5` |
     | `xlink.xsd` | 2 280 | `6E601F8EEB41618B50E4C7F944DFF754E57EA43B602755470DDA24C9C2F6DF92` |

   - `musicxml.xsd` imports `xml.xsd` and `xlink.xsd` from `http://www.musicxml.org/xsd/`. The job maps those two locations to the verified local copies (an XML catalog), so nothing unverified is fetched during validation.
   - The job changes the CI workflow, a protected path; its commit carries `Architecture-Change: ADR-033`.
   - The latest `gh-pages` schema (commit `5666459`, 2026-10-15) has changes after 4.0. It is not used.
## Alternatives

- **Export through the playback timeline**, as MIDI does. Rejected: the timeline has repeats expanded and no written durations, ties, voices or strings.
- **An XML library (`quick-xml`) in the product code.** Rejected for now: writing needs only escaping. Reconsider when MusicXML import is built.
- **One `mc-interchange` crate** for MIDI and MusicXML. Rejected: no shared code yet (ADR-023 reconsideration condition not met).
- **Spelling in the exporter.** Rejected: spelling is a music rule (GR-05).

## Trade-offs

- TAB-only output depends on the reader to make a notation staff. MuseScore can; others may not.
- Writing bars as they are can give "irregular measure" markings in strict readers. Truncating would lose notes.
- Evidence comes from one reader on one machine. Other readers may differ.

## Consequences

- New crate `mc-musicxml`, new `allowed-deps.toml` row and edges from `mc-ffi` and `mc-cli`, and a dependency-rules row.
- `mc-theory`: a key-aware spelling function and its public API snapshot.
- `mc-ffi`: `export_musicxml` and a report DTO; bindings regenerated.
- `mc-cli`: `export-musicxml`.
- Dart: service methods (real and fake), contract cases, a menu item, a `FileDialogs` method.
- CI: the schema job (choice 5.6).
- Roadmap: `.mxl` export and optional truncation of overfull bars recorded as possible later items.
- Docs: known limitations, release notes, a MuseScore result file in `benchmarks/`.

## Related fitness functions

FF-01, FF-05, FF-12, FF-19, FF-22, FF-34, cargo-deny.

## Reconsideration conditions

- MusicXML import (it needs a parser and `ScoreBuilder` validation, GR-09).
- Standard notation in the app (Phase 6c) would make two staves the natural choice (choice 5.1).
- A second reader or a user report showing incompatibility.
