//! Timeline events. The variant order of [`PerformanceEvent`] is also the same-tick
//! ordering (audio-dsp-architecture section 3a rule 5): note off, tempo, program
//! change, note on. `#[derive(PartialOrd, Ord)]` on the enum uses declaration order,
//! so sorting a slice of `(tick, discriminant)` gives the required order for free.

use crate::types::{Channel, GmProgram, MidiNote, Tick, Velocity};
use mc_score::{NoteId, TrackId};
use mc_theory::Tempo;

/// Where a note on came from, so the player (slice 4d) can map a sounding note back to
/// the score. `master_bar` is the 0-based master-bar index in the score, not a
/// played-bar order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoteSource {
    pub track: TrackId,
    pub master_bar: usize,
    pub note: NoteId,
}

/// One MIDI-like event, without its tick. Ordered by declaration for same-tick sorting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerformanceEvent {
    NoteOff {
        channel: Channel,
        key: MidiNote,
    },
    Tempo {
        bpm: Tempo,
    },
    ProgramChange {
        channel: Channel,
        program: GmProgram,
    },
    NoteOn {
        channel: Channel,
        key: MidiNote,
        velocity: Velocity,
        source: NoteSource,
    },
}

impl PerformanceEvent {
    /// Same-tick category (rule 5): 0 note off, 1 tempo, 2 program, 3 note on.
    pub(crate) fn category(&self) -> u8 {
        match self {
            PerformanceEvent::NoteOff { .. } => 0,
            PerformanceEvent::Tempo { .. } => 1,
            PerformanceEvent::ProgramChange { .. } => 2,
            PerformanceEvent::NoteOn { .. } => 3,
        }
    }
}

/// An event with its tick on the timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimedEvent {
    pub tick: Tick,
    pub event: PerformanceEvent,
}
