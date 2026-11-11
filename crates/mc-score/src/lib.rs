//! Composition core domain (`mc-score`).
//!
//! `Score` is the single aggregate (docs/architecture/score-model-and-undo.md). Its
//! fields are private; every state that exists passed either a validating aggregate
//! method or `Score::from_parts`. Internals use `Arc` per track and per bar so that
//! cloning a `Score` is a cheap structural-sharing snapshot (ADR-016).
//!
//! Rule ids (H-xx, E-xx, D-xx, PB-xx) refer to docs/architecture/domain-invariants.md.

mod diagnostics;
mod edit;
mod error;
mod events;
mod ids;
mod model;
mod repeats;
mod score;
mod timing;
mod validate;

#[cfg(feature = "fixtures")]
pub mod fixtures;

pub use diagnostics::{Diagnostic, DiagnosticKind};
pub use edit::{BeatRef, BeatSlot, FragmentBeat, FragmentNote};
pub use error::{DomainError, Violation};
pub use events::DomainEvent;
pub use ids::{BeatId, NoteId, TrackId};
pub use model::{
    BarContent, Beat, FrettedInstrument, InstrumentSpec, MasterBar, Note, NoteEffects, NoteKind,
    Stroke, TempoChange, Track, TrackData, Voice,
};
pub use repeats::PlaybackOrder;
pub use score::{Score, ScoreParts};
