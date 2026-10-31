//! Spelled pitches (letter, alteration, octave). Not bounded by MIDI range (T-03);
//! MIDI is a playback concern (D-05).

use crate::TheoryError;
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Letter {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl Letter {
    pub const fn natural_semitone(self) -> i32 {
        match self {
            Letter::C => 0,
            Letter::D => 2,
            Letter::E => 4,
            Letter::F => 5,
            Letter::G => 7,
            Letter::A => 9,
            Letter::B => 11,
        }
    }

    fn from_char(c: char) -> Option<Self> {
        Some(match c.to_ascii_uppercase() {
            'C' => Letter::C,
            'D' => Letter::D,
            'E' => Letter::E,
            'F' => Letter::F,
            'G' => Letter::G,
            'A' => Letter::A,
            'B' => Letter::B,
            _ => return None,
        })
    }

    const fn as_char(self) -> char {
        match self {
            Letter::C => 'C',
            Letter::D => 'D',
            Letter::E => 'E',
            Letter::F => 'F',
            Letter::G => 'G',
            Letter::A => 'A',
            Letter::B => 'B',
        }
    }
}

/// Chromatic alteration in semitones, -2 (double flat) to +2 (double sharp) (T-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Alter(i8);

impl Alter {
    pub const NATURAL: Alter = Alter(0);

    pub fn new(semitones: i8) -> Result<Self, TheoryError> {
        if !(-2..=2).contains(&semitones) {
            return Err(TheoryError::AlterOutOfRange(semitones));
        }
        Ok(Alter(semitones))
    }

    pub const fn semitones(self) -> i8 {
        self.0
    }
}

/// Pitch class 0..=11 (C = 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PitchClass(u8);

impl PitchClass {
    pub fn from_semitone(semitone: i32) -> Self {
        PitchClass(semitone.rem_euclid(12) as u8)
    }

    pub const fn value(self) -> u8 {
        self.0
    }
}

/// Letter + alteration + scientific octave (C4 = middle C). Enharmonic spelling is
/// preserved: F#4 and Gb4 are different values with the same semitone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpelledPitch {
    letter: Letter,
    alter: Alter,
    octave: i8,
}

impl SpelledPitch {
    pub const fn new(letter: Letter, alter: Alter, octave: i8) -> Self {
        SpelledPitch {
            letter,
            alter,
            octave,
        }
    }

    pub const fn letter(self) -> Letter {
        self.letter
    }

    pub const fn alter(self) -> Alter {
        self.alter
    }

    pub const fn octave(self) -> i8 {
        self.octave
    }

    /// Semitone number on the MIDI-compatible scale (C-1 = 0, A4 = 69), unbounded.
    pub const fn semitone(self) -> i32 {
        (self.octave as i32 + 1) * 12 + self.letter.natural_semitone() + self.alter.0 as i32
    }

    pub fn pitch_class(self) -> PitchClass {
        PitchClass::from_semitone(self.semitone())
    }

    /// Default spelling of a semitone number, using sharps.
    pub fn from_semitone_sharp(semitone: i32) -> Self {
        const TABLE: [(Letter, i8); 12] = [
            (Letter::C, 0),
            (Letter::C, 1),
            (Letter::D, 0),
            (Letter::D, 1),
            (Letter::E, 0),
            (Letter::F, 0),
            (Letter::F, 1),
            (Letter::G, 0),
            (Letter::G, 1),
            (Letter::A, 0),
            (Letter::A, 1),
            (Letter::B, 0),
        ];
        let (letter, alter) = TABLE[semitone.rem_euclid(12) as usize];
        let octave = semitone.div_euclid(12) - 1;
        SpelledPitch {
            letter,
            alter: Alter(alter),
            octave: octave.clamp(i8::MIN as i32, i8::MAX as i32) as i8,
        }
    }
}

impl fmt::Display for SpelledPitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let acc = match self.alter.0 {
            -2 => "bb",
            -1 => "b",
            1 => "#",
            2 => "##",
            _ => "",
        };
        write!(f, "{}{}{}", self.letter.as_char(), acc, self.octave)
    }
}

impl FromStr for SpelledPitch {
    type Err = TheoryError;

    /// Parses `E4`, `F#3`, `Bb2`, `C##5`, `Ebb-1`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || TheoryError::InvalidPitch(s.to_string());
        let mut chars = s.chars();
        let letter = chars.next().and_then(Letter::from_char).ok_or_else(bad)?;
        let rest: String = chars.collect();
        let (alter, octave_text) = if let Some(r) = rest.strip_prefix("##") {
            (2, r)
        } else if let Some(r) = rest.strip_prefix("bb") {
            (-2, r)
        } else if let Some(r) = rest.strip_prefix('#') {
            (1, r)
        } else if let Some(r) = rest.strip_prefix('b') {
            (-1, r)
        } else {
            (0, rest.as_str())
        };
        let octave: i8 = octave_text.parse().map_err(|_| bad())?;
        Ok(SpelledPitch {
            letter,
            alter: Alter(alter),
            octave,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> SpelledPitch {
        s.parse().unwrap()
    }

    #[test]
    fn semitone_numbers_follow_midi_numbering() {
        assert_eq!(p("A4").semitone(), 69);
        assert_eq!(p("C4").semitone(), 60);
        assert_eq!(p("E2").semitone(), 40);
        assert_eq!(p("C-1").semitone(), 0);
    }

    #[test]
    fn t02_alteration_range() {
        assert!(Alter::new(2).is_ok());
        assert_eq!(Alter::new(3), Err(TheoryError::AlterOutOfRange(3)));
        assert_eq!(Alter::new(-3), Err(TheoryError::AlterOutOfRange(-3)));
    }

    #[test]
    fn t03_pitches_outside_midi_range_are_representable() {
        assert_eq!(p("C-2").semitone(), -12);
        assert_eq!(p("C10").semitone(), 132);
    }

    #[test]
    fn enharmonic_spelling_is_preserved() {
        assert_ne!(p("F#4"), p("Gb4"));
        assert_eq!(p("F#4").semitone(), p("Gb4").semitone());
        assert_eq!(p("B#3").semitone(), p("C4").semitone());
    }

    #[test]
    fn parse_display_roundtrip() {
        for s in ["E4", "F#3", "Bb2", "C##5", "Ebb-1", "G0"] {
            assert_eq!(p(s).to_string(), s);
        }
        assert!("H4".parse::<SpelledPitch>().is_err());
        assert!("C".parse::<SpelledPitch>().is_err());
        assert!("C#x".parse::<SpelledPitch>().is_err());
    }

    #[test]
    fn default_sharp_spelling() {
        assert_eq!(SpelledPitch::from_semitone_sharp(61).to_string(), "C#4");
        assert_eq!(SpelledPitch::from_semitone_sharp(-1).to_string(), "B-2");
        assert_eq!(PitchClass::from_semitone(-1).value(), 11);
    }
}
