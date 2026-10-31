use crate::{Rational, TheoryError};

/// Time signature: numerator 1..=32, denominator a power of two in 1..=64.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimeSignature {
    numerator: u8,
    denominator: u8,
}

impl TimeSignature {
    pub const COMMON: TimeSignature = TimeSignature {
        numerator: 4,
        denominator: 4,
    };

    pub fn new(numerator: u8, denominator: u8) -> Result<Self, TheoryError> {
        let ok_den = denominator.is_power_of_two() && denominator <= 64;
        if !(1..=32).contains(&numerator) || !ok_den {
            return Err(TheoryError::InvalidTimeSignature {
                numerator,
                denominator,
            });
        }
        Ok(TimeSignature {
            numerator,
            denominator,
        })
    }

    pub const fn numerator(self) -> u8 {
        self.numerator
    }

    pub const fn denominator(self) -> u8 {
        self.denominator
    }

    /// Nominal bar length as a fraction of a whole note.
    pub fn length(self) -> Rational {
        Rational::new(self.numerator as i64, self.denominator as i64)
            .expect("validated denominator")
    }
}

/// Tempo in quarter-note beats per minute, an exact positive rational (T-05).
/// The user-input range (T-06, default 20 to 400) is a configurable constraint
/// checked by the application layer, not here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tempo {
    bpm: Rational,
}

impl Tempo {
    pub fn new(bpm: Rational) -> Result<Self, TheoryError> {
        if !bpm.is_positive() {
            return Err(TheoryError::InvalidTempo);
        }
        Ok(Tempo { bpm })
    }

    pub fn from_bpm(bpm: u32) -> Result<Self, TheoryError> {
        Self::new(Rational::from_int(bpm as i64))
    }

    pub const fn bpm(self) -> Rational {
        self.bpm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_signature_rules() {
        assert_eq!(
            TimeSignature::new(6, 8).unwrap().length(),
            Rational::new(3, 4).unwrap()
        );
        assert!(TimeSignature::new(0, 4).is_err());
        assert!(TimeSignature::new(4, 3).is_err());
        assert!(TimeSignature::new(4, 128).is_err());
        assert!(TimeSignature::new(33, 4).is_err());
    }

    #[test]
    fn t05_tempo_must_be_positive() {
        assert_eq!(Tempo::from_bpm(0), Err(TheoryError::InvalidTempo));
        assert_eq!(
            Tempo::new(Rational::new(-1, 2).unwrap()),
            Err(TheoryError::InvalidTempo)
        );
        assert!(Tempo::new(Rational::new(1, 2).unwrap()).is_ok());
    }
}
