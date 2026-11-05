//! Tuner API (slice 5d, ADR-029). `TunerHandle` wraps `mc_audio::TunerHandle`: it owns
//! the microphone input stream and the detection thread, independent of the playback
//! engine (a tuner and playback do not share state). The audio side produces only a
//! frequency and a clarity; mapping a frequency to "which string, how many cents off"
//! uses the active track's tuning and capo and is done by the application
//! (`DocumentSession::tuner_match`, GR-05, ADR-029 T8), so this module passes a document
//! and a track into the reading.
//!
//! No `unsafe` here (ADR-022); the tuner thread reuses the MMCSS call in `mc-audio`
//! (ADR-027). The microphone is live only while a `TunerHandle` exists; dropping it (or
//! calling `stop`) closes the input stream.

use crate::api::document::DocumentHandle;
use crate::api::types::{ApiError, TunerReadingDto};
use flutter_rust_bridge::frb;
use mc_audio::TunerHandle as AudioTuner;
use std::sync::Mutex;

/// A running microphone tuner. Opaque to Dart; dropping the Dart handle stops it.
#[frb(opaque)]
pub struct TunerHandle {
    inner: Mutex<Option<AudioTuner>>,
}

impl std::fmt::Debug for TunerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TunerHandle").finish_non_exhaustive()
    }
}

impl TunerHandle {
    /// Open the default microphone and start detecting. Fails with a typed error (A10)
    /// when there is no input device, the OS denies access, or the device is busy; the
    /// UI then shows the message and keeps the chord finder and fretboard working.
    pub fn start() -> Result<TunerHandle, ApiError> {
        let tuner =
            AudioTuner::start().map_err(|e| ApiError::new(tuner_rule(&e), e.to_string()))?;
        Ok(TunerHandle {
            inner: Mutex::new(Some(tuner)),
        })
    }

    /// Stop the tuner and release the microphone now (also happens on drop).
    #[frb(sync)]
    pub fn stop(&self) {
        *self.lock() = None;
    }

    /// `true` if the input device was lost after starting; the UI should stop and show a
    /// message.
    #[frb(sync)]
    pub fn failed(&self) -> bool {
        self.lock().as_ref().map(|t| t.failed()).unwrap_or(true)
    }

    /// The current reading, mapped to the strings of `track` in `document` (its tuning
    /// and capo, ADR-029 T8). Poll this at the UI rate (~30 Hz). When the track's tuning
    /// or capo changes, the next call reflects it with no restart. Returns a reading with
    /// `has_pitch = false` when there is no confident pitch or the tuner is stopped.
    #[frb(sync)]
    pub fn reading(&self, document: &DocumentHandle, track: i64) -> TunerReadingDto {
        let guard = self.lock();
        let Some(tuner) = guard.as_ref() else {
            return silent();
        };
        let r = tuner.reading();
        let Some(hz) = r.hz else {
            return TunerReadingDto {
                has_pitch: false,
                hz: 0.0,
                clarity: r.clarity as f64,
                string: 0,
                cents: 0.0,
                target_hz: 0.0,
                updates: r.updates as i64,
            };
        };
        // Map to the track's strings in the application layer (the music rule).
        let session = document.lock_for_transport();
        match session.tuner_match(mc_app::types::TrackId::from_raw(track as u64), hz) {
            Some(m) => TunerReadingDto {
                has_pitch: true,
                hz: hz as f64,
                clarity: r.clarity as f64,
                string: m.string as i64,
                cents: m.cents as f64,
                target_hz: m.target_hz as f64,
                updates: r.updates as i64,
            },
            None => TunerReadingDto {
                has_pitch: false,
                hz: hz as f64,
                clarity: r.clarity as f64,
                string: 0,
                cents: 0.0,
                target_hz: 0.0,
                updates: r.updates as i64,
            },
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<AudioTuner>> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn silent() -> TunerReadingDto {
    TunerReadingDto {
        has_pitch: false,
        hz: 0.0,
        clarity: 0.0,
        string: 0,
        cents: 0.0,
        target_hz: 0.0,
        updates: 0,
    }
}

/// A short rule code for the tuner start errors (A10), for the UI to branch on.
fn tuner_rule(e: &mc_audio::AudioError) -> &'static str {
    match e {
        mc_audio::AudioError::NoInputDevice => "tuner-no-device",
        mc_audio::AudioError::MicPermissionDenied(_) => "tuner-permission",
        _ => "tuner",
    }
}
