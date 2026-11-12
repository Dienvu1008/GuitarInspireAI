# ADR-039 Pitch semantics: sounding pitch with capo, written transposition from the instrument (Phase 6c)

Status: Accepted (user, 2026-11-12). The user chose sounding pitch with the capo included (S11 proposal 7.8, item 2).
Date: 2026-11-12
Related documents: [ADR-019](ADR-019-fretted-instrument-shared-kernel.md), [ADR-038](ADR-038-notation-staff-model.md), [ADR-040](ADR-040-theory-notation-rendering-boundary.md), [bounded-contexts.md](../bounded-contexts.md) (note model), [domain-invariants.md](../domain-invariants.md) (H-10), [audio-dsp-architecture.md](../audio-dsp-architecture.md) (rendering rule 7)

## Context

Standard notation needs a written pitch for every note. What the score stores today (checked in the code, not assumed):

- A fretted note stores `FretPosition { string, fret }`, and **no pitch**. The fret counts from the capo; the TAB shows it as stored.
- `mc_theory::fretted::pitch_at(tuning, capo, position)` = open pitch + capo + fret. It is the only place sounding pitch is computed. Playback (audio rendering rule 7) and the MusicXML export use it.
- `Score::set_instrument` changes tuning, fret count or capo and keeps every note's string and fret (H-03, H-04, H-10).
- The capo is part of `FrettedInstrument` in the score.

## Decision

1. **The score stays position-based.** No pitch is stored, and there is no format change. Sounding pitch is derived, only through `pitch_at`. The capo is therefore added **exactly once**, there.
2. **Sounding pitch includes the capo.** Example, standard tuning, first string (high E) open:

   | | Sounding | Written (treble 8vb) | TAB |
   |---|---|---|---|
   | No capo | E4 | E5 | 0 |
   | Capo at fret 2 | F♯4 | F♯5 | 0 |

   The sixth string open, with no capo, sounds E2 and is written E3.
3. **The written transposition belongs to the instrument definition, not to the view or the layout engine.**
   - In 6c the instrument definition derives it, without storing it: a fretted instrument is written one octave above sounding. This is true of guitar and of bass guitar.
   - The value type `Transposition` lives in `mc-theory`; `mc-score` provides `InstrumentSpec::written_transposition()`.
   - The notation engine applies it once. It never adds the capo, and it never computes pitch from string and fret itself.
   - The clef for guitar is `gClef8vb`. Clefs for other instruments come with those instruments.
4. **One notation mode in 6c: sounding pitch** (capo included, plus the instrument's octave). A later capo-relative mode, written as fingered without the capo, would be a view setting in the staff configuration (ADR-038). It needs its own ADR.

   Terminology: "concert pitch" in its strict sense means no octave transposition. That is not what 6c shows; 6c shows sounding pitch with the guitar's octave.
5. **Changing the capo keeps the fingering.** `set_instrument` keeps string and fret, so the TAB stays the same and the sounding pitch moves, as with a physical capo. This is the existing behaviour, now stated.
   - "Keep the sounding pitch" (move every fret by the capo difference) is a different user intention. Like retuning with kept pitches, it is a Guitar service that computes note edits and the application applies them as one command (bounded-contexts, note model). It is not part of 6c.
   - Neither operation changes anything else in the score silently.
6. **Cache dependency.** Tuning, capo and written transposition are inputs of every bar plan of the track (ADR-041 decision 3). A change re-plans the whole track.

## Alternatives

- **Store a sounding or written pitch per note.** Rejected: two sources of truth for one note, and a format change.
- **Transposition in the view.** That was the spike's design. Rejected by the user: the octave is a property of the instrument.
- **Written without the capo (capo-relative).** Deferred, decision 4.

## Trade-offs

- The notation for a capo piece shows the real key, not the shapes the player fingers. Players who read "in the shapes" need the later mode.

## Consequences

- `mc-theory`: `Transposition`. `mc-score`: `InstrumentSpec::written_transposition()`.
- `mc-notation` written-pitch tests check that a capo of n frets raises the written pitch by n semitones and that the octave comes from the instrument.
- A guard test: a written semitone equals `pitch_at` + 12 for every note of the fixtures, so a doubled capo or octave fails (the S11 N1 rule).

## Related fitness functions

FF-03, FF-04 (no I/O in the derivation). The S11 N1 checks become tests (ADR-041 decision 6).

## Reconsideration conditions

- A capo-relative notation mode is asked for.
- Non-fretted instruments (`NoteValue::Pitched`) or partial capos.
