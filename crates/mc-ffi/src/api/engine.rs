//! Playback engine API (slice 4d, ADR-028). `EngineHandle` wraps `mc_audio::AudioEngine`
//! and is the composition root's implementation of the application's `AudioPort`. The UI
//! drives transport through a document and an engine together: the application compiles
//! the timeline and decides start positions, mute and swap-on-edit; the engine only makes
//! sound (GR-05).
//!
//! No `unsafe` here (ADR-022); the one reviewed `unsafe` of the audio stack is the MMCSS
//! call inside `mc-audio` (ADR-027). Positions cross FFI as `i64` (ADR-005 amendment 2).

use crate::api::document::DocumentHandle;
use crate::api::types::{ApiError, OutcomeDto, PositionDto};
use crate::frb_generated::StreamSink;
use flutter_rust_bridge::frb;
use mc_app::playback::{Channel, Pan, PerformanceTimeline, TempoFactor, Tick, Volume};
use mc_app::{AudioPort, PlaybackState, TransportCommand, TransportError};
use mc_audio::{AudioEngine, EngineConfig};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// The audio engine. One per app; cloning the Dart handle shares the same engine.
#[frb(opaque)]
#[derive(Clone)]
pub struct EngineHandle {
    inner: Arc<AudioEngine>,
}

impl std::fmt::Debug for EngineHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineHandle").finish_non_exhaustive()
    }
}

/// Adapts `mc_audio::AudioEngine` to the application's `AudioPort` (ADR-028). The
/// application calls this; it never sees `mc-audio` directly. Engine errors become
/// `TransportError::Audio` with the engine message.
struct EnginePort<'a>(&'a AudioEngine);

impl AudioPort for EnginePort<'_> {
    fn load_timeline(&mut self, timeline: PerformanceTimeline) -> Result<(), TransportError> {
        self.0
            .load_timeline(timeline)
            .map_err(|e| TransportError::Audio(e.to_string()))
    }

    fn play(&mut self, start: Tick) -> Result<(), TransportError> {
        self.0
            .seek(start)
            .and_then(|()| self.0.play())
            .map_err(|e| TransportError::Audio(e.to_string()))
    }

    fn stop(&mut self) -> Result<(), TransportError> {
        self.0
            .stop()
            .map_err(|e| TransportError::Audio(e.to_string()))
    }

    fn seek(&mut self, tick: Tick) -> Result<(), TransportError> {
        self.0
            .seek(tick)
            .map_err(|e| TransportError::Audio(e.to_string()))
    }

    fn set_tempo_factor(&mut self, factor: TempoFactor) -> Result<(), TransportError> {
        self.0
            .set_tempo_factor(factor)
            .map_err(|e| TransportError::Audio(e.to_string()))
    }

    fn set_channel_mix(
        &mut self,
        channel: Channel,
        volume: Volume,
        pan: Pan,
    ) -> Result<(), TransportError> {
        self.0.set_channel_mix(channel, volume, pan);
        Ok(())
    }

    fn state(&self) -> PlaybackState {
        engine_state(self.0)
    }

    fn position(&self) -> Tick {
        Tick(self.0.status().tick)
    }

    fn last_error(&self) -> Option<String> {
        self.0.last_error()
    }
}

/// Maps engine status to the application's `PlaybackState`. A reported error while the
/// engine is not playing means playback failed (missing SoundFont, lost device, panic).
fn engine_state(engine: &AudioEngine) -> PlaybackState {
    let s = engine.status();
    if s.playing {
        PlaybackState::Playing
    } else if s.panicked || engine.last_error().is_some() {
        PlaybackState::Failed
    } else {
        PlaybackState::Stopped
    }
}

use crate::api::document::playback_state_dto;

impl EngineHandle {
    /// Start the engine on the default output device, loading the bundled SoundFont
    /// lazily on the first Play (Q10). Fails only if there is no output device at all;
    /// a missing SoundFont is reported later through [`EngineHandle::last_error`], not
    /// here (no sine fallback).
    pub fn start(soundfont_path: Option<String>) -> Result<EngineHandle, ApiError> {
        let config = EngineConfig {
            soundfont_path: soundfont_path.map(PathBuf::from),
        };
        let engine =
            AudioEngine::start(config).map_err(|e| ApiError::new("audio", e.to_string()))?;
        Ok(EngineHandle {
            inner: Arc::new(engine),
        })
    }

    /// Run `cmd` against `document` through this engine. The application compiles the
    /// timeline and drives the port; the returned outcome carries the session events
    /// (`PlaybackStateChanged`, `MuteChanged`, `TempoFactorChanged`) and is also
    /// published to the document's event stream.
    fn run(
        &self,
        document: &DocumentHandle,
        cmd: TransportCommand,
    ) -> Result<OutcomeDto, ApiError> {
        let mut port = EnginePort(&self.inner);
        let mut session = document.lock_for_transport();
        let outcome = session
            .transport(cmd, &mut port)
            .map_err(|e| ApiError::new("transport", e.to_string()))?;
        Ok(crate::api::document::outcome_dto(&outcome))
    }

    /// Start playback. `from_master_bar` is the bar containing the cursor (Q7); `None`
    /// resumes from the current position.
    pub fn play(
        &self,
        document: &DocumentHandle,
        from_master_bar: Option<u32>,
    ) -> Result<OutcomeDto, ApiError> {
        self.run(
            document,
            TransportCommand::Play {
                from_master_bar: from_master_bar.map(|b| b as usize),
            },
        )
    }

    pub fn stop(&self, document: &DocumentHandle) -> Result<OutcomeDto, ApiError> {
        self.run(document, TransportCommand::Stop)
    }

    /// Seek to a tick on the current timeline (click-to-seek, Q7).
    pub fn seek(&self, document: &DocumentHandle, tick: i64) -> Result<OutcomeDto, ApiError> {
        self.run(document, TransportCommand::Seek { tick: Tick(tick) })
    }

    pub fn set_muted(
        &self,
        document: &DocumentHandle,
        track: i64,
        muted: bool,
    ) -> Result<OutcomeDto, ApiError> {
        self.run(
            document,
            TransportCommand::SetMuted {
                track: mc_app::types::TrackId::from_raw(track as u64),
                muted,
            },
        )
    }

    /// Solo or unsolo a track (ADR-032). While any track is soloed the others are muted.
    pub fn set_soloed(
        &self,
        document: &DocumentHandle,
        track: i64,
        soloed: bool,
    ) -> Result<OutcomeDto, ApiError> {
        self.run(
            document,
            TransportCommand::SetSoloed {
                track: mc_app::types::TrackId::from_raw(track as u64),
                soloed,
            },
        )
    }

    /// Set a track's volume (0..=127, default 100) and pan (-63..=63, default 0)
    /// (ADR-032). Values outside the ranges are rejected with a `transport` error and
    /// change nothing.
    pub fn set_track_mix(
        &self,
        document: &DocumentHandle,
        track: i64,
        volume: i32,
        pan: i32,
    ) -> Result<OutcomeDto, ApiError> {
        let cmd = TransportCommand::track_mix(
            mc_app::types::TrackId::from_raw(track as u64),
            volume,
            pan,
        )
        .map_err(|e| ApiError::new("transport", e.to_string()))?;
        self.run(document, cmd)
    }

    /// Set the user tempo factor in hundredths (100 = 1.0x), 25..=200 in steps of 5 (Q6).
    pub fn set_tempo_factor(
        &self,
        document: &DocumentHandle,
        hundredths: u16,
    ) -> Result<OutcomeDto, ApiError> {
        let factor =
            TempoFactor::new(hundredths).map_err(|e| ApiError::new("transport", e.to_string()))?;
        self.run(document, TransportCommand::SetTempoFactor { factor })
    }

    /// Recompile and swap the timeline if an edit happened while playing (ADR-028 item 5).
    /// A no-op when stopped. Call after a command that changed the score during playback.
    pub fn refresh_after_edit(&self, document: &DocumentHandle) -> Result<OutcomeDto, ApiError> {
        let mut port = EnginePort(&self.inner);
        let mut session = document.lock_for_transport();
        let outcome = session
            .refresh_playback_after_edit(&mut port)
            .map_err(|e| ApiError::new("transport", e.to_string()))?;
        Ok(crate::api::document::outcome_dto(&outcome))
    }

    /// Current position for a one-off query. The continuous cursor uses
    /// [`EngineHandle::positions`].
    #[frb(sync)]
    pub fn position(&self) -> PositionDto {
        let s = self.inner.status();
        PositionDto {
            tick: s.tick,
            playing: s.playing,
            state: playback_state_dto(engine_state(&self.inner)),
        }
    }

    /// Why playback is not working, if it is not (missing SoundFont, lost device);
    /// `None` when all is well.
    #[frb(sync)]
    pub fn last_error(&self) -> Option<String> {
        self.inner.last_error()
    }

    /// A ~60 Hz stream of playback positions for the moving cursor (ADR-028, ADR-005
    /// amendment 5: Dart extrapolates between updates). The stream runs until the Dart
    /// side unsubscribes (drops the subscription) or the engine is dropped; Rust owns the
    /// lifecycle (ADR-005 amendment 1). All values map to Dart `int`/`bool`.
    pub fn positions(&self, sink: StreamSink<PositionDto>) {
        let engine = self.inner.clone();
        std::thread::Builder::new()
            .name("mc-ffi-position-stream".into())
            .spawn(move || {
                // ~60 Hz.
                let period = Duration::from_micros(16_667);
                loop {
                    let s = engine.status();
                    let update = PositionDto {
                        tick: s.tick,
                        playing: s.playing,
                        state: playback_state_dto(engine_state(&engine)),
                    };
                    // `add` fails once the Dart side drops its subscription; Rust then
                    // ends the thread and releases the sink (ADR-005 amendment 1).
                    if sink.add(update).is_err() {
                        break;
                    }
                    std::thread::sleep(period);
                }
            })
            .expect("spawn position stream thread");
    }
}
