//! `Score` to GP3, GP4 or GP5 (5.00) with an export report (ADR-036 section 4). It
//! reads the written score: repeats stay repeats and every track is written. Every
//! approximation is counted; notes written + notes dropped = `Score::note_count`.

use crate::Tally;
use crate::import::tuplet_normal;
use crate::raw::*;
use crate::read::{MAX_BARS, MAX_STRINGS, MAX_TRACKS};
use crate::write::{self, DEFAULT_VELOCITY};
use mc_score::{Beat, Score, Stroke};
use mc_theory::{Mode, Rational};

/// What the export had to change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpExportIssueKind {
    /// A beat with two or more dots, written with one.
    DotsReduced,
    /// A value shorter than a sixty-fourth, written as a sixty-fourth.
    ValueTooShort,
    /// A tuplet ratio the format cannot express, written with its "enters" only.
    TupletApproximated,
    /// A voice beyond the format's count (2 in GP5, 1 in GP3 and GP4); its notes are
    /// dropped.
    VoiceDropped,
    /// String-percussive has no Guitar Pro note equivalent.
    StringPercussiveNotWritten,
    /// A note both tied and dead: written as tied.
    DeadTiedNote,
    /// GP3 only (ADR-036 choice 7.6).
    AccentNotWritten,
    PalmMuteNotWritten,
    StrokeNotWritten,
    /// A tempo that is not a whole number of BPM, rounded.
    TempoRounded,
    /// A tempo change between beats of the first voice of the first track, moved to
    /// the next beat; or after the last beat, moved to the next bar.
    TempoMoved,
    /// A tempo change with no beat to carry it, or replaced by another at the same beat.
    TempoDropped,
    /// An alternate ending above pass 8 (the format holds a bitmask of 8 passes).
    EndingDropped,
    /// A pickup bar, written as an underfull bar (the format has no pickup).
    PickupWritten,
    /// A name or marker longer than the format's buffer, cut.
    TextCut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpExportIssue {
    pub kind: GpExportIssueKind,
    pub count: usize,
    pub first_bar: usize,
}

impl std::fmt::Display for GpExportIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use GpExportIssueKind as K;
        let what = match self.kind {
            K::DotsReduced => "beat with several dots written with one",
            K::ValueTooShort => "value shorter than a 64th written as a 64th",
            K::TupletApproximated => "tuplet ratio approximated",
            K::VoiceDropped => "voice dropped",
            K::StringPercussiveNotWritten => "string-percussive not written",
            K::DeadTiedNote => "dead tied note written as tied",
            K::AccentNotWritten => "accent not written (GP3)",
            K::PalmMuteNotWritten => "palm mute not written (GP3)",
            K::StrokeNotWritten => "stroke not written (GP3)",
            K::TempoRounded => "tempo rounded to whole BPM",
            K::TempoMoved => "tempo change moved to a beat",
            K::TempoDropped => "tempo change dropped",
            K::EndingDropped => "alternate ending above pass 8 dropped",
            K::PickupWritten => "pickup written as a short bar",
            K::TextCut => "text cut to the format's length",
        };
        write!(
            f,
            "{what}: {} (first in bar {})",
            self.count,
            self.first_bar + 1
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpExportReport {
    pub version: GpVersion,
    pub tracks: usize,
    pub bars: usize,
    pub notes_written: usize,
    /// Always `note_count - notes_written`; each drop is also an issue.
    pub notes_dropped: usize,
    pub issues: Vec<GpExportIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpExportError {
    TooManyStrings { track: usize, strings: usize },
    TooManyTracks { tracks: usize },
    TooManyBars { bars: usize },
}

impl std::fmt::Display for GpExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpExportError::TooManyStrings { track, strings } => write!(
                f,
                "track {} has {strings} strings; Guitar Pro holds at most {MAX_STRINGS}",
                track + 1
            ),
            GpExportError::TooManyTracks { tracks } => {
                write!(f, "{tracks} tracks; at most {MAX_TRACKS} can be written")
            }
            GpExportError::TooManyBars { bars } => {
                write!(f, "{bars} bars; at most {MAX_BARS} can be written")
            }
        }
    }
}

impl std::error::Error for GpExportError {}

/// Write `score` as a Guitar Pro file of `version`.
pub fn export_gp(
    score: &Score,
    version: GpVersion,
) -> Result<(Vec<u8>, GpExportReport), GpExportError> {
    let tracks: Vec<_> = score.tracks().collect();
    if tracks.len() > MAX_TRACKS {
        return Err(GpExportError::TooManyTracks {
            tracks: tracks.len(),
        });
    }
    let bars = score.master_bars();
    if bars.len() > MAX_BARS {
        return Err(GpExportError::TooManyBars { bars: bars.len() });
    }
    for (i, t) in tracks.iter().enumerate() {
        let n = t.fretted().tuning.string_count() as usize;
        if n > MAX_STRINGS {
            return Err(GpExportError::TooManyStrings {
                track: i,
                strings: n,
            });
        }
    }
    use GpExportIssueKind as K;
    let v = version;
    let mut issues: Tally<GpExportIssueKind> = Tally::default();
    let mut raw = RawSong {
        info: vec![String::new(); if v == GpVersion::Gp5 { 9 } else { 8 }],
        lyrics: vec![(0, String::new()); if v == GpVersion::Gp3 { 0 } else { 5 }],
        ..RawSong::default()
    };

    // Master bars. The time signature and key are written on the first bar and on
    // each change.
    let mut last: Option<&mc_score::MasterBar> = None;
    for (i, mb) in bars.iter().enumerate() {
        let mut h = RawHeader::default();
        if last.is_none_or(|l| l.time_signature != mb.time_signature) {
            h.numerator = Some(mb.time_signature.numerator());
            h.denominator = Some(mb.time_signature.denominator());
        }
        if last.is_none_or(|l| l.key != mb.key) {
            h.key = Some((mb.key.fifths(), (mb.key.mode() == Mode::Minor) as i8));
        }
        h.repeat_start = mb.repeat_start;
        h.repeat_close = mb.repeat_end.map(|plays| {
            if v == GpVersion::Gp5 {
                plays
            } else {
                plays - 1
            }
        });
        for &p in &mb.alternate_endings {
            if (1..=8).contains(&p) {
                h.alternative |= 1 << (p - 1);
            } else {
                issues.add(K::EndingDropped, i);
            }
        }
        if let Some(m) = &mb.marker {
            if m.len() > 255 {
                issues.add(K::TextCut, i);
            }
            h.marker = Some((m.clone(), [255, 0, 0, 0]));
        }
        if mb.pickup {
            issues.add(K::PickupWritten, i);
        }
        raw.headers.push(h);
        last = Some(mb);
    }
    raw.key = bars.first().map(|b| b.key.fifths() as i32).unwrap_or(0);

    // Tracks: port 1, a channel pair each, channel 10 (percussion) never used.
    let channels: Vec<i32> = (1..=16).filter(|c| *c != 10).collect();
    for (i, t) in tracks.iter().enumerate() {
        let inst = t.fretted();
        if t.name().len() > 40 {
            issues.add(K::TextCut, 0);
        }
        raw.tracks.push(RawTrack {
            percussion: false,
            name: t.name().to_string(),
            tuning: inst.tuning.strings().iter().map(|p| p.semitone()).collect(),
            port: 1,
            channel: channels[(2 * i) % channels.len()],
            effect_channel: channels[(2 * i + 1) % channels.len()],
            frets: inst.fret_count as i32,
            capo: inst.capo.0 as i32,
        });
    }

    // Bars.
    let mut written = 0usize;
    for m in 0..bars.len() {
        let mut row = Vec::with_capacity(tracks.len());
        for t in &tracks {
            let content = t.bar(m).expect("H-02: every track has every bar");
            let mut bar = RawBar::default();
            for (vi, voice) in content.voices.iter().enumerate() {
                if vi >= v.voices() {
                    if !voice.beats.is_empty() {
                        issues.add(K::VoiceDropped, m);
                    }
                    continue;
                }
                let mut beats = Vec::with_capacity(voice.beats.len());
                for b in &voice.beats {
                    let rb = beat(b, v, m, &mut issues);
                    written += rb.notes.len();
                    beats.push(rb);
                }
                bar.voices.push(beats);
            }
            while bar.voices.len() < v.voices() {
                bar.voices.push(Vec::new());
            }
            row.push(bar);
        }
        raw.bars.push(row);
    }

    tempos(score, &mut raw, v, &mut issues);
    // An empty voice in GP5 holds one empty beat, as TuxGuitar writes empty voices.
    if v == GpVersion::Gp5 {
        for bar in raw.bars.iter_mut().flatten() {
            for voice in bar.voices.iter_mut() {
                if voice.is_empty() {
                    voice.push(RawBeat {
                        status: BeatStatus::Empty,
                        ..RawBeat::default()
                    });
                }
            }
        }
    }
    let bytes = write::write(&raw, v);
    let note_count = score.note_count();
    let report = GpExportReport {
        version: v,
        tracks: tracks.len(),
        bars: bars.len(),
        notes_written: written,
        notes_dropped: note_count - written,
        issues: issues.into_vec(|kind, count, first_bar| GpExportIssue {
            kind,
            count,
            first_bar,
        }),
    };
    Ok((bytes, report))
}

fn beat(b: &Beat, v: GpVersion, bar: usize, issues: &mut Tally<GpExportIssueKind>) -> RawBeat {
    use GpExportIssueKind as K;
    let value = b.value;
    let exp = value.base().exponent();
    let duration = if exp > 6 {
        issues.add(K::ValueTooShort, bar);
        4
    } else {
        exp as i8 - 2
    };
    if value.dots() > 1 {
        issues.add(K::DotsReduced, bar);
    }
    let tuplet = value.tuplet().map(|t| {
        if tuplet_normal(t.actual() as i32) != Some(t.normal()) {
            issues.add(K::TupletApproximated, bar);
        }
        t.actual() as i32
    });
    let pick_stroke = match b.stroke {
        Some(_) if v == GpVersion::Gp3 => {
            issues.add(K::StrokeNotWritten, bar);
            0
        }
        Some(Stroke::Down) => 2,
        Some(Stroke::Up) => 1,
        None => 0,
    };
    let mut notes = Vec::with_capacity(b.notes.len());
    for n in &b.notes {
        let p = n.position();
        let e = n.effects;
        if e.string_percussive {
            issues.add(K::StringPercussiveNotWritten, bar);
        }
        if n.tie && e.dead {
            issues.add(K::DeadTiedNote, bar);
        }
        if v == GpVersion::Gp3 {
            if e.accent {
                issues.add(K::AccentNotWritten, bar);
            }
            if e.palm_mute {
                issues.add(K::PalmMuteNotWritten, bar);
            }
        }
        notes.push(RawNote {
            string: p.string.get(),
            kind: if n.tie {
                NoteType::Tie
            } else if e.dead {
                NoteType::Dead
            } else {
                NoteType::Normal
            },
            fret: p.fret.0 as i8,
            accent: e.accent && v != GpVersion::Gp3,
            let_ring: e.let_ring,
            palm_mute: e.palm_mute && v != GpVersion::Gp3,
            velocity: Some(DEFAULT_VELOCITY),
            ..RawNote::default()
        });
    }
    RawBeat {
        status: if notes.is_empty() {
            BeatStatus::Rest
        } else {
            BeatStatus::Normal
        },
        duration,
        dotted: value.dots() > 0,
        tuplet,
        pick_stroke,
        notes,
        ..RawBeat::default()
    }
}

/// The song tempo, then each change on the beat of the first voice of the first track
/// that starts at its offset (or the next beat; or the next bar's first beat).
fn tempos(score: &Score, raw: &mut RawSong, v: GpVersion, issues: &mut Tally<GpExportIssueKind>) {
    use GpExportIssueKind as K;
    let bpm = |r: Rational, bar: usize, issues: &mut Tally<GpExportIssueKind>| -> i32 {
        if r.denominator() != 1 {
            issues.add(K::TempoRounded, bar);
        }
        let n = r.numerator() as i128;
        let d = r.denominator() as i128;
        ((2 * n + d) / (2 * d)).clamp(1, i32::MAX as i128) as i32
    };
    let map = score.tempo_map();
    raw.tempo = map
        .first()
        .map(|t| bpm(t.tempo.bpm(), 0, issues))
        .unwrap_or(120);
    for change in map.iter().skip(1) {
        let m = change.bar;
        let tempo = bpm(change.tempo.bpm(), m, issues);
        // A beat at or after the nominal end (an overfull bar) would belong to the
        // next bar when read back.
        let mb = &score.master_bars()[m];
        let end = if mb.pickup {
            Rational::ZERO
        } else {
            mb.time_signature.length()
        };
        let row = &raw.bars[m];
        // 1. A beat of a first voice that starts exactly there, first track first.
        let exact = (0..row.len()).find_map(|t| {
            starts(&row[t].voices[0])
                .position(|o| o == change.offset)
                .map(|k| (t, k))
        });
        // 2. Else the next beat of the first voice of the first track in the bar.
        let next = || {
            starts(&row[0].voices[0])
                .position(|o| o > change.offset && o < end)
                .map(|k| (0, k))
        };
        // 3. Else the first beat at the start of the bar; an empty beat if there is none.
        let start = || {
            starts(&row[0].voices[0])
                .position(|o| o.is_zero())
                .map(|k| (0, k))
        };
        // 4. Else, in another track, the next beat or the first beat of the bar.
        let other = || {
            (1..row.len()).find_map(|u| {
                starts(&row[u].voices[0])
                    .position(|o| (o > change.offset && o < end) || o.is_zero())
                    .map(|k| (u, k))
            })
        };
        let (t, k) = match exact.or_else(next).or_else(start).or_else(other) {
            Some(target) => target,
            // 5. Else, in GP5, an empty beat at the start of the bar carries it. GP3
            // and GP4 get no empty beat: TuxGuitar reads one there as a rest (G9).
            None if v == GpVersion::Gp5 => {
                raw.bars[m][0].voices[0].insert(
                    0,
                    RawBeat {
                        status: BeatStatus::Empty,
                        ..RawBeat::default()
                    },
                );
                (0, 0)
            }
            None => {
                issues.add(K::TempoDropped, m);
                continue;
            }
        };
        // One tempo per position: if another track's beat at the same offset already
        // carries a change, that beat takes this one too (and the replacement is
        // counted), so the importer never reads two changes at one position.
        let row = &raw.bars[m];
        let at = starts(&row[t].voices[0]).nth(k).expect("target exists");
        let (t, k) = (0..row.len())
            .find_map(|u| {
                let voice = &row[u].voices[0];
                starts(voice)
                    .zip(voice)
                    .position(|(o, b)| o == at && b.tempo.is_some())
                    .map(|j| (u, j))
            })
            .unwrap_or((t, k));
        if exact.is_none() {
            issues.add(K::TempoMoved, m);
        }
        if raw.bars[m][t].voices[0][k].tempo.replace(tempo).is_some() {
            issues.add(K::TempoDropped, m);
        }
    }
}

/// The start offset of each beat of a voice.
fn starts(voice: &[RawBeat]) -> impl Iterator<Item = Rational> + '_ {
    voice.iter().scan(Rational::ZERO, |offset, b| {
        let start = *offset;
        *offset = offset.checked_add(raw_length(b)).unwrap_or(*offset);
        Some(start)
    })
}

/// Length of a written beat as a fraction of a whole note.
fn raw_length(b: &RawBeat) -> Rational {
    if b.status == BeatStatus::Empty {
        return Rational::ZERO;
    }
    let base = Rational::new(1, 1i64 << (b.duration + 2)).expect("non-zero");
    let dotted = if b.dotted {
        base.checked_mul(Rational::new(3, 2).expect("valid"))
            .unwrap_or(base)
    } else {
        base
    };
    match b.tuplet.and_then(|e| tuplet_normal(e).map(|n| (e, n))) {
        Some((e, n)) => dotted
            .checked_mul(Rational::new(n as i64, e as i64).expect("valid"))
            .unwrap_or(dotted),
        None => dotted,
    }
}
