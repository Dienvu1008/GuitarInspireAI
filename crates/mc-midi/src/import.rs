//! Standard MIDI File import (ADR-023, ADR-012, GR-09, IM-01). The score is built only
//! through `Score::from_parts`, which checks every hard invariant. Everything the import
//! changes or drops is listed in the [`ImportReport`].
//!
//! Rules (ADR-023 item 4):
//! - Parser limits: 16 MB file, 64 tracks, 1 000 000 events, 10 000 bars.
//! - One TAB track per (MIDI track, channel) that has notes, standard tuning. Channel 10
//!   (index 9, percussion) is skipped and reported.
//! - Bars come from the time signature events (default 4/4) and are cut from the first
//!   tick; a change inside a bar takes effect at the next bar line (reported).
//! - Quantization: every onset and end snaps to the 1/16 grid or the 1/12 (triplet) grid,
//!   chosen per bar by the smaller total deviation. The 1/12 grid is used only for bars
//!   whose length is a multiple of 1/12.
//! - A note longer than its bar is split at the bar line and tied. Gaps become rests.
//!   Notes of one chord share one beat; a chord note shorter or longer than the beat
//!   sounds for the beat (reported once per chord length mismatch is not tracked).
//! - Strings come from `mc_guitar::assign_group` (a Guitar rule, GR-05).

use mc_guitar::{Assigned, assign_group};
use mc_score::{
    BarContent, Beat, BeatId, FrettedInstrument, InstrumentSpec, MasterBar, Note, NoteEffects,
    NoteId, NoteKind, Score, ScoreParts, TempoChange, TrackData, TrackId, Voice,
};
use mc_theory::fretted::{Capo, Tuning};
use mc_theory::{BaseValue, KeySignature, NoteValue, Rational, Tempo, TimeSignature, Tuplet};
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use std::collections::BTreeMap;

pub const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_TRACKS: usize = 64;
pub const MAX_EVENTS: usize = 1_000_000;
pub const MAX_BARS: usize = 10_000;

/// Something the import changed or dropped. Order is deterministic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportIssue {
    /// Notes on channel 10 (percussion) were skipped.
    PercussionSkipped { notes: usize },
    /// Notes of zero length were dropped.
    ZeroLengthDropped { notes: usize },
    /// Note ons that were never switched off were ended at the end of their track.
    UnmatchedNoteOn { notes: usize },
    /// A pitch no string can sound (or no free string in its chord) was dropped.
    NotPlayable { midi: i32, bar: usize },
    /// A time signature event was invalid and ignored.
    InvalidTimeSignature {
        numerator: u8,
        denominator_power: u8,
    },
    /// A time signature change inside a bar took effect at the next bar line.
    TimeSignatureMidBar { tick: i64 },
    /// A tempo event could not be placed (invalid, or after the last bar) and was dropped.
    TempoDropped { tick: i64 },
    /// The file had no tempo at the start; 120 BPM was used.
    TempoDefaulted,
    /// A beat was too short or irregular to write exactly; its remainder was dropped.
    DurationDropped { bar: usize },
    /// Notes whose onset snapped onto the end of the last bar were dropped.
    DroppedAtEnd { notes: usize },
}

/// What the import did (ADR-012 `ImportReport`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub ppq: u16,
    pub tracks_imported: usize,
    pub notes_imported: usize,
    pub bars: usize,
    /// Onsets and ends that moved when snapped to the grid.
    pub quantized_points: usize,
    /// Largest move, in file ticks, rounded up.
    pub max_deviation_ticks: u32,
    /// Bars that used the 1/12 (triplet) grid.
    pub triplet_bars: usize,
    pub issues: Vec<ImportIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportError {
    TooLarge {
        bytes: usize,
    },
    Malformed(String),
    TooManyTracks {
        tracks: usize,
    },
    TooManyEvents,
    UnsupportedTiming,
    NoNotes,
    TooLong {
        bars: usize,
    },
    /// The built score broke a hard invariant (a bug in the importer, never expected).
    Invalid(Vec<String>),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::TooLarge { bytes } => write!(f, "file too large ({bytes} bytes)"),
            ImportError::Malformed(e) => write!(f, "not a readable MIDI file: {e}"),
            ImportError::TooManyTracks { tracks } => write!(f, "too many tracks ({tracks})"),
            ImportError::TooManyEvents => write!(f, "too many events"),
            ImportError::UnsupportedTiming => write!(f, "SMPTE timing is not supported"),
            ImportError::NoNotes => write!(f, "the file has no notes to import"),
            ImportError::TooLong { bars } => write!(f, "too long ({bars} bars)"),
            ImportError::Invalid(v) => write!(f, "invalid score: {}", v.join("; ")),
        }
    }
}

impl std::error::Error for ImportError {}

struct RawNote {
    track: usize,
    channel: u8,
    key: u8,
    start: i64,
    end: i64,
}

struct Bar {
    start_tick: i64,
    len_ticks: i64,
    num: u8,
    den: u8,
    start_abs: Rational,
    len_abs: Rational,
    /// Grid steps per whole note: 16 or 12.
    grid: i128,
}

fn r(n: i64, d: i64) -> Rational {
    Rational::new(n, d).expect("non-zero denominator")
}

/// Grid index for `off` ticks into a bar: round half up to the nearest of `n` steps per
/// whole note. `d` is the ticks of a whole note.
fn snap_index(off: i128, n: i128, d: i128) -> i128 {
    (2 * off * n + d).div_euclid(2 * d)
}

/// `|off*n - k*d|`, the deviation scaled by `n` ticks.
fn dev_scaled(off: i128, n: i128, d: i128) -> i128 {
    (off * n - snap_index(off, n, d) * d).abs()
}

/// Greedy decomposition of `d` whole notes into note values whose lengths are multiples
/// of the grid step (1/16 or 1/12), largest first.
fn decompose(d: Rational, grid: i128) -> (Vec<NoteValue>, bool) {
    let step = r(1, grid as i64);
    let mut table: Vec<(Rational, NoteValue)> = Vec::new();
    for e in 0..=6u8 {
        let Ok(base) = BaseValue::from_exponent(e) else {
            continue;
        };
        for dots in 0..=1u8 {
            for tuplet in [None, Some(Tuplet::TRIPLET)] {
                if let Ok(v) = NoteValue::new(base, dots, tuplet) {
                    let len = v.length();
                    let on_grid = len.checked_div(step).is_some_and(|q| q.denominator() == 1);
                    if on_grid {
                        table.push((len, v));
                    }
                }
            }
        }
    }
    table.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    table.dedup_by(|a, b| a.0 == b.0);
    let mut out = Vec::new();
    let mut rest = d;
    while rest.is_positive() {
        let Some((len, v)) = table.iter().find(|(l, _)| *l <= rest) else {
            return (out, false);
        };
        out.push(*v);
        rest = rest.checked_sub(*len).unwrap_or(Rational::ZERO);
    }
    (out, true)
}

/// A note snapped to the grid: start and end as absolute positions, and the MIDI key.
type SnappedNote = (Rational, Rational, u8);

/// A piece of a track timeline between two absolute positions.
struct Segment {
    start: Rational,
    end: Rational,
    /// Positions for a chord; empty for a rest.
    chord: Vec<mc_theory::fretted::FretPosition>,
}

/// Read a Standard MIDI File into a validated score and a report.
pub fn import_smf(bytes: &[u8]) -> Result<(Score, ImportReport), ImportError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(ImportError::TooLarge { bytes: bytes.len() });
    }
    // midly 0.5.3 panics on some malformed SMPTE timing headers (negating i8::MIN) rather
    // than returning an error. mc-midi reads untrusted files (IM-01), so a parser bug
    // must become a typed error here, not a crash of the whole process.
    let parsed = std::panic::catch_unwind(|| Smf::parse(bytes).map_err(|e| e.to_string()));
    let smf = match parsed {
        Ok(Ok(smf)) => smf,
        Ok(Err(e)) => return Err(ImportError::Malformed(e)),
        Err(_) => return Err(ImportError::Malformed("the MIDI parser panicked".into())),
    };
    if smf.tracks.len() > MAX_TRACKS {
        return Err(ImportError::TooManyTracks {
            tracks: smf.tracks.len(),
        });
    }
    let events: usize = smf.tracks.iter().map(Vec::len).sum();
    if events > MAX_EVENTS {
        return Err(ImportError::TooManyEvents);
    }
    let ppq = match smf.header.timing {
        Timing::Metrical(p) if p.as_int() > 0 => p.as_int() as i64,
        _ => return Err(ImportError::UnsupportedTiming),
    };
    let mut report = ImportReport {
        ppq: ppq as u16,
        ..ImportReport::default()
    };

    // 1. Read events: notes, tempos, time signatures, names.
    let mut raw: Vec<RawNote> = Vec::new();
    let mut tempos: Vec<(i64, u32)> = Vec::new();
    let mut sigs: Vec<(i64, u8, u8)> = Vec::new();
    let mut names: BTreeMap<usize, String> = BTreeMap::new();
    let (mut percussion, mut zero_len, mut unmatched) = (0usize, 0usize, 0usize);
    for (ti, track) in smf.tracks.iter().enumerate() {
        let mut tick = 0i64;
        let mut open: BTreeMap<(u8, u8), i64> = BTreeMap::new();
        let mut close = |raw: &mut Vec<RawNote>, ch: u8, key: u8, start: i64, end: i64| {
            if ch == 9 {
                percussion += 1;
            } else if end > start {
                raw.push(RawNote {
                    track: ti,
                    channel: ch,
                    key,
                    start,
                    end,
                });
            } else {
                zero_len += 1;
            }
        };
        for ev in track {
            tick += ev.delta.as_int() as i64;
            match ev.kind {
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int();
                    match message {
                        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                            let k = key.as_int();
                            if let Some(start) = open.insert((ch, k), tick) {
                                close(&mut raw, ch, k, start, tick);
                            }
                        }
                        MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                            let k = key.as_int();
                            if let Some(start) = open.remove(&(ch, k)) {
                                close(&mut raw, ch, k, start, tick);
                            }
                        }
                        _ => {}
                    }
                }
                TrackEventKind::Meta(MetaMessage::Tempo(us)) => {
                    tempos.push((tick, us.as_int()));
                }
                TrackEventKind::Meta(MetaMessage::TimeSignature(n, p, _, _)) => {
                    sigs.push((tick, n, p));
                }
                TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
                    names
                        .entry(ti)
                        .or_insert_with(|| String::from_utf8_lossy(name).trim().to_string());
                }
                _ => {}
            }
        }
        let keys: Vec<((u8, u8), i64)> = open.into_iter().collect();
        for ((ch, k), start) in keys {
            unmatched += usize::from(ch != 9);
            close(&mut raw, ch, k, start, tick);
        }
    }
    if percussion > 0 {
        report
            .issues
            .push(ImportIssue::PercussionSkipped { notes: percussion });
    }
    if zero_len > 0 {
        report
            .issues
            .push(ImportIssue::ZeroLengthDropped { notes: zero_len });
    }
    if unmatched > 0 {
        report
            .issues
            .push(ImportIssue::UnmatchedNoteOn { notes: unmatched });
    }
    if raw.is_empty() {
        return Err(ImportError::NoNotes);
    }
    tempos.sort_by_key(|t| t.0);
    sigs.sort_by_key(|s| s.0);
    let end_tick = raw.iter().map(|n| n.end).max().unwrap_or(0);

    // 2. Bars from the time signatures.
    let whole_ticks = 4 * ppq as i128;
    let mut bars: Vec<Bar> = Vec::new();
    let (mut num, mut den) = (4u8, 4u8);
    let mut sig_idx = 0usize;
    let mut t = 0i64;
    let mut start_abs = Rational::ZERO;
    while t < end_tick || bars.is_empty() {
        while sig_idx < sigs.len() && sigs[sig_idx].0 <= t {
            let (at, n, p) = sigs[sig_idx];
            sig_idx += 1;
            let d = 1u32
                .checked_shl(p as u32)
                .and_then(|d| u8::try_from(d).ok());
            match d.and_then(|d| TimeSignature::new(n, d).ok().map(|_| d)) {
                Some(d) => {
                    if at != t {
                        report
                            .issues
                            .push(ImportIssue::TimeSignatureMidBar { tick: at });
                    }
                    (num, den) = (n, d);
                }
                None => report.issues.push(ImportIssue::InvalidTimeSignature {
                    numerator: n,
                    denominator_power: p,
                }),
            }
        }
        if bars.len() >= MAX_BARS {
            return Err(ImportError::TooLong { bars: bars.len() });
        }
        let len_abs = r(num as i64, den as i64);
        let len_ticks = (num as i128 * whole_ticks / den as i128).max(1) as i64;
        bars.push(Bar {
            start_tick: t,
            len_ticks,
            num,
            den,
            start_abs,
            len_abs,
            grid: 16,
        });
        start_abs = start_abs
            .checked_add(len_abs)
            .ok_or(ImportError::TooLong { bars: bars.len() })?;
        t += len_ticks;
    }
    report.bars = bars.len();
    let total_abs = start_abs;

    // 3. Choose the grid of each bar from the points that fall in it.
    let bar_of = |bars: &[Bar], tick: i64| -> usize {
        bars.partition_point(|b| b.start_tick <= tick).max(1) - 1
    };
    let mut cost16 = vec![0i128; bars.len()];
    let mut cost12 = vec![0i128; bars.len()];
    for n in &raw {
        for tick in [n.start, n.end] {
            let b = bar_of(&bars, tick);
            let off = (tick - bars[b].start_tick).clamp(0, bars[b].len_ticks) as i128;
            cost16[b] += dev_scaled(off, 16, whole_ticks);
            cost12[b] += dev_scaled(off, 12, whole_ticks);
        }
    }
    for (b, bar) in bars.iter_mut().enumerate() {
        let allow12 = (12 * bar.num as i128) % bar.den as i128 == 0;
        if allow12 && cost12[b] * 16 < cost16[b] * 12 {
            bar.grid = 12;
            report.triplet_bars += 1;
        }
    }

    // 4. Snap a file tick to an absolute position on its bar's grid.
    let mut max_dev = 0i128;
    let mut moved = 0usize;
    let mut snap = |bars: &[Bar], tick: i64| -> Rational {
        let b = bar_of(bars, tick);
        let bar = &bars[b];
        let off = (tick - bar.start_tick).clamp(0, bar.len_ticks) as i128;
        let n = bar.grid;
        let kmax = bar.num as i128 * n / bar.den as i128;
        let k = snap_index(off, n, whole_ticks).min(kmax);
        let dev = dev_scaled(off, n, whole_ticks);
        if dev > 0 {
            moved += 1;
            max_dev = max_dev.max((dev + n - 1) / n);
        }
        bar.start_abs
            .checked_add(r(k as i64, n as i64))
            .unwrap_or(bar.start_abs)
    };

    // 5. Tracks.
    let mut groups: BTreeMap<(usize, u8), Vec<SnappedNote>> = BTreeMap::new();
    for n in &raw {
        let s = snap(&bars, n.start);
        let e = snap(&bars, n.end);
        groups
            .entry((n.track, n.channel))
            .or_default()
            .push((s, e, n.key));
    }
    report.quantized_points = moved;
    report.max_deviation_ticks = max_dev as u32;

    let tuning = Tuning::standard_guitar();
    let fret_count = 24u8;
    let bar_starts: Vec<Rational> = bars.iter().map(|b| b.start_abs).collect();
    let mut next_id = 1u64;
    let track_ids: Vec<TrackId> = (0..groups.len())
        .map(|_| {
            let id = TrackId::from_raw(next_id);
            next_id += 1;
            id
        })
        .collect();
    let channels_per_track = groups.keys().fold(BTreeMap::new(), |mut m, (t, _)| {
        *m.entry(*t).or_insert(0usize) += 1;
        m
    });

    let mut track_data = Vec::new();
    for (gi, ((ti, ch), mut notes)) in groups.into_iter().enumerate() {
        notes.sort();
        // Group notes with the same snapped onset into chords.
        let mut chords: Vec<(Rational, Rational, Vec<u8>)> = Vec::new();
        for (s, e, k) in notes {
            match chords.last_mut() {
                Some((cs, ce, keys)) if *cs == s => {
                    *ce = (*ce).max(e);
                    if !keys.contains(&k) {
                        keys.push(k);
                    }
                }
                _ => chords.push((s, e, vec![k])),
            }
        }
        // A note whose onset snaps onto the very end of the last bar has no room to
        // sound: drop it and say so (otherwise it would start outside every bar).
        let before: usize = chords.iter().map(|c| c.2.len()).sum();
        chords.retain(|c| c.0 < total_abs);
        let after: usize = chords.iter().map(|c| c.2.len()).sum();
        if after < before {
            report.issues.push(ImportIssue::DroppedAtEnd {
                notes: before - after,
            });
        }
        // A chord ends no later than the next onset and no later than the last bar; a
        // zero-length one lasts a sixteenth. Every end is now after its start.
        for i in 0..chords.len() {
            let next_start = chords.get(i + 1).map(|c| c.0);
            let (s, e, _) = &mut chords[i];
            if *e <= *s {
                *e = s.checked_add(r(1, 16)).unwrap_or(*s);
            }
            if let Some(ns) = next_start
                && *e > ns
            {
                *e = ns;
            }
            if *e > total_abs {
                *e = total_abs;
            }
        }
        // Positions, then segments (notes and rests) covering the whole timeline.
        let mut segments: Vec<Segment> = Vec::new();
        let mut cursor = Rational::ZERO;
        let mut anchor: Option<u8> = None;
        for (s, e, keys) in &chords {
            let bar = bar_starts.partition_point(|b| b <= s).max(1) - 1;
            let pitches: Vec<i32> = keys.iter().map(|k| *k as i32).collect();
            let assigned = assign_group(&tuning, Capo(0), fret_count, anchor, &pitches);
            let mut chord = Vec::new();
            for a in assigned {
                match a {
                    Assigned::At(p) => chord.push(p),
                    Assigned::OutOfRange { midi } | Assigned::NoFreeString { midi } => {
                        report.issues.push(ImportIssue::NotPlayable { midi, bar });
                    }
                }
            }
            if let Some(first) = chord.first() {
                anchor = Some(first.fret.0);
            }
            if *s > cursor {
                segments.push(Segment {
                    start: cursor,
                    end: *s,
                    chord: Vec::new(),
                });
            }
            segments.push(Segment {
                start: *s,
                end: *e,
                chord,
            });
            cursor = *e;
        }
        if total_abs > cursor {
            segments.push(Segment {
                start: cursor,
                end: total_abs,
                chord: Vec::new(),
            });
        }

        // Cut segments at bar lines and write beats.
        let mut bar_beats: Vec<Vec<Beat>> = vec![Vec::new(); bars.len()];
        let mut bar_has_notes = vec![false; bars.len()];
        for seg in &segments {
            let mut pos = seg.start;
            let mut first_part = true;
            while pos < seg.end {
                let b = bar_starts.partition_point(|x| *x <= pos).max(1) - 1;
                let bar_end = bars[b]
                    .start_abs
                    .checked_add(bars[b].len_abs)
                    .unwrap_or(seg.end);
                let part_end = seg.end.min(bar_end);
                if part_end <= pos {
                    // No progress is possible (position at or past the last bar). Never
                    // loop: report and stop this segment.
                    report.issues.push(ImportIssue::DurationDropped { bar: b });
                    break;
                }
                let len = part_end.checked_sub(pos).unwrap_or(Rational::ZERO);
                let (values, exact) = decompose(len, bars[b].grid);
                if !exact {
                    report.issues.push(ImportIssue::DurationDropped { bar: b });
                }
                for (vi, value) in values.into_iter().enumerate() {
                    let tie = !seg.chord.is_empty() && !(first_part && vi == 0);
                    let notes = seg
                        .chord
                        .iter()
                        .map(|p| {
                            let id = NoteId::from_raw(next_id);
                            next_id += 1;
                            Note {
                                id,
                                kind: NoteKind::Fretted(*p),
                                tie,
                                effects: NoteEffects::default(),
                            }
                        })
                        .collect::<Vec<_>>();
                    if !notes.is_empty() {
                        bar_has_notes[b] = true;
                        report.notes_imported += notes.len();
                    }
                    let id = BeatId::from_raw(next_id);
                    next_id += 1;
                    bar_beats[b].push(Beat { id, value, notes });
                }
                first_part = false;
                pos = part_end;
            }
        }
        let content: Vec<BarContent> = bar_beats
            .into_iter()
            .zip(bar_has_notes)
            .map(|(beats, has)| BarContent {
                voices: vec![Voice {
                    beats: if has { beats } else { Vec::new() },
                }],
            })
            .collect();
        let name = match (names.get(&ti), channels_per_track.get(&ti)) {
            (Some(n), Some(1)) if !n.is_empty() => n.clone(),
            (Some(n), _) if !n.is_empty() => format!("{n} ch{}", ch + 1),
            _ => format!("Track {}", gi + 1),
        };
        track_data.push(TrackData {
            id: track_ids[gi],
            name,
            instrument: InstrumentSpec::Fretted(FrettedInstrument {
                tuning: tuning.clone(),
                fret_count,
                capo: Capo(0),
            }),
            bars: content,
        });
    }
    report.tracks_imported = track_data.len();

    // 6. Tempo map: snap to the 1/16 grid, keep the last tempo per position.
    let mut tempo_at: BTreeMap<(usize, Rational), Tempo> = BTreeMap::new();
    for (tick, us) in &tempos {
        let placed = (|| {
            let tempo = Tempo::new(Rational::new(60_000_000, *us as i64).ok()?).ok()?;
            let b = bar_of(&bars, *tick);
            let bar = &bars[b];
            let off = (*tick - bar.start_tick).clamp(0, bar.len_ticks) as i128;
            let kmax = bar.num as i128 * 16 / bar.den as i128;
            let k = snap_index(off, 16, whole_ticks).min(kmax);
            let (bar_idx, offset) = if k >= kmax {
                (b + 1, Rational::ZERO)
            } else {
                (b, r(k as i64, 16))
            };
            (bar_idx < bars.len()).then_some(((bar_idx, offset), tempo))
        })();
        match placed {
            Some((key, tempo)) => {
                tempo_at.insert(key, tempo);
            }
            None => report
                .issues
                .push(ImportIssue::TempoDropped { tick: *tick }),
        }
    }
    if let std::collections::btree_map::Entry::Vacant(slot) = tempo_at.entry((0, Rational::ZERO)) {
        report.issues.push(ImportIssue::TempoDefaulted);
        slot.insert(Tempo::from_bpm(120).expect("120 is valid"));
    }
    let tempo_map: Vec<TempoChange> = tempo_at
        .into_iter()
        .map(|((bar, offset), tempo)| TempoChange { bar, offset, tempo })
        .collect();

    let master_bars: Vec<MasterBar> = bars
        .iter()
        .map(|b| {
            MasterBar::new(
                TimeSignature::new(b.num, b.den).expect("validated above"),
                KeySignature::C_MAJOR,
            )
        })
        .collect();
    let score = Score::from_parts(ScoreParts {
        master_bars,
        tracks: track_data,
        tempo_map,
    })
    .map_err(|v| ImportError::Invalid(v.iter().map(|x| x.to_string()).collect()))?;
    Ok((score, report))
}
