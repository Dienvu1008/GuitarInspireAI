//! Score content types. Value types have public fields because the only way into a
//! `Score` is validated (`Score::from_parts` or aggregate methods) and a `Score` hands
//! out shared references only.

use crate::{BeatId, NoteId, TrackId};
use mc_theory::fretted::{Capo, FretPosition, Tuning};
use mc_theory::{KeySignature, NoteValue, Rational, Tempo, TimeSignature};
use std::sync::Arc;

/// Song-level bar properties shared by all tracks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MasterBar {
    pub time_signature: TimeSignature,
    pub key: KeySignature,
    /// Pickup (anacrusis): nominal length 0, short content is not a diagnostic.
    pub pickup: bool,
    pub repeat_start: bool,
    /// Total play count of the repeated section (2..=16) ending at this bar.
    pub repeat_end: Option<u8>,
    /// Passes (1..=16, strictly increasing) on which this bar is played. Empty = every pass.
    pub alternate_endings: Vec<u8>,
    pub marker: Option<String>,
}

impl MasterBar {
    pub fn new(time_signature: TimeSignature, key: KeySignature) -> Self {
        MasterBar {
            time_signature,
            key,
            pickup: false,
            repeat_start: false,
            repeat_end: None,
            alternate_endings: Vec::new(),
            marker: None,
        }
    }
}

impl Default for MasterBar {
    fn default() -> Self {
        MasterBar::new(TimeSignature::COMMON, KeySignature::C_MAJOR)
    }
}

/// Instrument of a track. Non-fretted variants are added when such tracks exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstrumentSpec {
    Fretted(FrettedInstrument),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrettedInstrument {
    pub tuning: Tuning,
    /// Highest fret (C-01 range 1..=36 is checked by the application layer).
    pub fret_count: u8,
    pub capo: Capo,
}

impl FrettedInstrument {
    pub fn standard_guitar() -> Self {
        FrettedInstrument {
            tuning: Tuning::standard_guitar(),
            fret_count: 24,
            capo: Capo(0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct NoteEffects {
    pub let_ring: bool,
    pub dead: bool,
}

/// What a note plays. Only fretted notes exist until non-fretted tracks are introduced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteKind {
    /// Authoritative string and fret; sounding pitch is derived from tuning and capo.
    Fretted(FretPosition),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub id: NoteId,
    pub kind: NoteKind,
    /// Tied from the note on the same string in the preceding beat of the same voice (H-06).
    pub tie: bool,
    pub effects: NoteEffects,
}

impl Note {
    pub fn position(&self) -> FretPosition {
        match self.kind {
            NoteKind::Fretted(p) => p,
        }
    }
}

/// A single attack event. No notes means a rest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Beat {
    pub id: BeatId,
    pub value: NoteValue,
    pub notes: Vec<Note>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Voice {
    pub beats: Vec<Beat>,
}

/// One track's content for one master bar. At least one voice (H-09).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarContent {
    pub voices: Vec<Voice>,
}

impl BarContent {
    pub fn empty() -> Self {
        BarContent {
            voices: vec![Voice::default()],
        }
    }
}

/// Tempo change at a score position. Offset is within the bar's nominal length (H-11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TempoChange {
    pub bar: usize,
    pub offset: Rational,
    pub tempo: Tempo,
}

/// Track input for `Score::from_parts`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackData {
    pub id: TrackId,
    pub name: String,
    pub instrument: InstrumentSpec,
    pub bars: Vec<BarContent>,
}

/// A track inside a score. Bars are shared between snapshots until edited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub(crate) id: TrackId,
    pub(crate) name: String,
    pub(crate) instrument: InstrumentSpec,
    pub(crate) bars: Vec<Arc<BarContent>>,
}

impl Track {
    pub fn id(&self) -> TrackId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn instrument(&self) -> &InstrumentSpec {
        &self.instrument
    }

    pub fn bars(&self) -> impl ExactSizeIterator<Item = &BarContent> {
        self.bars.iter().map(|b| b.as_ref())
    }

    pub fn bar(&self, index: usize) -> Option<&BarContent> {
        self.bars.get(index).map(|b| b.as_ref())
    }

    /// The shared content of one bar. Edits copy only what they change (ADR-016), so
    /// an unchanged bar keeps the same `Arc` across revisions; a view cache that holds
    /// the `Arc` can recognise it with `Arc::ptr_eq` (ADR-031). Read-only.
    pub fn bar_shared(&self, index: usize) -> Option<&Arc<BarContent>> {
        self.bars.get(index)
    }

    pub fn fretted(&self) -> &FrettedInstrument {
        match &self.instrument {
            InstrumentSpec::Fretted(f) => f,
        }
    }

    /// True if bar `index` is the same allocation in both tracks (structural sharing).
    pub fn shares_bar_with(&self, other: &Track, index: usize) -> bool {
        match (self.bars.get(index), other.bars.get(index)) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
