//! The control thread and the public engine handle (audio-dsp-architecture sections 1
//! and 6). The handle is `Send + Sync`; it forwards requests over a channel to a single
//! control thread that owns the cpal stream, the command ring producer, the SoundFont
//! and the retired-object ring. The control thread:
//! - loads the SoundFont lazily on the first Play (Q10) and installs a warmed-up synth;
//! - sends timelines and transport commands to the callback;
//! - frees objects the callback hands back, so no free happens in the callback;
//! - rebuilds the stream after a device error and restores synth, timeline, position
//!   and play state (Q11: default output device only).

use crate::device::{Running, open_default};
use crate::error::AudioError;
use crate::rt::{Retired, RtCommand};
use crate::soundfont::{build_synth, parse_soundfont};
use crate::status::{EngineStatus, LastError, Status};
use mc_playback::{Channel, Pan, PerformanceTimeline, TempoFactor, Tick, Volume};
use rustysynth::SoundFont;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Engine settings.
#[derive(Clone, Debug, Default)]
pub struct EngineConfig {
    /// Path of the SoundFont (`GuitarSteel.sf2`). Loaded at the first Play; if it is
    /// missing or invalid, playback is disabled and [`AudioEngine::last_error`] says why.
    pub soundfont_path: Option<PathBuf>,
}

enum Msg {
    Load(Box<PerformanceTimeline>),
    Play,
    Stop,
    Seek(Tick),
    Factor(TempoFactor),
    Shutdown,
}

/// Handle to the audio engine. Dropping it stops the stream and joins the thread.
#[derive(Debug)]
pub struct AudioEngine {
    tx: Mutex<Sender<Msg>>,
    status: Arc<Status>,
    last_error: Arc<LastError>,
    thread: Option<JoinHandle<()>>,
}

impl AudioEngine {
    /// Open the default output device and start the control thread. Fails if there is no
    /// output device or the stream cannot start; the caller then disables playback.
    pub fn start(config: EngineConfig) -> Result<AudioEngine, AudioError> {
        let status = Arc::new(Status::default());
        let last_error = Arc::new(LastError::default());
        let (tx, rx) = mpsc::channel::<Msg>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), AudioError>>();
        let (st, le) = (status.clone(), last_error.clone());
        let thread = std::thread::Builder::new()
            .name("mc-audio-control".into())
            .spawn(move || {
                let device_failed = Arc::new(AtomicBool::new(false));
                let running = match open_default(st.clone(), device_failed.clone()) {
                    Ok(r) => {
                        let _ = ready_tx.send(Ok(()));
                        r
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                let mut control = Control {
                    config,
                    status: st,
                    last_error: le,
                    running: Some(running),
                    device_failed,
                    font: None,
                    timeline: None,
                    want_playing: false,
                    next_retry: None,
                };
                control.run(rx);
            })
            .map_err(|e| AudioError::Stream(e.to_string()))?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(AudioEngine {
                tx: Mutex::new(tx),
                status,
                last_error,
                thread: Some(thread),
            }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => Err(AudioError::EngineStopped),
        }
    }

    fn send(&self, msg: Msg) -> Result<(), AudioError> {
        self.tx
            .lock()
            .map_err(|_| AudioError::EngineStopped)?
            .send(msg)
            .map_err(|_| AudioError::EngineStopped)
    }

    /// Replace the timeline; playback continues at the same tick (all notes released).
    pub fn load_timeline(&self, timeline: PerformanceTimeline) -> Result<(), AudioError> {
        self.send(Msg::Load(Box::new(timeline)))
    }

    pub fn play(&self) -> Result<(), AudioError> {
        self.send(Msg::Play)
    }

    pub fn stop(&self) -> Result<(), AudioError> {
        self.send(Msg::Stop)
    }

    pub fn seek(&self, tick: Tick) -> Result<(), AudioError> {
        self.send(Msg::Seek(tick))
    }

    pub fn set_tempo_factor(&self, factor: TempoFactor) -> Result<(), AudioError> {
        self.send(Msg::Factor(factor))
    }

    /// Sets the volume and pan of a MIDI channel (ADR-032). Wait-free and never fails:
    /// it overwrites two atomics that the callback polls once per callback, so it is
    /// heard in the next callback, also on notes that are already sounding, and it
    /// survives a device rebuild and a synthesizer re-install. The last value wins, so
    /// a slider drag cannot fill any queue.
    pub fn set_channel_mix(&self, channel: Channel, volume: Volume, pan: Pan) {
        self.status
            .mix
            .set(channel.get() as usize, volume.get(), pan.cc());
    }

    pub fn status(&self) -> EngineStatus {
        self.status.snapshot()
    }

    /// Why playback is not working right now, if it is not (missing SoundFont, lost
    /// device); `None` when all is well.
    pub fn last_error(&self) -> Option<String> {
        self.last_error.get()
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        let _ = self.send(Msg::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

struct Control {
    config: EngineConfig,
    status: Arc<Status>,
    last_error: Arc<LastError>,
    running: Option<Running>,
    device_failed: Arc<AtomicBool>,
    font: Option<Arc<SoundFont>>,
    /// The current timeline, kept to restore it after a rebuild.
    timeline: Option<PerformanceTimeline>,
    want_playing: bool,
    next_retry: Option<Instant>,
}

impl Control {
    fn run(&mut self, rx: mpsc::Receiver<Msg>) {
        loop {
            match rx.recv_timeout(Duration::from_millis(20)) {
                Ok(Msg::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                Ok(msg) => self.handle(msg),
                Err(RecvTimeoutError::Timeout) => {}
            }
            self.housekeeping();
        }
    }

    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Load(tl) => {
                self.timeline = Some((*tl).clone());
                self.send(RtCommand::SwapTimeline(tl));
            }
            Msg::Play => {
                if !self.ensure_synth() {
                    return;
                }
                // Playing again from the end starts over.
                if self.status.finished.load(Relaxed) {
                    self.send(RtCommand::Seek(Tick(0)));
                }
                self.want_playing = true;
                self.send(RtCommand::Play);
            }
            Msg::Stop => {
                self.want_playing = false;
                self.send(RtCommand::Stop);
            }
            Msg::Seek(t) => self.send(RtCommand::Seek(t)),
            Msg::Factor(f) => self.send(RtCommand::SetFactor(f)),
            Msg::Shutdown => {}
        }
    }

    /// Make sure a synthesizer is installed; loads the SoundFont on first use. Playback
    /// is disabled with a message when this fails (no sine fallback).
    fn ensure_synth(&mut self) -> bool {
        if self.status.synth_ready.load(Relaxed) {
            return true;
        }
        let Some(rate) = self.running.as_ref().map(|r| r.sample_rate) else {
            self.last_error.set("no audio output device");
            return false;
        };
        match self.load_font().and_then(|f| build_synth(&f, rate)) {
            Ok(synth) => {
                self.last_error.clear();
                self.send(RtCommand::InstallSynth(synth));
                true
            }
            Err(e) => {
                self.last_error.set(e.to_string());
                false
            }
        }
    }

    fn load_font(&mut self) -> Result<Arc<SoundFont>, AudioError> {
        if let Some(f) = &self.font {
            return Ok(f.clone());
        }
        let path = self
            .config
            .soundfont_path
            .clone()
            .ok_or_else(|| AudioError::SoundFont("no SoundFont configured".into()))?;
        let bytes = std::fs::read(&path)
            .map_err(|e| AudioError::SoundFont(format!("{}: {e}", path.display())))?;
        let font = parse_soundfont(&bytes)?;
        self.font = Some(font.clone());
        Ok(font)
    }

    /// Push a command to the callback, waiting briefly if the ring is full.
    fn send(&mut self, cmd: RtCommand) {
        let Some(running) = self.running.as_mut() else {
            return;
        };
        let mut cmd = cmd;
        for _ in 0..100 {
            match running.ends.cmd.push(cmd) {
                Ok(()) => return,
                Err(rtrb::PushError::Full(back)) => {
                    cmd = back;
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
        self.last_error
            .set("audio engine is not accepting commands");
    }

    fn housekeeping(&mut self) {
        // Free what the callback handed back (never freed in the callback).
        if let Some(running) = self.running.as_mut() {
            while let Ok(item) = running.ends.ret.pop() {
                match item {
                    Retired::Timeline(t) => drop(t),
                    Retired::Synth(s) => drop(s),
                }
            }
        }
        if self.device_failed.swap(false, Relaxed) {
            self.running = None;
            self.status.playing.store(false, Relaxed);
            self.status.synth_ready.store(false, Relaxed);
            self.next_retry = Some(Instant::now());
        }
        if self.running.is_none() && self.next_retry.is_some_and(|t| Instant::now() >= t) {
            self.rebuild();
        }
    }

    /// Reopen the default device and restore synth, timeline, position and play state.
    fn rebuild(&mut self) {
        match open_default(self.status.clone(), self.device_failed.clone()) {
            Ok(running) => {
                self.running = Some(running);
                self.next_retry = None;
                self.status.device_rebuilds.fetch_add(1, Relaxed);
                self.last_error.clear();
                let tick = Tick(self.status.tick.load(Relaxed));
                if let Some(tl) = self.timeline.clone() {
                    self.send(RtCommand::SwapTimeline(Box::new(tl)));
                    self.send(RtCommand::Seek(tick));
                }
                if self.font.is_some() && self.ensure_synth() && self.want_playing {
                    self.send(RtCommand::Play);
                }
            }
            Err(e) => {
                self.last_error.set(e.to_string());
                self.next_retry = Some(Instant::now() + Duration::from_secs(1));
            }
        }
    }
}
