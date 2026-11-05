//! The engine without a device: the same callback core, driven by explicit `process`
//! calls. Used by tests, the allocation check (FF-16), benchmarks and the offline part
//! of the glitch procedure (FF-18).

use crate::engine::{ControlEnds, build_rt};
use crate::error::AudioError;
use crate::rt::{Retired, Rt, RtCommand};
use crate::soundfont::{build_synth, parse_soundfont};
use crate::status::{EngineStatus, Status};
use mc_playback::{PerformanceTimeline, TempoFactor, Tick};
use rustysynth::SoundFont;
use std::sync::Arc;

#[derive(Debug)]
pub struct OfflineEngine {
    rt: Rt,
    ends: ControlEnds,
    rate: u32,
    font: Option<Arc<SoundFont>>,
}

impl OfflineEngine {
    pub fn new(sample_rate: u32) -> Self {
        let (rt, ends) = build_rt(sample_rate, Arc::new(Status::default()));
        OfflineEngine {
            rt,
            ends,
            rate: sample_rate,
            font: None,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    /// Parse `sf2` bytes, build a warmed-up synthesizer and install it.
    pub fn install_soundfont(&mut self, sf2: &[u8]) -> Result<(), AudioError> {
        let font = parse_soundfont(sf2)?;
        let synth = build_synth(&font, self.rate)?;
        self.font = Some(font);
        self.send(RtCommand::InstallSynth(synth));
        Ok(())
    }

    /// Send a command; panics if the command ring is full (tests drain between calls).
    pub fn send(&mut self, cmd: RtCommand) {
        assert!(self.ends.cmd.push(cmd).is_ok(), "command ring full");
    }

    pub fn load_timeline(&mut self, timeline: PerformanceTimeline) {
        self.send(RtCommand::SwapTimeline(Box::new(timeline)));
    }

    pub fn play(&mut self) {
        self.send(RtCommand::Play);
    }

    pub fn stop(&mut self) {
        self.send(RtCommand::Stop);
    }

    pub fn seek(&mut self, tick: Tick) {
        self.send(RtCommand::Seek(tick));
    }

    pub fn set_tempo_factor(&mut self, factor: TempoFactor) {
        self.send(RtCommand::SetFactor(factor));
    }

    /// Run the callback core over `out` (interleaved, `channels` per frame).
    pub fn process(&mut self, out: &mut [f32], channels: usize) {
        self.rt.process(out, channels);
    }

    /// Render `frames` stereo frames (interleaved).
    pub fn render(&mut self, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames * 2];
        self.rt.process(&mut out, 2);
        out
    }

    pub fn status(&self) -> EngineStatus {
        self.ends.status.snapshot()
    }

    /// Free what the callback handed back; returns how many objects.
    pub fn drain_retired(&mut self) -> usize {
        let mut n = 0;
        while let Ok(item) = self.ends.ret.pop() {
            match item {
                Retired::Timeline(t) => drop(t),
                Retired::Synth(s) => drop(s),
            }
            n += 1;
        }
        n
    }

    #[cfg(test)]
    pub(crate) fn rt_mut(&mut self) -> &mut Rt {
        &mut self.rt
    }
}
