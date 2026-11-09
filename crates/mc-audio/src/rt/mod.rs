//! Real-time core (GR-10, ADR-007, ADR-024). Everything in this directory runs inside
//! the audio callback, so it must not allocate or free memory, take locks, block, do
//! I/O, log or call into Dart. It uses only wait-free ring operations, atomics and
//! buffers that were allocated before the callback started (FF-16 checks allocation at
//! run time, FF-17 scans this directory for forbidden constructs). Construction of the
//! buffers, rings and the shared status happens outside this directory.
//!
//! Events are applied sample-accurately: a block is split at every event, so an event
//! never lands late by up to a block. Time is tracked in samples under the current
//! tempo factor; a factor change re-anchors the sample position at the same tick.
//!
//! Failure handling (ADR-024 item 7): the callback body runs inside `catch_unwind`. On a
//! panic the engine outputs silence from then on and raises the `panicked` flag; the
//! control thread reports it and can rebuild the engine.

use crate::status::{MIX_CHANNELS, Status};
use mc_playback::{PerformanceEvent, PerformanceTimeline, TempoFactor, Tick, TimedEvent};
use rtrb::{Consumer, Producer};
use rustysynth::Synthesizer;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

/// Frames rendered per synth call at most; the preallocated buffers have this size.
pub const MAX_BLOCK: usize = 8192;
/// Commands handled per callback (bounded work per callback).
const COMMANDS_PER_CALLBACK: usize = 8;

/// Control thread to callback.
#[derive(Debug)]
pub enum RtCommand {
    Play,
    Stop,
    Seek(Tick),
    SetFactor(TempoFactor),
    /// Replace the timeline. The callback continues at the same tick (ADR-024 item 6:
    /// all notes are released first).
    SwapTimeline(Box<PerformanceTimeline>),
    /// Install a synthesizer that was built and warmed up off the callback thread.
    InstallSynth(Box<Synthesizer>),
}

/// Callback to control thread: objects that must be freed off the callback thread.
#[derive(Debug)]
pub enum Retired {
    Timeline(Box<PerformanceTimeline>),
    Synth(Box<Synthesizer>),
}

/// Everything the callback owns, prepared by the control thread.
#[derive(Debug)]
pub struct RtParts {
    pub sample_rate: u32,
    pub left: Box<[f32]>,
    pub right: Box<[f32]>,
    pub cmd: Consumer<RtCommand>,
    pub ret: Producer<Retired>,
    pub status: Arc<Status>,
}

#[derive(Debug)]
pub struct Rt {
    rate: u32,
    synth: Option<Box<Synthesizer>>,
    timeline: Option<Box<PerformanceTimeline>>,
    /// Index of the next event to apply.
    cursor: usize,
    /// Position in samples under `factor`.
    pos: u64,
    factor: TempoFactor,
    playing: bool,
    failed: bool,
    left: Box<[f32]>,
    right: Box<[f32]>,
    cmd: Consumer<RtCommand>,
    ret: Producer<Retired>,
    status: Arc<Status>,
    /// The `(volume, pan)` controller values last sent to the synthesizer per channel;
    /// `NOT_SENT` forces a send. Fixed size, so applying the mix never allocates.
    applied: [[u8; 2]; MIX_CHANNELS],
    #[cfg(test)]
    pub(crate) inject_panic: bool,
    /// Test only: skip applying the mix, which is how the engine behaved before ADR-032
    /// (ADR-032 X1 compares against it).
    #[cfg(test)]
    pub(crate) mix_disabled: bool,
}

/// Marks a controller value as not yet sent (valid values are 0..=127).
const NOT_SENT: u8 = 0xFF;

fn silence(out: &mut [f32]) {
    for s in out.iter_mut() {
        *s = 0.0;
    }
}

impl Rt {
    pub fn new(parts: RtParts) -> Self {
        Rt {
            rate: parts.sample_rate,
            synth: None,
            timeline: None,
            cursor: 0,
            pos: 0,
            factor: TempoFactor::NORMAL,
            playing: false,
            failed: false,
            left: parts.left,
            right: parts.right,
            cmd: parts.cmd,
            ret: parts.ret,
            status: parts.status,
            applied: [[NOT_SENT; 2]; MIX_CHANNELS],
            #[cfg(test)]
            inject_panic: false,
            #[cfg(test)]
            mix_disabled: false,
        }
    }

    /// Fill an interleaved output buffer. Must not allocate, lock or block.
    pub fn process(&mut self, out: &mut [f32], channels: usize) {
        if self.failed {
            silence(out);
            return;
        }
        let result = catch_unwind(AssertUnwindSafe(|| self.process_inner(out, channels)));
        if result.is_err() {
            self.failed = true;
            self.playing = false;
            self.status.panicked.store(true, Relaxed);
            self.status.playing.store(false, Relaxed);
            silence(out);
        }
    }

    fn process_inner(&mut self, out: &mut [f32], channels: usize) {
        #[cfg(test)]
        if self.inject_panic {
            panic!("injected panic");
        }
        self.drain_commands();
        self.apply_mix();
        let channels = channels.max(1);
        let total = out.len() / channels;
        let mut done = 0;
        while done < total {
            let want = (total - done).min(MAX_BLOCK);
            let n = if self.playing {
                self.next_span(want)
            } else {
                want
            };
            self.render(
                &mut out[done * channels..(done + n) * channels],
                channels,
                n,
            );
            if self.playing {
                self.pos += n as u64;
            }
            done += n;
        }
        self.publish_position();
    }

    /// Frames to render before the next event or the end, at most `want`, at least 1.
    /// Applies every event that is due first.
    fn next_span(&mut self, want: usize) -> usize {
        self.apply_due_events();
        let Some(end) = self.end_sample() else {
            return want;
        };
        if self.pos >= end {
            self.finish();
            return want;
        }
        let mut span = want.min((end - self.pos).min(MAX_BLOCK as u64) as usize);
        if let Some(at) = self.event_sample(self.cursor) {
            span = span.min(at.saturating_sub(self.pos).max(1) as usize);
        }
        span.max(1)
    }

    fn render(&mut self, out: &mut [f32], channels: usize, n: usize) {
        match self.synth.as_mut() {
            Some(synth) => synth.render(&mut self.left[..n], &mut self.right[..n]),
            None => {
                silence(&mut self.left[..n]);
                silence(&mut self.right[..n]);
            }
        }
        for i in 0..n {
            let (l, r) = (self.left[i], self.right[i]);
            let frame = &mut out[i * channels..(i + 1) * channels];
            if channels == 1 {
                frame[0] = 0.5 * (l + r);
            } else {
                frame[0] = l;
                frame[1] = r;
                for s in frame.iter_mut().skip(2) {
                    *s = 0.0;
                }
            }
        }
    }

    fn drain_commands(&mut self) {
        for _ in 0..COMMANDS_PER_CALLBACK {
            match self.cmd.pop() {
                Ok(c) => self.handle(c),
                Err(_) => break,
            }
        }
    }

    fn handle(&mut self, cmd: RtCommand) {
        match cmd {
            RtCommand::Play => {
                self.playing = true;
                self.status.finished.store(false, Relaxed);
                self.status.playing.store(true, Relaxed);
            }
            RtCommand::Stop => {
                self.playing = false;
                self.release_all();
                self.status.playing.store(false, Relaxed);
            }
            RtCommand::Seek(tick) => {
                self.release_all();
                self.relocate(tick);
                self.status.finished.store(false, Relaxed);
            }
            RtCommand::SetFactor(factor) => {
                let tick = self.current_tick();
                self.factor = factor;
                self.pos = self.sample_of(tick);
            }
            RtCommand::SwapTimeline(new) => {
                let tick = self.current_tick();
                self.release_all();
                if let Some(old) = self.timeline.replace(new) {
                    self.retire(Retired::Timeline(old));
                }
                self.relocate(tick);
                self.status.swaps.fetch_add(1, Relaxed);
            }
            RtCommand::InstallSynth(new) => {
                if let Some(old) = self.synth.replace(new) {
                    self.retire(Retired::Synth(old));
                }
                // A new synthesizer starts with default controllers: send the mix again.
                self.applied = [[NOT_SENT; 2]; MIX_CHANNELS];
                self.replay_programs(self.cursor);
                self.status.synth_ready.store(true, Relaxed);
            }
        }
    }

    /// Send the channels whose volume (CC 7) or pan (CC 10) changed since the last call
    /// to the synthesizer (ADR-032). Once per callback, before rendering, so a change
    /// is heard in the next callback, also on notes that are already sounding. Reads 32
    /// atomics and allocates, locks and logs nothing. Channel 9 (percussion) is never
    /// used by playback and is skipped.
    fn apply_mix(&mut self) {
        #[cfg(test)]
        if self.mix_disabled {
            return;
        }
        let Some(synth) = self.synth.as_mut() else {
            return;
        };
        for ch in 0..MIX_CHANNELS {
            if ch == 9 {
                continue;
            }
            let (volume, pan) = self.status.mix.get(ch);
            let sent = &mut self.applied[ch];
            if sent[0] != volume {
                synth.process_midi_message(ch as i32, 0xB0, 7, volume as i32);
                sent[0] = volume;
            }
            if sent[1] != pan {
                synth.process_midi_message(ch as i32, 0xB0, 10, pan as i32);
                sent[1] = pan;
            }
        }
    }

    /// Hand an object back to the control thread. If the ring is full the object is
    /// leaked and counted instead of being freed here.
    fn retire(&mut self, item: Retired) {
        if let Err(e) = self.ret.push(item) {
            std::mem::forget(e);
            self.status.return_ring_full.fetch_add(1, Relaxed);
        }
    }

    fn release_all(&mut self) {
        if let Some(synth) = self.synth.as_mut() {
            synth.note_off_all(false);
        }
    }

    /// Move to `tick`: position in samples, event cursor, and the program changes that
    /// were due before it (notes before `tick` are not replayed).
    fn relocate(&mut self, tick: Tick) {
        let Some(tl) = self.timeline.as_deref() else {
            self.pos = 0;
            self.cursor = 0;
            return;
        };
        let tick = Tick(tick.0.clamp(0, tl.end().0));
        self.pos = tl.tick_to_sample(tick, self.rate, self.factor);
        self.cursor = tl.event_index_at(tick);
        self.replay_programs(self.cursor);
        self.publish_position();
    }

    /// Apply the program changes among the first `upto` events.
    fn replay_programs(&mut self, upto: usize) {
        for i in 0..upto {
            let Some(te) = self.event(i) else { break };
            if let PerformanceEvent::ProgramChange { .. } = te.event {
                self.dispatch(te.event);
            }
        }
    }

    fn apply_due_events(&mut self) {
        while let Some(at) = self.event_sample(self.cursor) {
            if at > self.pos {
                break;
            }
            if let Some(te) = self.event(self.cursor) {
                self.dispatch(te.event);
            }
            self.cursor += 1;
        }
    }

    fn dispatch(&mut self, event: PerformanceEvent) {
        let Some(synth) = self.synth.as_mut() else {
            return;
        };
        match event {
            PerformanceEvent::NoteOn {
                channel,
                key,
                velocity,
                ..
            } => synth.note_on(
                channel.get() as i32,
                key.get() as i32,
                velocity.get() as i32,
            ),
            PerformanceEvent::NoteOff { channel, key } => {
                synth.note_off(channel.get() as i32, key.get() as i32)
            }
            PerformanceEvent::ProgramChange { channel, program } => {
                synth.process_midi_message(channel.get() as i32, 0xC0, program.get() as i32, 0)
            }
            // Tempo is handled by the tick-to-sample conversion, not by the synth.
            PerformanceEvent::Tempo { .. } => {}
        }
    }

    fn finish(&mut self) {
        self.release_all();
        self.playing = false;
        self.status.playing.store(false, Relaxed);
        self.status.finished.store(true, Relaxed);
    }

    fn event(&self, i: usize) -> Option<TimedEvent> {
        self.timeline.as_deref()?.events().get(i).copied()
    }

    fn event_sample(&self, i: usize) -> Option<u64> {
        let te = self.event(i)?;
        Some(self.sample_of(te.tick))
    }

    fn sample_of(&self, tick: Tick) -> u64 {
        self.timeline
            .as_deref()
            .map_or(0, |t| t.tick_to_sample(tick, self.rate, self.factor))
    }

    fn end_sample(&self) -> Option<u64> {
        let t = self.timeline.as_deref()?;
        Some(t.tick_to_sample(t.end(), self.rate, self.factor))
    }

    fn current_tick(&self) -> Tick {
        self.timeline.as_deref().map_or(Tick(0), |t| {
            t.sample_to_tick(self.pos, self.rate, self.factor)
        })
    }

    fn publish_position(&self) {
        self.status.tick.store(self.current_tick().0, Relaxed);
    }
}
