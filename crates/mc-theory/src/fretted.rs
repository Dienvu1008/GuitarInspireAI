//! Fretted-instrument value objects (ADR-019). String numbering is physical and
//! 1-based: string 1 is the highest-pitched string of the standard layout. Numbers
//! never change when the tuning changes (Q-07).

use crate::{SpelledPitch, TheoryError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StringIndex(u8);

impl StringIndex {
    pub fn new(index: u8) -> Result<Self, TheoryError> {
        if index == 0 {
            return Err(TheoryError::InvalidStringIndex);
        }
        Ok(StringIndex(index))
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Fret number; 0 is the open string. The upper bound depends on the track (H-04).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fret(pub u8);

/// Capo position as a fret offset applied to all strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Capo(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FretPosition {
    pub string: StringIndex,
    pub fret: Fret,
}

/// Open-string pitches, string 1 first. At least one string (T-04).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tuning {
    strings: Vec<SpelledPitch>,
}

impl Tuning {
    pub fn new(strings: Vec<SpelledPitch>) -> Result<Self, TheoryError> {
        if strings.is_empty() {
            return Err(TheoryError::EmptyTuning);
        }
        Ok(Tuning { strings })
    }

    /// E4 B3 G3 D3 A2 E2.
    pub fn standard_guitar() -> Self {
        let s = ["E4", "B3", "G3", "D3", "A2", "E2"].map(|p| p.parse().expect("valid literal"));
        Tuning {
            strings: s.to_vec(),
        }
    }

    pub fn string_count(&self) -> u8 {
        self.strings.len().min(u8::MAX as usize) as u8
    }

    pub fn strings(&self) -> &[SpelledPitch] {
        &self.strings
    }

    pub fn open_pitch(&self, string: StringIndex) -> Option<SpelledPitch> {
        self.strings.get(string.0 as usize - 1).copied()
    }
}

/// Sounding semitone of a fretted position: open pitch + capo + fret.
/// `None` if the string does not exist in the tuning.
pub fn pitch_at(tuning: &Tuning, capo: Capo, position: FretPosition) -> Option<i32> {
    tuning
        .open_pitch(position.string)
        .map(|p| p.semitone() + capo.0 as i32 + position.fret.0 as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t04_tuning_needs_a_string() {
        assert_eq!(Tuning::new(vec![]), Err(TheoryError::EmptyTuning));
    }

    #[test]
    fn string_one_is_the_highest_and_numbering_is_physical() {
        let t = Tuning::standard_guitar();
        assert_eq!(
            t.open_pitch(StringIndex::new(1).unwrap())
                .unwrap()
                .to_string(),
            "E4"
        );
        assert_eq!(
            t.open_pitch(StringIndex::new(6).unwrap())
                .unwrap()
                .to_string(),
            "E2"
        );
        // A re-entrant tuning keeps physical numbering even though string 2 is higher.
        let reentrant = Tuning::new(vec!["E4".parse().unwrap(), "G4".parse().unwrap()]).unwrap();
        assert_eq!(
            reentrant
                .open_pitch(StringIndex::new(2).unwrap())
                .unwrap()
                .to_string(),
            "G4"
        );
        assert!(StringIndex::new(0).is_err());
    }

    #[test]
    fn pitch_at_includes_capo_and_fret() {
        let t = Tuning::standard_guitar();
        let pos = FretPosition {
            string: StringIndex::new(6).unwrap(),
            fret: Fret(3),
        };
        assert_eq!(pitch_at(&t, Capo(0), pos), Some(43)); // G2
        assert_eq!(pitch_at(&t, Capo(2), pos), Some(45));
        let missing = FretPosition {
            string: StringIndex::new(7).unwrap(),
            fret: Fret(0),
        };
        assert_eq!(pitch_at(&t, Capo(0), missing), None);
    }
}
