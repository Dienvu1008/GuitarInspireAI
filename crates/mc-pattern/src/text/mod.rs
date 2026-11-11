//! The text front end of the pattern engine (ADR-034 section 9, slice 6a-4). The text
//! is one way to write the Pattern IR and has no semantics of its own: `parse` builds
//! `PatternParts`, `print` writes them back, and `parse(print(ir)) == ir` for every
//! valid IR. The syntax is in docs/architecture/pattern-text-syntax.md.
//!
//! Gestures are written by name and stored by id, so both functions take the library
//! whose gestures the text refers to.

mod parse;
mod print;

use std::fmt;
use std::ops::Range;

use mc_theory::Rational;

use crate::error::PatternError;
use crate::ids::GestureId;
use crate::library::PatternLibrary;
use crate::model::{LaneId, PatternParts};

/// Parses pattern text into the IR (not yet validated; see [`PatternLibrary`]).
pub fn parse(text: &str, library: &PatternLibrary) -> Result<PatternParts, ParseError> {
    parse::parse(text, library)
}

/// Writes the IR as canonical text: explicit values and onsets, the index form of
/// spans, a bar check where a bar starts.
pub fn print(parts: &PatternParts, library: &PatternLibrary) -> Result<String, PrintError> {
    print::print(parts, library)
}

/// A syntax error: where it is, and what it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// Byte range in the input; empty at the end of the input.
    pub span: Range<usize>,
    /// 1-based line and column (in characters) of the start of the span.
    pub line: usize,
    pub column: usize,
    pub kind: ParseErrorKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Something else was expected here (the argument says what).
    Expected(&'static str),
    UnterminatedString,
    BadEscape,
    BadNumber,
    BadRational,
    /// Not a note value: a power of two from 1 to 256, dots, an optional tuplet.
    BadDuration,
    BadTuplet,
    BadMeter,
    UnknownKey(String),
    BadOrigin(String),
    DuplicateHeader(&'static str),
    MissingLength,
    /// A step, bar check or bracket before any lane header.
    StepOutsideLane,
    /// A step without `:value` and with no `unit` in force (Q-03).
    NoValue,
    /// Direction or marks on a rest, a hold or a gesture.
    MarksOnNonStrike,
    /// A string number outside 1 to 255.
    InvalidString(u32),
    UnknownGesture(String),
    /// A reserved word where a gesture name is expected.
    KeywordNotAllowed(String),
    BarCheckWithoutMeter,
    BarCheckOffBarLine {
        position: Rational,
        bar_length: Rational,
    },
    /// `strum:` together with `thumb:` or `fingers:`.
    LaneMix,
    DuplicateLane,
    MissingFingers,
    MissingThumb,
    NoLane,
    EmptyBracket,
    UnclosedBracket,
    TimeOverflow,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {:?}",
            self.line, self.column, self.kind
        )
    }
}

impl std::error::Error for ParseError {}

/// Why the IR cannot be written as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrintError {
    /// A step refers to a gesture the library does not have.
    UnknownGesture(GestureId),
    /// A strike with no target, or a hold with an empty list (invalid IR).
    NoTargets { lane: LaneId, step: usize },
    /// Time out of the rational range.
    TimeOverflow { lane: LaneId, step: usize },
}

impl fmt::Display for PrintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for PrintError {}

/// Why [`PatternLibrary::define_pattern_text`] refused a text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextError {
    Parse(ParseError),
    /// The text is well formed but the pattern is not valid (PT-01 to PT-07).
    Invalid(Vec<PatternError>),
}
