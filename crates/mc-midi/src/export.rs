//! Standard MIDI File export (ADR-023): SMF type 1, 960 ticks per quarter note, repeats
//! expanded in playback order (the timeline already is), every track exported including
//! muted ones when the caller compiled without a mute set, tempo factor not applied.
//! Track 0 is the conductor track (tempo and time signature); each channel of the
//! timeline gets one track with its name, program changes and notes.

use mc_playback::{PerformanceEvent, PerformanceTimeline};
use mc_score::Score;
use midly::num::{u4, u7, u15, u24, u28};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use std::collections::BTreeMap;

/// Ticks per quarter note of exported files.
pub const PPQ: u16 = 960;
/// Ticks per whole note at [`PPQ`].
const TICKS_PER_WHOLE: i128 = 4 * PPQ as i128;
/// Largest delta a variable-length quantity can carry (28 bits).
const MAX_DELTA: i64 = (1 << 28) - 1;
/// Largest tempo value in a tempo meta event (24 bits, microseconds per quarter note).
const MAX_TEMPO_US: i128 = (1 << 24) - 1;

/// What the export did that the user may want to know (ADR-012 `ExportReport`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExportReport {
    /// MIDI tracks written, including the conductor track.
    pub tracks: usize,
    pub notes: usize,
    /// Events whose tick was rounded when converting the timeline resolution to 960 PPQ.
    pub rounded_ticks: usize,
    /// Tempo values below the SMF minimum (about 3.6 BPM) that were clamped.
    pub tempo_clamped: usize,
    /// Time signature meta events written (the first one counts).
    pub time_signatures: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportError {
    /// A gap between two events exceeds the 28-bit SMF delta limit.
    TickOverflow,
    Write(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::TickOverflow => write!(f, "timeline too long for a standard MIDI file"),
            ExportError::Write(e) => write!(f, "could not write the MIDI file: {e}"),
        }
    }
}

impl std::error::Error for ExportError {}

/// Timeline tick to 960-PPQ tick, rounding half up; the flag says whether it rounded.
fn to_midi_tick(tick: i64, ticks_per_whole: u32) -> (i64, bool) {
    let num = tick as i128 * TICKS_PER_WHOLE;
    let den = ticks_per_whole.max(1) as i128;
    let q = num.div_euclid(den);
    let r = num.rem_euclid(den);
    let rounded_up = 2 * r >= den;
    let v = if rounded_up { q + 1 } else { q };
    (v as i64, r != 0)
}

/// Microseconds per quarter note for a tempo, rounded; the flag says whether it clamped.
fn tempo_micros(bpm_num: i64, bpm_den: i64) -> (u32, bool) {
    // 60 000 000 / bpm = 60 000 000 * den / num
    let num = bpm_num.max(1) as i128;
    let us = (60_000_000i128 * bpm_den as i128 + num / 2) / num;
    if us > MAX_TEMPO_US {
        (MAX_TEMPO_US as u32, true)
    } else {
        (us.max(1) as u32, false)
    }
}

/// One pending event: absolute tick, insertion order (stable sort key), kind.
type Pending<'a> = (i64, usize, TrackEventKind<'a>);

fn into_track<'a>(
    mut events: Vec<Pending<'a>>,
    end: i64,
) -> Result<Vec<TrackEvent<'a>>, ExportError> {
    events.sort_by_key(|e| (e.0, e.1));
    let mut out = Vec::with_capacity(events.len() + 1);
    let mut last = 0i64;
    for (tick, _, kind) in events {
        let delta = tick - last;
        if !(0..=MAX_DELTA).contains(&delta) {
            return Err(ExportError::TickOverflow);
        }
        out.push(TrackEvent {
            delta: u28::new(delta as u32),
            kind,
        });
        last = tick;
    }
    let delta = (end - last).max(0);
    if delta > MAX_DELTA {
        return Err(ExportError::TickOverflow);
    }
    out.push(TrackEvent {
        delta: u28::new(delta as u32),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    Ok(out)
}

/// Write `timeline` (compiled from `score`) as a Standard MIDI File.
pub fn export_smf(
    score: &Score,
    timeline: &PerformanceTimeline,
) -> Result<(Vec<u8>, ExportReport), ExportError> {
    let tpw = timeline.ticks_per_whole().get();
    let mut report = ExportReport::default();
    let mut seq = 0usize;
    let mut next = || {
        seq += 1;
        seq
    };
    let conv = |tick: i64, report: &mut ExportReport| {
        let (t, rounded) = to_midi_tick(tick, tpw);
        if rounded {
            report.rounded_ticks += 1;
        }
        t
    };

    // Conductor track: time signatures at played-bar starts, then tempo changes.
    let mut conductor: Vec<Pending> = Vec::new();
    let mut last_sig: Option<(u8, u8)> = None;
    for pb in timeline.played_bars() {
        let Some(mb) = score.master_bars().get(pb.master_bar) else {
            continue;
        };
        let sig = (
            mb.time_signature.numerator(),
            mb.time_signature.denominator(),
        );
        if last_sig != Some(sig) {
            last_sig = Some(sig);
            let tick = conv(pb.start.0, &mut report);
            let den_pow = sig.1.trailing_zeros() as u8;
            conductor.push((
                tick,
                next(),
                TrackEventKind::Meta(MetaMessage::TimeSignature(sig.0, den_pow, 24, 8)),
            ));
            report.time_signatures += 1;
        }
    }
    let mut channel_events: BTreeMap<u8, Vec<Pending>> = BTreeMap::new();
    let mut end_tick = 0i64;
    for te in timeline.events() {
        let tick = conv(te.tick.0, &mut report);
        end_tick = end_tick.max(tick);
        match te.event {
            PerformanceEvent::Tempo { bpm } => {
                let b = bpm.bpm();
                let (us, clamped) = tempo_micros(b.numerator(), b.denominator());
                if clamped {
                    report.tempo_clamped += 1;
                }
                conductor.push((
                    tick,
                    next(),
                    TrackEventKind::Meta(MetaMessage::Tempo(u24::new(us))),
                ));
            }
            PerformanceEvent::ProgramChange { channel, program } => {
                channel_events.entry(channel.get()).or_default().push((
                    tick,
                    next(),
                    TrackEventKind::Midi {
                        channel: u4::new(channel.get()),
                        message: MidiMessage::ProgramChange {
                            program: u7::new(program.get()),
                        },
                    },
                ));
            }
            PerformanceEvent::NoteOn {
                channel,
                key,
                velocity,
                ..
            } => {
                report.notes += 1;
                channel_events.entry(channel.get()).or_default().push((
                    tick,
                    next(),
                    TrackEventKind::Midi {
                        channel: u4::new(channel.get()),
                        message: MidiMessage::NoteOn {
                            key: u7::new(key.get()),
                            vel: u7::new(velocity.get()),
                        },
                    },
                ));
            }
            PerformanceEvent::NoteOff { channel, key } => {
                channel_events.entry(channel.get()).or_default().push((
                    tick,
                    next(),
                    TrackEventKind::Midi {
                        channel: u4::new(channel.get()),
                        message: MidiMessage::NoteOff {
                            key: u7::new(key.get()),
                            vel: u7::new(64),
                        },
                    },
                ));
            }
        }
    }
    if let Some(last) = timeline.played_bars().last() {
        let bar_end = conv(last.start.0 + last.length.0, &mut report);
        end_tick = end_tick.max(bar_end);
    }

    // Track names: the score track that owns each channel.
    let names: BTreeMap<u8, String> = timeline
        .channels()
        .iter()
        .filter_map(|c| {
            score
                .track(c.track)
                .map(|t| (c.channel.get(), t.name().to_string()))
        })
        .collect();

    let mut tracks = Vec::new();
    tracks.push(into_track(conductor, end_tick)?);
    for (ch, mut events) in channel_events {
        if let Some(name) = names.get(&ch) {
            events.push((
                0,
                0,
                TrackEventKind::Meta(MetaMessage::TrackName(name.as_bytes())),
            ));
        }
        tracks.push(into_track(events, end_tick)?);
    }
    report.tracks = tracks.len();

    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(PPQ))),
        tracks,
    };
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes)
        .map_err(|e| ExportError::Write(e.to_string()))?;
    Ok((bytes, report))
}
