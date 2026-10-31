//! Note values: base value, dots and tuplet ratio (H-07). Lengths are exact rationals
//! of a whole note (ADR-011).

use crate::{Rational, TheoryError};

/// Base value as a power of two: 0 = whole, 1 = half, 2 = quarter ... 8 = 1/256.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BaseValue(u8);

impl BaseValue {
    pub const WHOLE: BaseValue = BaseValue(0);
    pub const HALF: BaseValue = BaseValue(1);
    pub const QUARTER: BaseValue = BaseValue(2);
    pub const EIGHTH: BaseValue = BaseValue(3);
    pub const SIXTEENTH: BaseValue = BaseValue(4);
    pub const MAX_EXPONENT: u8 = 8;

    pub fn from_exponent(exponent: u8) -> Result<Self, TheoryError> {
        if exponent > Self::MAX_EXPONENT {
            return Err(TheoryError::InvalidBaseValue(exponent));
        }
        Ok(BaseValue(exponent))
    }

    pub const fn exponent(self) -> u8 {
        self.0
    }
}

/// `actual` notes in the time of `normal` notes, for example 3:2 for a triplet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tuplet {
    actual: u8,
    normal: u8,
}

impl Tuplet {
    pub const TRIPLET: Tuplet = Tuplet {
        actual: 3,
        normal: 2,
    };

    /// Supported set (H-07): actual in 2..=13, normal in 1..=12, actual != normal.
    pub fn new(actual: u8, normal: u8) -> Result<Self, TheoryError> {
        if !(2..=13).contains(&actual) || !(1..=12).contains(&normal) || actual == normal {
            return Err(TheoryError::UnsupportedTuplet { actual, normal });
        }
        Ok(Tuplet { actual, normal })
    }

    pub const fn actual(self) -> u8 {
        self.actual
    }

    pub const fn normal(self) -> u8 {
        self.normal
    }
}

/// Written duration of a beat. Always strictly positive (T-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NoteValue {
    base: BaseValue,
    dots: u8,
    tuplet: Option<Tuplet>,
}

impl NoteValue {
    pub const MAX_DOTS: u8 = 3;

    pub fn new(base: BaseValue, dots: u8, tuplet: Option<Tuplet>) -> Result<Self, TheoryError> {
        if dots > Self::MAX_DOTS {
            return Err(TheoryError::TooManyDots(dots));
        }
        Ok(NoteValue { base, dots, tuplet })
    }

    pub const fn plain(base: BaseValue) -> Self {
        NoteValue {
            base,
            dots: 0,
            tuplet: None,
        }
    }

    pub const fn base(self) -> BaseValue {
        self.base
    }

    pub const fn dots(self) -> u8 {
        self.dots
    }

    pub const fn tuplet(self) -> Option<Tuplet> {
        self.tuplet
    }

    /// Length as a fraction of a whole note: 2^-e * (2 - 2^-dots) * normal / actual.
    /// Cannot overflow: the largest denominator is 2^11 * 13.
    pub fn length(self) -> Rational {
        let e = self.base.0 as u32;
        let d = self.dots as u32;
        // (2^(d+1) - 1) / 2^(e+d)
        let mut num: i64 = (1i64 << (d + 1)) - 1;
        let mut den: i64 = 1i64 << (e + d);
        if let Some(t) = self.tuplet {
            num *= t.normal as i64;
            den *= t.actual as i64;
        }
        Rational::new(num, den).expect("note value lengths are small and non-zero")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: i64) -> Rational {
        Rational::new(n, d).unwrap()
    }

    #[test]
    fn h07_lengths_of_plain_dotted_and_tuplet_values() {
        assert_eq!(NoteValue::plain(BaseValue::QUARTER).length(), r(1, 4));
        assert_eq!(
            NoteValue::new(BaseValue::QUARTER, 1, None)
                .unwrap()
                .length(),
            r(3, 8)
        );
        assert_eq!(
            NoteValue::new(BaseValue::HALF, 2, None).unwrap().length(),
            r(7, 8)
        );
        assert_eq!(
            NoteValue::new(BaseValue::EIGHTH, 0, Some(Tuplet::TRIPLET))
                .unwrap()
                .length(),
            r(1, 12)
        );
        let smallest = NoteValue::new(
            BaseValue::from_exponent(8).unwrap(),
            3,
            Some(Tuplet::new(13, 12).unwrap()),
        )
        .unwrap();
        assert!(smallest.length().is_positive());
    }

    #[test]
    fn h07_rejects_unsupported_values() {
        assert_eq!(
            BaseValue::from_exponent(9),
            Err(TheoryError::InvalidBaseValue(9))
        );
        assert_eq!(
            NoteValue::new(BaseValue::QUARTER, 4, None),
            Err(TheoryError::TooManyDots(4))
        );
        assert!(Tuplet::new(3, 3).is_err());
        assert!(Tuplet::new(1, 2).is_err());
        assert!(Tuplet::new(14, 2).is_err());
        assert!(Tuplet::new(5, 4).is_ok());
    }

    #[test]
    fn t01_every_supported_value_is_positive() {
        for e in 0..=8 {
            for d in 0..=3 {
                for t in [
                    None,
                    Some(Tuplet::TRIPLET),
                    Some(Tuplet::new(7, 4).unwrap()),
                ] {
                    let v = NoteValue::new(BaseValue::from_exponent(e).unwrap(), d, t).unwrap();
                    assert!(v.length().is_positive());
                }
            }
        }
    }
}
