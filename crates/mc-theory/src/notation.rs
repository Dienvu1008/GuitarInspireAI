//! Notation rules of the shared kernel (ADR-040 decision 1): the written transposition,
//! the spelling of a tied continuation, and the accidental state of a bar.
//!
//! These are pure rules on values. The notation layout (`mc-notation`) and the export
//! adapters collect their inputs from a score and use the results; they do not decide
//! spellings or accidentals themselves. Spelling of a sounding pitch in a key is
//! [`KeySignature::spell`].

use crate::{Alter, KeySignature, Rational, SpelledPitch, TheoryError};
use std::collections::BTreeMap;

/// An octave transposition between sounding and written pitch (ADR-039). Fretted
/// instruments are written one octave above sounding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Transposition {
    octaves: i8,
}

impl Transposition {
    /// Written as sounding.
    pub const NONE: Transposition = Transposition { octaves: 0 };
    /// Written one octave above sounding (guitar, bass guitar).
    pub const OCTAVE_UP: Transposition = Transposition { octaves: 1 };

    pub const fn new(octaves: i8) -> Self {
        Transposition { octaves }
    }

    pub const fn octaves(self) -> i8 {
        self.octaves
    }

    /// The written pitch of a sounding pitch: same letter and alteration, the octave
    /// moved. An octave outside the `i8` range is an error, not a wrap.
    pub fn written(self, sounding: SpelledPitch) -> Result<SpelledPitch, TheoryError> {
        let octave = sounding
            .octave()
            .checked_add(self.octaves)
            .ok_or(TheoryError::Overflow)?;
        Ok(SpelledPitch::new(
            sounding.letter(),
            sounding.alter(),
            octave,
        ))
    }
}

/// The spelling of a note a tie continues: the source's spelling while the semitone
/// is the same (a tie never changes the letter, also across a key change), otherwise
/// the key's spelling.
pub fn tie_continuation(key: KeySignature, source: SpelledPitch, sounding: i32) -> SpelledPitch {
    if source.semitone() == sounding {
        source
    } else {
        key.spell(sounding)
    }
}

/// How a note in a bar relates to a tie that ends on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TieRole {
    /// Not the end of a tie.
    NotTied,
    /// Continues a note earlier in the same bar.
    TiedInBar,
    /// Continues a note of the previous bar, across the bar line.
    TiedAcrossBarLine,
}

/// One note of a bar, as the accidental rule needs it: its onset in the bar, its
/// written spelled pitch and its tie role. All voices of one staff go into one call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarNote {
    pub onset: Rational,
    pub written: SpelledPitch,
    pub tie: TieRole,
}

/// The alteration in force on one step (letter and octave).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InForce {
    Known(Alter),
    /// Two different alterations sounded together on this step: the next note on it
    /// shows its sign whatever it is.
    Ambiguous,
}

/// The accidental to show for each note of one bar of one staff, in input order:
/// `None` for no sign, `Some(Alter::NATURAL)` for a natural sign.
///
/// Rules (ADR-040 decision 1):
/// - The state starts from the key signature in every bar; its scope is a letter in one
///   octave (a step).
/// - Notes are taken by onset. Every note at one onset is compared with the state
///   before that onset, so the order of the input and of the voices does not matter.
/// - A note shows its alteration when it differs from the one in force. When two
///   notes at one onset share a step with different alterations, both show theirs, and
///   the step stays ambiguous: the next note on it shows its sign.
/// - A tied note shows nothing. Tied within the bar it sets the state; tied across the
///   bar line it does not.
pub fn bar_accidentals(key: KeySignature, notes: &[BarNote]) -> Vec<Option<Alter>> {
    let mut by_onset: BTreeMap<Rational, Vec<usize>> = BTreeMap::new();
    for (i, n) in notes.iter().enumerate() {
        by_onset.entry(n.onset).or_default().push(i);
    }
    let mut state: BTreeMap<i32, InForce> = BTreeMap::new();
    let mut shown = vec![None; notes.len()];
    for group in by_onset.values() {
        // Alterations sounding on each step at this onset (ties across the bar line do
        // not count for the state).
        let mut here: BTreeMap<i32, Vec<Alter>> = BTreeMap::new();
        for &i in group {
            let n = &notes[i];
            if n.tie != TieRole::TiedAcrossBarLine {
                here.entry(n.written.diatonic_step())
                    .or_default()
                    .push(n.written.alter());
            }
        }
        for &i in group {
            let n = &notes[i];
            if n.tie != TieRole::NotTied {
                continue;
            }
            let step = n.written.diatonic_step();
            let alter = n.written.alter();
            let clash = here
                .get(&step)
                .is_some_and(|a| a.iter().any(|x| *x != alter));
            let before = state
                .get(&step)
                .copied()
                .unwrap_or(InForce::Known(key.alteration(n.written.letter())));
            if clash || before != InForce::Known(alter) {
                shown[i] = Some(alter);
            }
        }
        for (step, alters) in here {
            let first = alters[0];
            let entry = if alters.iter().all(|a| *a == first) {
                InForce::Known(first)
            } else {
                InForce::Ambiguous
            };
            state.insert(step, entry);
        }
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Letter, Mode};

    fn key(f: i8) -> KeySignature {
        KeySignature::new(f, Mode::Major).unwrap()
    }

    fn p(s: &str) -> SpelledPitch {
        s.parse().unwrap()
    }

    fn at(n: i64, d: i64, pitch: &str, tie: TieRole) -> BarNote {
        BarNote {
            onset: Rational::new(n, d).unwrap(),
            written: p(pitch),
            tie,
        }
    }

    fn plain(n: i64, pitch: &str) -> BarNote {
        at(n, 4, pitch, TieRole::NotTied)
    }

    const S: Option<Alter> = Some(Alter::SHARP);
    const F: Option<Alter> = Some(Alter::FLAT);
    const N: Option<Alter> = Some(Alter::NATURAL);

    #[test]
    fn t3_alterations_in_force_until_the_bar_line() {
        // C major: F#, F# again, F natural, F#5 (another octave is another step).
        let bar = [
            plain(0, "F#5"),
            plain(1, "F#5"),
            plain(2, "F5"),
            plain(3, "F#6"),
        ];
        assert_eq!(bar_accidentals(key(0), &bar), [S, None, N, S]);
        // G major: F# is in the key, F natural shows a natural, then F# shows its sharp.
        let bar = [plain(0, "F#5"), plain(1, "F5"), plain(2, "F#5")];
        assert_eq!(bar_accidentals(key(1), &bar), [None, N, S]);
    }

    #[test]
    fn t3_s11_keys_input() {
        // The S11 `keys` bars, written pitches (README 4.2).
        // F major: Bb (key), B natural, Bb (flat again), Eb.
        let bar = [
            plain(0, "Bb5"),
            plain(1, "B5"),
            plain(2, "Bb5"),
            plain(3, "Eb6"),
        ];
        assert_eq!(bar_accidentals(key(-1), &bar), [None, N, F, F]);
        // Bb major: Eb (key), E natural, Ab, A natural.
        let bar = [
            plain(0, "Eb5"),
            plain(1, "E5"),
            plain(2, "Ab5"),
            plain(3, "A5"),
        ];
        assert_eq!(bar_accidentals(key(-2), &bar), [None, N, F, N]);
        // E major, next bar: the tied D#6 shows nothing and sets no state; D natural
        // shows a natural; D# then shows its sharp; D natural again.
        let bar = [
            at(0, 4, "D#6", TieRole::TiedAcrossBarLine),
            plain(1, "D6"),
            plain(2, "D#6"),
            plain(3, "D6"),
        ];
        assert_eq!(bar_accidentals(key(4), &bar), [None, N, S, N]);
    }

    #[test]
    fn t3_ties() {
        // A tie within the bar shows nothing and keeps the state it continues.
        let bar = [
            plain(0, "F#5"),
            at(1, 4, "F#5", TieRole::TiedInBar),
            plain(2, "F#5"),
        ];
        assert_eq!(bar_accidentals(key(0), &bar), [S, None, None]);
        // Across the bar line: no sign, no state, so a later F# shows its sharp again.
        let bar = [at(0, 4, "F#5", TieRole::TiedAcrossBarLine), plain(1, "F#5")];
        assert_eq!(bar_accidentals(key(0), &bar), [None, S]);
    }

    #[test]
    fn t3_simultaneous_notes_on_one_step() {
        // F and F# together: both show, and the step is ambiguous afterwards.
        let bar = [plain(0, "F5"), plain(0, "F#5"), plain(1, "F#5")];
        assert_eq!(bar_accidentals(key(0), &bar), [N, S, S]);
        // A unison in two voices shows the same sign on both.
        let bar = [plain(0, "C#5"), plain(0, "C#5")];
        assert_eq!(bar_accidentals(key(0), &bar), [S, S]);
    }

    #[test]
    fn t3_in_key_spellings_show_nothing() {
        // F# major: E# is in the key.
        let bar = [plain(0, "E#5"), plain(1, "E5"), plain(2, "E#5")];
        assert_eq!(bar_accidentals(key(6), &bar), [None, N, S]);
    }

    #[test]
    fn t4_tie_continuation_keeps_the_source_spelling() {
        // An F# tied into a bar in F major stays F#, not Gb.
        assert_eq!(tie_continuation(key(-1), p("F#4"), 66), p("F#4"));
        // A different semitone takes the key's spelling.
        assert_eq!(tie_continuation(key(-1), p("F#4"), 68), p("Ab4"));
        // Across a key change into F# major, an F tied from C major stays F.
        assert_eq!(tie_continuation(key(6), p("F4"), 65), p("F4"));
    }

    #[test]
    fn t5_transposition() {
        assert_eq!(Transposition::OCTAVE_UP.written(p("E2")).unwrap(), p("E3"));
        assert_eq!(
            Transposition::OCTAVE_UP.written(p("Cb4")).unwrap(),
            p("Cb5")
        );
        assert_eq!(Transposition::NONE.written(p("F#4")).unwrap(), p("F#4"));
        let top = SpelledPitch::new(Letter::C, Alter::NATURAL, i8::MAX);
        assert_eq!(
            Transposition::OCTAVE_UP.written(top),
            Err(TheoryError::Overflow)
        );
        // The written semitone is 12 above the sounding one, for every spelling.
        for s in -12..140 {
            for f in -7..=7 {
                let sounding = key(f).spell(s);
                let w = Transposition::OCTAVE_UP.written(sounding).unwrap();
                assert_eq!(w.semitone(), s + 12);
                assert_eq!(
                    (w.letter(), w.alter()),
                    (sounding.letter(), sounding.alter())
                );
            }
        }
    }

    mod oracle {
        //! T3: an oracle written separately from the state machine. For each note it
        //! looks back at the latest earlier onset that sounded its step.
        use super::*;
        use proptest::prelude::*;

        pub fn expected(key: KeySignature, notes: &[BarNote]) -> Vec<Option<Alter>> {
            notes
                .iter()
                .map(|n| {
                    if n.tie != TieRole::NotTied {
                        return None;
                    }
                    let step = n.written.diatonic_step();
                    let counts = |m: &BarNote| {
                        m.tie != TieRole::TiedAcrossBarLine && m.written.diatonic_step() == step
                    };
                    // Same onset, same step, other alteration: always shown.
                    if notes.iter().any(|m| {
                        counts(m) && m.onset == n.onset && m.written.alter() != n.written.alter()
                    }) {
                        return Some(n.written.alter());
                    }
                    let latest = notes
                        .iter()
                        .filter(|m| counts(m) && m.onset < n.onset)
                        .map(|m| m.onset)
                        .max();
                    let in_force: Option<Alter> = match latest {
                        None => Some(key.alteration(n.written.letter())),
                        Some(o) => {
                            let alters: Vec<Alter> = notes
                                .iter()
                                .filter(|m| counts(m) && m.onset == o)
                                .map(|m| m.written.alter())
                                .collect();
                            alters.iter().all(|a| *a == alters[0]).then_some(alters[0])
                        }
                    };
                    (in_force != Some(n.written.alter())).then_some(n.written.alter())
                })
                .collect()
        }

        fn note() -> impl Strategy<Value = BarNote> {
            (0i64..6, 0usize..7, -1i8..=1, 4i8..=5, 0u8..10).prop_map(|(o, l, a, oct, t)| BarNote {
                onset: Rational::new(o, 8).unwrap(),
                written: SpelledPitch::new(Letter::ALL[l], Alter::new(a).unwrap(), oct),
                tie: match t {
                    0 => TieRole::TiedInBar,
                    1 => TieRole::TiedAcrossBarLine,
                    _ => TieRole::NotTied,
                },
            })
        }

        proptest! {
            #[test]
            fn t3_state_machine_agrees_with_the_oracle(
                fifths in -7i8..=7,
                notes in prop::collection::vec(note(), 0..24),
            ) {
                let k = key(fifths);
                prop_assert_eq!(bar_accidentals(k, &notes), expected(k, &notes));
            }

            #[test]
            fn t3_result_per_note_does_not_depend_on_input_order(
                fifths in -7i8..=7,
                notes in prop::collection::vec(note(), 1..24),
                seed in any::<u64>(),
            ) {
                let k = key(fifths);
                let base = bar_accidentals(k, &notes);
                // A deterministic shuffle from the seed.
                let mut order: Vec<usize> = (0..notes.len()).collect();
                let mut x = seed | 1;
                for i in (1..order.len()).rev() {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    order.swap(i, (x % (i as u64 + 1)) as usize);
                }
                let shuffled: Vec<BarNote> = order.iter().map(|i| notes[*i]).collect();
                let got = bar_accidentals(k, &shuffled);
                for (pos, i) in order.iter().enumerate() {
                    prop_assert_eq!(got[pos], base[*i]);
                }
            }
        }
    }
}
