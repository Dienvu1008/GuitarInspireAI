//! Playback transport: session-level state and the port the application drives.
//!
//! The application owns every playback decision (GR-05). It decides what to compile
//! (which tracks are muted, which snapshot), when to recompile and swap during an edit,
//! where playback starts, and how the user tempo factor maps to the engine. The audio
//! adapter ([`AudioPort`]) only carries those decisions to the real-time engine; it owns
//! no music rules.
//!
//! State kept here is *session state*, not document content (Q3, Q6): muting a track or
//! changing the tempo factor never edits the score, never touches undo, and is not saved
//! to the project file.

use crate::events::DocumentId;
use mc_playback::{PerformanceTimeline, TempoFactor, Tick};
use mc_score::TrackId;
use std::collections::BTreeSet;

/// What the engine is doing, mirrored into the session so the UI can query it without
/// reaching into the audio adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PlaybackState {
    /// No stream, or stopped at a tick.
    #[default]
    Stopped,
    Playing,
    /// Playback could not run (for example the SoundFont is missing). The message is
    /// reported separately through the port's error query.
    Failed,
}

/// Session-level transport settings (not document content). Cloneable and comparable so
/// the session can tell when a setting actually changed and emit an event.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TransportState {
    /// Tracks the user has muted. Compilation omits their notes but keeps timing (PB-04).
    muted: BTreeSet<TrackId>,
    /// User tempo factor (Q6). A render scalar, never recompiles the timeline.
    tempo_factor: TempoFactor,
    /// Last known engine state, mirrored from [`AudioPort`] status.
    state: PlaybackState,
}

impl TransportState {
    pub fn muted(&self) -> &BTreeSet<TrackId> {
        &self.muted
    }

    pub fn is_muted(&self, track: TrackId) -> bool {
        self.muted.contains(&track)
    }

    pub fn tempo_factor(&self) -> TempoFactor {
        self.tempo_factor
    }

    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// Adds `track` to the muted set. Returns whether the set changed.
    pub(crate) fn set_muted_insert(&mut self, track: TrackId) -> bool {
        self.muted.insert(track)
    }

    /// Removes `track` from the muted set. Returns whether the set changed.
    pub(crate) fn set_muted_remove(&mut self, track: TrackId) -> bool {
        self.muted.remove(&track)
    }

    pub(crate) fn set_tempo_factor(&mut self, factor: TempoFactor) {
        self.tempo_factor = factor;
    }

    /// Mirrors the engine state. Returns whether it changed.
    pub(crate) fn set_state(&mut self, state: PlaybackState) -> bool {
        let changed = self.state != state;
        self.state = state;
        changed
    }
}

/// A transport request from the UI. The session turns each into [`AudioPort`] calls,
/// compiling a fresh timeline from the current snapshot when playback (re)starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportCommand {
    /// Start playback. `from_master_bar` is the bar that contains the user's cursor
    /// (Q7); playback starts at that bar's first played occurrence. `None` resumes from
    /// the current position.
    Play {
        from_master_bar: Option<usize>,
    },
    Stop,
    /// Seek to a tick on the current timeline (click-to-seek, Q7).
    Seek {
        tick: Tick,
    },
    /// Mute or unmute a track. While playing, the timeline is recompiled and swapped so
    /// the change is heard without a restart (Q8 releases held notes on swap). Mute omits
    /// a track's note events only; its channel assignment and all bar timing are
    /// unchanged (PB-04), so a muted track still occupies a channel in the timeline.
    SetMuted {
        track: TrackId,
        muted: bool,
    },
    /// Change the user tempo factor (Q6). Sends a scalar; never recompiles.
    SetTempoFactor {
        factor: TempoFactor,
    },
}

/// Why a transport command could not be carried out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportError {
    /// Compiling the snapshot into a timeline failed (the message explains why).
    Compile(String),
    /// The audio adapter rejected the operation (device gone, engine stopped, ...).
    Audio(String),
    /// `from_master_bar` is past the end of the expanded playback order.
    BarOutOfRange { master_bar: usize },
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Compile(m) => write!(f, "cannot compile playback: {m}"),
            TransportError::Audio(m) => write!(f, "audio error: {m}"),
            TransportError::BarOutOfRange { master_bar } => {
                write!(f, "master bar {master_bar} is past the end of playback")
            }
        }
    }
}

impl std::error::Error for TransportError {}

/// The port the application drives to produce sound. Implemented by the audio adapter
/// (`mc-audio::AudioEngine`, wired in `mc-ffi`); a fake implements it in tests. It takes
/// a fully compiled [`PerformanceTimeline`] and simple transport calls, so no music rule
/// lives behind it (GR-05).
pub trait AudioPort: Send {
    /// Install a freshly compiled timeline. While playing, the engine swaps it and
    /// releases held notes (Q8). Replaces any previous timeline.
    fn load_timeline(&mut self, timeline: PerformanceTimeline) -> Result<(), TransportError>;
    /// Start (or resume) at `start` ticks on the installed timeline.
    fn play(&mut self, start: Tick) -> Result<(), TransportError>;
    fn stop(&mut self) -> Result<(), TransportError>;
    fn seek(&mut self, tick: Tick) -> Result<(), TransportError>;
    /// Apply the user tempo factor as a render scalar (no recompile).
    fn set_tempo_factor(&mut self, factor: TempoFactor) -> Result<(), TransportError>;
    /// Current engine state (polled after a command and by the position stream).
    fn state(&self) -> PlaybackState;
    /// The current playback position in ticks, for the UI cursor.
    fn position(&self) -> Tick;
    /// A user-facing failure message if playback failed (for example a missing
    /// SoundFont), else `None`.
    fn last_error(&self) -> Option<String>;
}

/// A position update the UI can render as a moving cursor. Carried over FFI as plain
/// integers (Dart `int`), so the fields are `i64` (ADR-005, 4d position stream).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PositionUpdate {
    pub document: DocumentId,
    /// Tick on the current timeline.
    pub tick: i64,
    /// True while the transport is running.
    pub playing: bool,
}

/// One played bar for the playback cursor's tick-to-x mapping (ADR-028). After repeat
/// and volta expansion: playback order, the score master-bar index, and the tick range.
/// Ticks are `i64` for Dart (ADR-005 amendment 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayedBarInfo {
    pub order: u32,
    pub master_bar: usize,
    pub start_tick: i64,
    pub length_ticks: i64,
}
