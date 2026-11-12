# ADR-040 Boundary between mc-theory, mc-notation and rendering; spelling and accidentals in mc-theory (Phase 6c)

Status: Accepted (user, 2026-11-12), including the spelling rule: the user chose **option A** of section 5 on 2026-11-12. ADR-033 choice 5.3 is superseded for spelling by this ADR.
Date: 2026-11-12
Related documents: [ADR-002](ADR-002-rust-core-source-of-truth.md), [ADR-018](ADR-018-notation-layout-in-rust.md), [ADR-033](ADR-033-musicxml-export-adapter.md) (choice 5.3), [ADR-038](ADR-038-notation-staff-model.md), [ADR-039](ADR-039-pitch-capo-and-written-transposition.md), [dependency-rules.md](../dependency-rules.md) (GR-05, GR-06), `spikes/s11-notation/README.md` (section 4.2)

## Context

Standard notation needs four musical decisions per note: the spelling (letter and alteration), the accidental shown, the spelling of a tied continuation, and the written pitch. These are music rules. They must not be decided in layout code, in an adapter or in Dart (GR-05), and every consumer must get the same answer. The consumers are the screen, the PDF, the MusicXML export and later other instruments.

Today `mc-theory` has `KeySignature::spell` (ADR-033 choice 5.3), used by the MusicXML export. S11 used a different spelling rule in spike code, and kept the accidental state in the spike's own module.

The dependency matrix allows `notation -> theory, score`. `theory` depends on nothing.

## Decision

1. **`mc-theory` owns the rules**, as pure functions and values with no score and no geometry:
   - spelling of a sounding pitch in a key (section 5);
   - a tied continuation keeps its source's spelling while the semitone is the same;
   - the key signature's alteration of each letter;
   - **the accidental state of a bar**. Given the key and the bar's notes (onset, spelled written pitch, tie role: none, within the bar, across the bar line), it returns the accidental shown for each note.
     - The order (onset, then low to high), the reset at the bar line, letter-and-octave scope and the tie rules are all inside this function.
     - A tie across the bar line shows nothing and sets no state.
   - the written transposition value (ADR-039).
2. **`mc-notation` has two layers.**
   - **Semantic** (no font, no geometry). It reads the score and calls `mc-theory`; it collects inputs and records outputs. Per note it produces a notated note: written spelled pitch, accidental, tie role, voice and onset.
   - **Layout.** It turns notated notes into staff positions, stems, beams, glyphs, spacing and the display list.
   - Neither layer decides a spelling or an accidental. `mc-notation` does not call the semitone-spelling functions or keep accidental state of its own. A source-scan test in `mc-notation` enforces this.
3. **The MusicXML export uses the same `mc-theory` functions** for spelling and accidentals, so the screen, the PDF and MusicXML agree.
4. **Rendering (Flutter) paints only.**
   - It paints the display list and does read-only hit testing on regions Rust computed (ADR-041 decision 4).
   - It holds no musical rule: no pitch, spelling, accidental, staff position or spacing.
   - The PDF adapter paints the same layout model.
5. **Instrument independence.** The semantic layer's input is sounding pitch, key, onset and tie role, nothing fretted. A future piano or bass reuses it unchanged and adds its own staff kinds (ADR-038). Fretted specifics stay in `pitch_at` (ADR-039). 6c implements nothing for other instruments.
6. **Tests at each level.**
   - `mc-theory`: rule tests for spelling and accidental state.
   - `mc-notation` semantic: per-fixture notated notes as goldens, plus the S11 N1 oracle.
   - Layout: N2, N3 and bounds (ADR-041 decision 6).

## 5. Spelling rule (decided: option A, user, 2026-11-12)

Checked before the choice: no MusicXML export golden has a key with 6 or 7 accidentals, so no export golden changes. One `mc-theory` unit test changes with the behaviour. It is `spelling_keeps_the_semitone_and_the_octave_boundaries` in `key.rs`, which asserts that C♭ major spells semitone 59 as B3. Under option A, 59 in C♭ major is C♭4 (in key), and the octave number follows the letter. The test is rewritten to the new rule in 6c-1 and still checks the semitone and the octave boundary. It is a decided change of behaviour, not a weakened test.

The existing rule (ADR-033 choice 5.3) spells by key side: sharps in sharp and neutral keys, flats in flat keys, naturals as naturals. It cannot spell an in-key note that needs another letter, such as E♯ in F♯ major or C♭ in G♭ major. S11 used scale degrees first, then the same fallback.

The two agree in every key with at most 5 sharps or flats. They differ only on the in-key notes E♯ and B♯ (6 or 7 sharps) and C♭ and F♭ (6 or 7 flats).

Example: F♯ major, sounding semitone F.
- The existing rule writes F with a natural sign, because F is sharp in the key.
- The scale-degree rule writes E♯ with no sign, as the key signature implies.

| Option | Effect |
|---|---|
| **A (recommended): the scale-degree rule replaces `KeySignature::spell` for every consumer** | Correct notation in all 15 keys. The MusicXML export changes for those 4 in-key notes in keys with 6 or 7 accidentals. Whether any export golden has such a key is checked in 6c-1. Any changed golden is listed for approval. ADR-033 choice 5.3 is superseded by this choice. |
| B: keep the existing rule for every consumer | No export change. Notation in keys with 6 or 7 accidentals shows needless naturals (F♮ for E♯). The S11 N1 rule is changed to match. |

Not offered: different rules for notation and export, because the same score would be spelled two ways (decision 3).

## 6. Slice 6c-1 acceptance criteria (fixed 2026-11-12, before any code)

Scope: the `mc-theory` rules of decision 1, `Transposition` and `InstrumentSpec::written_transposition()` (ADR-039), and the MusicXML export using the new spelling. Thresholds are not lowered to match a result.

| Id | Criterion | Checked by |
|---|---|---|
| T1 | `KeySignature::spell` (option A).<br>– In each of the 15 keys, every semitone from -12 to 140 keeps its semitone.<br>– A scale degree is spelled with the key's letter and alteration.<br>– Any other note is spelled by the sharp or flat table of ADR-033 5.3.<br>– Cases: E♯4 in F♯ major; B♯3 and E♯4 in C♯ major; C♭4 in G♭ major; C♭4 and F♭4 in C♭ major; mode does not change spelling. | Unit and property tests |
| T2 | `KeySignature::alteration(letter)`: the order of sharps (F C G D A E B) and flats (B E A D G C F), and as many altered letters as fifths, for all 15 keys | Unit test |
| T3 | Bar accidental state (`bar_accidentals`).<br>– The result for each note does not depend on the order of the input.<br>– Notes are ordered by onset, then low to high.<br>– Scope is letter and octave; the state starts from the key every bar.<br>– A natural after an alteration shows ♮; the same alteration again shows nothing.<br>– A tie within the bar shows nothing and sets the state.<br>– A tie across the bar line shows nothing and sets no state.<br>– The cases of the S11 `keys` input.<br>– An independent oracle (written separately, as in S11 N1) agrees on random bars. | Unit and property tests |
| T4 | Tie continuation: the source spelling is kept while the semitone is the same; otherwise the key's spelling | Unit test |
| T5 | `Transposition`: an octave shift keeps letter and alteration, and overflow is an error, not a wrap. `InstrumentSpec::written_transposition()` is one octave up for fretted instruments. On every `mc-score` fixture, the written semitone equals `pitch_at` + 12 for every note (no doubled capo or octave), with a capo case. | Unit and fixture tests |
| T6 | The MusicXML export uses the new `spell`.<br>– Its three goldens are unchanged (none has 6 or 7 accidentals; checked before the change).<br>– A new test exports F♯ major with an in-key E♯. | `mc-musicxml` tests |
| T7 | No new dependency, no `unsafe`, no I/O in `mc-theory` (FF-03, FF-04, FF-34). Public API changes are additions only, plus the decided behaviour change of `spell` (FF-22). The whole workspace builds and tests pass, clippy clean. | CI and the fitness functions |

**Results (2026-11-12).**

| Id | Result |
|---|---|
| T1 | Pass. In-key cases (E♯4, B♯3, C♭4, F♭4), all 15 keys over -12 to 140, mode independence. Up to 5 accidentals, the new rule equals the old one on every semitone. The octave-boundary test was rewritten as decided in section 5. |
| T2 | Pass, all 15 keys. |
| T3 | Pass. Unit cases, including the S11 `keys` bars, plus two property tests: agreement with a separately written look-back oracle, and per-note independence from input order. A planted fault (ties across the bar line counted in the state) was caught by `t3_ties` and the oracle, then removed. |
| T4 | Pass. |
| T5 | Pass, on SMALL, TYPICAL, `feature_rich` (capo 2 track) and `techniques`, plus the ADR-039 capo example. |
| T6 | Pass. The three goldens are unchanged; the new F♯ major test exports E♯4. |
| T7 | Pass: fmt, clippy `-D warnings`, `cargo test --workspace --all-features` and FF-01, FF-03, FF-04, FF-20, FF-33, FF-34 locally. FF-22 snapshots of `mc-theory` and `mc-score`: 79 lines added, 0 removed. |

One rule was made precise while writing the code. In S11, two notes on one step at the same onset (a unison across voices, or F with F♯) got a result that depended on their order. Now every note at one onset is compared with the state before that onset. On a clash both notes show their sign, and the step stays ambiguous, so the next note on it shows its sign. This rule is the one T3 checks.

Not in 6c-1, and kept for 6c-2:
- the `mc-notation` semantic layer;
- tie continuation and accidentals inside the MusicXML export (it writes spelled pitches today and no `<accidental>` element).

## Alternatives

- **Accidental state in `mc-notation`**, as in the spike. Rejected by the user: a music rule in layout code, and not reusable by the export.
- **Spelling stored per note in the score.** Not needed for 6c. A per-note spelling override is mentioned in bounded-contexts as a future option. It would need its own ADR and a format change.

## Trade-offs

- The accidental function takes a whole bar, not one note at a time, so it can own the order. Callers collect a bar first; that is cheap and cached with the bar plan (ADR-041).

## Consequences

- `mc-theory`: a notation module (spelling as chosen in section 5, tie continuation, key alteration, bar accidental state), with tests ported from the S11 spike and its oracle.
- `mc-notation`: a semantic module and the source-scan test.
- `mc-musicxml`: switches to the shared functions. Any golden changes are shown to the user (option A only).

## Related fitness functions

FF-01 (matrix unchanged), FF-03, FF-04, FF-22 (additions only, or the reviewed supersession of `KeySignature::spell` under option A).

## Reconsideration conditions

- A user wants per-note spelling control.
- Double sharps or flats beyond the key's own (theoretical keys).
