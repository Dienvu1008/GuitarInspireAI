//! Pattern engine domain (`mc-pattern`, Phase 6a, ADR-034).
//!
//! The Pattern IR is the source of truth: the grid editor, the gesture palette, the
//! text DSL (6a-4) and later tap-to-create all produce and edit it. This slice (6a-2)
//! holds the IR, its semantic validation (PT-01, PT-03 to PT-07) and the pattern
//! library of patterns and user gestures (PT-06, PT-08), in memory. Slice 6a-3 adds
//! binding to a voicing (ADR-034 section 5, PT-02) and placing over a bar range
//! (section 7, PT-09). Slice 6a-4 adds the text front end, `text` (section 9).
//!
//! Rule ids (PT-xx) refer to docs/architecture/domain-invariants.md.

mod bind;
mod error;
mod ids;
mod library;
mod model;
mod names;
mod place;
pub mod text;
mod validate;

pub use bind::{BindError, BoundBeat, BoundLane, BoundNote, PatternFragment, bind};
pub use error::{NameProblem, PatternError, StepRef, StrikeAt};
pub use ids::{GestureId, PatternId};
pub use library::{
    BindFailure, GestureDefinition, GestureParts, LibraryParts, PatternDefinition, PatternLibrary,
    ResolvedAction, ResolvedPattern, ResolvedStep,
};
pub use model::{
    Action, Direction, Lane, LaneId, Lanes, Origin, PatternMeta, PatternParts, Step, Strike,
    StringSelector, Technique, TechniqueSet, TechniqueSpan,
};
pub use names::{KEYWORDS, check_name, name_key};
pub use place::{
    Fit, PlaceError, PlaceReport, PlacedBar, Placement, StepChange, place, values_for,
};
pub use validate::covers;
