use mc_theory::TheoryError;
use std::fmt;

/// A hard invariant violation found by validation (`Score::validate`, `Score::from_parts`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    /// Rule id from domain-invariants.md, for example "H-05".
    pub rule: &'static str,
    /// Human-readable location, for example "track 3, bar 4, voice 0, beat 17".
    pub location: String,
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}: {}", self.rule, self.location, self.message)
    }
}

/// Rejection of an aggregate operation. The score is unchanged when an error is returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainError {
    /// A value object could not be constructed.
    Theory(TheoryError),
    /// The operation would break a hard invariant (H-xx).
    Invariant { rule: &'static str, message: String },
    /// E-02: target track, bar, voice, beat or note does not exist.
    NotFound(String),
    /// E-03: the last track or the last master bar cannot be removed.
    CannotRemoveLast(&'static str),
    /// E-04: insertion position or range is outside the valid range.
    InvalidPosition(String),
    /// Rational arithmetic overflow while computing timing.
    Overflow,
}

impl DomainError {
    pub(crate) fn invariant(rule: &'static str, message: impl Into<String>) -> Self {
        DomainError::Invariant {
            rule,
            message: message.into(),
        }
    }

    /// Rule id of the protected invariant or editing constraint.
    pub fn rule(&self) -> &'static str {
        match self {
            DomainError::Theory(_) => "T",
            DomainError::Invariant { rule, .. } => rule,
            DomainError::NotFound(_) => "E-02",
            DomainError::CannotRemoveLast(_) => "E-03",
            DomainError::InvalidPosition(_) => "E-04",
            DomainError::Overflow => "T-01",
        }
    }
}

impl From<TheoryError> for DomainError {
    fn from(e: TheoryError) -> Self {
        DomainError::Theory(e)
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::Theory(e) => write!(f, "{e}"),
            DomainError::Invariant { rule, message } => write!(f, "{rule}: {message}"),
            DomainError::NotFound(what) => write!(f, "E-02: {what} not found"),
            DomainError::CannotRemoveLast(what) => write!(f, "E-03: cannot remove the last {what}"),
            DomainError::InvalidPosition(what) => write!(f, "E-04: invalid position: {what}"),
            DomainError::Overflow => write!(f, "timing arithmetic overflow"),
        }
    }
}

impl std::error::Error for DomainError {}
