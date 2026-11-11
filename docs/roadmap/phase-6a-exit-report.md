# Phase 6a Exit Report (pattern engine and playing techniques)

Purpose: Evidence for Phase 6a (the pattern engine of Phase 6, with the playing techniques it needs in the score), the evidence gaps accepted at closure, and what comes next.
Status: Final. Phase 6a accepted by the user on 2026-11-11, with the evidence gaps below as conditions of acceptance. The user will return to them later. **Read "Known evidence gaps" before relying on this report.**
Related documents: [implementation-phases.md](implementation-phases.md), [milestones.md](milestones.md), [mvp-known-limitations.md](mvp-known-limitations.md), [mvp-release-notes.md](mvp-release-notes.md), [phase-6b-exit-report.md](phase-6b-exit-report.md), [ADR-034](../architecture/decisions/ADR-034-pattern-engine-ir-first.md), [ADR-035](../architecture/decisions/ADR-035-playing-techniques-in-the-score.md), [pattern-corpus-proposed.md](../architecture/pattern-corpus-proposed.md), [pattern-text-syntax.md](../architecture/pattern-text-syntax.md).

## 1. Outcome

Phase 6a was designed in two ADRs, accepted on 2026-11-09 with their acceptance criteria fixed before any code. Thresholds were never lowered to match a result. The user reviewed every slice before the next one started.

| Slice | What the user gets | ADR | Result |
|---|---|---|---|
| 6a-1 | Accent, palm mute, string-percussive and stroke direction in the score: edited in the inspector, played, shown on TAB, exported to MIDI and MusicXML; format 1.1 | ADR-035 | TQ-1 to TQ-11 met, except TQ-8 (gap a) |
| 6a-2 | The Pattern IR, its validation and user gestures, in the core | ADR-034 | PE-2 met; PE-1 and PE-9 completed in 6a-3 and 6a-5 |
| 6a-3 | Binding a pattern to a chord voicing, placing it over bars, `ApplyPattern` with the overwrite safeguard and an explicit fit; Tools > Apply pattern... | ADR-034 | PE-3 to PE-9 and PE-15 met (PE-15 with gap c) |
| 6a-4 | The pattern text syntax (parser and canonical printer) and the user-confirmed corpus as goldens | ADR-034 | PE-10 and PE-11 met |
| 6a-5 | The pattern library saved with the project (format 1.2), undoable library commands, Tools > Pattern editor... (grid and gesture palette), and the apply dialog on the stored library | ADR-034 | PE-1, PE-12 and PE-13 met; PE-14 met except dragging (gap b) |

Details, numbers and per-criterion evidence are in the ADRs: ADR-035 "Evidence per criterion", and ADR-034 sections 14 to 22 (results and the user's decisions per slice; the 6a-5 criteria table is in section 22). This report does not repeat them.

## 2. Slices and commits

All on `phase-6a-pattern-engine`, 19 commits after `main` at `c789862`. Every commit that touches a protected path carries its `Architecture-Change` trailer (FF-19 ok).

- **Design** (ADR-034, ADR-035, the corpus): `8e3c8ee`, `a20dbe3`, `fcf936c`.
- **6a-1** (ADR-035): `1290325`, `6c894f2` (MusicXML interop investigation), `4f1a31d` (the P.M. annotation decision, section 10).
- **6a-2**: `0fac527`, `77dc8f1` (review, section 15).
- **6a-3**: `f380740`, `edee5fb`, `8955530`, `15b1a56`, then the review fixes `0e702f8` and `7feb4cd` (section 20).
- **6a-4**: `4af8245`.
- **6a-5**: `47bb66b` (storage, format 1.2), `855a0e6` (core commands), `b1a1295` (bridge), `2bff890` (editor and palette).

## Known evidence gaps

These are conditions of acceptance (user decision 2026-11-11), to be resolved later. All evidence comes from one machine (ADR-025, R-21 open).

- **(a) MuseScore loses three techniques, ADR-035 TQ-8.**
  - MuseScore 4.7.5 opens our MusicXML. It keeps up-strokes and down-strokes on notes and the `x` notehead.
  - After MuseScore's own re-export, it has lost palm mute, string-percussive (read as a dead note) and the stroke on a rest. It keeps one accent per chord, so a partly accented chord cannot be told apart from a fully accented one.
  - MuseScore cannot round-trip its own palm-mute line either. Our encoding follows MusicXML 4.0 and is not changed to suit MuseScore (ADR-035 section 10). Record: [benchmarks/musescore-musicxml.md](../../benchmarks/musescore-musicxml.md).
  - To close: re-run TQ-8 when a MuseScore release reads `<play><mute>palm</mute></play>`, `<other-technical>` or `<other-direction smufl>`, or check other readers.
- **(b) No dragging in the grid editor, ADR-034 PE-14 (6a-5d, interpretation 1).**
  - ADR-034 section 9 says "drag sets its duration". The editor moves and resizes a step one cell at a time, with keys (Alt+arrows, `]` and `[`) or palette buttons, so both work by mouse and by keyboard, but there is no drag.
  - To close: add dragging in the editor. The core needs no change: a drag ends in the same `Move` and `Resize` commands.
- **(c) PE-15 was measured once, on one machine.**
  - Binding, placing and applying a 4-bar pattern to 64 bars: p99 1.52 ms on the typical score, against a 20 ms budget, release build, one run on 2026-11-10 (ADR-034 section 18).
  - The margin is large, but there is no repeat run and no second machine (R-21).
- **(d) The check with an older reader is manual, ADR-034 PE-13 and ADR-035 TQ-3.**
  - The `mc-cli` built from an older commit (`2fe101f` for format 1.0, `1290325` for format 1.1) was run by hand on the newer fixtures: it reports the unknown content, refuses to save over the file and allows Save As. The results are recorded in the ADRs.
  - No CI job repeats it, so a later change to the loaders is not checked against older readers automatically.
- **(e) The editor's one-action-at-a-time queue is covered only by the Windows integration test.**
  - The fake document service completes commands at once, so widget tests cannot show two key presses racing. The real-core integration test found the race (E-01 on the second of two quick keys) and now covers it (6a-5d).
  - A widget-level test needs a slow fake.

Smaller items recorded, not gaps in the criteria:
- The text syntax (6a-4) exists only in the core; the app has no way to type or load a pattern as text.
- The pattern library belongs to one project; a library shared between projects is out of scope (ADR-034 section 8).
- One doc comment in `crates/mc-ffi/src/api/types.rs` ("A created id is in OutcomeDto") lost its code formatting in an edit. Fixing it means regenerating the bindings, so it waits for the next change to that file.

Carried over and unchanged by 6a: the five gaps of Phase 6b ([phase-6b-exit-report.md](phase-6b-exit-report.md), (a) to (e)). They were not re-measured in 6a. The possible M3 flake (6b gap e) did not occur in the three local integration runs of 6a-5d: the five earlier acceptance tests passed in each, and the new 6a-5 test failed once, on the race of gap (e), before the fix. The tuner has not been measured on a real guitar, and ADR-029 stays Draft.

## 3. Defects found and corrected in 6a

- **Dead, palm-muted and string-percussive notes cut by `RepeatAndCut` to a length that needs a tie were played twice.** The review described them as already refused; they were not. They are now refused (`ShortNoteNeedsTie`), with a test that reproduced the defect first (6a-3 review, ADR-034 section 20).
- **A confirmed overwrite could apply to content that changed after the preview.** The confirmation now carries the preview's revision, and the core refuses a stale one (E-01) (section 20).
- **`DirtyChanged` was never sent by commands, undo or redo** (a Phase 1 defect). The session compared the dirty state after the change on both sides. It now takes it before. The Dart UI reads the dirty state directly, so nothing visible changed (6a-5b).
- **The parser split a multi-byte character in one error span.** It was found by the property test "the parser never panics" (6a-4).
- **The grid could make a triple-dotted triplet** (5/16 is exactly one). The grid makes no dotted triplets now (6a-5b).
- **The editor sent two quick commands with the same revision**, and the core refused the second (E-01). It was found by the real-core integration test, and fixed with a queue (6a-5d, gap e).
- **The first MusicXML encoding missed the native palm-mute playback value**; it is now written (6a-1, TQ-7).

## 4. Fitness functions and governance

- New crate `mc-pattern` (domain, phase 6), as declared in `allowed-deps.toml` since Phase 0. The planned edges `app -> pattern` (6a-3) and `project -> pattern` (6a-5) were added, both already in the dependency-rules matrix. FF-01: 14 crates.
- Project format: 1.1 (ADR-035) and 1.2 (ADR-034, `patterns/library.json`). The 1.1 and 1.2 fixtures and schema snapshots were appended; released fixtures and snapshots are untouched (FF-11, FF-32).
- No new external dependency, no new runtime dependency outside the allow-list, and no new `unsafe` (FF-34: 15 crates).
- Public API snapshots, lines added / removed since `c789862` (FF-22): `mc-pattern` 1295/0 (new), `mc-project` 542/1, `mc-app` 244/0, `mc-score` 28/0, `mc-notation` 6/0, `mc-playback` 4/0. The one removed line is `mc_project::schemas()`, which now returns three schemas (6a-5a).
- Bindings were regenerated by codegen only, twice per change, with identical hashes (FF-12; 17 generated files after 6a-5c).
- Mutation checks were run on every slice and are reported in the ADRs. Each surviving mutant was either killed by a new test or shown equivalent.
- No fitness function, lint or rule was weakened.

## 5. CI status

Every slice commit passed GitHub Actions: governance, cargo-deny, bridge and musicxml-schema. The merge commit is checked again after it is pushed.

## 6. Closure

Decisions (user, 2026-11-11):
- The interpretations of 6a-5 (ADR-034 section 22) are accepted.
- Phase 6a is accepted, with gaps (a) to (e) on the record as conditions of acceptance.
- `phase-6a-pattern-engine` is merged into `main` with `--no-ff`. The merge commit carries the trailers ADR-034 and ADR-035.

Next: the order of 6c (standard notation and PDF, entry gate S11) and 6d (Guitar Pro import, entry gate S10) is the user's choice. The items ADR-034 section 10 keeps open (figures, tap-to-create, a stroke without attack, more lanes, a chord track, pickups, muted-string or body percussion, strum spread) each need their own ADR.
