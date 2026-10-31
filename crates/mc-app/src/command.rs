//! Commands are application-layer types with plain values, as adapters (CLI, later FFI)
//! send them. Handlers convert them into value objects; the domain validates the rest.

use mc_score::{BeatId, MasterBar, NoteEffects, NoteId, TrackId};

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
