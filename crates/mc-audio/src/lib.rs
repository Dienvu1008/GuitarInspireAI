//! Audio engine adapter (ADR-007, ADR-024, audio-dsp-architecture sections 1 to 3 and 6).
//!
//! Control plane and data plane are separate. A control thread ([`AudioEngine`]) owns the
//! cpal output stream, loads the SoundFont and compiles nothing itself: it receives a
//! `PerformanceTimeline` from `mc-playback`, sends it with transport commands through
//! wait-free rings to the audio callback, and frees what the callback hands back. The
//! callback core in [`rt`] renders with rustysynth and follows the real-time rules of
//! GR-10. [`OfflineEngine`] runs the same core without a device.

mod control;
mod device;
mod engine;
mod error;
mod offline;
mod platform;
pub mod rt;
mod soundfont;
mod status;

#[cfg(test)]
mod tests;

pub use control::{AudioEngine, EngineConfig};
pub use error::AudioError;
pub use offline::OfflineEngine;
pub use soundfont::{build_synth, parse_soundfont};
pub use status::{EngineStatus, LastError, Promotion, Status};
