//! Commands are application-layer types with plain values, as adapters (CLI, later FFI)
//! send them. Handlers convert them into value objects; the domain validates the rest.

use mc_guitar::RetuneMode;
use mc_score::{BeatId, MasterBar, NoteEffects, NoteId, Stroke, TrackId};
use mc_theory::NoteValue;

/// Opaque id chosen by the caller; copied into every event as the correlation id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(pub u64);

/// Document revision. Increases by one per committed command, undo or redo (P-04).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Revision(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub struct CommandEnvelope {
    pub id: CommandId,
    /// E-01: if set, the command is rejected unless it equals the current revision.
    pub expected_revision: Option<Revision>,
    /// Consecutive commands with the same key are one undo step (for example digits
    /// typed into one fret). The exact rules for typing are decided in Phase 3 (Q-15).
    pub coalesce_key: Option<u64>,
    pub command: Command,
}

impl CommandEnvelope {
    pub fn new(id: u64, command: Command) -> Self {
        CommandEnvelope {
            id: CommandId(id),
            expected_revision: None,
            coalesce_key: None,
            command,
        }
    }
}

/// Every variant must be routed by the dispatcher (exhaustive `match`).
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Score(ScoreCommand),
    Track(TrackCommand),
    Guitar(GuitarCommand),
    /// Commands that use the session pattern library (ADR-034).
    Pattern(PatternCommand),
}

/// Pattern engine commands (ADR-034 section 7).
#[derive(Clone, Debug, PartialEq)]
pub enum PatternCommand {
    /// Binds a library pattern to one voicing and replaces voices 1 and 2 of the track
    /// over whole bars `first..=last`. One atomic command, one undo step, one revision.
    ApplyPattern(ApplyPattern),
}

/// The arguments of `ApplyPattern`, as plain values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyPattern {
    pub track: TrackId,
    /// First and last master bar of the range, inclusive.
    pub first: usize,
    pub last: usize,
    pub pattern: crate::pattern::PatternId,
    /// Per string, string 1 first: a fret, or `None` for a muted string.
    pub voicing: Vec<Option<u8>>,
    /// The chord symbol of the voicing (for `Alt`), for example "C" or "G/B".
    pub chord: String,
    /// Required when the range is not a whole multiple of the pattern (no default).
    pub fit: Option<crate::pattern::Fit>,
    /// False: refuse with `WouldOverwrite` if a written voice holds beats in the range.
    pub overwrite: bool,
}

/// Commands that use Guitar context services to plan a score change.
#[derive(Clone, Debug, PartialEq)]
pub enum GuitarCommand {
    /// Changes the track's tuning and capo. `KeepPitches` moves every note so it sounds
    /// the same; `KeepFrets` keeps strings and frets. All or nothing.
    Retune {
        track: TrackId,
        instrument: InstrumentSpecInput,
        mode: RetuneMode,
    },
}

/// Written duration as plain values: base exponent (0 = whole ... 8), dots, tuplet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurationSpec {
    pub base: u8,
    pub dots: u8,
    pub tuplet: Option<(u8, u8)>,
}

impl DurationSpec {
    pub const fn plain(base: u8) -> Self {
        DurationSpec {
            base,
            dots: 0,
            tuplet: None,
        }
    }
}

impl DurationSpec {
    /// Plain values of a written duration (the reverse of the handler conversion).
    pub fn from_value(v: NoteValue) -> Self {
        DurationSpec {
            base: v.base().exponent(),
            dots: v.dots(),
            tuplet: v.tuplet().map(|t| (t.actual(), t.normal())),
        }
    }
}

/// A note of a clipboard fragment as plain values, no ids (ADR-030). Strings and frets,
/// not pitches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipNote {
    pub string: u8,
    pub fret: u8,
    pub tie: bool,
    pub effects: NoteEffects,
}

/// A beat of a clipboard fragment: its written duration, its notes and its stroke, no
/// ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipBeat {
    pub duration: DurationSpec,
    pub notes: Vec<ClipNote>,
    /// Stroke direction (ADR-035), kept by copy and paste.
    pub stroke: Option<Stroke>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeatAddress {
    pub track: TrackId,
    pub bar: usize,
    pub voice: usize,
    pub beat: BeatId,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ScoreCommand {
    InsertMasterBars {
        at: usize,
        count: usize,
        template: MasterBar,
    },
    RemoveMasterBars {
        at: usize,
        count: usize,
    },
    SetMasterBar {
        index: usize,
        bar: MasterBar,
    },
    /// Offset as (numerator, denominator) of a whole note; bpm as quarter notes per minute.
    SetTempo {
        bar: usize,
        offset: (i64, i64),
        bpm: (i64, i64),
    },
    RemoveTempo {
        bar: usize,
        offset: (i64, i64),
    },
    InsertBeat {
        track: TrackId,
        bar: usize,
        voice: usize,
        index: usize,
        duration: DurationSpec,
    },
    RemoveBeat {
        beat: BeatAddress,
    },
    SetBeatDuration {
        beat: BeatAddress,
        duration: DurationSpec,
    },
    SetNote {
        beat: BeatAddress,
        string: u8,
        fret: u8,
    },
    RemoveNote {
        beat: BeatAddress,
        note: NoteId,
    },
    SetTie {
        beat: BeatAddress,
        note: NoteId,
        tie: bool,
    },
    SetNoteEffects {
        beat: BeatAddress,
        note: NoteId,
        effects: NoteEffects,
    },
    /// Sets or clears the stroke direction of a beat (ADR-035).
    SetBeatStroke {
        beat: BeatAddress,
        stroke: Option<Stroke>,
    },
    AddVoice {
        track: TrackId,
        bar: usize,
    },
    RemoveVoice {
        track: TrackId,
        bar: usize,
        voice: usize,
    },
    ShiftFrets {
        track: TrackId,
        bars: (usize, usize),
        delta: i16,
    },
    /// Inserts a copied run of beats at `index` of a voice (ADR-030). All or nothing;
    /// one undo step. Never creates bars.
    PasteBeats {
        track: TrackId,
        bar: usize,
        voice: usize,
        index: usize,
        beats: Vec<ClipBeat>,
    },
    /// Removes the beats of one voice from `from` to `to`, both `(bar, index)` and
    /// inclusive, possibly across bars (ADR-030 cut). All or nothing; one undo step.
    RemoveBeats {
        track: TrackId,
        voice: usize,
        from: (usize, usize),
        to: (usize, usize),
    },
}

/// Fretted instrument as plain values; tuning lists string 1 (highest) first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentSpecInput {
    pub tuning: Vec<String>,
    pub fret_count: u8,
    pub capo: u8,
}

impl InstrumentSpecInput {
    pub fn standard_guitar() -> Self {
        InstrumentSpecInput {
            tuning: ["E4", "B3", "G3", "D3", "A2", "E2"]
                .map(String::from)
                .to_vec(),
            fret_count: 24,
            capo: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TrackCommand {
    Add {
        name: String,
        instrument: InstrumentSpecInput,
    },
    Remove {
        track: TrackId,
    },
    Rename {
        track: TrackId,
        name: String,
    },
    SetInstrument {
        track: TrackId,
        instrument: InstrumentSpecInput,
    },
    Move {
        track: TrackId,
        to: usize,
    },
}
