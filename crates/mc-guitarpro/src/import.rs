//! `RawSong` to a validated `Score` with an import report (ADR-036 section 3). Every
//! value the score cannot hold, and every repair, is counted; nothing is dropped
//! silently. Notes in the file = notes imported + notes dropped.

use crate::raw::*;
use crate::read::{self, Failure};
use crate::write::DEFAULT_VELOCITY;
use crate::{Feature, Tally};
use mc_score::{
    BarContent, Beat, BeatId, FrettedInstrument, InstrumentSpec, MasterBar, Note, NoteEffects,
    NoteId, NoteKind, Score, ScoreParts, Stroke, TempoChange, TrackData, TrackId, Voice,
};
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning};
use mc_theory::{
    BaseValue, KeySignature, Mode, NoteValue, Rational, SpelledPitch, Tempo, TimeSignature, Tuplet,
};
use std::collections::BTreeMap;

/// Largest file the importer reads (ADR-036 section 3).
pub const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;

/// Highest fret count a track may have (C-01).
const MAX_FRET_COUNT: i32 = 36;

/// What the import changed or dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpImportIssueKind {
    /// Content the score does not hold; counted per occurrence.
    NotKept(Feature),
    /// Kept in a simpler form: a heavy accent as an accent, a brush as a stroke
    /// (direction as TuxGuitar reads it, speed dropped; ADR-036 choice 7.1).
    Approximated(Feature),
    /// A percussion track (ADR-036 choice 7.5); its notes count as dropped.
    PercussionTrackSkipped,
    /// A note on a string the track lacks, on a negative fret, or above the fret count.
    NoteOutsideTrack,
    /// A note in a rest or an empty beat.
    NoteInRest,
    /// A tie with no equal note before it on the same string: the tie flag was removed.
    TieWithoutSource,
    /// A tuplet whose "enters" is not in the format's table: written without a tuplet.
    UnsupportedTuplet,
    /// A time signature the score does not allow: the previous one was kept.
    InvalidTimeSignature,
    /// A key outside -7..=7 fifths: the previous one was kept.
    InvalidKey,
    /// A repeat played more than 16 times: clamped to 16.
    RepeatCountClamped,
    /// Repeat marks removed: a repeat played fewer than 2 times, or a structure that
    /// breaks H-12 (ADR-036 choice 7.4).
    RepeatRemoved,
    /// The song tempo was not positive: 120 BPM was used.
    TempoDefaulted,
    /// A tempo change after the end of its bar moved to the start of the next bar.
    TempoMoved,
    /// A tempo change that is not positive or lies after the last bar.
    TempoDropped,
    /// A fret count outside 1..=36 (C-01) was clamped.
    FretCountClamped,
    /// A capo outside 0..=fret count was clamped.
    CapoClamped,
}

/// One kind of change, with how often it happened and the first bar (0-based).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpImportIssue {
    pub kind: GpImportIssueKind,
    pub count: usize,
    pub first_bar: usize,
}

impl std::fmt::Display for GpImportIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use GpImportIssueKind as K;
        let what = match self.kind {
            K::NotKept(x) => format!("{} not kept", x.name()),
            K::Approximated(Feature::HeavyAccent) => "heavy accent kept as an accent".into(),
            K::Approximated(Feature::Brush) => "brush kept as a stroke, without its speed".into(),
            K::Approximated(x) => format!("{} approximated", x.name()),
            K::PercussionTrackSkipped => "percussion track skipped".into(),
            K::NoteOutsideTrack => "note outside the track's strings or frets dropped".into(),
            K::NoteInRest => "note in a rest dropped".into(),
            K::TieWithoutSource => "tie without a note before it removed".into(),
            K::UnsupportedTuplet => "unsupported tuplet written as a plain value".into(),
            K::InvalidTimeSignature => "invalid time signature ignored".into(),
            K::InvalidKey => "invalid key signature ignored".into(),
            K::RepeatCountClamped => "repeat count above 16 clamped".into(),
            K::RepeatRemoved => "repeat marks removed".into(),
            K::TempoDefaulted => "missing tempo set to 120 BPM".into(),
            K::TempoMoved => "tempo change moved to the next bar".into(),
            K::TempoDropped => "tempo change dropped".into(),
            K::FretCountClamped => "fret count clamped to 1..=36".into(),
            K::CapoClamped => "capo clamped to the fret count".into(),
        };
        write!(
            f,
            "{what}: {} (first in bar {})",
            self.count,
            self.first_bar + 1
        )
    }
}

/// What the import did (ADR-012 `ImportReport`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GpImportReport {
    /// The version string of the file, for example "FICHIER GUITAR PRO v5.00".
    pub version: String,
    pub tracks_imported: usize,
    pub bars: usize,
    pub notes_in_file: usize,
    pub notes_imported: usize,
    /// Always `notes_in_file - notes_imported`; each drop is also an issue.
    pub notes_dropped: usize,
    /// In a fixed order of kinds.
    pub issues: Vec<GpImportIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpImportError {
    TooLarge {
        bytes: usize,
    },
    /// Not a GP3, GP4 or GP5 5.00 file; the text names what it is (ADR-036 choice 7.8).
    UnsupportedVersion(String),
    /// The bytes do not follow the format. `offset` is where reading stopped.
    Malformed {
        offset: usize,
        message: String,
    },
    /// Every track is a percussion track (ADR-036 choice 7.5).
    NoGuitarTracks,
    /// The built score broke a hard invariant (a bug in the importer, never expected).
    Invalid(Vec<String>),
}

impl std::fmt::Display for GpImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpImportError::TooLarge { bytes } => write!(f, "file too large ({bytes} bytes)"),
            GpImportError::UnsupportedVersion(v) => {
                write!(
                    f,
                    "unsupported file: {v}; GP3, GP4 and GP5 (5.00) are supported"
                )
            }
            GpImportError::Malformed { offset, message } => {
                write!(
                    f,
                    "not a readable Guitar Pro file: {message} (at byte {offset})"
                )
            }
            GpImportError::NoGuitarTracks => write!(f, "the file has only percussion tracks"),
            GpImportError::Invalid(v) => write!(f, "invalid score: {}", v.join("; ")),
        }
    }
}

impl std::error::Error for GpImportError {}

/// Normal count of a tuplet from its "enters", the format's fixed table.
pub(crate) fn tuplet_normal(enters: i32) -> Option<u8> {
    match enters {
        3 => Some(2),
        5..=7 => Some(4),
        9..=13 => Some(8),
        _ => None,
    }
}

/// Import a GP3, GP4 or GP5 (5.00) file. The bytes are the file content.
pub fn import_gp(bytes: &[u8]) -> Result<(Score, GpImportReport), GpImportError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(GpImportError::TooLarge { bytes: bytes.len() });
    }
    let (v, version) = read::version(bytes).map_err(failure)?;
    let (_, raw) = read::read(bytes).map_err(failure)?;
    let mut report = GpImportReport {
        version,
        ..GpImportReport::default()
    };
    let score = build(&raw, v, &mut report)?;
    Ok((score, report))
}

fn failure(f: Failure) -> GpImportError {
    match f {
        Failure::Version(v) => GpImportError::UnsupportedVersion(v),
        Failure::Layout(e) => GpImportError::Malformed {
            offset: e.at,
            message: e.what,
        },
    }
}

/// The value of a beat that takes time, or `None` for an unsupported tuplet's ratio.
fn value(b: &RawBeat) -> (NoteValue, bool) {
    let base = BaseValue::from_exponent((b.duration + 2) as u8).expect("checked by the reader");
    let (tuplet, ok) = match b.tuplet {
        None | Some(1) => (None, true),
        Some(e) => match tuplet_normal(e).and_then(|n| Tuplet::new(e as u8, n).ok()) {
            Some(t) => (Some(t), true),
            None => (None, false),
        },
    };
    let v = NoteValue::new(base, b.dotted as u8, tuplet).expect("one dot and a valid tuplet");
    (v, ok)
}

struct Ids(u64);

impl Ids {
    fn next(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
}

fn build(raw: &RawSong, v: GpVersion, report: &mut GpImportReport) -> Result<Score, GpImportError> {
    let mut issues: Tally<GpImportIssueKind> = Tally::default();
    use GpImportIssueKind as K;

    if raw.info.iter().chain(&raw.notice).any(|s| !s.is_empty()) {
        issues.add(K::NotKept(Feature::SongInformation), 0);
    }
    if raw.lyrics.iter().any(|(_, t)| !t.trim().is_empty()) {
        issues.add(K::NotKept(Feature::Lyrics), 0);
    }
    if raw.triplet_feel {
        issues.add(K::NotKept(Feature::TripletFeel), 0);
    }

    let master_bars = master_bars(raw, v, &mut issues);
    let bars = master_bars.len();
    report.bars = bars;
    report.notes_in_file = raw
        .bars
        .iter()
        .flatten()
        .flat_map(|b| b.voices.iter().flatten())
        .map(|b| b.notes.len())
        .sum();

    let mut ids = Ids(0);
    let mut tracks = Vec::new();
    for (t, rt) in raw.tracks.iter().enumerate() {
        if rt.percussion {
            issues.add(K::PercussionTrackSkipped, 0);
            continue;
        }
        tracks.push(track(
            raw,
            t,
            rt,
            &master_bars,
            &mut ids,
            &mut issues,
            report,
        )?);
    }
    if tracks.is_empty() {
        return Err(GpImportError::NoGuitarTracks);
    }
    report.tracks_imported = tracks.len();
    report.notes_dropped = report.notes_in_file - report.notes_imported;

    let tempo_map = tempo_map(raw, &master_bars, &mut issues);
    let master_bars = repair_repeats(master_bars, &mut issues);
    report.issues = issues.into_vec(|kind, count, first_bar| GpImportIssue {
        kind,
        count,
        first_bar,
    });
    Score::from_parts(ScoreParts {
        master_bars,
        tracks,
        tempo_map,
    })
    .map_err(|v| GpImportError::Invalid(v.iter().map(|x| x.to_string()).collect()))
}

fn master_bars(
    raw: &RawSong,
    v: GpVersion,
    issues: &mut Tally<GpImportIssueKind>,
) -> Vec<MasterBar> {
    use GpImportIssueKind as K;
    let (mut num, mut den) = (4u8, 4u8);
    let mut ts = TimeSignature::COMMON;
    let mut key = match i8::try_from(raw.key)
        .ok()
        .and_then(|k| KeySignature::new(k, Mode::Major).ok())
    {
        Some(k) => k,
        None => {
            issues.add(K::InvalidKey, 0);
            KeySignature::C_MAJOR
        }
    };
    let mut out = Vec::with_capacity(raw.headers.len());
    for (i, h) in raw.headers.iter().enumerate() {
        // Version rule: the parts of the time signature are written only on a change.
        if h.numerator.is_some() || h.denominator.is_some() {
            num = h.numerator.unwrap_or(num);
            den = h.denominator.unwrap_or(den);
            match TimeSignature::new(num, den) {
                Ok(t) => ts = t,
                Err(_) => {
                    issues.add(K::InvalidTimeSignature, i);
                    (num, den) = (ts.numerator(), ts.denominator());
                }
            }
        }
        if let Some((fifths, minor)) = h.key {
            let mode = if minor != 0 { Mode::Minor } else { Mode::Major };
            match KeySignature::new(fifths, mode) {
                Ok(k) => key = k,
                Err(_) => issues.add(K::InvalidKey, i),
            }
        }
        let mut mb = MasterBar::new(ts, key);
        mb.repeat_start = h.repeat_start;
        if let Some(stored) = h.repeat_close {
            // Version rule: GP5 stores the plays, GP3 and GP4 the repeats.
            let plays = if v == GpVersion::Gp5 {
                stored as u16
            } else {
                stored as u16 + 1
            };
            mb.repeat_end = match plays {
                0 | 1 => {
                    issues.add(K::RepeatRemoved, i);
                    None
                }
                2..=16 => Some(plays as u8),
                _ => {
                    issues.add(K::RepeatCountClamped, i);
                    Some(16)
                }
            };
        }
        mb.alternate_endings = (0..8u8)
            .filter(|b| h.alternative & (1 << b) != 0)
            .map(|b| b + 1)
            .collect();
        mb.marker = h.marker.as_ref().map(|(name, _)| name.clone());
        if h.double_bar {
            issues.add(K::NotKept(Feature::DoubleBar), i);
        }
        if h.triplet_feel != 0 {
            issues.add(K::NotKept(Feature::TripletFeel), i);
        }
        out.push(mb);
    }
    out
}

/// Remove the repeat marks of bars that break H-12 (ADR-036 choice 7.4). The check is
/// the score's own: a probe with the master bars alone, so no rule is copied here.
fn repair_repeats(
    mut bars: Vec<MasterBar>,
    issues: &mut Tally<GpImportIssueKind>,
) -> Vec<MasterBar> {
    for _ in 0..4 {
        let broken = h12_bars(&bars);
        if broken.is_empty() {
            return bars;
        }
        for i in broken {
            let Some(b) = i.and_then(|i| bars.get_mut(i)) else {
                // A location we cannot place: remove every repeat mark.
                strip(&mut bars, issues);
                return bars;
            };
            if !b.alternate_endings.is_empty() {
                b.alternate_endings.clear();
            } else {
                b.repeat_start = false;
                b.repeat_end = None;
            }
            issues.add(GpImportIssueKind::RepeatRemoved, i.unwrap_or(0));
        }
    }
    if !h12_bars(&bars).is_empty() {
        strip(&mut bars, issues);
    }
    bars
}

fn strip(bars: &mut [MasterBar], issues: &mut Tally<GpImportIssueKind>) {
    for (i, b) in bars.iter_mut().enumerate() {
        if b.repeat_start || b.repeat_end.is_some() || !b.alternate_endings.is_empty() {
            b.repeat_start = false;
            b.repeat_end = None;
            b.alternate_endings.clear();
            issues.add(GpImportIssueKind::RepeatRemoved, i);
        }
    }
}

/// Bars named by H-12 violations ("master bar N"); `None` if a location does not parse.
fn h12_bars(bars: &[MasterBar]) -> Vec<Option<usize>> {
    let probe = ScoreParts {
        master_bars: bars.to_vec(),
        tracks: Vec::new(),
        tempo_map: vec![TempoChange {
            bar: 0,
            offset: Rational::ZERO,
            tempo: Tempo::from_bpm(120).expect("valid"),
        }],
    };
    match Score::from_parts(probe) {
        Ok(_) => Vec::new(),
        Err(v) => {
            let mut out: Vec<Option<usize>> = v
                .iter()
                .filter(|x| x.rule == "H-12")
                .map(|x| {
                    x.location
                        .strip_prefix("master bar ")
                        .and_then(|n| n.parse().ok())
                })
                .collect();
            out.dedup();
            out
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn track(
    raw: &RawSong,
    t: usize,
    rt: &RawTrack,
    master_bars: &[MasterBar],
    ids: &mut Ids,
    issues: &mut Tally<GpImportIssueKind>,
    report: &mut GpImportReport,
) -> Result<TrackData, GpImportError> {
    use GpImportIssueKind as K;
    let mut pitches = Vec::with_capacity(rt.tuning.len());
    for &p in &rt.tuning {
        if !(0..=127).contains(&p) {
            return Err(GpImportError::Malformed {
                offset: 0,
                message: format!("track {} has an open string of MIDI pitch {p}", t + 1),
            });
        }
        pitches.push(SpelledPitch::from_semitone_sharp(p));
    }
    let tuning = Tuning::new(pitches).map_err(|e| GpImportError::Malformed {
        offset: 0,
        message: format!("track {}: {e:?}", t + 1),
    })?;
    let strings = rt.tuning.len() as u8;
    let frets = rt.frets.clamp(1, MAX_FRET_COUNT);
    if frets != rt.frets {
        issues.add(K::FretCountClamped, 0);
    }
    let capo = rt.capo.clamp(0, frets);
    if capo != rt.capo {
        issues.add(K::CapoClamped, 0);
    }
    let (frets, capo) = (frets as u8, capo as u8);
    let id = TrackId::from_raw(ids.next());
    let name = if rt.name.trim().is_empty() {
        format!("Track {}", t + 1)
    } else {
        rt.name.clone()
    };

    // The previous imported beat of each voice, for ties (H-06): (string, fret) pairs.
    let mut prev: Vec<Option<Vec<(u8, u8)>>> = vec![None; 2];
    let mut content = Vec::with_capacity(master_bars.len());
    for (m, row) in raw.bars.iter().enumerate() {
        let bar = &row[t];
        let mut voices = Vec::new();
        for (vi, raw_voice) in bar.voices.iter().enumerate() {
            let mut beats = Vec::new();
            for b in raw_voice {
                for f in &b.dropped {
                    issues.add(K::NotKept(*f), m);
                }
                if b.chord.is_some() {
                    issues.add(K::NotKept(Feature::ChordDiagram), m);
                }
                if b.text.is_some() {
                    issues.add(K::NotKept(Feature::BeatText), m);
                }
                if b.mix_other {
                    issues.add(K::NotKept(Feature::MixTable), m);
                }
                if b.status != BeatStatus::Normal {
                    issues.add_n(K::NoteInRest, m, b.notes.len());
                    if b.status == BeatStatus::Empty {
                        continue;
                    }
                }
                let (value, tuplet_ok) = value(b);
                if !tuplet_ok {
                    issues.add(K::UnsupportedTuplet, m);
                }
                let stroke = match b.pick_stroke {
                    2 => Some(Stroke::Down),
                    1 => Some(Stroke::Up),
                    _ => None,
                };
                let stroke = if b.brush != (0, 0) {
                    issues.add(K::Approximated(Feature::Brush), m);
                    // TuxGuitar's "down" is stored in the byte the description calls
                    // "up" (S10 section 6.3); confirmed against TuxGuitar only.
                    stroke.or(Some(if b.brush.0 != 0 {
                        Stroke::Down
                    } else {
                        Stroke::Up
                    }))
                } else {
                    stroke
                };
                let mut notes = Vec::new();
                if b.status == BeatStatus::Normal {
                    for n in &b.notes {
                        if let Some(note) = note(
                            n,
                            prev[vi.min(1)].as_deref(),
                            strings,
                            frets,
                            capo,
                            m,
                            ids,
                            issues,
                        ) {
                            notes.push(note);
                        }
                    }
                }
                report.notes_imported += notes.len();
                prev[vi.min(1)] = Some(
                    notes
                        .iter()
                        .map(|n: &Note| (n.position().string.get(), n.position().fret.0))
                        .collect(),
                );
                beats.push(Beat {
                    id: BeatId::from_raw(ids.next()),
                    value,
                    notes,
                    stroke,
                });
            }
            if beats.is_empty() {
                prev[vi.min(1)] = None;
            }
            if vi == 0 || !beats.is_empty() {
                voices.push(Voice { beats });
            }
        }
        if voices.is_empty() {
            voices.push(Voice::default());
        }
        content.push(BarContent { voices });
    }
    Ok(TrackData {
        id,
        name,
        instrument: InstrumentSpec::Fretted(FrettedInstrument {
            tuning,
            fret_count: frets,
            capo: Capo(capo),
        }),
        bars: content,
    })
}

#[allow(clippy::too_many_arguments)]
fn note(
    n: &RawNote,
    prev: Option<&[(u8, u8)]>,
    strings: u8,
    frets: u8,
    capo: u8,
    bar: usize,
    ids: &mut Ids,
    issues: &mut Tally<GpImportIssueKind>,
) -> Option<Note> {
    use GpImportIssueKind as K;
    for f in &n.dropped {
        issues.add(K::NotKept(*f), bar);
    }
    if n.ghost {
        issues.add(K::NotKept(Feature::GhostNote), bar);
    }
    if n.staccato {
        issues.add(K::NotKept(Feature::Staccato), bar);
    }
    if n.velocity.is_some_and(|x| x != DEFAULT_VELOCITY) {
        issues.add(K::NotKept(Feature::Dynamics), bar);
    }
    if n.heavy_accent {
        issues.add(K::Approximated(Feature::HeavyAccent), bar);
    }
    let mut tie = n.kind == NoteType::Tie;
    let mut fret = n.fret as i16;
    if tie {
        // The fret of a tied note is the fret of the note it continues.
        match prev.and_then(|p| p.iter().find(|(s, _)| *s == n.string)) {
            Some((_, f)) => fret = *f as i16,
            None => {
                issues.add(K::TieWithoutSource, bar);
                tie = false;
            }
        }
    }
    if n.string == 0 || n.string > strings || fret < 0 || fret + capo as i16 > frets as i16 {
        issues.add(K::NoteOutsideTrack, bar);
        return None;
    }
    Some(Note {
        id: NoteId::from_raw(ids.next()),
        kind: NoteKind::Fretted(FretPosition {
            string: StringIndex::new(n.string).expect("not 0"),
            fret: Fret(fret as u8),
        }),
        tie,
        effects: NoteEffects {
            let_ring: n.let_ring,
            dead: n.kind == NoteType::Dead,
            accent: n.accent || n.heavy_accent,
            palm_mute: n.palm_mute,
            string_percussive: false,
        },
    })
}

/// The song tempo at the start, then each beat's tempo change at its offset in the bar
/// (tempo is song-wide in the format; every track and voice is read, later ones win).
fn tempo_map(
    raw: &RawSong,
    bars: &[MasterBar],
    issues: &mut Tally<GpImportIssueKind>,
) -> Vec<TempoChange> {
    use GpImportIssueKind as K;
    let mut at: BTreeMap<(usize, Rational), Tempo> = BTreeMap::new();
    let start = if raw.tempo > 0 {
        Tempo::from_bpm(raw.tempo as u32).expect("positive")
    } else {
        issues.add(K::TempoDefaulted, 0);
        Tempo::from_bpm(120).expect("valid")
    };
    at.insert((0, Rational::ZERO), start);
    for (m, row) in raw.bars.iter().enumerate() {
        let length = bars[m].time_signature.length();
        for bar in row {
            for voice in &bar.voices {
                let mut offset = Rational::ZERO;
                for b in voice {
                    if let Some(bpm) = b.tempo {
                        let place = if offset < length {
                            Some((m, offset))
                        } else if m + 1 < bars.len() {
                            issues.add(K::TempoMoved, m);
                            Some((m + 1, Rational::ZERO))
                        } else {
                            None
                        };
                        match place {
                            Some(p) if bpm > 0 => {
                                at.insert(p, Tempo::from_bpm(bpm as u32).expect("positive"));
                            }
                            _ => issues.add(K::TempoDropped, m),
                        }
                    }
                    if b.status != BeatStatus::Empty {
                        offset = offset.checked_add(value(b).0.length()).unwrap_or(offset);
                    }
                }
            }
        }
    }
    at.into_iter()
        .map(|((bar, offset), tempo)| TempoChange { bar, offset, tempo })
        .collect()
}

#[cfg(test)]
mod tests {
    //! G4: every repair of ADR-036 section 3 on a hand-built file; the repaired score
    //! passes `Score::from_parts` (the import returns it).
    use super::*;
    use crate::write::write;

    fn n(string: u8, fret: i8) -> RawNote {
        RawNote {
            string,
            fret,
            ..RawNote::default()
        }
    }

    fn b(duration: i8, notes: Vec<RawNote>) -> RawBeat {
        RawBeat {
            duration,
            notes,
            ..RawBeat::default()
        }
    }

    /// One standard-tuned track; one bar per entry of `bars` (voice 1 beats), 4/4.
    fn song(bars: Vec<Vec<RawBeat>>) -> RawSong {
        let mut headers = vec![RawHeader::default(); bars.len()];
        headers[0].numerator = Some(4);
        headers[0].denominator = Some(4);
        RawSong {
            tempo: 120,
            headers,
            tracks: vec![RawTrack {
                name: "T".into(),
                tuning: vec![64, 59, 55, 50, 45, 40],
                port: 1,
                channel: 1,
                effect_channel: 2,
                frets: 24,
                ..RawTrack::default()
            }],
            bars: bars
                .into_iter()
                .map(|beats| {
                    vec![RawBar {
                        voices: vec![beats],
                    }]
                })
                .collect(),
            ..RawSong::default()
        }
    }

    fn run(s: &RawSong, v: GpVersion) -> (Score, GpImportReport) {
        import_gp(&write(s, v)).unwrap()
    }

    fn count(r: &GpImportReport, kind: GpImportIssueKind) -> usize {
        r.issues
            .iter()
            .filter(|i| i.kind == kind)
            .map(|i| i.count)
            .sum()
    }

    fn first_note(s: &Score, beat: usize) -> Option<Note> {
        let t = s.tracks().next().unwrap();
        t.bar(0).unwrap().voices[0].beats[beat]
            .notes
            .first()
            .cloned()
    }

    #[test]
    fn g4_tie_without_a_source_loses_its_tie() {
        let mut tied = n(1, 3);
        tied.kind = NoteType::Tie;
        let (s, r) = run(&song(vec![vec![b(0, vec![tied])]]), GpVersion::Gp5);
        let note = first_note(&s, 0).unwrap();
        assert!(!note.tie);
        assert_eq!(note.position().fret.0, 3);
        assert_eq!(count(&r, GpImportIssueKind::TieWithoutSource), 1);
    }

    #[test]
    fn g4_a_tied_note_takes_the_fret_of_its_source() {
        let mut tied = n(1, 0);
        tied.kind = NoteType::Tie;
        let s = song(vec![vec![b(0, vec![n(1, 5)])], vec![b(0, vec![tied])]]);
        for v in [GpVersion::Gp3, GpVersion::Gp4, GpVersion::Gp5] {
            let (score, r) = run(&s, v);
            let t = score.tracks().next().unwrap();
            let note = &t.bar(1).unwrap().voices[0].beats[0].notes[0];
            assert!(note.tie, "{v:?}");
            assert_eq!(note.position().fret.0, 5);
            assert!(r.issues.is_empty(), "{:?}", r.issues);
        }
    }

    #[test]
    fn g4_notes_outside_the_track_are_dropped() {
        let mut s = song(vec![vec![b(
            0,
            vec![n(1, 23), n(2, -1), n(7, 0), n(3, 22)],
        )]]);
        s.tracks[0].capo = 2;
        let (score, r) = run(&s, GpVersion::Gp5);
        assert_eq!(score.note_count(), 1);
        assert_eq!(first_note(&score, 0).unwrap().position().fret.0, 22);
        assert_eq!(count(&r, GpImportIssueKind::NoteOutsideTrack), 3);
        assert_eq!((r.notes_in_file, r.notes_dropped), (4, 3));
    }

    #[test]
    fn g4_a_note_in_a_rest_is_dropped() {
        let mut rest = b(0, vec![n(1, 0)]);
        rest.status = BeatStatus::Rest;
        let (score, r) = run(&song(vec![vec![rest]]), GpVersion::Gp4);
        assert_eq!(score.note_count(), 0);
        assert_eq!(count(&r, GpImportIssueKind::NoteInRest), 1);
    }

    fn repeats(v: GpVersion, marks: &[(bool, Option<u8>, u8)]) -> (Score, GpImportReport) {
        let mut s = song(vec![vec![b(-2, vec![])]; marks.len()]);
        for (h, &(start, close, alt)) in s.headers.iter_mut().zip(marks) {
            h.repeat_start = start;
            h.repeat_close = close;
            h.alternative = alt;
        }
        run(&s, v)
    }

    #[test]
    fn g4_a_nested_repeat_start_is_removed() {
        let (s, r) = repeats(
            GpVersion::Gp5,
            &[(true, None, 0), (true, None, 0), (false, Some(2), 0)],
        );
        let b = s.master_bars();
        assert!(b[0].repeat_start && !b[1].repeat_start);
        assert_eq!(b[2].repeat_end, Some(2));
        assert_eq!(count(&r, GpImportIssueKind::RepeatRemoved), 1);
    }

    #[test]
    fn g4_an_ending_outside_a_section_or_above_the_count_is_removed() {
        let (s, r) = repeats(GpVersion::Gp5, &[(false, None, 1), (false, None, 0)]);
        assert!(s.master_bars()[0].alternate_endings.is_empty());
        assert_eq!(count(&r, GpImportIssueKind::RepeatRemoved), 1);
        // Pass 3 in a section played twice.
        let (s, r) = repeats(GpVersion::Gp5, &[(true, None, 0), (false, Some(2), 0b100)]);
        assert!(s.master_bars()[1].alternate_endings.is_empty());
        assert_eq!(s.master_bars()[1].repeat_end, Some(2));
        assert_eq!(count(&r, GpImportIssueKind::RepeatRemoved), 1);
    }

    #[test]
    fn g4_repeat_counts_are_clamped_or_removed() {
        let (s, r) = repeats(GpVersion::Gp5, &[(true, Some(20), 0)]);
        assert_eq!(s.master_bars()[0].repeat_end, Some(16));
        assert_eq!(count(&r, GpImportIssueKind::RepeatCountClamped), 1);
        let (s, r) = repeats(GpVersion::Gp5, &[(true, Some(1), 0)]);
        assert_eq!(s.master_bars()[0].repeat_end, None);
        assert_eq!(count(&r, GpImportIssueKind::RepeatRemoved), 1);
        // GP4 stores repeats: 0 repeats is 1 play.
        let (s, _) = repeats(GpVersion::Gp4, &[(true, Some(0), 0)]);
        assert_eq!(s.master_bars()[0].repeat_end, None);
    }

    #[test]
    fn g4_tempo_repairs() {
        let mut s = song(vec![vec![b(0, vec![])], vec![b(0, vec![])]]);
        s.tempo = 0;
        let (score, r) = run(&s, GpVersion::Gp5);
        assert_eq!(score.tempo_map()[0].tempo, Tempo::from_bpm(120).unwrap());
        assert_eq!(count(&r, GpImportIssueKind::TempoDefaulted), 1);

        // Bar 1 is overfull: a change on its fifth quarter moves to bar 2; one in the
        // last bar after its end is dropped; a negative tempo is dropped.
        let mut s = song(vec![vec![b(0, vec![]); 5], vec![b(0, vec![]); 5]]);
        s.bars[0][0].voices[0][4].tempo = Some(150);
        s.bars[1][0].voices[0][4].tempo = Some(160);
        s.bars[1][0].voices[0][1].tempo = Some(-3);
        let (score, r) = run(&s, GpVersion::Gp5);
        let map: Vec<_> = score
            .tempo_map()
            .iter()
            .map(|t| (t.bar, t.offset))
            .collect();
        assert_eq!(map, [(0, Rational::ZERO), (1, Rational::ZERO)]);
        assert_eq!(count(&r, GpImportIssueKind::TempoMoved), 1);
        assert_eq!(count(&r, GpImportIssueKind::TempoDropped), 2);
    }

    #[test]
    fn g4_fret_count_and_capo_are_clamped() {
        for (frets, capo, want) in [
            (40, 0, (36, 0)),
            (0, 0, (1, 0)),
            (24, 30, (24, 24)),
            (24, -1, (24, 0)),
        ] {
            let mut s = song(vec![vec![b(0, vec![])]]);
            s.tracks[0].frets = frets;
            s.tracks[0].capo = capo;
            let (score, r) = run(&s, GpVersion::Gp5);
            let f = score.tracks().next().unwrap().fretted().clone();
            assert_eq!((f.fret_count, f.capo.0), want);
            let clamped = count(&r, GpImportIssueKind::FretCountClamped)
                + count(&r, GpImportIssueKind::CapoClamped);
            assert_eq!(clamped, 1, "{frets} {capo}");
        }
    }

    #[test]
    fn g4_invalid_time_signature_and_key_keep_the_previous() {
        let mut s = song(vec![vec![b(0, vec![])]; 3]);
        s.headers[1].numerator = Some(0);
        s.headers[2].key = Some((9, 0));
        s.key = 20;
        let (score, r) = run(&s, GpVersion::Gp5);
        let b = score.master_bars();
        assert_eq!(b[1].time_signature, TimeSignature::COMMON);
        assert_eq!(b[2].key, KeySignature::C_MAJOR);
        assert_eq!(count(&r, GpImportIssueKind::InvalidTimeSignature), 1);
        assert_eq!(count(&r, GpImportIssueKind::InvalidKey), 2);
    }

    #[test]
    fn g4_an_unsupported_tuplet_becomes_a_plain_value() {
        let mut beat = b(1, vec![n(1, 0)]);
        beat.tuplet = Some(4);
        let (score, r) = run(&song(vec![vec![beat]]), GpVersion::Gp5);
        let t = score.tracks().next().unwrap();
        assert_eq!(t.bar(0).unwrap().voices[0].beats[0].value.tuplet(), None);
        assert_eq!(count(&r, GpImportIssueKind::UnsupportedTuplet), 1);
    }

    /// ADR-036 choice 7.7: the old chord layout is read as described (no real file
    /// shows it: this hand-built block is the only evidence); the GP3 new layout is
    /// refused.
    #[test]
    fn g4_chord_layouts() {
        let mut beat = b(0, vec![n(1, 0)]);
        beat.chord = Some(RawChord {
            name: "C".into(),
            first_fret: 1,
            frets: vec![0; 7],
        });
        let bytes = write(&song(vec![vec![beat]]), GpVersion::Gp4);
        let new = bytes
            .windows(6)
            .position(|w| w == [1, 1, 0, 0, 0, 12])
            .expect("new-layout block");
        let mut old = vec![0u8];
        old.extend_from_slice(&2i32.to_le_bytes());
        old.extend_from_slice(&[1, b'C']);
        old.extend_from_slice(&1i32.to_le_bytes());
        for f in [0, 1, 0, 2, 3, -1i32] {
            old.extend_from_slice(&f.to_le_bytes());
        }
        let mut patched = bytes[..new].to_vec();
        patched.extend_from_slice(&old);
        patched.extend_from_slice(&bytes[new + 107..]);
        let (score, r) = import_gp(&patched).unwrap();
        assert_eq!(score.note_count(), 1);
        assert_eq!(
            count(&r, GpImportIssueKind::NotKept(Feature::ChordDiagram)),
            1
        );

        // GP3: set the chord flag and a new-layout header after the duration.
        let mut gp3 = write(&song(vec![vec![b(0, vec![n(1, 0)])]]), GpVersion::Gp3);
        let end = gp3.len();
        assert_eq!(gp3[end - 6..], [0, 0, 0x40, 0x20, 1, 0]);
        gp3[end - 6] |= 0x02;
        gp3.insert(end - 4, 1);
        match import_gp(&gp3) {
            Err(GpImportError::Malformed { message, .. }) => {
                assert!(message.contains("not verified"), "{message}")
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn g5_count_limits() {
        // Bars, tracks, strings and beats beyond the limits are refused before
        // anything is allocated for them.
        let mut s = song(vec![vec![]]);
        s.headers = vec![RawHeader::default(); read::MAX_BARS + 1];
        s.bars = vec![
            vec![RawBar {
                voices: vec![vec![]]
            }];
            read::MAX_BARS + 1
        ];
        assert!(
            matches!(import_gp(&write(&s, GpVersion::Gp4)), Err(GpImportError::Malformed { message, .. }) if message.contains("bars"))
        );

        let mut s = song(vec![vec![]]);
        s.tracks = vec![s.tracks[0].clone(); read::MAX_TRACKS + 1];
        s.bars = vec![vec![
            RawBar {
                voices: vec![vec![]]
            };
            read::MAX_TRACKS + 1
        ]];
        assert!(
            matches!(import_gp(&write(&s, GpVersion::Gp4)), Err(GpImportError::Malformed { message, .. }) if message.contains("tracks"))
        );

        let mut s = song(vec![vec![]]);
        s.tracks[0].tuning = vec![64; 8];
        assert!(
            matches!(import_gp(&write(&s, GpVersion::Gp4)), Err(GpImportError::Malformed { message, .. }) if message.contains("strings"))
        );

        let s = song(vec![vec![b(4, vec![]); read::MAX_BEATS + 1]]);
        assert!(
            matches!(import_gp(&write(&s, GpVersion::Gp5)), Err(GpImportError::Malformed { message, .. }) if message.contains("beats"))
        );

        // A tuning pitch outside MIDI.
        let mut s = song(vec![vec![]]);
        s.tracks[0].tuning[0] = 200;
        assert!(matches!(
            import_gp(&write(&s, GpVersion::Gp5)),
            Err(GpImportError::Malformed { .. })
        ));
    }
}
