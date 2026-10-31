//! Real-time engine core shared by offline and device modes.
//! Control plane: commands in through an SPSC ring, old timelines out through a return ring.
//! Data plane: `Rt::process` renders from a preallocated synth and an event cursor.

use rtrb::{Consumer, Producer, RingBuffer};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Off(u8),
    Program(u8),
    On(u8, u8),
}

#[derive(Clone, Copy, Debug)]
pub struct Ev {
    pub at: u64, // sample index
    pub ch: u8,
    pub kind: Kind,
}

pub struct Timeline {
    pub events: Vec<Ev>,
    pub len: u64,
}

pub enum Cmd {
    Play { requested_ns: u64 },
    Swap(Box<Timeline>),
}

pub static SWAPS_APPLIED: AtomicU64 = AtomicU64::new(0);
pub static RETURN_RING_FULL: AtomicU64 = AtomicU64::new(0);
pub static PLAY_REQUESTED_NS: AtomicU64 = AtomicU64::new(0);
pub static FIRST_AUDIO_NS: AtomicU64 = AtomicU64::new(0);
pub static LOOPS: AtomicU64 = AtomicU64::new(0);
pub static CURSOR_REGRESSIONS: AtomicU64 = AtomicU64::new(0);

static BASE: OnceLock<Instant> = OnceLock::new();

pub fn init_clock() {
    BASE.get_or_init(Instant::now);
}

#[inline]
pub fn now_ns() -> u64 {
    BASE.get().expect("init_clock first").elapsed().as_nanos() as u64
}

pub const MAX_BLOCK: usize = 8192;

pub struct Rt {
    synth: Synthesizer,
    timeline: Box<Timeline>,
    cursor: usize,
    pos: u64,
    playing: bool,
    left: Box<[f32]>,
    right: Box<[f32]>,
    cmd: Consumer<Cmd>,
    ret: Producer<Box<Timeline>>,
}

/// SoundFont path from `S2_SF2`, or the generated sine font when unset.
pub fn external_sf2() -> Option<String> {
    std::env::var("S2_SF2").ok()
}

pub fn synth(sample_rate: u32, effects: bool) -> Synthesizer {
    let bytes = match external_sf2() {
        Some(path) => std::fs::read(&path).expect("read S2_SF2"),
        None => crate::sf2::build(),
    };
    let sf = Arc::new(SoundFont::new(&mut &bytes[..]).expect("sf2 parses"));
    let mut settings = SynthesizerSettings::new(sample_rate as i32);
    settings.enable_reverb_and_chorus = effects;
    Synthesizer::new(&sf, &settings).expect("synth")
}

/// Returns the RT side plus the control-side ring ends.
pub fn new_rt(sample_rate: u32, timeline: Timeline) -> (Rt, Producer<Cmd>, Consumer<Box<Timeline>>) {
    let (cmd_tx, cmd_rx) = RingBuffer::new(64);
    let (ret_tx, ret_rx) = RingBuffer::new(64);
    let rt = Rt {
        synth: synth(sample_rate, true),
        timeline: Box::new(timeline),
        cursor: 0,
        pos: 0,
        playing: false,
        left: vec![0.0; MAX_BLOCK].into_boxed_slice(),
        right: vec![0.0; MAX_BLOCK].into_boxed_slice(),
        cmd: cmd_rx,
        ret: ret_tx,
    };
    (rt, cmd_tx, ret_rx)
}

impl Rt {
    /// Fills an interleaved buffer. Must not allocate, lock or block.
    pub fn process(&mut self, out: &mut [f32], channels: usize) {
        for _ in 0..8 {
            match self.cmd.pop() {
                Ok(c) => self.handle(c),
                Err(_) => break,
            }
        }
        let total = out.len() / channels;
        let mut done = 0;
        while done < total {
            let n = (total - done).min(MAX_BLOCK);
            if self.playing {
                self.apply_due_events(n as u64);
            }
            let (l, r) = (&mut self.left[..n], &mut self.right[..n]);
            self.synth.render(l, r);
            for i in 0..n {
                let frame = &mut out[(done + i) * channels..(done + i + 1) * channels];
                frame[0] = l[i];
                if channels > 1 {
                    frame[1] = r[i];
                }
                for s in frame.iter_mut().skip(2) {
                    *s = 0.0;
                }
            }
            if self.playing {
                self.pos += n as u64;
                if self.pos >= self.timeline.len {
                    self.pos = 0;
                    self.cursor = 0;
                    self.synth.note_off_all(false);
                    LOOPS.fetch_add(1, Relaxed);
                }
            }
            done += n;
        }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Play { requested_ns } => {
                self.playing = true;
                PLAY_REQUESTED_NS.store(requested_ns, Relaxed);
                FIRST_AUDIO_NS.store(now_ns(), Relaxed);
            }
            Cmd::Swap(new) => {
                let old = std::mem::replace(&mut self.timeline, new);
                let before = self.cursor;
                self.cursor = self.timeline.events.partition_point(|e| e.at < self.pos);
                if self.pos > 0 && self.cursor == 0 && before > 0 {
                    CURSOR_REGRESSIONS.fetch_add(1, Relaxed);
                }
                // Release notes of the old timeline gracefully (release envelope, no click).
                self.synth.note_off_all(false);
                if let Err(e) = self.ret.push(old) {
                    // Never drop on the RT thread; leak and count instead.
                    std::mem::forget(e);
                    RETURN_RING_FULL.fetch_add(1, Relaxed);
                }
                SWAPS_APPLIED.fetch_add(1, Relaxed);
            }
        }
    }

    // Events are applied at block start (block-quantized). Product code would split blocks.
    fn apply_due_events(&mut self, n: u64) {
        let end = self.pos + n;
        let events = &self.timeline.events;
        while self.cursor < events.len() && events[self.cursor].at < end {
            let e = events[self.cursor];
            let ch = e.ch as i32;
            match e.kind {
                Kind::On(k, v) => self.synth.note_on(ch, k as i32, v as i32),
                Kind::Off(k) => self.synth.note_off(ch, k as i32),
                Kind::Program(p) => self.synth.process_midi_message(ch, 0xC0, p as i32, 0),
            }
            self.cursor += 1;
        }
    }
}

/// Deterministic reference workload: 4 tracks, 120 BPM, 4/4, `bars` bars (2 s per bar).
/// Chord strums, bass eighths, melody sixteenths, pad; program changes every 8 bars.
pub fn workload(sample_rate: u32, bars: u64, seed: u64) -> Timeline {
    let mut rng = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) | 1;
    let mut next = move |m: u64| {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng % m
    };
    let sr = sample_rate as u64;
    let beat = sr / 2; // quarter note at 120 BPM
    let mut ev = Vec::new();
    let note = |ev: &mut Vec<Ev>, ch: u8, key: u8, start: u64, len: u64| {
        ev.push(Ev { at: start, ch, kind: Kind::On(key, 90) });
        ev.push(Ev { at: start + len, ch, kind: Kind::Off(key) });
    };
    let chords: [[u8; 6]; 4] = [[40, 47, 52, 55, 59, 64], [45, 52, 57, 60, 64, 69], [43, 47, 50, 55, 59, 67], [38, 45, 50, 57, 62, 66]];
    for bar in 0..bars {
        let t0 = bar * 4 * beat;
        if bar % 8 == 0 {
            ev.push(Ev { at: t0, ch: 2, kind: Kind::Program((bar / 8 % 4) as u8 * 8) });
        }
        let chord = chords[(bar % 4) as usize];
        for b in 0..4 {
            let tb = t0 + b * beat;
            for (i, k) in chord.iter().enumerate() {
                note(&mut ev, 0, *k, tb + i as u64 * sr / 100, beat - i as u64 * sr / 100);
            }
        }
        for e in 0..8 {
            note(&mut ev, 1, chord[0] - 12 + (next(2) as u8) * 7, t0 + e * beat / 2, beat / 2);
        }
        for s in 0..16 {
            if next(4) != 0 {
                note(&mut ev, 2, 64 + next(12) as u8, t0 + s * beat / 4, beat / 4);
            }
        }
        for k in &chord[2..6] {
            note(&mut ev, 3, *k + 12, t0, 4 * beat);
        }
        if external_sf2().is_some() {
            // Percussion channel (MIDI channel 10): kick, snare, hi-hat eighths.
            for e in 0..8 {
                let t = t0 + e * beat / 2;
                note(&mut ev, 9, 42, t, beat / 4);
                if e % 4 == 0 {
                    note(&mut ev, 9, 36, t, beat / 4);
                }
                if e % 4 == 2 {
                    note(&mut ev, 9, 38, t, beat / 4);
                }
            }
        }
    }
    let rank = |k: Kind| match k {
        Kind::Off(_) => 0,
        Kind::Program(_) => 1,
        Kind::On(..) => 2,
    };
    ev.sort_by_key(|e| (e.at, rank(e.kind), e.ch));
    Timeline { events: ev, len: bars * 4 * beat }
}
