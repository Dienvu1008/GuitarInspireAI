//! Application layer (`mc-app`): document sessions, the command dispatcher, undo,
//! events and ports. Adapters (CLI now, FFI in Phase 3) depend on this crate only, so
//! it re-exports the domain types they need (dependency-rules section 1).

mod command;
mod constraints;
mod diff;
mod error;
mod events;
pub mod guitar;
mod handlers;
mod history;
pub mod notation;
pub mod ports;
pub mod recovery;
mod session;
pub mod transport;

#[cfg(feature = "native-fs")]
pub mod infra;

pub use command::{
    BeatAddress, ClipBeat, ClipNote, Command, CommandEnvelope, CommandId, DurationSpec,
    GuitarCommand, InstrumentSpecInput, Revision, ScoreCommand, TrackCommand,
};
pub use constraints::Constraints;
pub use error::{CommandError, OpenError, SaveError};
pub use events::{AppEvent, CollectingSink, DocumentId, EventEnvelope, EventPayload, EventSink};
pub use history::HistoryLimits;
pub use session::{CommandOutcome, DocumentSession, OpenReport, SessionConfig, TunerReference};
pub use transport::{
    AudioPort, PlaybackState, PlayedBarInfo, PositionUpdate, TransportCommand, TransportError,
    TransportState,
};

/// Playback value objects re-exported for adapters (so `mc-ffi` and `mc-cli` map DTOs
/// without depending on `mc-playback` directly; dependency-rules section 1).
pub mod playback {
    pub use mc_playback::{PerformanceTimeline, TempoFactor, Tick};

    /// Compile a score for MIDI export: every track included, no mute (ADR-023 item 3,
    /// ADR-028). Compilation is an application decision (GR-05), so adapters call this
    /// rather than invoking the compiler with their own options. Returns the warning-free
    /// timeline or a message.
    pub fn compile_for_export(score: &mc_score::Score) -> Result<PerformanceTimeline, String> {
        mc_playback::compile(score, &mc_playback::CompileOptions::new())
            .map(|out| out.timeline)
            .map_err(|e| e.to_string())
    }
}

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
