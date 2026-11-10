//! Event envelopes (ADR-006, interop-architecture section 4). Events are invalidation
//! hints; consumers re-query projections. There is no global bus: each session numbers
//! its own events and publishes them to its own sink.

use crate::{CommandId, Revision};
use mc_score::DomainEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DocumentId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppEvent {
    DocumentSaved {
        revision: Revision,
    },
    DirtyChanged {
        dirty: bool,
    },
    UndoAvailabilityChanged {
        can_undo: bool,
        can_redo: bool,
    },
    /// Undo or redo replaced content; the ranges in the accompanying domain events say where.
    HistoryApplied,
    /// The pattern library changed (a library command, or undo or redo of one). The UI
    /// re-queries the patterns, gestures and grid.
    PatternLibraryChanged,
    /// The playback transport changed state (started, stopped or failed). Session state
    /// only (Q3, Q6): it never changes the revision or the document.
    PlaybackStateChanged {
        state: crate::transport::PlaybackState,
    },
    /// The set of muted tracks changed (session state, not saved).
    MuteChanged {
        track: mc_score::TrackId,
        muted: bool,
    },
    /// A track was soloed or unsoloed (session state, not saved; ADR-032).
    SoloChanged {
        track: mc_score::TrackId,
        soloed: bool,
    },
    /// The volume or pan of a track changed (session state, not saved; ADR-032).
    MixChanged {
        track: mc_score::TrackId,
        mix: mc_playback::TrackMix,
    },
    /// The user tempo factor changed (session state, not saved).
    TempoFactorChanged {
        hundredths: u16,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventPayload {
    Domain(DomainEvent),
    App(AppEvent),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventEnvelope {
    /// Per document, gapless, starting at 1.
    pub seq: u64,
    pub document_id: DocumentId,
    /// Revision after the change that produced the event.
    pub revision: Revision,
    /// Command that caused the event; `None` for undo, redo and save without a command id.
    pub correlation_id: Option<CommandId>,
    /// Reserved for follow-up events of background jobs; always `None` in Phase 1.
    pub causation: Option<u64>,
    pub payload: EventPayload,
}

/// Port for publishing events (implemented by mc-ffi streams later, by tests now).
pub trait EventSink: Send {
    fn publish(&mut self, events: &[EventEnvelope]);
}

/// Sink that keeps everything in memory (tests, CLI).
#[derive(Debug, Default)]
pub struct CollectingSink {
    pub events: Vec<EventEnvelope>,
}

impl EventSink for CollectingSink {
    fn publish(&mut self, events: &[EventEnvelope]) {
        self.events.extend_from_slice(events);
    }
}
