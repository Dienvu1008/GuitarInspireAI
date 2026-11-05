//! The compiled [`PerformanceTimeline`]: sorted events, a tempo map, played bars and
//! channel assignments. All fields are private; queries are pure and the hot ones
//! (binary search, tick-to-sample) do not allocate, so slice 4b can call them in the
//! audio callback.

use crate::event::{PerformanceEvent, TimedEvent};
use crate::types::{Channel, TempoFactor, Tick, TickRate};
use mc_score::TrackId;
use mc_theory::Tempo;
use std::fmt::Write as _;

/// A tempo change already placed on the tick axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TempoPoint {
    pub tick: Tick,
    pub bpm: Tempo,
}

/// One played bar after repeat expansion, in playback order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayedBar {
    /// Position in playback order, from 0.
    pub order: u32,
    /// Master-bar index in the score.
    pub master_bar: usize,
    pub start: Tick,
    pub length: Tick,
}

/// Track to MIDI channel (ADR-024 item 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelAssignment {
    pub track: TrackId,
    pub channel: Channel,
}

/// A deterministic, immutable performance of a score (PB-09).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerformanceTimeline {
    ticks_per_whole: TickRate,
    events: Vec<TimedEvent>,
    tempo: Vec<TempoPoint>,
    played_bars: Vec<PlayedBar>,
    channels: Vec<ChannelAssignment>,
    end: Tick,
}

impl PerformanceTimeline {
    pub(crate) fn new(
        ticks_per_whole: TickRate,
        events: Vec<TimedEvent>,
        tempo: Vec<TempoPoint>,
        played_bars: Vec<PlayedBar>,
        channels: Vec<ChannelAssignment>,
        end: Tick,
    ) -> Self {
        PerformanceTimeline {
            ticks_per_whole,
            events,
            tempo,
            played_bars,
            channels,
            end,
        }
    }

    pub fn ticks_per_whole(&self) -> TickRate {
        self.ticks_per_whole
    }

    pub fn events(&self) -> &[TimedEvent] {
        &self.events
    }

    pub fn tempo(&self) -> &[TempoPoint] {
        &self.tempo
    }

    pub fn played_bars(&self) -> &[PlayedBar] {
        &self.played_bars
    }

    pub fn channels(&self) -> &[ChannelAssignment] {
        &self.channels
    }

    pub fn end(&self) -> Tick {
        self.end
    }

    pub fn channel_of(&self, track: TrackId) -> Option<Channel> {
        self.channels
            .iter()
            .find(|c| c.track == track)
            .map(|c| c.channel)
    }

    /// Start tick of the first time `master_bar` appears in [`Self::played_bars`] after
    /// repeat and volta expansion; `None` if that master bar is never played (for
    /// example skipped by an alternate ending). This maps a score bar to its first
    /// played occurrence (Q7: play from the bar containing the cursor); it is not an
    /// index into played bars by playback order.
    pub fn first_played_start_of_master_bar(&self, master_bar: usize) -> Option<Tick> {
        self.played_bars
            .iter()
            .find(|b| b.master_bar == master_bar)
            .map(|b| b.start)
    }

    /// The played bar covering `tick` (`start <= tick < start + length`), or `None` if
    /// `tick` is before the first bar or at/after the end.
    pub fn played_bar_at(&self, tick: Tick) -> Option<&PlayedBar> {
        self.played_bars
            .iter()
            .find(|b| tick.0 >= b.start.0 && tick.0 < b.start.0 + b.length.0)
    }

    /// Index of the first event with `tick >= target` (lower bound). If `target` is at
    /// or past [`Self::end`], returns `events().len()`. `target` must be non-negative;
    /// a negative tick panics, because playback ticks are never negative. No allocation.
    pub fn event_index_at(&self, target: Tick) -> usize {
        assert!(target.0 >= 0, "event_index_at: negative tick {}", target.0);
        self.events.partition_point(|e| e.tick.0 < target.0)
    }

    /// Samples at the device rate for `tick`, under `factor` (audio-dsp section 3a rule
    /// 6). Time is integrated piecewise over the tempo points: for each tempo segment
    /// the samples are, with floor rounding and `u128` intermediates,
    ///
    /// ```text
    /// seconds = ticks / ticks_per_whole * (4 / bpm) * 60 * (100 / factor)
    /// samples = ticks * 4 * 60 * 100 * sample_rate * bpm_den
    ///           / (ticks_per_whole * bpm_num * factor)
    /// ```
    ///
    /// and the per-segment values are summed. `bpm` is the quarter-note tempo as a
    /// reduced rational. If the timeline has no tempo point, 120 BPM is used; ticks before
    /// the first tempo point use the first tempo. Saturates to `u64::MAX` on overflow
    /// instead of panicking. RT-safe: no allocation, no locks, a loop over the (few)
    /// tempo points.
    pub fn tick_to_sample(&self, tick: Tick, sample_rate: u32, factor: TempoFactor) -> u64 {
        if tick.0 <= 0 || sample_rate == 0 {
            return 0;
        }
        let mut total: u128 = 0;
        for i in 0..self.segment_count() {
            let (start, end, bpm) = self.segment(i);
            if tick.0 <= start {
                break;
            }
            let upto = end.map_or(tick.0, |e| e.min(tick.0));
            let ticks = (upto - start) as u128;
            total = total.saturating_add(self.segment_samples(ticks, bpm, sample_rate, factor));
        }
        u64::try_from(total).unwrap_or(u64::MAX)
    }

    /// Inverse of [`Self::tick_to_sample`], floor rounding, with the same segments.
    /// Saturates at [`Self::end`]: a sample at or past the end maps to `end`.
    pub fn sample_to_tick(&self, sample: u64, sample_rate: u32, factor: TempoFactor) -> Tick {
        if sample == 0 || sample_rate == 0 {
            return Tick(0);
        }
        let mut acc: u128 = 0;
        for i in 0..self.segment_count() {
            let (start, end, bpm) = self.segment(i);
            let remaining = sample as u128 - acc.min(sample as u128);
            if let Some(end) = end {
                let seg_total =
                    self.segment_samples((end - start) as u128, bpm, sample_rate, factor);
                if (sample as u128) >= acc + seg_total {
                    acc += seg_total;
                    continue;
                }
            }
            // The sample lies in this segment: invert the per-segment formula.
            // ticks = remaining * tpw * bpm_num * factor / (240 * 100 * sr * bpm_den)
            let num = remaining
                .saturating_mul(self.ticks_per_whole.get() as u128)
                .saturating_mul(bpm.numerator().max(1) as u128)
                .saturating_mul(factor.hundredths() as u128);
            let den = 240u128
                .saturating_mul(100)
                .saturating_mul(sample_rate as u128)
                .saturating_mul(bpm.denominator() as u128);
            let ticks = i64::try_from(num / den.max(1)).unwrap_or(i64::MAX);
            return Tick(start.saturating_add(ticks).min(self.end.0));
        }
        self.end
    }

    /// Number of tempo segments: one per tempo point, or one default segment.
    fn segment_count(&self) -> usize {
        self.tempo.len().max(1)
    }

    /// Segment `i` as (start tick, end tick if bounded, quarter-note bpm). The first
    /// segment always starts at tick 0, even if the first tempo point sits later.
    fn segment(&self, i: usize) -> (i64, Option<i64>, mc_theory::Rational) {
        let bpm_of = |p: &TempoPoint| p.bpm.bpm();
        let default = Tempo::from_bpm(120).expect("120 is valid").bpm();
        let start = if i == 0 {
            0
        } else {
            self.tempo.get(i).map_or(0, |p| p.tick.0)
        };
        let end = self.tempo.get(i + 1).map(|p| p.tick.0);
        let bpm = self.tempo.get(i).map_or(default, bpm_of);
        (start, end, bpm)
    }

    fn segment_samples(
        &self,
        ticks: u128,
        bpm: mc_theory::Rational,
        sample_rate: u32,
        factor: TempoFactor,
    ) -> u128 {
        let num = ticks
            .saturating_mul(240)
            .saturating_mul(100)
            .saturating_mul(sample_rate as u128)
            .saturating_mul(bpm.denominator() as u128);
        let den = (self.ticks_per_whole.get() as u128)
            .saturating_mul(bpm.numerator().max(1) as u128)
            .saturating_mul(factor.hundredths() as u128);
        num.checked_div(den).unwrap_or(0)
    }

    /// Canonical text form (slice 4a golden-test and PB-09 contract). One event per line
    /// terminated by `\n` (LF), no trailing blank line. Integers are base 10. Header
    /// first, then one line per event in stored order:
    ///
    /// ```text
    /// TPW <ticks_per_whole>
    /// <tick> noteoff ch=<c> key=<k>
    /// <tick> tempo bpm=<num>/<den>
    /// <tick> program ch=<c> prog=<p>
    /// <tick> noteon ch=<c> key=<k> vel=<v> src=<track>/<masterbar>/<note>
    /// ```
    ///
    /// `bpm` is the reduced rational (denominator always printed). `src` prints the raw
    /// i64 ids of the track and note and the 0-based master-bar index.
    pub fn canonical_text(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "TPW {}", self.ticks_per_whole.get());
        for te in &self.events {
            let t = te.tick.0;
            match te.event {
                PerformanceEvent::NoteOff { channel, key } => {
                    let _ = writeln!(s, "{t} noteoff ch={} key={}", channel.get(), key.get());
                }
                PerformanceEvent::Tempo { bpm } => {
                    let b = bpm.bpm();
                    let _ = writeln!(s, "{t} tempo bpm={}/{}", b.numerator(), b.denominator());
                }
                PerformanceEvent::ProgramChange { channel, program } => {
                    let _ = writeln!(s, "{t} program ch={} prog={}", channel.get(), program.get());
                }
                PerformanceEvent::NoteOn {
                    channel,
                    key,
                    velocity,
                    source,
                } => {
                    let _ = writeln!(
                        s,
                        "{t} noteon ch={} key={} vel={} src={}/{}/{}",
                        channel.get(),
                        key.get(),
                        velocity.get(),
                        source.track.raw(),
                        source.master_bar,
                        source.note.raw(),
                    );
                }
            }
        }
        s
    }
}
