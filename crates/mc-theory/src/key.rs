use crate::{SpelledPitch, TheoryError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Major,
    Minor,
}

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

    /// Spells a semitone number for this key (ADR-033 choice 5.3): with sharps when the
    /// key has no accidentals or has sharps, with flats when it has flats. Naturals are
    /// spelled as naturals in every key. This is the simple rule the user chose; it does
    /// not spell in-key notes such as E# in F# major or Cb in Gb major.
    pub fn spell(self, semitone: i32) -> SpelledPitch {
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
        // B and C sit on the octave boundary and are never spelled as Cb or B#.
        let flat = KeySignature::new(-7, Mode::Major).unwrap();
        assert_eq!(flat.spell(59).to_string(), "B3");
        assert_eq!(flat.spell(60).to_string(), "C4");
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
}
