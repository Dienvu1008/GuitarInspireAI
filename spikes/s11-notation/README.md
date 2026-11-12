# S11: standard notation above TAB, and PDF (spike, plan)

Purpose: Entry gate of Phase 6c ([technical-spikes.md](../../docs/research/technical-spikes.md) S11). Show, before any production code, that our own engine in `mc-notation` (ADR-018) can engrave guitar standard notation above the TAB from the same score and the same bar plan. Also measure what it costs, and show that a PDF can be written from the same layout model. The answers feed the Phase 6c ADRs with their acceptance criteria fixed first.
Status: Plan. Its criteria (section 4) are fixed before the spike runs; the user reviews them first.
Related: [ADR-018](../../docs/architecture/decisions/ADR-018-notation-layout-in-rust.md), [ADR-031](../../docs/architecture/decisions/ADR-031-multi-track-tab-display.md), [ADR-033](../../docs/architecture/decisions/ADR-033-musicxml-export-adapter.md), [ADR-037](../../docs/architecture/decisions/ADR-037-bravura-smufl-font.md) (Proposed), risk R-13.

## 1. Decisions taken before the spike (user, 2026-11-11)

1. **Engraving engine: our own, in `mc-notation`, following ADR-018.**
   - Verovio is not a runtime dependency.
   - Rust turns the score into an immutable layout model, and Flutter renders it.
   - The PDF backend renders the same layout model and computes no layout of its own.
2. **Music font: Bravura, the SMuFL reference font,** bundled unmodified, with a license audit, the license text, attribution, version and checksum.
   - Done in [ADR-037](../../docs/architecture/decisions/ADR-037-bravura-smufl-font.md) and ADR-015 amendment 1, both Proposed.
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
5. **PDF.** One page of `feature_rich` (both staves), written from the same layout output:
   - with an own minimal writer that embeds Bravura in full;
   - with `pdf-writer` and `subsetter`, which embed a subset.
   
   Compare file size, code size and the dependencies added. The page must open in Microsoft Edge and in MuseScore's PDF import if there is one; otherwise in Edge only, recorded.
6. **Effort estimate** for the 6c slices.

## 4. Success criteria (fixed before the spike runs)

| Id | Criterion |
|---|---|
| N1 | Every input lays out without panic. Automated checks hold for every note: the staff position equals the written pitch's diatonic step (line or space), and ledger lines are present exactly when needed. An accidental is shown exactly when the note's alteration differs from the key signature or from the previous note on that step in the bar. Stem direction follows the middle-line rule, one direction per beam group. Beams group by beat as on the TAB. |
| N2 | Bar lines and beats that start together line up between the notation staff and the TAB, in every system, from one bar plan. |
| N3 | No notehead, accidental, stem or ledger line overlaps another in the review images of the six hand-built inputs (checked by bounding boxes in code, and by eye). |
| N4 | The user judges the side-by-side images against MuseScore 4.7.5 as readable, correct guitar notation. Pixel identity is not asked. Differences are listed. |
| N5 | Layout time and payload, release build. The TYPICAL score with notation stays within ADR-031 P1 (16 ms for an edit with the visible projection) with the in-view layout path. The cost per system is measured and the LARGE numbers are recorded. If P1 cannot hold, the spike says what would be needed (caching, simplification). |
| N6 | A PDF page of `feature_rich` with both staves is produced from the same layout output, with Bravura embedded, and opens in Edge. Both PDF routes are compared (size, code, dependencies) and one is recommended. |
| N7 | A risk note (R-13 update) and an effort estimate for the 6c slices, with the decisions the ADRs must take. These include: the stave model change of ADR-031; written pitch and the clef; the accidental rule; the metrics source (SMuFL metadata in staff spaces versus Dart measurement); hit testing on the notation staff; and the PDF route. |

Timebox: 4 FTE days (technical-spikes S11). If the timebox runs out, the spike stops and reports what it reached.

## 5. Clean-room and license rules for this spike

- No code or text is copied from Verovio, MuseScore, LilyPond or any other engraver. Facts about engraving practice come from public descriptions, such as the SMuFL specification and general notation references. Each source is listed here before it is used.
- MuseScore is run as an application, a black box; its source is not read.
- Bravura and `Bravura.json` are read from a local copy outside the repository (`%TEMP%\bravura-1.482`, checked against the SHA-256 in ADR-037). They are not committed until ADR-037 is accepted.
- Images and PDFs produced from our own test scores may be committed as results.
