//! Guitar context (`mc-guitar`).
//!
//! Pure services over the fretted-instrument kernel types of `mc-theory` (ADR-019):
//! tuning presets, fretboard position queries, the chord finder in both directions
//! (symbol to voicings, voicing to ranked chord names), voicing constraints, scale
//! positions and retune planning. No I/O, no score mutation: the application layer
//! applies retune plans through the score aggregate.
//!
//! Rule ids in comments and test names refer to docs/architecture/domain-invariants.md.

mod constraints;
mod error;
mod fretboard;
mod identify;
mod presets;
mod retune;
mod scales;
mod search;
mod string_assignment;
mod tuner;
mod voicing;

pub use constraints::{BassRule, ConstraintViolation, VoicingConstraints};
pub use error::GuitarError;
pub use fretboard::positions_of;
pub use identify::{
    CandidateTone, ChordCandidate, Identification, IdentifyOptions, IdentifyStatus,
    MIN_DISTINCT_PITCH_CLASSES, candidates_for, identify, interval_name,
};
pub use presets::TuningPreset;
pub use retune::{RetuneMode, RetunePlan, plan_retune};
pub use scales::{ScaleNote, scale_positions};
pub use search::{SearchOptions, find_voicings};
pub use string_assignment::{Assigned, assign_group, assign_sequence};
pub use tuner::{A4_HZ, TunerMatch, TunerTarget, nearest, semitone_hz, tuner_targets};
pub use voicing::{SoundingNote, Voicing};
