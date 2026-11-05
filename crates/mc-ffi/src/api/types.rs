//! DTOs exchanged with Dart (interop-architecture section 7). Plain values only; ids and
//! counters are `i64` (maps to Dart `int`; `u64` would map to `BigInt`, ADR-005
//! amendment 2). Conversions into application types live here; every musical rule is
//! checked by the core, never by this module.

use mc_app::types::{BeatId, NoteId, TrackId};
use mc_app::{BeatAddress, CommandError, DurationSpec, InstrumentSpecInput, OpenError, SaveError};

/// Error returned to Dart. `rule` is the domain rule id when one applies (for example
/// `H-04`), otherwise a short category such as `save` or `layout`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
    pub rule: String,
    pub message: String,
}

impl ApiError {
    pub(crate) fn new(rule: &str, message: impl Into<String>) -> Self {
        ApiError {
            rule: rule.to_string(),
            message: message.into(),
        }
    }
}

impl From<CommandError> for ApiError {
    fn from(e: CommandError) -> Self {
        ApiError::new(e.rule(), e.to_string())
    }
}

impl From<SaveError> for ApiError {
    fn from(e: SaveError) -> Self {
        let rule = match e {
            SaveError::WouldDropUnknownContent { .. } => "U2",
            SaveError::NoLocation => "save-location",
            SaveError::Store(_) => "save",
        };
        ApiError::new(rule, e.to_string())
    }
}

impl From<OpenError> for ApiError {
    fn from(e: OpenError) -> Self {
        let rule = match e {
            OpenError::Store(_) => "open",
            OpenError::Load(_) => "load",
        };
        ApiError::new(rule, e.to_string())
    }
}

pub(crate) fn track_id(raw: i64) -> TrackId {
    TrackId::from_raw(raw as u64)
}

pub(crate) fn note_id(raw: i64) -> NoteId {
    NoteId::from_raw(raw as u64)
}

/// Address of a beat: written location plus its stable id (checked by the core).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeatAddressDto {
    pub track: i64,
    pub bar: u32,
    pub voice: u32,
    pub beat: i64,
}

impl BeatAddressDto {
    pub(crate) fn to_app(self) -> BeatAddress {
        BeatAddress {
            track: track_id(self.track),
            bar: self.bar as usize,
            voice: self.voice as usize,
            beat: BeatId::from_raw(self.beat as u64),
        }
    }
}

/// Written duration: base exponent (0 whole ... 8), dots, optional tuplet ratio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurationDto {
    pub base: u8,
    pub dots: u8,
    /// `actual:normal`, for example 3:2 for a triplet. `None` for no tuplet.
    pub tuplet: Option<TupletDto>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TupletDto {
    pub actual: u8,
    pub normal: u8,
}

impl DurationDto {
    pub(crate) fn to_app(self) -> DurationSpec {
        DurationSpec {
            base: self.base,
            dots: self.dots,
            tuplet: self.tuplet.map(|t| (t.actual, t.normal)),
        }
    }
}

/// Fretted instrument; tuning lists string 1 (highest) first, for example "E4".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentDto {
    pub tuning: Vec<String>,
    pub fret_count: u8,
    pub capo: u8,
}

impl InstrumentDto {
    pub(crate) fn to_app(&self) -> InstrumentSpecInput {
        InstrumentSpecInput {
            tuning: self.tuning.clone(),
            fret_count: self.fret_count,
            capo: self.capo,
        }
    }
}

/// Master bar properties. The key signature is kept as it is in the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MasterBarDto {
    pub numerator: u8,
    pub denominator: u8,
    pub pickup: bool,
    pub repeat_start: bool,
    pub repeat_end: Option<u8>,
    pub alternate_endings: Vec<u8>,
    pub marker: Option<String>,
}

/// Every editing command the Phase 3 editor sends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandDto {
    InsertMasterBars {
        at: u32,
        count: u32,
    },
    RemoveMasterBars {
        at: u32,
        count: u32,
    },
    SetMasterBar {
        index: u32,
        bar: MasterBarDto,
    },
    /// Offset is `offset_num / offset_den` of a whole note; bpm in quarter notes per minute.
    SetTempo {
        bar: u32,
        offset_num: i64,
        offset_den: i64,
        bpm: u32,
    },
    RemoveTempo {
        bar: u32,
        offset_num: i64,
        offset_den: i64,
    },
    InsertBeat {
        track: i64,
        bar: u32,
        voice: u32,
        index: u32,
        duration: DurationDto,
    },
    RemoveBeat {
        beat: BeatAddressDto,
    },
    SetBeatDuration {
        beat: BeatAddressDto,
        duration: DurationDto,
    },
    SetNote {
        beat: BeatAddressDto,
        string: u8,
        fret: u8,
    },
    RemoveNote {
        beat: BeatAddressDto,
        note: i64,
    },
    SetTie {
        beat: BeatAddressDto,
        note: i64,
        tie: bool,
    },
    SetNoteEffects {
        beat: BeatAddressDto,
        note: i64,
        let_ring: bool,
        dead: bool,
    },
    AddVoice {
        track: i64,
        bar: u32,
    },
    RemoveVoice {
        track: i64,
        bar: u32,
        voice: u32,
    },
    /// Moves notes in bars `from_bar..=to_bar` by `delta` frets.
    ShiftFrets {
        track: i64,
        from_bar: u32,
        to_bar: u32,
        delta: i16,
    },
    AddTrack {
        name: String,
        instrument: InstrumentDto,
    },
    RemoveTrack {
        track: i64,
    },
    RenameTrack {
        track: i64,
        name: String,
    },
    SetInstrument {
        track: i64,
        instrument: InstrumentDto,
    },
    MoveTrack {
        track: i64,
        to: u32,
    },
    /// `keep_pitches`: notes keep their sound (frets move); otherwise frets are kept.
    Retune {
        track: i64,
        instrument: InstrumentDto,
        keep_pitches: bool,
    },
}

/// Invalidation hint (ADR-006). Consumers re-query summary, beats or layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventKindDto {
    TrackAdded {
        track: i64,
    },
    TrackRemoved {
        track: i64,
    },
    TrackChanged {
        track: i64,
    },
    TracksReordered,
    MasterBarsInserted {
        at: u32,
        count: u32,
    },
    MasterBarsRemoved {
        at: u32,
        count: u32,
    },
    MasterBarsChanged {
        start: u32,
        end: u32,
    },
    BarContentChanged {
        track: i64,
        start: u32,
        end: u32,
    },
    TempoMapChanged,
    TiesRemoved {
        track: i64,
        notes: Vec<i64>,
    },
    DocumentSaved,
    DirtyChanged {
        dirty: bool,
    },
    UndoAvailabilityChanged {
        can_undo: bool,
        can_redo: bool,
    },
    HistoryApplied,
    /// Playback transport state changed (session state, ADR-028). The revision is
    /// unchanged on the carrying event.
    PlaybackStateChanged {
        state: PlaybackStateDto,
    },
    /// A track was muted or unmuted (session state, ADR-028).
    MuteChanged {
        track: i64,
        muted: bool,
    },
    /// The user tempo factor changed (session state, ADR-028). Hundredths of normal
    /// speed (100 = 1.0x), 25..=200 in steps of 5.
    TempoFactorChanged {
        hundredths: u16,
    },
}

/// Playback transport state mirrored to Dart (ADR-028).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackStateDto {
    Stopped,
    Playing,
    /// Playback could not run; the message is read with `EngineHandle::last_error`.
    Failed,
}

/// A playback position update for the UI cursor (60 Hz stream, ADR-028). All fields map
/// to Dart `int`/`bool` (ADR-005 amendment 2: never `u64`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PositionDto {
    /// Tick on the current timeline (i64, non-negative in a running timeline).
    pub tick: i64,
    pub playing: bool,
    pub state: PlaybackStateDto,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDto {
    /// Per document, gapless, starting at 1.
    pub seq: i64,
    pub revision: i64,
    /// Command id that caused the event; `None` for undo, redo and save.
    pub correlation: Option<i64>,
    pub kind: EventKindDto,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutcomeDto {
    pub revision: i64,
    pub events: Vec<EventDto>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackSummaryDto {
    pub id: i64,
    pub name: String,
    pub instrument: InstrumentDto,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticDto {
    pub rule: String,
    pub track: i64,
    pub bar: u32,
    pub voice: u32,
    /// "underfull" or "overfull".
    pub kind: String,
}

/// Document state for the shell: tracks, bars, history and save state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentSummaryDto {
    pub revision: i64,
    pub dirty: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    /// File path of the document, `None` until saved.
    pub location: Option<String>,
    /// Unknown content (U1); non-empty means Save over the original is refused (U2).
    pub unknown_content: Vec<String>,
    pub tracks: Vec<TrackSummaryDto>,
    pub master_bars: Vec<MasterBarDto>,
    pub diagnostics: Vec<DiagnosticDto>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteDto {
    pub id: i64,
    pub string: u8,
    pub fret: u8,
    pub tie: bool,
    pub let_ring: bool,
    pub dead: bool,
}

/// One beat with its notes (inspector and editor queries).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeatDto {
    pub id: i64,
    pub duration: DurationDto,
    pub notes: Vec<NoteDto>,
}
