# S11: standard notation above TAB, and PDF (spike, plan)

Purpose: Entry gate of Phase 6c ([technical-spikes.md](../../docs/research/technical-spikes.md) S11). Show, before any production code, that our own engine in `mc-notation` (ADR-018) can engrave guitar standard notation above the TAB from the same score and the same bar plan. Also measure what it costs, and show that a PDF can be written from the same layout model. The answers feed the Phase 6c ADRs with their acceptance criteria fixed first.
Status: Plan approved by the user on 2026-11-11, with the additions of sections 4 and 5 (N8, and the designs for written pitch, accidental state and the staff model). The criteria are fixed before the spike runs. The spike ends with evidence for a Go, Revise or No-Go decision (section 5); the user reviews it before any implementation slice.
Related: [ADR-018](../../docs/architecture/decisions/ADR-018-notation-layout-in-rust.md), [ADR-031](../../docs/architecture/decisions/ADR-031-multi-track-tab-display.md), [ADR-033](../../docs/architecture/decisions/ADR-033-musicxml-export-adapter.md), [ADR-037](../../docs/architecture/decisions/ADR-037-bravura-smufl-font.md) (Proposed), risk R-13.

## 1. Decisions taken before the spike (user, 2026-11-11)

1. **Engraving engine: our own, in `mc-notation`, following ADR-018.**
   - Verovio is not a runtime dependency.
   - Rust turns the score into an immutable layout model, and Flutter renders it.
   - The PDF backend renders the same layout model and computes no layout of its own.
2. **Music font: Bravura, the SMuFL reference font,** bundled unmodified, with a license audit, the license text, attribution, version and checksum.
   - Done in [ADR-037](../../docs/architecture/decisions/ADR-037-bravura-smufl-font.md) and ADR-015 amendment 1, both accepted by the user on 2026-11-11.
   - The SIL OFL 1.1 is accepted as a controlled exception for named fonts only, not added to the allow-list.
3. **Scope:**
   - Standard notation is shown **above the TAB in the app**, not only in PDF export.
   - Both staves come from one semantic score.
   - The first version covers:
     - the guitar treble clef;
     - notes, rests and durations;
     - accidentals;
     - key and time signatures;
     - barlines;
     - stems and basic beams.
   - Not in Phase 6c: a piano grand staff, an orchestral score, or advanced engraving.
4. **Reference: MuseScore 4.7.5**, as installed.
   - Used for MusicXML interoperability, visual layout and regression.
   - Rendering need not be pixel-identical.
   - MuseScore is not the authority on semantics it does not keep: the ADR-035 TQ-8 evidence gaps stay.

## 2. Facts found before the spike (code and sources, not assumptions)

**What exists today**
- `mc-notation` lays out TAB only.
  - A bar plan is shared by every stave of a system: columns are the distinct onsets of all shown tracks, so bar lines and beats line up across staves (ADR-031).
  - The TAB already has rhythm stems, flat beams, rest glyphs and dots, placed relative to the TAB staff.
  - There is no clef other than TAB, and no notehead, accidental, key signature, ledger line or flag.
  - Units are logical pixels derived from the TAB font (`LayoutMetrics`); there is no staff-space unit.
- The display list crosses the bridge as typed arrays (lines, hit regions) plus a list of glyph objects. `SystemPainter` (Flutter) maps each glyph kind to a symbol. Rests are drawn as vectors, without a music font.

**What notation needs that the core lacks**
- **Written pitch.** Guitar is written one octave above its sounding pitch, in treble clef with an 8 below. `pitch_at` gives the sounding semitone. Nothing in the code handles written pitch.
- **Accidentals.** `KeySignature::spell` is the simple ADR-033 rule (sharps unless the key has flats). Notation also needs accidentals relative to the key signature, and accidentals carried through the bar.

**Constraints from ADR-031**
- Its own reconsideration condition says "Standard notation (6c) adds staves of a different kind and density, which would revisit the stave model".
- Its budgets (P1 to P4) and the "fast machine state" caveat for P2 and P3 apply. Doubling the glyphs per track is the main risk.
- Hit regions carry no stave kind.

**PDF**
- Candidate crates, all MIT OR Apache-2.0 or MIT, on the ADR-015 allow-list (crates.io, 2026-11-11): `pdf-writer` 0.15.0, `subsetter` 0.2.6, `printpdf` 0.12.9.
- An own minimal writer is the other option, as with the XML writer of ADR-033.
- OFL FAQ 1.12: embedding Bravura in a PDF, in full or as a subset, is allowed.

**Font**
- Bravura 1.482 ships `Bravura.json`, the SMuFL metadata: glyph bounding boxes, anchors such as stem attachment points, and engraving defaults (staff line, stem, beam and ledger thicknesses, in staff spaces).
- The layout can therefore be computed in staff spaces from the metadata, without measuring glyphs in Dart.

## 3. Method (throwaway code in this folder, own Cargo workspace; nothing merged)

1. **Inputs.**
   - `feature_rich`, `techniques` and `reference(SMALL)` from `mc_score::fixtures`.
   - Six small hand-built scores:
     - a C major scale with ledger lines from low E;
     - one-bar phrases in G, F, B-flat and E major with chromatic notes;
     - the open C chord `x32010`;
     - eighths and sixteenths in 4/4, and eighths in 6/8;
     - a triplet and a dotted rhythm;
     - every rest value.
2. **Engine prototype.** It reads the font metrics from `Bravura.json` (a local copy outside the repository until ADR-037 is accepted), in staff spaces, and lays out a treble-8vb staff above the TAB.
   - **Bar plan:** shared with the TAB, so each bar is as wide as the wider of the two staves needs.
   - **Pitch:** written pitch = sounding pitch + 12.
   - **Spelling and accidentals:** spelling from the key; an accidental when it differs from the key signature or from an earlier note in the bar.
   - **Stems:** direction by the notehead's distance from the middle line, one direction per beam group.
   - **Beams:** by beat, using the same windows as the TAB.
   - **Also:** ledger lines, chords, rests at standard positions, key and time signatures, dots.
3. **Output for review.** For each input:
   - an SVG of the two staves, using Bravura glyphs and the metadata positions;
   - the same notes written as a spike-only MusicXML with a treble-8vb staff, rendered by MuseScore 4.7.5 headless to PNG.
   
   The user reviews the two side by side.
4. **Measurements, release build, reference machine:**
   - layout time per system, and for the whole TYPICAL and LARGE scores;
   - glyph and line count, and payload size per system, against today's TAB-only numbers.
5. **PDF.** `feature_rich` (both staves) and TYPICAL, written from the same layout output by two routes (N6):
   - **vector:** the layout model straight to PDF with `pdf-writer`, Bravura embedded as a font subset with `subsetter`;
   - **SVG-to-PDF:** our SVG converted by `svg2pdf`, the converter of the same ecosystem.
   
   The pages must open in Microsoft Edge. The candidate crates' licenses are checked before use and listed in the results.
6. **Effort estimate** for the 6c slices.

## 4. Designs the spike must test (user requirements, 2026-11-11)

### 4.1 Written pitch and sounding pitch

- **The semantic model keeps the sounding pitch, canonically.** A note stores its string and fret. Its sounding semitone is `pitch_at(tuning, capo, position)` (mc-theory), and it is never stored. The model gains no written pitch.
- **The instrument's transposition lives in one place, the notation engine.** A staff's configuration carries a `Transposition`; guitar uses one octave up, written = sounding + 12 semitones. One function applies it, `written(sounding, transposition)`, and nothing else in the engine adds octaves.
  - An octave transposition does not change the spelling; only the octave number moves.
  - The clef is the treble clef with an 8 below (SMuFL `gClef8vb`, U+E052). It shows the reader that the sound is an octave lower than written.
- **Spelling happens on the sounding pitch, before transposing.** Spelling therefore never depends on the transposition, and the TAB and the notation agree on the pitch.
- **The capo is part of the sounding pitch,** because `pitch_at` adds it. The notation therefore shows what sounds, not "capo notation" relative to the capo. This is the spike's assumption; the ADR asks the user.
- **Not in scope:** a per-note spelling override (bounded-contexts mentions one; the model has none), other transposing instruments, and concert-pitch toggling.

### 4.2 Accidental state

- **Spelling of a sounding semitone in a key** (a pure function in the engine; it later moves to mc-theory if a second user appears, GR-05):
  1. If the semitone is a degree of the key's scale (major, or natural minor for a minor key), that spelling is used. This gives E-sharp in F-sharp major and C-flat in G-flat major, which the ADR-033 rule cannot.
  2. Otherwise, a chromatic note gets a sharp in keys without accidentals or with sharps, and a flat in keys with flats.
  3. A tied note keeps the spelling of the note it is tied from, even across a bar line or a key change. A tie never changes the letter.
- **State per staff and per bar.** A map from staff position (letter and octave) to the alteration in force starts from the key signature at every bar line.
  - Notes are processed in onset order across the staff's voices, so all voices of the staff share the state.
  - A note shows an accidental exactly when its alteration differs from the state at its position. The state then takes the note's alteration.
  - The scope is the position (letter and octave), not the letter in every octave, as in current common practice.
- **Reset at every bar line.** The next bar starts again from the key signature.
- **Ties.**
  - A note tied from the previous beat shows no accidental, within the bar or across a bar line.
  - Across a bar line it does not change the new bar's state either. A later untied note at the same position is compared with the key signature, so it shows its accidental again if needed.
- **Chords with two notes on one position** (for example F and F-sharp) both show an accidental.
- **Not in scope:** courtesy accidentals, accidentals cancelled by an explicit natural after a key change (the new key signature is drawn instead), and microtones.

### 4.3 Staff model (revisiting ADR-031's stave)

ADR-031 has one `Stave` per shown track. It is replaced by three plain parts, enough for "notation above TAB" for fretted tracks, and nothing more:

- **`StaffConfiguration`** (per track, a view setting; not stored in the score):
  - which staves the track shows: `Notation`, `Tab`, or both, notation first;
  - for `Notation`: the clef (`Treble8vb`) and the `Transposition` (an octave up);
  - for `Tab`: the string count, as today.
  
  The default for a fretted track is both. There is no other clef, no grand staff, and no per-instrument catalogue.
- **`Staff`**: the laid-out content of one staff of one track in one system (lines, glyphs and hit regions in staff-local coordinates), with its kind (`Notation` or `Tab`).
- **`StaffPosition`**: where a staff sits in its system:
  - the track index;
  - the staff index within the track;
  - the top offset;
  - the height (five lines and their margins, or the strings and theirs);
  - the gap to the next staff of the same track.
  
  Painting, the cursor and hit testing use it.

Rules that do not change:
- One bar plan per system: every staff of every track shares bar x-positions and widths, and the onset columns (ADR-031 5.1).
- Each staff kind adds its needs to a bar's width: for notation, the clef and key signature at the system start, and the accidental width before a column.
- Editing stays on the TAB. Hit regions gain the staff kind, so a click on the notation staff selects the beat; note entry on the notation staff is out of scope.

## 5. Success criteria (fixed before the spike runs)

| Id | Criterion |
|---|---|
| N1 | Every input lays out without panic. For every note, automated checks hold: <br>– the staff position equals the written pitch's diatonic step (line or space); <br>– ledger lines are present exactly when needed; <br>– an accidental is shown exactly as section 4.2 says, including ties and the reset at bar lines; <br>– the stem direction follows the middle-line rule, with one direction per beam group; <br>– beams group by beat as on the TAB. |
| N2 | **Alignment, quantified.** In every system of every input: <br>– bar-line x on the notation staff equals bar-line x on the TAB, with \|Δx\| ≤ 0.01 layout units; <br>– for every beat, the anchor of its notes on the notation staff (the notehead's horizontal centre, or the rest's) and on the TAB (the fret number's centre) satisfy \|Δx\| ≤ 0.05 staff spaces. <br>The maximum Δ is reported. |
| N3 | **Collision detection by engraving rules,** not only box intersection. A checker reports every violation of these rules in every input: <br>– noteheads in a chord on adjacent steps (a second) are on opposite sides of the stem; <br>– accidentals of a chord are placed in columns so that no two overlap (with their SMuFL bounding boxes) and each clears its notehead by at least 0.12 staff spaces; <br>– a ledger line extends beyond the notehead on both sides by `legerLineExtension` (engraving defaults) and does not touch a neighbouring column's ledger line; <br>– a stem is at least 3.5 staff spaces long, measured from the outermost notehead, unless a beam shortens it to no less than 2.5; <br>– a beam does not cross a notehead of its own group; <br>– a rest does not overlap a notehead of another voice; <br>– dots sit in a space, never on a line. <br>The six hand-built inputs must show zero violations; the reference scores report their count. Rules are checked in code, then by eye. |
| N4 | The user judges the side-by-side images against MuseScore 4.7.5 as readable, correct guitar notation. Pixel identity is not asked. Differences are listed. |
| N5 | **Timing, release build, measured separately:** <br>(a) layout computation (score to layout model); <br>(b) rendering (layout model to SVG, as the stand-in for painting); <br>(c) PDF generation, per route. <br>Each is reported as median, p95 and p99 over at least 200 runs, for one system, the in-view systems of TYPICAL, and all of TYPICAL and LARGE. The layout of TYPICAL in view, with notation, must stay within ADR-031 P1 (16 ms per edit, layout part only). If it cannot, the spike says what would be needed. Payload per system (bytes, glyphs, lines) is reported against today's TAB-only numbers. |
| N6 | **PDF, two routes compared directly** on `feature_rich` (both staves) and TYPICAL: <br>(1) a vector backend that writes the layout model straight to PDF, with Bravura glyphs as embedded font text and lines as paths; <br>(2) an SVG-to-PDF backend that converts our SVG. <br>Compared: <br>– fidelity: the glyph positions read back from each file against the layout model; <br>– file size and font embedding (subset or full); <br>– text and vector quality (no rasterization); <br>– determinism; <br>– generation time (N5c); <br>– dependencies and licenses; <br>– code size. <br>Both files must open in Edge. One route is recommended. |
| N7 | A risk note (R-13 update) and an effort estimate for the 6c slices, with the decisions the ADRs must take. They include: <br>– the staff model of section 4.3 (ADR-031 change); <br>– written pitch, the clef and the capo (4.1); <br>– the spelling and accidental rule (4.2); <br>– the metrics source (SMuFL metadata in staff spaces versus Dart measurement); <br>– hit testing on the notation staff; <br>– the PDF route. |
| N8 | **Determinism.** For the same semantic input, font version and configuration, the layout model is byte-identical in its canonical serialization: <br>– across 100 runs in one process; <br>– across two separate processes; <br>– across a debug and a release build. <br>The same holds for the SVG and for the PDF of each route; a PDF's creation date is fixed or omitted. A different font version or configuration (staff size, transposition) must change the hash, so the hash is shown to cover them. |

### Decision rule after the spike

- **Go:** N1, N2, N3, N5, N6 and N8 are met, and the user judges N4 readable. The 6c ADRs are written from the evidence.
- **Revise:** a measurable criterion fails, but the spike shows a credible fix within the estimate, for example a cache for N5 or an extra rule for N3. The ADRs carry the fix as a requirement, and the failed criterion becomes an acceptance criterion of the slice.
- **No-Go:** N1 or N3 cannot be met for the six basic inputs within the timebox, or N5 exceeds P1 by more than two times with no credible fix. Then ADR-018's reconsideration condition applies: an engraving engine in an isolated process. That needs a license plan, because Verovio is LGPL-3.0 and outside ADR-015.

Timebox: 4 FTE days (technical-spikes S11). If the timebox runs out, the spike stops and reports what it reached. Not built in the spike: the full engine, the app integration, slurs, articulations, tuplet brackets beyond a number, multiple voices with shared noteheads, cross-staff anything, and other instruments.

## 6. Clean-room and license rules for this spike

- No code or text is copied from Verovio, MuseScore, LilyPond or any other engraver. Facts about engraving practice come from public descriptions, such as the SMuFL specification and general notation references. Each source is listed here before it is used.
- MuseScore is run as an application, a black box; its source is not read.
- Bravura and `Bravura.json` are read from a local copy outside the repository (`%TEMP%\bravura-1.482`, checked against the SHA-256 in ADR-037). They are not committed until ADR-037 is accepted.
- Images and PDFs produced from our own test scores may be committed as results.

## 7. Results (2026-11-12)

The engine is in `engine/` (its own workspace, excluded from the main one; throwaway). All numbers below are from `results/`, which the commands in 7.9 regenerate. Machine: the developer's Windows 11 desktop, release build unless stated.

### 7.1 Summary

| Criterion | Result | Evidence |
|---|---|---|
| N1 correctness | **Met.** 0 errors on all 9 inputs (6 hand-built, `feature_rich`, `techniques`, SMALL). | `results/n1-n3-check.txt`, `results/xcheck-musescore.txt` |
| N2 alignment | **Met, exactly.** Bar lines and beat anchors of notation and TAB differ by 0.0000 (tolerance 0.05 sp). | `results/n1-n3-check.txt` |
| N3 engraving rules | **Met.** 0 violations on all 9 inputs, in 4 024 rule checks. | `results/n1-n3-check.txt` |
| N4 review | **Waiting for the user.** Images side by side with MuseScore 4.7.5 in `results/review/`. Known differences in 7.5. | `results/review/` |
| N5 timing | **Met for TYPICAL in view; LARGE needs the plan cache** (measured: p99 4.9 ms with it, 212 ms without). | `results/n5-timing.md` |
| N6 PDF | **Vector route recommended.** Both read back every glyph; only the vector route is deterministic and fast. | `results/n6-pdf.md`, `results/pdf/` |
| N7 risks and effort | Drafted for approval (R-13 in the risk register, slice estimate in 7.8). | this section |
| N8 determinism | **Met for layout, SVG and the vector PDF; not met by SVG-to-PDF.** | `results/n8-determinism.md`, `results/determinism/` |

The checks catch deliberate faults. Each breakage (`S11_BREAK=<name>`) is reported, summed over the 9 inputs:

| Breakage | What it breaks | Caught |
|---|---|---|
| `stem-rule` | stem direction (inverted middle-line rule) | N1 38 |
| `no-bar-reset` | accidental state carried over the bar line | N1 33; MuseScore cross-check 33 differ |
| `accidental-touches` | no clearance between accidental and head | N3 60 |
| `no-seconds` | both heads of a second on one side | N3 14 |
| `dot-on-line` | dots on lines | N3 2 |
| `short-ledger` | ledger lines shorter than the head | N3 127 |
| `no-headroom` | fixed staff box, content ignored | N3 124 |
| `no-tuplet-number` | tuplet numbers left out | N1 2 |
| `tuplet-touches` | tuplet number on the notes | N3 2 |
| `tuplet-by-window` | a triplet split by beat-window beaming | N1 1 |
| `no-bar-rest` | empty bars without a whole-bar rest | N1 2 |
| `double-key` | key change drawn again after the system header | N1 1 |

### 7.2 Faults found during the spike

Reviewing the PDFs and images, at 2x (thin lines vanish in smaller previews), found six faults that the first version of the checks did not report. Each became a check before it was fixed:

1. A stem above the staff was clipped at the top of the page. The staff box had fixed margins. Now each bar's plan computes its vertical reach, a system's box follows its bars, and N3 (i) checks every painted item is inside its staff box and every box inside its system and the page.
2. The last bar line was centred on the page edge, so half of it fell outside. Bar lines now end on the bar's end.
3. The clef's 8 reached below a small box; TAB digits on the first string reached above the TAB box. Fixed by the same reach rule (clef counted; TAB top margin 1.0 sp).
4. Tuplets had no number, and a triplet crossing a beat was beamed by beat window. A tuplet is now its own beam unit, with its number on the beam side.
5. Empty bars had no rest. They get a whole-bar rest.
6. A key change in a system's first bar was drawn twice (header and bar). Found in the `keys` review image.

A seventh finding was in the spike's input, not the engine: the hand-built `rhythm` bar was 11/8 in 4/4. It now fills exactly 4/4 and adds a sixteenth-sixteenth-eighth group (two beam levels in one group).

Lesson for 6c: the checks must cover page geometry (bounds, overlap between staves), not only the rules inside one staff. A review at 2x by eye found what the rules did not.

### 7.3 N1 and the MuseScore cross-check

229 of 229 notes agree with MuseScore 4.7.5 on written position and shown accidental. The check works on MuseScore's own re-export of our MusicXML (written treble 8vb, ties kept). This is an independent check of the accidental rule (4.2). It is not a check of spelling, because MuseScore keeps the spelling we write. Rests are not compared. The TQ-8 gaps remain.

### 7.4 N2 and N3

N2 is exact by construction: notation and TAB take their x from one bar plan and one set of column anchors (4.3). The check guards that construction; it does not measure a tolerance that happens to be met. N3 rules (a) to (j) and the bounds rule (i) run on every input; the check output gives the total per input.

### 7.5 N4: differences from MuseScore (for the user's review)

Readable guitar notation in our judgement, and the user decides. Differences seen:

- Spacing is wider than MuseScore's. `keys` puts 3 bars on its first system, where MuseScore puts 5 on a wider page, and TYPICAL fits 1 bar per system at 125 staff spaces of width. Every column reserves room for accidentals and two-digit fret numbers. To tune in 6c; not a correctness issue.
- No courtesy key signature at a system's end, no double bar line before a key change, no cancelling naturals.
- A whole rest that fills a bar sits at its onset; MuseScore centres a full-bar rest.
- Ties are thinner and flatter than MuseScore's.
- Out of the MVP and not drawn: repeats, voltas, markers, tempo, dead-note heads, let ring, slurs, articulations. `feature_rich` and `techniques` show those beats as plain notes.
- The TAB digits use Helvetica (vector PDF) or Consolas (SVG). Production uses Noto Sans Mono (ADR-021).

### 7.6 N5 timing (release, ms)

| Case | Median | p95 | p99 |
|---|---|---|---|
| TYPICAL in view (4 systems), notation + TAB, no cache | 6.6 | 9.5 | 9.9 |
| TYPICAL one system, no cache | 6.8 | 16.6 | 19.5 |
| TYPICAL edit: re-plan 2 bars + in view from cached plans | 0.37 | 0.62 | 0.70 |
| LARGE in view, no cache | 130 | 186 | 212 |
| LARGE edit: re-plan 2 bars + in view from cached plans | 3.8 | 4.3 | 4.9 |
| LARGE building all plans (once per open) | 118 | 134 | 167 |
| Today's mc-notation TAB in view, TYPICAL / LARGE | 0.79 / 12.2 | 1.2 / 13.8 | 1.4 / 15.1 |
| SVG rendering, TYPICAL in view | 0.58 | 0.60 | 0.70 |
| PDF, TYPICAL whole score: vector / SVG-to-PDF | 60 / 4 180 | 66 / 5 215 | 83 / 5 349 |

- Without a cache, every layout plans every bar of the score. The cost grows with the score: within P1 for TYPICAL in view, though p99 varies between runs (9.9 to 19.5 ms for the same work). It is more than six times over the 33 ms budget for LARGE.
- With cached bar plans, an edit re-plans its bar and the next one; the next one reads the edited bar for key and time display and tie spelling. The view is then laid out from the cache: 4.9 ms p99 on LARGE. Filling the cache costs 118 ms once per open, inside the "open large project" budget of 2 s. **The cache is a requirement for 6c, not an option.**
- Payload: about 4.6 KB per bar for notation + TAB on TYPICAL, against about 2.2 KB per bar for today's TAB-only DTO (estimate from 8 748 B per 4 bars).

### 7.7 N6 PDF routes and N8 determinism

| | Vector (`pdf-writer` + `subsetter`) | SVG-to-PDF (`svg2pdf` 0.13) |
|---|---|---|
| Glyph positions read back, feature_rich / TYPICAL | 43/43, 13 657/13 657, max error 0.0000 pt | all, max error 0.0059 pt |
| Size, feature_rich / TYPICAL | 19 990 / 416 306 B | 18 519 / 383 129 B |
| Fonts | Bravura subset (OFL-FAQ 1.12); TAB in a standard font | Bravura subset plus the system font for TAB |
| Time, TYPICAL | 60 ms median | 4.2 s median (the system font scan is only 13 ms of it) |
| Determinism | byte-identical: 100 runs in one process, two processes, debug and release | **not deterministic.** Font objects come out in a varying order in one process and across processes. Probably an unordered map inside the dependency (inferred from the output, its source not read) |
| Code | about 150 lines in the spike | about 15 lines, plus our SVG writer |
| Licenses | MIT / Apache-2.0 | MIT / Apache-2.0 |

Recommendation: **the vector route.** It meets N8, it is about 70 times faster, it embeds only the fonts the layout names, and it paints from the same layout model as the screen. The size difference (about 9 % on TYPICAL, from the knockout rectangles under the TAB digits) can be reduced in 6c.

Determinism (N8), 9 inputs × 4 artefacts:

- Layout, SVG and vector PDF are identical in every case: 100 layouts in one process, 10 renders, two processes and a debug build against a release build.
- Staff size and transposition change the canonical layout (the hash covers the configuration).
- Bravura 1.481 against 1.482 changes the canonical layout and both PDFs. In the layout only the font line changes: the metrics of the glyphs the spike uses are equal in the two versions. So the font version is covered by name and version, not shown through changed geometry.

The vector PDFs are in `results/pdf/`. The SVG-to-PDF files are not committed, because they embed a subset of a system font. Both kinds of file open in the Windows PDF renderer (`tools/pdf-to-png.ps1`). Opening them in Edge (N6) is left to the user, because headless Edge does not render PDF.

### 7.8 Proposals for approval (N7)

**Decision record (2026-11-12).** The user approved items 1 to 9, with added conditions, and they are now ADRs. This list is kept as the spike's proposal, and the ADRs are the authority.

| Item | Decided in | Status |
|---|---|---|
| 1 staff model | ADR-038 (amends ADR-031) | Accepted |
| 2 written pitch, clef, capo | ADR-039: sounding pitch with the capo, the octave from the instrument | Accepted |
| 3 spelling and accidentals | ADR-040: rules in `mc-theory`; a semantic layer in `mc-notation` | Accepted. The spelling rule (ADR-040 section 5) is **pending the user**, because `KeySignature::spell` (ADR-033) differs from the spike rule in keys with 6 or 7 accidentals |
| 4 metrics | ADR-041 decision 1: a generated, pinned table; missing glyphs and anchors are explicit errors | Accepted |
| 5 bar-plan cache | ADR-041 decision 3: full key and forward tie propagation | Accepted, mandatory |
| 6 vertical layout | ADR-038 decision 5 | Accepted |
| 7 hit testing | ADR-041 decision 4: read-only | Accepted |
| 8 PDF | ADR-041 decision 5: `mc-pdf` vector, only bundled fonts embedded, pagination checkpoint, Edge check by the user | Accepted |
| 9 checks as tests | ADR-041 decision 6 | Accepted |

Two corrections to this spike, found while writing the ADRs:
- **`Engine::replan` re-plans only the edited bar and the next one.** A tie chain that spans more than two bars can therefore keep a stale spelling. The timing measured in 7.6 is not affected (same work per bar), but the invalidation rule was incomplete. ADR-041 decision 3 fixes it: re-plan forward until the entering tie spellings are unchanged.
- **The vector PDFs in `results/pdf/` use Helvetica**, a non-embedded standard font, for TAB digits. They do not meet the user's "no dependence on installed fonts" condition. ADR-041 decision 5 requires embedded Noto Sans Mono.

The original proposals follow. They were Proposed until the user approved them on 2026-11-12.

1. **ADR-031 change, the staff model.** A track shows a `StaffConfiguration` (notation, TAB, transposition), a view setting that is not stored in the score. Each staff is a `Staff` at a `StaffPosition` in its system. Every staff of a system takes its x from one shared bar plan. Editing stays on the TAB staff in 6c.
2. **Written pitch, clef and capo.** The model keeps the sounding pitch (no change to the score or the format). The notation engine alone writes it an octave up under `gClef8vb`. **Question for the user:** the spike puts the capo into the sounding pitch, so the notes are written as they sound. Should the capo instead be shown as fingered (written without the capo)?
3. **Spelling and accidentals.** A pitch spells as a scale degree of the key. Otherwise it takes a sharp in sharp and neutral keys and a flat in flat keys. A tie keeps its source spelling. Accidental state is per letter and octave and resets at each bar line; a tie across the bar line shows nothing and sets no state. Proposal: the pure rule moves to `mc-theory` (music rules stay in Rust, GR-05), and `mc-notation` uses it.
4. **Metrics.** Glyph boxes, anchors and engraving defaults come from `Bravura.json`, in staff spaces, read in Rust. Codepoints are checked against the font's cmap at load. No measurement in Dart.
5. **Bar-plan cache** (7.6) as a requirement, with re-planning of the edited bar and the next one.
6. **Vertical layout from content.** The reach is computed per bar, and the bounds checks become tests.
7. **Hit testing on the notation staff:** not measured in the spike. Proposal: read-only in 6c (hits only select the beat, which the TAB staff then edits). The hit regions come from the same notehead facts as N1.
8. **PDF:** the vector route (7.7), with Noto Sans Mono embedded for TAB text.
9. **Checks as tests.** The N1–N3 checks, the bounds checks and the breakage list move into `mc-notation` as tests, and one canonical layout per input becomes a golden file (N8). Per GR-14, they may be extended but not weakened.

R-13 and the effort estimate per slice are drafted in `docs/roadmap/risk-register.md` and `docs/research/technical-spikes.md` (S11 entry).

### 7.9 Reproducing

```powershell
$env:BRAVURA_DIR = "$env:TEMP\bravura-1.482"   # SHA-256 as in ADR-037
cd spikes/s11-notation/engine
cargo build --release
.\target\release\s11-notation.exe check          # N1-N3 (S11_BREAK=<name> for a breakage)
.\target\release\s11-notation.exe musicxml <dir> # then MuseScore4.exe -o <name>.ms.musicxml <name>.musicxml
.\target\release\s11-notation.exe xcheck <dir>
.\target\release\s11-notation.exe timing         # N5
.\target\release\s11-notation.exe pdf <dir>      # N6
.\target\release\s11-notation.exe determinism <dir>  # N8, run twice and in a debug build, compare SHA-256
$env:S11_SCALE = "2"; .\target\release\s11-notation.exe render <dir>  # review SVGs
```

Sources used for engraving practice: the SMuFL specification (glyph names, codepoints, metadata and engraving defaults) and Bravura's own metadata. General practice was taken from public notation references (stem direction by the middle line, accidental columns, ledger extension, dots in spaces, beaming by beat and tuplet as unit, whole-bar rests). No engraver's source was read.
