use std::fmt;

/// Construction errors of theory value objects. Each variant names the rule it protects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TheoryError {
    /// Rational with a zero denominator.
    ZeroDenominator,
    /// Arithmetic result does not fit the representation.
    Overflow,
    /// T-01: a duration or position that must be positive (or non-negative) is not.
    NotPositive,
    /// T-02: alteration outside -2..=2.
    AlterOutOfRange(i8),
    /// Unparseable pitch text.
    InvalidPitch(String),
    /// H-07: base value exponent outside 0..=8 (whole to 1/256).
    InvalidBaseValue(u8),
    /// H-07: more than three dots.
    TooManyDots(u8),
    /// H-07: tuplet ratio outside the supported set.
    UnsupportedTuplet { actual: u8, normal: u8 },
    /// Time signature numerator outside 1..=32 or denominator not a power of two in 1..=64.
    InvalidTimeSignature { numerator: u8, denominator: u8 },
    /// Key signature with more than seven sharps or flats.
    InvalidKey(i8),
    /// T-05: tempo must be greater than zero.
    InvalidTempo,
    /// T-04: a tuning needs at least one string.
    EmptyTuning,
    /// String index 0 (string numbering is 1-based, string 1 is the highest).
    InvalidStringIndex,
}

impl fmt::Display for TheoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDenominator => write!(f, "rational with zero denominator"),
            Self::Overflow => write!(f, "arithmetic overflow"),
            Self::NotPositive => write!(f, "value must be positive (T-01)"),
            Self::AlterOutOfRange(a) => write!(f, "alteration {a} outside -2..=2 (T-02)"),
            Self::InvalidPitch(s) => write!(f, "invalid pitch '{s}'"),
            Self::InvalidBaseValue(e) => write!(f, "base value exponent {e} outside 0..=8 (H-07)"),
            Self::TooManyDots(d) => write!(f, "{d} dots; at most 3 are supported (H-07)"),
            Self::UnsupportedTuplet { actual, normal } => {
                write!(f, "unsupported tuplet {actual}:{normal} (H-07)")
            }
            Self::InvalidTimeSignature {
                numerator,
                denominator,
            } => {
                write!(f, "invalid time signature {numerator}/{denominator}")
            }
            Self::InvalidKey(k) => write!(f, "key signature with {k} fifths; allowed -7..=7"),
            Self::InvalidTempo => write!(f, "tempo must be greater than zero (T-05)"),
            Self::EmptyTuning => write!(f, "a tuning needs at least one string (T-04)"),
            Self::InvalidStringIndex => write!(f, "string index must be 1 or greater"),
        }
    }
}

impl std::error::Error for TheoryError {}
