//! Scale formulas as semitone sets (Phase 2: what the scale finder needs).

use crate::PitchClass;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScaleFormula {
    Major,
    NaturalMinor,
    MajorPentatonic,
    MinorPentatonic,
}

impl ScaleFormula {
    pub const ALL: [ScaleFormula; 4] = [
        ScaleFormula::Major,
        ScaleFormula::NaturalMinor,
        ScaleFormula::MajorPentatonic,
        ScaleFormula::MinorPentatonic,
    ];

    /// Semitones above the root, ascending, starting at 0.
    pub fn semitones(self) -> &'static [u8] {
        match self {
            ScaleFormula::Major => &[0, 2, 4, 5, 7, 9, 11],
            ScaleFormula::NaturalMinor => &[0, 2, 3, 5, 7, 8, 10],
            ScaleFormula::MajorPentatonic => &[0, 2, 4, 7, 9],
            ScaleFormula::MinorPentatonic => &[0, 3, 5, 7, 10],
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ScaleFormula::Major => "major",
            ScaleFormula::NaturalMinor => "minor",
            ScaleFormula::MajorPentatonic => "major-pentatonic",
            ScaleFormula::MinorPentatonic => "minor-pentatonic",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name() == name)
    }

    /// 1-based degree of `pc` in the scale on `root`, if it belongs to it.
    pub fn degree_of(self, root: PitchClass, pc: PitchClass) -> Option<u8> {
        let offset = (pc.value() as i32 - root.value() as i32).rem_euclid(12) as u8;
        self.semitones()
            .iter()
            .position(|s| *s == offset)
            .map(|i| i as u8 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degrees() {
        let a = PitchClass::from_semitone(9);
        assert_eq!(
            ScaleFormula::MinorPentatonic.degree_of(a, PitchClass::from_semitone(0)),
            Some(2)
        ); // C in A minor pentatonic
        assert_eq!(
            ScaleFormula::MinorPentatonic.degree_of(a, PitchClass::from_semitone(11)),
            None
        );
        assert_eq!(ScaleFormula::from_name("major"), Some(ScaleFormula::Major));
    }
}
