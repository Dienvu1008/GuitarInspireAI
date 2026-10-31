//! Application layer (`mc-app`): document sessions, the command dispatcher, undo,
//! events and ports. Adapters (CLI now, FFI in Phase 3) depend on this crate only, so
//! it re-exports the domain types they need (dependency-rules section 1).

mod command;
mod constraints;
mod diff;
mod error;
mod events;
mod handlers;
mod history;
pub mod ports;
mod session;

#[cfg(feature = "native-fs")]
pub mod infra;

pub use command::{
    BeatAddress, Command, CommandEnvelope, CommandId, DurationSpec, InstrumentSpecInput, Revision,
    ScoreCommand, TrackCommand,
};
pub use constraints::Constraints;
pub use error::{CommandError, OpenError, SaveError};
pub use events::{AppEvent, CollectingSink, DocumentId, EventEnvelope, EventPayload, EventSink};
pub use history::HistoryLimits;
pub use session::{CommandOutcome, DocumentSession, OpenReport, SessionConfig};

/// Domain and format types re-exported for adapters.
pub mod types {
    pub use mc_project::{AppInfo, FORMAT_VERSION, FormatVersion, LoadError};
    pub use mc_score::{
        BarContent, Beat, BeatId, Diagnostic, DiagnosticKind, DomainError, DomainEvent,
        InstrumentSpec, MasterBar, Note, NoteEffects, NoteId, Score, Track, TrackId, Violation,
        Voice,
    };
    pub use mc_theory::{KeySignature, Mode, Rational, TimeSignature};
}

/// Builds a new document score: one standard-guitar track with `bars` empty 4/4 bars at 120 BPM.
pub fn new_score(track_name: &str, bars: usize) -> Result<mc_score::Score, mc_score::DomainError> {
    use mc_score::{FrettedInstrument, InstrumentSpec, MasterBar};
    let mut s = mc_score::Score::new(
        track_name,
        InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
        MasterBar::default(),
        mc_theory::Tempo::from_bpm(120).expect("positive"),
    )?;
    if bars > 1 {
        s.insert_master_bars(1, bars - 1, MasterBar::default())?;
    }
    Ok(s)
}

#[cfg(feature = "fixtures")]
pub mod fixtures {
    pub use mc_score::fixtures::{LARGE, SMALL, Shape, TYPICAL, feature_rich, reference};
}
