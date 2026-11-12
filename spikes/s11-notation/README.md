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
