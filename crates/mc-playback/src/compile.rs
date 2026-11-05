//! `compile(score, options) -> CompileOutput`: the pure, deterministic timeline
//! compiler (PB-09). Expands repeats, lays beats on an integer tick axis, resolves
//! ties, let ring, dead notes and retriggers, and assigns channels and the program.
//! No clocks, randomness or hash-map iteration order (audio-dsp-architecture 3a).

use crate::event::{NoteSource, PerformanceEvent, TimedEvent};
use crate::timeline::{ChannelAssignment, PerformanceTimeline, PlayedBar, TempoPoint};
use crate::types::{Channel, GmProgram, MidiNote, Tick, TickRate, Velocity};
use mc_score::{NoteId, Score, TrackId};
use mc_theory::Rational;
use mc_theory::fretted::pitch_at;
use std::collections::{BTreeMap, BTreeSet};

/// Channel order (ADR-024 item 5): 0..=8, 10..=15, never 9. Reused cyclically beyond 15.
const CHANNEL_ORDER: [u8; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15];
/// Tick resolution fallback (audio-dsp 3a rule 3): 2^11 * 3 * 5 * 7.
const TICK_FALLBACK: u64 = 215_040;
/// Maximum LCM before falling back.
const TICK_MAX: u64 = 1 << 20;
/// Dead-note length: 1/64 of a whole note (ADR-024 item 2).
const DEAD_DENOM: i64 = 64;

/// Options for [`compile`]. `#[non_exhaustive]`: build it with [`CompileOptions::new`]
/// and set fields, so later options do not break callers.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CompileOptions {
    /// Muted tracks: their notes are omitted; timing is unchanged (PB-04).
    pub muted: BTreeSet<TrackId>,
}

impl CompileOptions {
    pub fn new() -> Self {
        CompileOptions::default()
    }
}

/// A playback issue that does not stop compilation (D-05 and the caps of rule 1/3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaybackWarning {
    /// D-05: a note's sounding pitch is outside MIDI 0..=127; the note is omitted.
    PitchOutOfRange {
        track: TrackId,
        master_bar: usize,
        note: NoteId,
        semitone: i32,
    },
    /// Rule 3: the tick resolution overflowed and fell back; these bars were rounded.
    TicksRounded { bars: Vec<usize> },
    /// Rule 1: expansion stopped at the played-bar cap.
    PlaybackTruncated { played_bars: u32 },
    /// Item 5: more than 15 tracks share channels.
    ChannelShared { tracks: Vec<TrackId> },
}

/// Compilation failed (an error the compiler itself raises, distinct from warnings).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompileError {
    /// Tick arithmetic overflowed while placing this master bar.
    TimingOverflow { master_bar: usize },
    /// A query on the score failed (for example bar start times).
    Score(mc_score::DomainError),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::TimingOverflow { master_bar } => {
                write!(f, "tick overflow at master bar {master_bar}")
            }
            CompileError::Score(e) => write!(f, "score query failed: {e}"),
        }
    }
}

impl std::error::Error for CompileError {}

/// The compiled timeline and any warnings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileOutput {
    pub timeline: PerformanceTimeline,
    pub warnings: Vec<PlaybackWarning>,
}

/// Compile `score` into a [`PerformanceTimeline`]. Pure and deterministic.
pub fn compile(score: &Score, options: &CompileOptions) -> Result<CompileOutput, CompileError> {
    let mut warnings = Vec::new();

    // 1. Played bars with start times (rational), in playback order (PB-04, PB-06).
    let starts = score.bar_start_times().map_err(CompileError::Score)?;
    let order = score.playback_order();
    if order.truncated {
        warnings.push(PlaybackWarning::PlaybackTruncated {
            played_bars: order.bars.len() as u32,
        });
    }

    // 2. Tick resolution: LCM of all beat-value denominators, plus the dead-note 1/64,
    //    capped with the documented fallback (rule 3).
    let (rate, rounded_bars) = tick_resolution(score, &starts);
    let tpw = rate as i128;
    if !rounded_bars.is_empty() {
        warnings.push(PlaybackWarning::TicksRounded { bars: rounded_bars });
    }

    // Convert a rational count of whole notes to an integer tick (floor), checked.
    let to_tick = |r: Rational| -> Option<i64> {
        let n = r.numerator() as i128 * tpw;
        let d = r.denominator() as i128;
        i64::try_from(n.div_euclid(d)).ok()
    };

    // 3. Channel assignment (item 5), deterministic in score order.
    let track_ids: Vec<TrackId> = score.tracks().map(|t| t.id()).collect();
    let mut channels = Vec::new();
    let mut shared = Vec::new();
    for (i, &tid) in track_ids.iter().enumerate() {
        let ch = Channel::try_new(CHANNEL_ORDER[i % CHANNEL_ORDER.len()])
            .expect("CHANNEL_ORDER values are valid");
        channels.push(ChannelAssignment {
            track: tid,
            channel: ch,
        });
        if i >= CHANNEL_ORDER.len() {
            shared.push(tid);
        }
    }
    if !shared.is_empty() {
        warnings.push(PlaybackWarning::ChannelShared { tracks: shared });
    }
    let channel_of = |tid: TrackId| -> Channel {
        channels
            .iter()
            .find(|c| c.track == tid)
            .map(|c| c.channel)
            .expect("every track has a channel")
    };

    // 4. Played bars on the tick axis.
    let mut played_bars = Vec::with_capacity(starts.len());
    for (order_idx, &(master_bar, start_r)) in starts.iter().enumerate() {
        let start = to_tick(start_r).ok_or(CompileError::TimingOverflow { master_bar })?;
        let dur_r = score
            .effective_bar_duration(master_bar)
            .map_err(CompileError::Score)?;
        let end = to_tick(
            start_r
                .checked_add(dur_r)
                .ok_or(CompileError::TimingOverflow { master_bar })?,
        )
        .ok_or(CompileError::TimingOverflow { master_bar })?;
        played_bars.push(PlayedBar {
            order: order_idx as u32,
            master_bar,
            start: Tick(start),
            length: Tick(end - start),
        });
    }
    let end_tick = played_bars
        .last()
        .map(|b| Tick(b.start.0 + b.length.0))
        .unwrap_or(Tick(0));

    // 5. Note events. Collected unsorted, then sorted by (tick, category, tie-breaks).
    let mut builder = EventBuilder::new();

    // Program change on every used channel at tick 0.
    for c in &channels {
        builder.push(
            0,
            PerformanceEvent::ProgramChange {
                channel: c.channel,
                program: GmProgram::ACOUSTIC_GUITAR_STEEL,
            },
            0,
            0,
            0,
        );
    }

    // Tempo map on the tick axis. Each entry sits at start(firstOccurrence(bar)) + offset.
    let mut tempo = Vec::new();
    for tc in score.tempo_map() {
        let Some(bar_start) = first_start_of(&starts, tc.bar) else {
            continue; // a tempo on a never-played bar is dropped
        };
        let at = bar_start
            .checked_add(tc.offset)
            .and_then(to_tick)
            .ok_or(CompileError::TimingOverflow { master_bar: tc.bar })?;
        tempo.push(TempoPoint {
            tick: Tick(at),
            bpm: tc.tempo,
        });
        builder.push(at, PerformanceEvent::Tempo { bpm: tc.tempo }, 0, 0, 0);
    }
    tempo.sort_by_key(|p| p.tick.0);

    // Notes, track by track, voice by voice, with tie and let-ring resolution.
    for (ti, track) in score.tracks().enumerate() {
        if options.muted.contains(&track.id()) {
            continue;
        }
        let ch = channel_of(track.id());
        let fretted = track.fretted();
        compile_track(
            &starts,
            &played_bars,
            track,
            ti,
            ch,
            fretted,
            &to_tick,
            &mut builder,
            &mut warnings,
        )?;
    }

    let events = builder.finish();
    let timeline = PerformanceTimeline::new(
        TickRate::new(rate as u32),
        events,
        tempo,
        played_bars,
        channels,
        end_tick,
    );
    // Deterministic warning order: emitted in the order above, pitch warnings already
    // in played-bar/track/voice/string order from compile_track.
    Ok(CompileOutput { timeline, warnings })
}

fn first_start_of(starts: &[(usize, Rational)], bar: usize) -> Option<Rational> {
    starts.iter().find(|(m, _)| *m == bar).map(|(_, s)| *s)
}

/// LCM of every beat value's denominator across the score, times the dead-note 1/64,
/// capped by `TICK_MAX`. Returns the chosen rate and the master bars that were rounded
/// (empty unless the fallback was used). Always a multiple of 64 so a dead note is exact.
fn tick_resolution(score: &Score, starts: &[(usize, Rational)]) -> (u64, Vec<usize>) {
    let mut lcm_val: u64 = DEAD_DENOM as u64;
    let mut overflowed = false;
    for track in score.tracks() {
        for &(master_bar, _) in starts {
            let Some(bar) = track.bar(master_bar) else {
                continue;
            };
            for voice in &bar.voices {
                for beat in &voice.beats {
                    let d = beat.value.length().denominator() as u64;
                    match lcm(lcm_val, d) {
                        Some(v) if v <= TICK_MAX => lcm_val = v,
                        _ => overflowed = true,
                    }
                }
            }
        }
    }
    if overflowed {
        // Fallback rounds positions to TICK_FALLBACK; report every played bar.
        let bars: BTreeSet<usize> = starts.iter().map(|(m, _)| *m).collect();
        (TICK_FALLBACK, bars.into_iter().collect())
    } else {
        (lcm_val, Vec::new())
    }
}

fn lcm(a: u64, b: u64) -> Option<u64> {
    if a == 0 || b == 0 {
        return Some(0);
    }
    let g = gcd(a, b);
    (a / g).checked_mul(b)
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// The note currently sounding on one (voice, string), carried across beats and bars so
/// a tie on the next beat can extend it (H-06).
#[derive(Clone, Copy)]
struct SoundingNote {
    key: MidiNote,
    on_at: i64,
    off_at: i64,
    /// Index into the builder of the note-off event, so a tie or let ring can extend it.
    off_index: usize,
    /// True while the chain can still be extended by a tie on the next beat.
    let_ring: bool,
    dead: bool,
}

#[allow(clippy::too_many_arguments)]
fn compile_track(
    starts: &[(usize, Rational)],
    played_bars: &[PlayedBar],
    track: &mc_score::Track,
    track_index: usize,
    channel: Channel,
    fretted: &mc_score::FrettedInstrument,
    to_tick: &dyn Fn(Rational) -> Option<i64>,
    builder: &mut EventBuilder,
    warnings: &mut Vec<PlaybackWarning>,
) -> Result<(), CompileError> {
    // Tie chains are tracked per (voice, string): a tie continues the note on the same
    // string of the preceding beat of the same voice (H-06). Overlap and retrigger are
    // resolved per MIDI key on the channel: `by_key` holds the note currently sounding
    // for each key, so a new attack of a key already sounding (unison on another string,
    // another voice, or a let-ring tail) first ends the earlier one. This keeps every
    // note on paired with exactly one note off on the channel.
    let mut state: BTreeMap<(usize, u8), Option<SoundingNote>> = BTreeMap::new();

    // `starts` is parallel to `played_bars`: both come from bar_start_times in playback
    // order, so index `i` is the same played bar in each. Using the played bar's own
    // rational start (not the first occurrence of its master bar) places repeat passes
    // at distinct ticks.
    for (pb, &(start_master, bar_start)) in played_bars.iter().zip(starts) {
        debug_assert_eq!(pb.master_bar, start_master);
        let master_bar = pb.master_bar;
        let Some(bar) = track.bar(master_bar) else {
            continue;
        };
        for (vi, voice) in bar.voices.iter().enumerate() {
            let mut onset = bar_start; // rational position in whole notes
            for beat in &voice.beats {
                let beat_len = beat.value.length();
                let beat_start_tick =
                    to_tick(onset).ok_or(CompileError::TimingOverflow { master_bar })?;
                let beat_end = onset
                    .checked_add(beat_len)
                    .ok_or(CompileError::TimingOverflow { master_bar })?;
                let beat_end_tick =
                    to_tick(beat_end).ok_or(CompileError::TimingOverflow { master_bar })?;

                for note in &beat.notes {
                    let pos = note.position();
                    let string = pos.string.get();
                    let semi = pitch_at(&fretted.tuning, fretted.capo, pos);
                    let key = match semi.and_then(|s| MidiNote::try_new(u8::try_from(s).ok()?).ok())
                    {
                        Some(k) => k,
                        None => {
                            warnings.push(PlaybackWarning::PitchOutOfRange {
                                track: track.id(),
                                master_bar,
                                note: note.id,
                                semitone: semi.unwrap_or(-1),
                            });
                            continue;
                        }
                    };
                    let st = state.entry((vi, string)).or_insert(None);

                    // A tie extends the note already sounding on this string, if its key
                    // matches, the chain is open, not dead, and still sounding at this
                    // onset. The last condition matters across a repeat: on a later
                    // pass the tie's source was played on an earlier pass and has already
                    // ended, so the tied note becomes a fresh attack instead.
                    if note.tie
                        && let Some(prev) = *st
                        && prev.key == key
                        && !prev.dead
                        && prev.on_at < beat_start_tick
                        && prev.off_at >= beat_start_tick
                    {
                        // Extend the chain's single note-off to the end of this beat,
                        // or to the end of the played bar if the chain lets ring
                        // (let ring taken from the first note, ADR-024 item 3).
                        let chain_let_ring = prev.let_ring;
                        let off = if chain_let_ring {
                            pb.start.0 + pb.length.0
                        } else {
                            beat_end_tick
                        };
                        builder.set_off_tick(prev.off_index, off);
                        *st = Some(SoundingNote {
                            key,
                            on_at: prev.on_at,
                            off_at: off,
                            off_index: prev.off_index,
                            let_ring: chain_let_ring,
                            dead: false,
                        });
                        continue;
                    }

                    // New attack. Note on now, note off at the beat end, extended to the
                    // played-bar end if let ring (audio-dsp 3a rule 4) or shortened to a
                    // fixed 1/64 if dead. Overlaps between notes of the same key on this
                    // channel (unison on two strings, overlapping voices, let-ring tails)
                    // are clipped afterwards in resolve_overlaps, so one channel never
                    // sounds a key twice at once.
                    let dead = note.effects.dead;
                    let off_tick = if dead {
                        let dead_end = onset
                            .checked_add(Rational::new(1, DEAD_DENOM).expect("1/64 valid"))
                            .and_then(to_tick)
                            .unwrap_or(beat_end_tick);
                        dead_end.min(beat_end_tick)
                    } else if note.effects.let_ring {
                        pb.start.0 + pb.length.0
                    } else {
                        beat_end_tick
                    };
                    let velocity = if dead {
                        Velocity::DEAD
                    } else {
                        Velocity::NORMAL
                    };
                    let off_index = builder.push_note(
                        beat_start_tick,
                        off_tick,
                        PerformanceEvent::NoteOn {
                            channel,
                            key,
                            velocity,
                            source: NoteSource {
                                track: track.id(),
                                master_bar,
                                note: note.id,
                            },
                        },
                        PerformanceEvent::NoteOff { channel, key },
                        (track_index, vi, string),
                    );
                    *st = Some(SoundingNote {
                        key,
                        on_at: beat_start_tick,
                        off_at: off_tick,
                        off_index,
                        let_ring: note.effects.let_ring && !dead,
                        dead,
                    });
                }
                onset = beat_end;
            }
        }
    }

    // Let ring is applied at attack time (the note-off is placed at the end of the
    // played bar) and shortened by the next same-string attack via the retrigger
    // branch, so no trailing pass is needed.
    Ok(())
}

/// Collects events and lets note-off ticks be rewritten (tie / retrigger), then sorts
/// deterministically: by tick, then same-tick category (rule 5), then track, voice,
/// string (audio-dsp 3a rule 5 tie-breaks).
struct EventBuilder {
    items: Vec<BuilderItem>,
    /// (note-on index, note-off index) for every emitted note, for overlap resolution.
    pairs: Vec<(usize, usize)>,
}

struct BuilderItem {
    tick: i64,
    event: PerformanceEvent,
    track: usize,
    voice: usize,
    string: u8,
}

impl EventBuilder {
    fn new() -> Self {
        EventBuilder {
            items: Vec::new(),
            pairs: Vec::new(),
        }
    }

    /// Returns the index of the pushed item (used to rewrite a note-off tick).
    fn push(
        &mut self,
        tick: i64,
        event: PerformanceEvent,
        track: usize,
        voice: usize,
        string: u8,
    ) -> usize {
        self.items.push(BuilderItem {
            tick,
            event,
            track,
            voice,
            string,
        });
        self.items.len() - 1
    }

    /// Emit a note on and its note off, recording the pair. Returns the note-off index
    /// so a tie can extend it. `meta` is (track, voice, string) for the sort tie-break.
    fn push_note(
        &mut self,
        on_tick: i64,
        off_tick: i64,
        on: PerformanceEvent,
        off: PerformanceEvent,
        meta: (usize, usize, u8),
    ) -> usize {
        let (track, voice, string) = meta;
        let on_idx = self.push(on_tick, on, track, voice, string);
        let off_idx = self.push(off_tick, off, track, voice, string);
        self.pairs.push((on_idx, off_idx));
        off_idx
    }

    fn set_off_tick(&mut self, index: usize, tick: i64) {
        self.items[index].tick = tick;
    }

    /// Enforce one sounding note per (channel, key): a channel cannot play the same key
    /// twice at once (unison on two strings, overlapping voices, or let-ring tails).
    /// For each (channel, key), process note ons in tick order; clip the previous note
    /// off to the next note on, and drop a note whose on and off coincide. This runs
    /// before sorting and is deterministic. Paired note on/off share an `off_index`
    /// link: the note on at index i has its off at `pair[i]`.
    fn resolve_overlaps(&mut self, pairs: &[(usize, usize)]) {
        use std::collections::BTreeMap;
        // Group note-on indices by (channel, key).
        let mut groups: BTreeMap<(u8, u8), Vec<(usize, usize)>> = BTreeMap::new();
        for &(on_idx, off_idx) in pairs {
            if let PerformanceEvent::NoteOn { channel, key, .. } = self.items[on_idx].event {
                groups
                    .entry((channel.get(), key.get()))
                    .or_default()
                    .push((on_idx, off_idx));
            }
        }
        let mut dropped: Vec<bool> = vec![false; self.items.len()];
        for notes in groups.values() {
            // Order by on tick, then off tick, deterministically.
            let mut ns = notes.clone();
            ns.sort_by(|&(a_on, a_off), &(b_on, b_off)| {
                self.items[a_on]
                    .tick
                    .cmp(&self.items[b_on].tick)
                    .then(self.items[a_off].tick.cmp(&self.items[b_off].tick))
            });
            for w in 0..ns.len() {
                let (on_idx, off_idx) = ns[w];
                let on_t = self.items[on_idx].tick;
                let mut off_t = self.items[off_idx].tick;
                if let Some(&(next_on, _)) = ns.get(w + 1) {
                    let next_t = self.items[next_on].tick;
                    if off_t > next_t {
                        off_t = next_t;
                        self.items[off_idx].tick = next_t;
                    }
                }
                if off_t <= on_t {
                    // Zero-length after clipping (a same-tick unison): drop the pair.
                    dropped[on_idx] = true;
                    dropped[off_idx] = true;
                }
            }
        }
        if dropped.iter().any(|&d| d) {
            let mut i = 0;
            self.items.retain(|_| {
                let keep = !dropped[i];
                i += 1;
                keep
            });
        }
    }

    fn finish(mut self) -> Vec<TimedEvent> {
        let pairs = std::mem::take(&mut self.pairs);
        self.resolve_overlaps(&pairs);
        self.items.sort_by(|a, b| {
            a.tick
                .cmp(&b.tick)
                .then(a.event.category().cmp(&b.event.category()))
                .then(a.track.cmp(&b.track))
                .then(a.voice.cmp(&b.voice))
                .then(a.string.cmp(&b.string))
        });
        self.items
            .into_iter()
            .map(|i| TimedEvent {
                tick: Tick(i.tick),
                event: i.event,
            })
            .collect()
    }
}
