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
use mc_playback::{Channel, Pan, PerformanceTimeline, TempoFactor, Tick, TrackMix, Volume};
use mc_score::TrackId;
use std::collections::{BTreeMap, BTreeSet};

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
    /// Tracks the user has soloed (ADR-032). When any existing track is soloed, every
    /// other track is compiled as muted; see [effective_muted].
    soloed: BTreeSet<TrackId>,
    /// Per-track volume and pan (ADR-032). Only non-default values are stored.
    mix: BTreeMap<TrackId, TrackMix>,
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

    pub fn soloed(&self) -> &BTreeSet<TrackId> {
        &self.soloed
    }

    pub fn is_soloed(&self, track: TrackId) -> bool {
        self.soloed.contains(&track)
    }

    /// The mix of `track`: the stored one, or the default (volume 100, pan centre).
    pub fn mix_of(&self, track: TrackId) -> TrackMix {
        self.mix.get(&track).copied().unwrap_or_default()
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

    pub(crate) fn set_soloed_insert(&mut self, track: TrackId) -> bool {
        self.soloed.insert(track)
    }

    pub(crate) fn set_soloed_remove(&mut self, track: TrackId) -> bool {
        self.soloed.remove(&track)
    }

    /// Stores `mix` for `track`. Returns whether the effective mix changed.
    pub(crate) fn set_mix(&mut self, track: TrackId, mix: TrackMix) -> bool {
        let changed = self.mix_of(track) != mix;
        if mix == TrackMix::default() {
            self.mix.remove(&track);
        } else {
            self.mix.insert(track, mix);
        }
        changed
    }

    /// Drops mute, solo and mix entries of tracks that no longer exist (ADR-032).
    pub(crate) fn prune(&mut self, existing: &[TrackId]) {
        self.muted.retain(|t| existing.contains(t));
        self.soloed.retain(|t| existing.contains(t));
        self.mix.retain(|t, _| existing.contains(t));
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
    /// Solo or unsolo a track (ADR-032). While any track is soloed, the others are
    /// compiled as muted. Recompiles and swaps while playing, as a mute change does.
    SetSoloed {
        track: TrackId,
        soloed: bool,
    },
    /// Set the volume and pan of a track (ADR-032). Never recompiles: the values go to
    /// the audio port as they are. Build it from raw numbers with
    /// [`TransportCommand::track_mix`], which rejects out-of-range values.
    SetTrackMix {
        track: TrackId,
        volume: Volume,
        pan: Pan,
    },
    /// Change the user tempo factor (Q6). Sends a scalar; never recompiles.
    SetTempoFactor {
        factor: TempoFactor,
    },
}

impl TransportCommand {
    /// A [`TransportCommand::SetTrackMix`] from raw numbers (volume 0..=127, pan
    /// -63..=63). Out-of-range values are rejected, never clamped (ADR-032 X7).
    pub fn track_mix(track: TrackId, volume: i32, pan: i32) -> Result<Self, TransportError> {
        // Any integer the UI sends is accepted as input; only the ranges decide.
        let bad =
            |name: &str, v: i32| TransportError::InvalidMix(format!("{name} out of range: {v}"));
        let volume = u8::try_from(volume)
            .ok()
            .and_then(|v| Volume::try_new(v).ok())
            .ok_or_else(|| bad("Volume", volume))?;
        let pan = i16::try_from(pan)
            .ok()
            .and_then(|v| Pan::try_new(v).ok())
            .ok_or_else(|| bad("Pan", pan))?;
        Ok(TransportCommand::SetTrackMix { track, volume, pan })
    }
}

/// The tracks compiled as muted: `muted` plus, when any existing track is soloed, every
/// existing track that is not soloed (ADR-032 section 3). Mute always wins: a muted and
/// soloed track stays silent. Entries for tracks that no longer exist are ignored, so a
/// deleted soloed track no longer silences the others.
pub fn effective_muted(
    existing: &[TrackId],
    muted: &BTreeSet<TrackId>,
    soloed: &BTreeSet<TrackId>,
) -> BTreeSet<TrackId> {
    let any_solo = existing.iter().any(|t| soloed.contains(t));
    existing
        .iter()
        .copied()
        .filter(|t| muted.contains(t) || (any_solo && !soloed.contains(t)))
        .collect()
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
    /// A volume or pan outside its range (ADR-032 X7); the message names the value.
    InvalidMix(String),
    /// The track does not exist in the document.
    UnknownTrack(TrackId),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Compile(m) => write!(f, "cannot compile playback: {m}"),
            TransportError::Audio(m) => write!(f, "audio error: {m}"),
            TransportError::BarOutOfRange { master_bar } => {
                write!(f, "master bar {master_bar} is past the end of playback")
            }
            TransportError::InvalidMix(m) => write!(f, "invalid mix: {m}"),
            TransportError::UnknownTrack(t) => write!(f, "unknown track {t:?}"),
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
    /// Set the volume and pan of a MIDI channel (ADR-032). No recompile, no swap.
    fn set_channel_mix(
        &mut self,
        channel: Channel,
        volume: Volume,
        pan: Pan,
    ) -> Result<(), TransportError>;
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
