# MusicXML export: MuseScore check and export timing (ADR-033 E8 to E11)

Purpose: Record what MuseScore makes of our MusicXML export, and how long the export takes. This is evidence for [ADR-033](../docs/architecture/decisions/ADR-033-musicxml-export-adapter.md) section 4.
Scope: one machine (`reference-machine.md`, ADR-025) and one reader, MuseScore 4.7.5. Finale, Dorico and Guitar Pro were not checked. Layout quality was not judged.

## Procedure (reproducible)

```
cargo build --release -p mc-cli --locked
powershell -ExecutionPolicy Bypass -File tools/musescore-check/run.ps1
```

`run.ps1` does the following:
- It writes the E1 inputs with `mc-cli`: `feature_rich`, `reference(SMALL)`, `reference(TYPICAL)`, an empty document, and `feature_rich` re-imported from our own MIDI export.
- It has MuseScore convert each one headless to MIDI and to MusicXML (`MuseScore4.exe -o`).
- It compares the results with `tools/musescore-check/compare.py` (Python standard library):
  - MIDI notes as pitch, onset and length in quarter notes, with repeats as each file plays them.
  - String and fret per part and measure.
- It renders `feature_rich` to PNG.

The full output of the run on 2026-11-09 is in [musescore/summary-2026-11-09.txt](musescore/summary-2026-11-09.txt). The procedure ran twice with identical results.

## E8: MuseScore opens every file; same notes as our MIDI export

All 10 conversions (5 inputs, each to MIDI and to MusicXML) exited 0 and wrote their file.

| Score | Notes (ours / MuseScore) | Same pitch and onset | Same length | Length differs, rendering only | Real difference |
|---|---|---|---|---|---|
| `reference(TYPICAL)` | 9 856 / 9 856 | 9 856 | 9 376 | 480 let ring | 0 |
| `feature_rich` | 14 / 13 | 13 | 9 | 2 dead, 2 let ring | 1 |

Explanations:
- **MuseScore releases every note one tick early** (1/480 quarter). Such notes count as the same length.
- **Let ring.**
  - Our MIDI export plays a let-ring note to the end of its bar (ADR-024); MuseScore's MIDI export plays it for its written length.
  - The MusicXML carries let ring correctly: MuseScore read and re-wrote all 552 `<tied type="let-ring"/>` of TYPICAL (see E9).
  - This is a difference in how MIDI is rendered, not lost content.
- **Dead notes.** Our MIDI export plays them as 1/64 at velocity 40 (ADR-024). MuseScore plays them at full length and shows them as a fret number on TAB, not as X. The `<notehead>x</notehead>` survives MuseScore's own MusicXML export.
- **The one real difference** is in `feature_rich`: G4 (pitch 67) at onset 9 quarters.
  - The pickup bar ties G4 into the first bar of the repeated section.
  - On the second pass the previous bar is ending 1, which has no G4. Our playback therefore plays the note again; MuseScore does not play it on the second pass.
  - MusicXML has no way to say "tied on the first pass only": `<tie>` applies on every pass. The file states the score as written.
  - So this is a difference in how the reader handles a tie into a repeat. It is not an approximation that the report counts.

**E8 is not met as written.** The criterion asks for the same notes, including length, except in bars the report counts as approximated:
- Every note's pitch and onset match, except the one tie-into-repeat note.
- The length differences come from rendering (let ring, dead notes) and from MuseScore releasing one tick early. They are not losses in the file.

The criterion is not changed. The gap is recorded in ADR-033 section 6.

## E9: string and fret survive MuseScore's MusicXML re-export

| Input | Notes with string and fret | Measures that differ | Let ring (ours / MuseScore) |
|---|---|---|---|
| `reference(TYPICAL)` | 9 600 / 9 600 | 0 | 552 / 552 |
| from MIDI | 20 / 20 | 0 | 0 / 0 |
| `feature_rich` | 10 / 10 | 1 | 1 / 1 |

The difference is the drop-D chord of the "Rhythm" track, which has **capo 2**.
- We write:
  - the open-string tuning in `<staff-tuning>`
  - `<capo>2</capo>`
  - frets counted from the capo
  - the sounding pitch (open string + capo + fret)
- This follows the MusicXML 4.0 definition of `<capo>`, which raises the open strings of `<staff-tuning>`.
- MuseScore ignores `<capo>` on import. Its own issue tracker records this ([MuseScore: "[MusicXML] support capo element"](https://musescore.org/en/node/20513)). It then derives the frets from the pitch:
  - frets +2 on strings 6, 5 and 4
  - one note moved from string 3, fret 4, to the open string 2
- Its re-export has no `<capo>`.

The encoding is not changed to suit MuseScore, because that would misstate the score.

**E9: met for tracks without a capo. A track with a capo is re-fingered by MuseScore (a known limitation of MuseScore). This is an evidence gap.**

## E10: rendering

![MuseScore rendering of feature_rich](musescore/feature-rich-preview.png)

Reviewed on 2026-11-09. The rendering shows:
- Both tracks on TAB staves, with names.
- The pickup.
- The tie from the pickup into bar 1.
- The "Verse" marker and both tempo marks (96, and 120.5 in the 6/8 bar).
- The repeat barlines, endings 1 and 2, and the 6/8 change.
- The triplet bracket.
- The let-ring tie on fret 7.
- The rest that pads the short voice.
- The five quarters of the overfull bar, all kept.

No collision hides a note. Seen in the rendering, and both explained above: the capo chord is re-fingered (0-4-4-2), and the dead note shows as "0".

**E10: met.**

## E11: export time (release, reference machine, 2026-11-09)

`mc-cli export-musicxml` in release mode, 7 runs each, timing only the export:

| Score | Median | Max | Criterion |
|---|---|---|---|
| `reference(TYPICAL)` (4 tracks, 150 bars, 9 600 notes) | 6.3 ms | 8.7 ms | ≤ 200 ms |
| `reference(LARGE)` (20 tracks, 300 bars, 288 000 notes, a 100.2 MB file) | 154.9 ms | 164.7 ms | ≤ 2 s |

**E11: met** on this machine.

## ADR-035 TQ-8: playing techniques through MuseScore (6a-1, 2026-11-09)

The procedure also exports the `techniques` fixture (ADR-035: every technique and combination, two bars of 4/4, open C shape). MuseScore converts it to MusicXML, MIDI, `.mscz` and PNG. `compare.py techniques` then checks, note by note, which of our marks MuseScore's MusicXML re-export still has. `run.ps1` also counts what MuseScore keeps in its own `.mscz` (the import side, before its exporter runs). In each run, E8 and E9 gave exactly the results above.

Three kinds of result are kept apart below:
- **native**: a MusicXML 4.0 element or value defined for the meaning;
- **our approximation**: an extension, counted in the export report;
- **MuseScore loss**: the file is right, but MuseScore drops it on import or on re-export.

### First encoding (commit `1290325`)

Full output: [musescore/summary-6a1-2026-11-09.txt](musescore/summary-6a1-2026-11-09.txt). Rendering: [musescore/techniques-preview-1290325.png](musescore/techniques-preview-1290325.png).

Palm mute was only `<other-technical>` text; a stroke on a rest was `<technical><down-bow/>` on the rest. MuseScore lost palm mute, string-percussive and the rest stroke, on import already.

### Interop investigation (user review of `1290325`)

Two MusicXML 4.0 mechanisms were tested. Neither had been considered in the first encoding.

- **Palm mute as playback:** `<note>…<play><mute>palm</mute></play></note>`.
  - The reference says `<play>` inside a `<note>` "applies to the current note only" ([play](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/play/)).
  - The `mute` type includes `palm` ([mute](https://www.w3.org/2021/06/musicxml40/musicxml-reference/data-types/mute/)).
- **Stroke on a rest as a direction:** `<direction placement="above"><direction-type><other-direction smufl="guitarStrumDown"/></direction-type><voice>1</voice></direction>`, directly before the rest, so it sits at the rest's onset in its voice.
  - `<other-direction>` with `smufl` exists "allowing application interoperability" ([other-direction](https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/other-direction/)).
  - SMuFL defines `guitarStrumDown` (U+E847) and `guitarStrumUp` (U+E846) ([glyphnames.json](https://github.com/w3c/smufl/blob/gh-pages/metadata/glyphnames.json)).

Method:
- [tools/musescore-check/variants.py](../tools/musescore-check/variants.py) derives the candidate files from the first-encoding export. Its usage line reproduces them byte for byte.
- Each file was validated with xmllint 2.9.14 against the pinned XSD (commit `799e2de`, SHA-256 checked, `--nonet`, as in `tools/musicxml-schema/validate.sh`).
- Three negative controls were rejected: `<play>` before `<notations>`, `mute` value `palm-mute`, and an unknown element. So the passes are real.
- MuseScore 4.7.5 then converted each file. Its own `.mscx` was compared with the control's, ignoring the random element ids.

| Candidate | XSD | MuseScore import (its `.mscx`) | MuseScore MIDI | MuseScore MusicXML re-export |
|---|---|---|---|---|
| Control (first encoding, ElementTree round trip) | valid | reference | reference | reference |
| `<play><mute>palm</mute></play>`, text kept | valid | **identical to the control**: ignored | byte-identical to the control: no effect | no `<play>`, no `<mute>` |
| `<play><mute>palm</mute></play>` only | valid | identical to the control | byte-identical | none. Two of four conversions exited with 0xC0000005 after writing their output; 9 repeats of this and the other files all exited 0, so this was not reproduced |
| `<play>` plus `<words>P.M.</words>` and `<dashes>` over each run | valid | generic `TextLine` and `StaffText` "P.M." (not MuseScore's palm-mute line) | byte-identical: no effect | 3 `<words>P.M.</words>`, 2 `<dashes>`: visible text only |
| Rest stroke as `<other-direction smufl="guitarStrumDown"/>` | valid | **a `guitarStrumDown` symbol, in bar 2, voice 1, directly before the eighth rest** | byte-identical | no `<other-direction>`: lost |
| The same, with the `<technical>` on the rest kept | valid | the same symbol (the `<technical>` is still ignored) | byte-identical | lost |

**MuseScore's own palm mute** (a one-off check, not part of `run.ps1`). In MuseScore's `.mscz` of the words variant, the `TextLine` was renamed to a native `PalmMute` spanner (same anchors), and the file was run through MuseScore again:
- Its MIDI changes: the palm-muted notes are shortened.
- Its MusicXML export is `<words>P.M.</words>` with a dashed `<bracket>`, not `<play><mute>`.
- Re-importing that export gives a generic `TextLine`, not a `PalmMute`.

So MuseScore 4.7.5 cannot round-trip even its own palm mute through MusicXML. No encoding we could choose would keep it.

String-percussive: SMuFL has no glyph for a pitched percussive string hit, only percussion pictograms. So no `smufl` reference would be truthful, and the counted extension stays (ADR-035 section 9).

### Current encoding (after the investigation)

The exporter writes palm mute playback natively, as `<play><mute>palm</mute></play>` on each palm-muted note. It keeps the per-note `<other-technical>palm mute</other-technical>` mark, counted, because 4.0 has no palm-mute notation element. A stroke on a rest is now the strum direction above. The other mappings are unchanged.

Full output: [musescore/summary-6a1-interop-2026-11-09.txt](musescore/summary-6a1-interop-2026-11-09.txt).

| Technique (our encoding) | Kind | Ours | MuseScore import | MuseScore re-export | Result |
|---|---|---|---|---|---|
| Accent (`<articulations><accent/>`) | native | 11 notes on 3 beats | 3 accent articulations, one per chord | 3, on the chord's first note | Kept per beat. MuseScore stores articulations per chord, so a chord where only some notes are accented cannot be told apart. Our score keeps the per-note flag |
| Down-stroke on notes (`<technical><down-bow/>`) | native | 2 | 2 | 2 | Kept |
| Up-stroke (`<technical><up-bow/>`) | native | 1 | 1 | 1 | Kept |
| Stroke on a rest (`<other-direction smufl="guitarStrumDown"/>`) | native mechanism (SMuFL glyph) | 1 | **1 symbol, drawn at the rest** | 0 | **Kept on import, lost on re-export** (MuseScore loss) |
| `x` notehead (dead, string-percussive) | native | 7 | 7 cross noteheads | 7 | Kept in the file. On TAB MuseScore draws digits |
| Palm mute playback (`<play><mute>palm</mute></play>`) | native | 4 | ignored | 0 | **Lost** (MuseScore loss) |
| Palm mute mark (`<other-technical>palm mute</other-technical>`) | our approximation, counted | 4 | ignored | 0 | **Lost** (MuseScore loss) |
| String-percussive mark (`<other-technical>string percussive</other-technical>`) | our approximation, counted | 6 | ignored | 0 | **Lost**. The `x` notehead stays, so MuseScore reads a dead note |

![MuseScore rendering of the techniques fixture, current encoding](musescore/techniques-preview.png)

The rendering shows the accents, the down-bow and up-bow symbols, the let-ring ties and, new, the down-strum arrow above the rest in bar 2. It shows no palm-mute or string-percussive text.

**TQ-8: MuseScore opens the file, and the result for each technique is recorded.** After MuseScore's MusicXML re-export:
- Palm mute (both its native playback and its mark) and string-percussive are lost.
- The rest stroke is lost too, although MuseScore now imports and draws it.
- Per-note accents in a chord collapse to one accent per chord.

**This is still an evidence gap, not a pass.** The remaining losses are MuseScore behaviour: the file is valid MusicXML 4.0 and uses the native element wherever one exists. The encoding is not bent to suit MuseScore.