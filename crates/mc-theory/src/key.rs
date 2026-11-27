use crate::{Alter, Letter, SpelledPitch, TheoryError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Major,
    Minor,
}

/// The order of sharps in a key signature; flats are the reverse.
const SHARP_ORDER: [Letter; 7] = [
    Letter::F,
    Letter::C,
    Letter::G,
    Letter::D,
    Letter::A,
    Letter::E,
    Letter::B,
];

/// Key signature as a position on the circle of fifths: -7 (seven flats) to +7.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeySignature {
    fifths: i8,
    mode: Mode,
}

impl KeySignature {
    pub const C_MAJOR: KeySignature = KeySignature {
        fifths: 0,
        mode: Mode::Major,
    };

    pub fn new(fifths: i8, mode: Mode) -> Result<Self, TheoryError> {
        if !(-7..=7).contains(&fifths) {
            return Err(TheoryError::InvalidKey(fifths));
        }
        Ok(KeySignature { fifths, mode })
    }

    pub const fn fifths(self) -> i8 {
        self.fifths
    }

    pub const fn mode(self) -> Mode {
        self.mode
    }

    /// The letters this key signature alters, in the order its signs are written:
    /// sharps F C G D A E B, flats B E A D G C F.
    pub fn signs(self) -> Vec<Letter> {
        let n = self.fifths.unsigned_abs() as usize;
        if self.fifths >= 0 {
            SHARP_ORDER[..n].to_vec()
        } else {
            SHARP_ORDER.iter().rev().take(n).copied().collect()
        }
    }

    /// The signs of this key that a change to `next` cancels with a natural, in this
    /// key's order (ADR-041 section 13, EP3). A sign is cancelled when its letter is
    /// natural in `next`. A letter that `next` alters again (sharp to flat, or the same
    /// sign) needs no natural: the new signature states it. This is the usual modern
    /// convention (Gould, *Behind Bars*), so a change to C major cancels every sign,
    /// and D major (F♯ C♯) to F major (B♭) cancels F and C.
    pub fn cancelled_by(self, next: KeySignature) -> Vec<Letter> {
        self.signs()
            .into_iter()
            .filter(|l| next.alteration(*l) == Alter::NATURAL)
            .collect()
    }

    /// The alteration this key signature gives `letter`: sharps in the order F C G D A E
    /// B, flats in the order B E A D G C F, as many as `fifths` says.
    pub fn alteration(self, letter: Letter) -> Alter {
        let n = self.fifths.unsigned_abs() as usize;
        let altered = if self.fifths > 0 {
            SHARP_ORDER[..n].contains(&letter)
        } else {
            SHARP_ORDER[7 - n..].contains(&letter)
        };
        match (altered, self.fifths > 0) {
            (false, _) => Alter::NATURAL,
            (true, true) => Alter::SHARP,
            (true, false) => Alter::FLAT,
        }
    }

    /// Spells a semitone number for this key (ADR-040 section 5, option A):
    /// - a degree of the key's scale is spelled as the key spells it (E#4 in F# major,
    ///   Cb4 in Gb major; the octave number follows the letter);
    /// - any other note takes a sharp when the key has no accidentals or has sharps, a
    ///   flat when it has flats (the table of ADR-033 choice 5.3).
    ///
    /// The mode does not change the spelling. The semitone is always kept.
    pub fn spell(self, semitone: i32) -> SpelledPitch {
        let pc = semitone.rem_euclid(12);
        for letter in Letter::ALL {
            let alter = self.alteration(letter);
            let base = letter.natural_semitone() + alter.semitones() as i32;
            if base.rem_euclid(12) == pc {
                return SpelledPitch::with_semitone(letter, alter, semitone);
            }
        }
        if self.fifths < 0 {
            SpelledPitch::from_semitone_flat(semitone)
        } else {
            SpelledPitch::from_semitone_sharp(semitone)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(key: KeySignature, from: i32) -> Vec<String> {
        (from..from + 12)
            .map(|s| key.spell(s).to_string())
            .collect()
    }

    #[test]
    fn spelling_in_a_key_without_accidentals_uses_sharps() {
        let c = KeySignature::C_MAJOR;
        assert_eq!(
            names(c, 60),
            [
                "C4", "C#4", "D4", "D#4", "E4", "F4", "F#4", "G4", "G#4", "A4", "A#4", "B4"
            ]
        );
        // A minor has no accidentals either.
        let a_minor = KeySignature::new(0, Mode::Minor).unwrap();
        assert_eq!(names(a_minor, 60), names(c, 60));
    }

    #[test]
    fn spelling_in_sharp_keys_uses_sharps() {
        for fifths in 1..=7 {
            let k = KeySignature::new(fifths, Mode::Major).unwrap();
            assert_eq!(k.spell(66).to_string(), "F#4", "fifths {fifths}");
            assert_eq!(k.spell(70).to_string(), "A#4", "fifths {fifths}");
            assert_eq!(k.spell(64).to_string(), "E4", "naturals stay natural");
        }
    }

    #[test]
    fn spelling_in_flat_keys_uses_flats() {
        let e_flat_minor = KeySignature::new(-3, Mode::Minor).unwrap();
        assert_eq!(
            names(e_flat_minor, 60),
            [
                "C4", "Db4", "D4", "Eb4", "E4", "F4", "Gb4", "G4", "Ab4", "A4", "Bb4", "B4"
            ]
        );
        for fifths in -7..=-1 {
            let k = KeySignature::new(fifths, Mode::Major).unwrap();
            assert_eq!(k.spell(70).to_string(), "Bb4", "fifths {fifths}");
        }
    }

    #[test]
    fn spelling_keeps_the_semitone_and_the_octave_boundaries() {
        for fifths in -7..=7 {
            let k = KeySignature::new(fifths, Mode::Major).unwrap();
            for s in -12..140 {
                assert_eq!(k.spell(s).semitone(), s, "fifths {fifths} semitone {s}");
            }
        }
        // B and C sit on the octave boundary. In-key Cb and B# take the octave number of
        // their letter (ADR-040 section 5, option A): Cb4 sounds as B3, B#3 as C4.
        let c_flat = KeySignature::new(-7, Mode::Major).unwrap();
        assert_eq!(c_flat.spell(59).to_string(), "Cb4");
        assert_eq!(c_flat.spell(60).to_string(), "C4"); // C natural is not in the key
        let c_sharp = KeySignature::new(7, Mode::Major).unwrap();
        assert_eq!(c_sharp.spell(60).to_string(), "B#3");
        assert_eq!(c_sharp.spell(59).to_string(), "B3"); // B natural is not in the key
    }

    fn key(fifths: i8) -> KeySignature {
        KeySignature::new(fifths, Mode::Major).unwrap()
    }

    #[test]
    fn t1_in_key_notes_take_the_key_spelling() {
        assert_eq!(key(6).spell(65).to_string(), "E#4"); // F# major
        assert_eq!(key(7).spell(60).to_string(), "B#3"); // C# major
        assert_eq!(key(7).spell(65).to_string(), "E#4");
        assert_eq!(key(-6).spell(59).to_string(), "Cb4"); // Gb major
        assert_eq!(key(-7).spell(59).to_string(), "Cb4"); // Cb major
        assert_eq!(key(-7).spell(64).to_string(), "Fb4");
        // Up to five accidentals the old and the new rule agree on every semitone.
        for fifths in -5..=5 {
            let k = key(fifths);
            for s in 0..128 {
                let old = if fifths < 0 {
                    SpelledPitch::from_semitone_flat(s)
                } else {
                    SpelledPitch::from_semitone_sharp(s)
                };
                assert_eq!(k.spell(s), old, "fifths {fifths} semitone {s}");
            }
        }
    }

    #[test]
    fn t1_scale_degrees_and_chromatic_notes() {
        for fifths in -7..=7 {
            let k = key(fifths);
            for s in -12..140 {
                let p = k.spell(s);
                let in_key = Letter::ALL.iter().any(|l| {
                    (l.natural_semitone() + k.alteration(*l).semitones() as i32).rem_euclid(12)
                        == s.rem_euclid(12)
                });
                if in_key {
                    assert_eq!(
                        p.alter(),
                        k.alteration(p.letter()),
                        "fifths {fifths} semitone {s}"
                    );
                } else if fifths < 0 {
                    assert_eq!(
                        p,
                        SpelledPitch::from_semitone_flat(s),
                        "fifths {fifths} semitone {s}"
                    );
                } else {
                    assert_eq!(
                        p,
                        SpelledPitch::from_semitone_sharp(s),
                        "fifths {fifths} semitone {s}"
                    );
                }
            }
        }
    }

    #[test]
    fn t1_mode_does_not_change_spelling() {
        for fifths in -7..=7 {
            let major = KeySignature::new(fifths, Mode::Major).unwrap();
            let minor = KeySignature::new(fifths, Mode::Minor).unwrap();
            for s in 0..128 {
                assert_eq!(major.spell(s), minor.spell(s));
            }
        }
    }

    #[test]
    fn t2_key_alterations_follow_the_order_of_sharps_and_flats() {
        use Letter::*;
        let sharps = [F, C, G, D, A, E, B];
        for fifths in -7i8..=7 {
            let k = key(fifths);
            let n = fifths.unsigned_abs() as usize;
            let altered: Vec<Letter> = if fifths >= 0 {
                sharps[..n].to_vec()
            } else {
                sharps.iter().rev().take(n).copied().collect()
            };
            for l in Letter::ALL {
                let expected = if !altered.contains(&l) {
                    Alter::NATURAL
                } else if fifths > 0 {
                    Alter::SHARP
                } else {
                    Alter::FLAT
                };
                assert_eq!(k.alteration(l), expected, "fifths {fifths} letter {l:?}");
            }
            let count = Letter::ALL
                .iter()
                .filter(|l| k.alteration(**l) != Alter::NATURAL)
                .count();
            assert_eq!(count, n);
        }
    }

    #[test]
    fn key_range() {
        assert!(KeySignature::new(7, Mode::Major).is_ok());
        assert!(KeySignature::new(-7, Mode::Minor).is_ok());
        assert_eq!(
            KeySignature::new(8, Mode::Major),
            Err(TheoryError::InvalidKey(8))
        );
    }

    #[test]
    fn signs_are_written_in_the_order_of_sharps_and_flats() {
        use Letter::*;
        assert_eq!(key(0).signs(), Vec::<Letter>::new());
        assert_eq!(key(3).signs(), vec![F, C, G]);
        assert_eq!(key(-4).signs(), vec![B, E, A, D]);
        assert_eq!(key(7).signs(), vec![F, C, G, D, A, E, B]);
        assert_eq!(key(-7).signs(), vec![B, E, A, D, G, C, F]);
    }

    /// EP3 (ADR-041 section 13): the cancelling naturals of a key change, by convention.
    #[test]
    fn ep3_cancelled_signs_by_convention() {
        use Letter::*;
        let cases: [(i8, i8, Vec<Letter>); 10] = [
            // To C major every sign is cancelled.
            (2, 0, vec![F, C]),
            (-3, 0, vec![B, E, A]),
            // Fewer signs of the same kind: the dropped ones.
            (4, 1, vec![C, G, D]),
            (-5, -2, vec![A, D, G]),
            // More signs of the same kind: nothing to cancel.
            (1, 4, vec![]),
            (-2, -6, vec![]),
            // Sharps to flats: the sharps whose letter the new key leaves natural.
            (2, -1, vec![F, C]),
            (7, -1, vec![F, C, G, D, A, E]),
            (-2, 3, vec![B, E]),
            // No change: nothing.
            (5, 5, vec![]),
        ];
        for (from, to, want) in cases {
            assert_eq!(key(from).cancelled_by(key(to)), want, "{from} to {to}");
        }
    }

    /// EP3: all 15 x 15 pairs. A cancelled letter was altered and becomes natural; a
    /// letter the old key alters is either cancelled or altered by the new key.
    #[test]
    fn ep3_cancellation_holds_for_all_key_pairs() {
        for from in -7..=7 {
            for to in -7..=7 {
                let (a, b) = (key(from), key(to));
                let c = a.cancelled_by(b);
                for l in &c {
                    assert_ne!(a.alteration(*l), Alter::NATURAL, "{from} {to} {l:?}");
                    assert_eq!(b.alteration(*l), Alter::NATURAL, "{from} {to} {l:?}");
                }
                for l in a.signs() {
                    assert!(
                        c.contains(&l) != (b.alteration(l) != Alter::NATURAL),
                        "{from} {to} {l:?}"
                    );
                }
                // In the old key's order, without repeats.
                let order: Vec<Letter> = a.signs().into_iter().filter(|l| c.contains(l)).collect();
                assert_eq!(c, order, "{from} {to}");
                // The mode never changes the result.
                let minor = KeySignature::new(from, Mode::Minor).unwrap();
                assert_eq!(minor.cancelled_by(b), c);
            }
        }
    }
}
