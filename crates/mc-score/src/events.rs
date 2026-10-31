//! Domain events returned by aggregate methods (ADR-006). They are invalidation
//! hints: ids and ranges, never content. The application layer wraps them in envelopes.

use crate::{NoteId, TrackId};
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainEvent {
    TrackAdded {
        track: TrackId,
    },
    TrackRemoved {
        track: TrackId,
    },
    /// Name or instrument changed.
    TrackChanged {
        track: TrackId,
    },
    TracksReordered,
    MasterBarsInserted {
        at: usize,
        count: usize,
    },
    MasterBarsRemoved {
        at: usize,
        count: usize,
    },
    /// Properties (time signature, key, repeats, markers) of these master bars changed.
    MasterBarsChanged {
        bars: Range<usize>,
    },
    BarContentChanged {
        track: TrackId,
        bars: Range<usize>,
    },
    TempoMapChanged,
    /// H-06: ties that became invalid because their preceding note changed were removed.
    TiesRemoved {
        track: TrackId,
        notes: Vec<NoteId>,
    },
}
