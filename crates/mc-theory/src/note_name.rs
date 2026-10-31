//! Note names without octave (letter + alteration), for chord roots and bass notes.

use crate::{Alter, Letter, PitchClass, TheoryError};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NoteName {
    letter: Letter,
    alter: Alter,
}

const LETTERS: [Letter; 7] = [
    Letter::C,
    Letter::D,
    Letter::E,
    Letter::F,
    Letter::G,
    Letter::A,
    Letter::B,
];

impl NoteName {
    pub const fn new(letter: Letter, alter: Alter) -> Self {
        NoteName { letter, alter }
    }

    pub const fn letter(self) -> Letter {
        self.letter
    }

    pub const fn alter(self) -> Alter {
        self.alter
    }

    pub fn pitch_class(self) -> PitchClass {
        PitchClass::from_semitone(self.letter.natural_semitone() + self.alter.semitones() as i32)
    }

    /// Canonical display spelling of a pitch class when no key or harmonic context
    /// decides it (Phase 2 convention): naturals, and C#, Eb, F#, Ab, Bb for the black
    /// keys, a deterministic guitar-chart choice (C#m rather than Dbm). This is display
    /// only; pitch identity is the pitch class. Contextual spelling can replace it later.
    pub fn default_for(pc: PitchClass) -> Self {
        const TABLE: [(Letter, i8); 12] = [
            (Letter::C, 0),
            (Letter::C, 1),
            (Letter::D, 0),
            (Letter::E, -1),
            (Letter::E, 0),
            (Letter::F, 0),
            (Letter::F, 1),
            (Letter::G, 0),
            (Letter::A, -1),
            (Letter::A, 0),
            (Letter::B, -1),
            (Letter::B, 0),
        ];
        let (letter, alter) = TABLE[pc.value() as usize];
        NoteName {
            letter,
            alter: Alter::new(alter).expect("table values are in range"),
        }
    }

    /// The note `steps` letters above this one whose pitch class is `self + semitones`.
    /// `None` if that needs more than a double sharp or flat (T-02).
    pub fn up(self, steps: u8, semitones: u8) -> Option<NoteName> {
        let from = LETTERS
            .iter()
            .position(|l| *l == self.letter)
            .expect("all letters listed");
        let letter = LETTERS[(from + steps as usize) % 7];
        let base = self.pitch_class().value() as i32 + semitones as i32;
        let natural = letter.natural_semitone();
        let mut alter = (base - natural).rem_euclid(12);
        if alter > 6 {
            alter -= 12;
        }
        Alter::new(alter as i8)
            .ok()
            .map(|a| NoteName { letter, alter: a })
    }
}

impl fmt::Display for NoteName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = crate::SpelledPitch::new(self.letter, self.alter, 0).to_string();
        f.write_str(p.trim_end_matches('0'))
    }
}

impl FromStr for NoteName {
    type Err = TheoryError;

    /// Parses `C`, `F#`, `Bb`, `C##`, `Ebb`. The letter must be upper case and
    /// nothing may follow the accidentals (no octave number).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || TheoryError::InvalidPitch(s.to_string());
        let mut chars = s.chars();
        let first = chars.next().ok_or_else(bad)?;
        if !first.is_ascii_uppercase() {
            return Err(bad());
        }
        let letter = Letter::from_char(first).ok_or_else(bad)?;
        let rest = chars.as_str();
        let alter = match rest {
            "" => 0,
            "#" => 1,
            "##" => 2,
            "b" => -1,
            "bb" => -2,
            _ => return Err(bad()),
        };
        Ok(NoteName {
            letter,
            alter: Alter::new(alter)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> NoteName {
        s.parse().unwrap()
    }

    #[test]
    fn parse_display_and_pitch_class() {
        for s in ["C", "F#", "Bb", "C##", "Ebb"] {
            assert_eq!(n(s).to_string(), s);
        }
        assert_eq!(n("Bb").pitch_class().value(), 10);
        assert_eq!(n("B#").pitch_class().value(), 0);
        assert!("H".parse::<NoteName>().is_err());
        assert!("C4".parse::<NoteName>().is_err());
    }

    #[test]
    fn spelled_steps() {
        assert_eq!(n("C").up(2, 3).unwrap().to_string(), "Eb");
        assert_eq!(n("C").up(6, 9).unwrap().to_string(), "Bbb");
        assert_eq!(n("F#").up(2, 4).unwrap().to_string(), "A#");
        assert_eq!(n("B").up(1, 2).unwrap().to_string(), "C#");
        // Fb dim7 would need Ebbb (triple flat): not representable (T-02).
        assert!(n("Fb").up(6, 9).is_none());
        assert!("c".parse::<NoteName>().is_err());
        assert!("C#b".parse::<NoteName>().is_err());
    }

    #[test]
    fn default_spelling() {
        let names: Vec<String> = (0..12)
            .map(|i| NoteName::default_for(PitchClass::from_semitone(i)).to_string())
            .collect();
        assert_eq!(
            names,
            [
                "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"
            ]
        );
    }
}
