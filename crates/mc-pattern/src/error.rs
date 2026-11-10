//! Typed errors. Every validation error names its step (lane, index, onset), its span,
//! its gesture or its id; nothing is skipped silently (ADR-034 section 4).

use std::fmt;

use mc_theory::Rational;

use crate::ids::{GestureId, PatternId};
use crate::model::{LaneId, StringSelector};

/// A step of a pattern: its lane, its index in the lane, and its onset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepRef {
    pub lane: LaneId,
    pub index: usize,
    pub onset: Rational,
}

/// Where a strike is written: in a step of a pattern, or in a gesture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StrikeAt {
    Step(StepRef),
    Gesture { name: String },
}

/// Why a gesture name is not allowed (PT-08).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameProblem {
    Empty,
    /// The first character is not a letter (digits would read as string numbers).
    MustStartWithLetter,
    /// Only letters and digits are allowed, so the DSL can always print the name.
    InvalidCharacter(char),
    /// Equal, without case, to a reserved DSL keyword.
    Keyword(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternError {
    /// PT-01: the pattern length must be greater than 0.
    NonPositiveLength { length: Rational },
    /// PT-01: the step ends after the pattern end.
    OutsidePattern {
        step: StepRef,
        end: Rational,
        length: Rational,
    },
    /// PT-04: onsets are non-negative.
    NegativeOnset { step: StepRef },
    /// PT-04: onset plus duration does not fit the rational range.
    TimeOverflow { step: StepRef },
    /// PT-03: the step starts before the previous step of its lane.
    OutOfOrder {
        step: StepRef,
        previous_onset: Rational,
    },
    /// PT-03: the step starts before the previous step of its lane has ended.
    Overlap {
        step: StepRef,
        previous_end: Rational,
    },
    /// A strike or a hold with no target.
    EmptyTargets { at: StrikeAt },
    /// `Treble(0)`: n is at least 1.
    InvalidSelector {
        at: StrikeAt,
        selector: StringSelector,
    },
    /// PT-05: a hold does not directly follow a strike or a hold of its lane (a gap is
    /// a rest, PT-09).
    HoldWithoutStrike { step: StepRef },
    /// PT-05: the hold selects a string the held strike does not certainly play, for
    /// every voicing.
    HoldNotInStrike {
        step: StepRef,
        selector: StringSelector,
    },
    /// PT-06: the gesture is not in the library.
    UnknownGesture { step: StepRef, gesture: GestureId },
    /// PT-07: the span names a lane the pattern does not have.
    SpanLaneMissing { span: usize, lane: LaneId },
    /// PT-07: the span does not cover existing steps of its lane.
    SpanOutOfRange {
        span: usize,
        lane: LaneId,
        first: usize,
        last: usize,
        steps: usize,
    },
    /// PT-08: the gesture name is not allowed.
    InvalidGestureName { name: String, problem: NameProblem },
    /// PT-08: another gesture has the same name without case.
    DuplicateGestureName { name: String, existing: GestureId },
    /// The library has no pattern with this id.
    UnknownPattern(PatternId),
    /// The library has no gesture with this id.
    UnknownGestureId(GestureId),
    /// A gesture cannot be removed while patterns use it.
    GestureInUse {
        gesture: GestureId,
        patterns: Vec<PatternId>,
    },
    /// Changing a gesture would make a pattern that uses it invalid.
    InvalidPattern {
        pattern: PatternId,
        errors: Vec<PatternError>,
    },
    /// Library parts: an id is used twice.
    DuplicateId(u64),
    /// Library parts: an id is not below the id counter.
    IdNotBelowCounter { id: u64, next_id: u64 },
}

impl fmt::Display for StepRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} lane, step {} at {}",
            self.lane, self.index, self.onset
        )
    }
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use PatternError::*;
        match self {
            NonPositiveLength { length } => {
                write!(f, "PT-01: pattern length {length} is not greater than 0")
            }
            OutsidePattern { step, end, length } => {
                write!(
                    f,
                    "PT-01: {step} ends at {end}, after the pattern end {length}"
                )
            }
            NegativeOnset { step } => write!(f, "PT-04: {step} has a negative onset"),
            TimeOverflow { step } => write!(f, "PT-04: {step}: time out of range"),
            OutOfOrder {
                step,
                previous_onset,
            } => {
                write!(
                    f,
                    "PT-03: {step} starts before the previous step at {previous_onset}"
                )
            }
            Overlap { step, previous_end } => {
                write!(
                    f,
                    "PT-03: {step} starts before the previous step ends at {previous_end}"
                )
            }
            EmptyTargets { at } => write!(f, "{at:?}: no target"),
            InvalidSelector { at, selector } => write!(f, "{at:?}: invalid selector {selector:?}"),
            HoldWithoutStrike { step } => {
                write!(
                    f,
                    "PT-05: {step}: a hold must directly follow a strike or a hold"
                )
            }
            HoldNotInStrike { step, selector } => {
                write!(
                    f,
                    "PT-05: {step}: the held strike does not certainly play {selector:?}"
                )
            }
            UnknownGesture { step, gesture } => {
                write!(f, "PT-06: {step}: unknown gesture {}", gesture.raw())
            }
            SpanLaneMissing { span, lane } => {
                write!(f, "PT-07: span {span}: the pattern has no {lane:?} lane")
            }
            SpanOutOfRange {
                span,
                lane,
                first,
                last,
                steps,
            } => write!(
                f,
                "PT-07: span {span} covers steps {first} to {last} of the {lane:?} lane, which has {steps}"
            ),
            InvalidGestureName { name, problem } => {
                write!(
                    f,
                    "PT-08: gesture name {name:?} is not allowed: {problem:?}"
                )
            }
            DuplicateGestureName { name, existing } => write!(
                f,
                "PT-08: gesture name {name:?} is already used by gesture {}",
                existing.raw()
            ),
            UnknownPattern(id) => write!(f, "no pattern {}", id.raw()),
            UnknownGestureId(id) => write!(f, "no gesture {}", id.raw()),
            GestureInUse { gesture, patterns } => write!(
                f,
                "gesture {} is used by {} pattern(s)",
                gesture.raw(),
                patterns.len()
            ),
            InvalidPattern { pattern, errors } => write!(
                f,
                "pattern {} would become invalid ({} error(s))",
                pattern.raw(),
                errors.len()
            ),
            DuplicateId(id) => write!(f, "id {id} is used twice"),
            IdNotBelowCounter { id, next_id } => {
                write!(f, "id {id} is not below the id counter {next_id}")
            }
        }
    }
}

impl std::error::Error for PatternError {}
