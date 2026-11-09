//! Playback domain service (ADR-011, ADR-024, audio-dsp-architecture section 3a).
//!
//! `mc-playback` turns a score snapshot into a [`PerformanceTimeline`]: a sorted list
//! of MIDI-like events with a tempo map, played-bar table and channel assignments. The
//! compiler is a pure, deterministic function (PB-09): the same snapshot and options
//! always produce a byte-identical [`PerformanceTimeline::canonical_text`]. It has no
//! clocks, randomness, I/O or hash-map iteration order, and it is wasm-compatible
//! (FF-04) and free of the disallowed domain constructs (FF-03).
//!
//! The crate owns the MIDI value objects ([`MidiNote`], [`Channel`], [`Velocity`],
//! [`GmProgram`]) and the rendering conventions of ADR-024 (fixed velocity, dead-note
//! length, channel order, program). Audio output, SoundFont loading and the real-time
//! thread live in `mc-audio` (slice 4b); MIDI file interchange lives in `mc-midi`
//! (ADR-023). This crate does none of that.

mod compile;
mod event;
mod timeline;
mod types;

pub use compile::{
    CompileError, CompileOptions, CompileOutput, PlaybackWarning, assign_channels, compile,
};
pub use event::{NoteSource, PerformanceEvent, TimedEvent};
pub use timeline::{ChannelAssignment, PerformanceTimeline, PlayedBar, TempoPoint};
pub use types::{
    Channel, GmProgram, MidiNote, OutOfRange, Pan, TempoFactor, Tick, TickRate, TrackMix, Velocity,
    Volume,
};
