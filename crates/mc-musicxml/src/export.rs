//! Score to MusicXML 4.0 partwise (ADR-033 section 3).

use crate::xml::Xml;
use crate::{MusicXmlError, MusicXmlReport};
use mc_score::{Beat, MasterBar, Note, Score, Track, Voice};
use mc_theory::fretted::pitch_at;
use mc_theory::{KeySignature, Letter, Mode, NoteValue, Rational, Tuplet};

type Res<T> = Result<T, MusicXmlError>;

const OVERFLOW: MusicXmlError = MusicXmlError::DurationOverflow;

fn add(a: Rational, b: Rational) -> Res<Rational> {
    a.checked_add(b).ok_or(OVERFLOW)
}

fn gcd(a: i128, b: i128) -> i128 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

/// Divisions per quarter note: the least common multiple of the denominators of every
/// length the file contains, in quarter notes, so all durations are exact integers.
struct Divisions {
    per_quarter: i128,
}

impl Divisions {
    fn new() -> Self {
        Divisions { per_quarter: 1 }
    }

    fn include(&mut self, length: Rational) -> Res<()> {
        // length is in whole notes; in quarters it is 4n/d.
        let n = length.numerator() as i128 * 4;
        let d = length.denominator() as i128;
        let den = d / gcd(n, d).max(1);
        let lcm = self.per_quarter / gcd(self.per_quarter, den) * den;
        if lcm > i32::MAX as i128 {
            return Err(OVERFLOW);
        }
        self.per_quarter = lcm;
        Ok(())
    }

    /// Duration of `length` (whole notes) in divisions.
    fn of(&self, length: Rational) -> Res<i64> {
        let v = length.numerator() as i128 * 4 * self.per_quarter;
        let d = length.denominator() as i128;
        debug_assert_eq!(v % d, 0, "divisions include every length");
        let q = v / d;
        if q > i32::MAX as i128 || q < 0 {
            return Err(OVERFLOW);
        }
        Ok(q as i64)
    }
}

fn type_name(value: NoteValue) -> &'static str {
    match value.base().exponent() {
        0 => "whole",
        1 => "half",
        2 => "quarter",
        3 => "eighth",
        4 => "16th",
        5 => "32nd",
        6 => "64th",
        7 => "128th",
        _ => "256th",
    }
}

fn step(letter: Letter) -> &'static str {
    match letter {
        Letter::C => "C",
        Letter::D => "D",
        Letter::E => "E",
        Letter::F => "F",
        Letter::G => "G",
        Letter::A => "A",
        Letter::B => "B",
    }
}

/// A tempo as a decimal: exact when it has a short decimal form, else rounded to three
/// decimals (the flag says so).
fn decimal(r: Rational) -> (String, bool) {
    let (n, d) = (r.numerator() as i128, r.denominator() as i128);
    let mut rest = d;
    let (mut twos, mut fives) = (0u32, 0u32);
    while rest % 2 == 0 {
        rest /= 2;
        twos += 1;
    }
    while rest % 5 == 0 {
        rest /= 5;
        fives += 1;
    }
    let (scaled, places, rounded) = if rest == 1 && twos.max(fives) <= 6 {
        let places = twos.max(fives);
        (n * 10i128.pow(places) / d, places, false)
    } else {
        ((n * 2000 + d) / (2 * d), 3, true)
    };
    let p = 10i128.pow(places);
    let mut s = format!("{}", scaled / p);
    if places > 0 {
        let frac = format!("{:0width$}", scaled % p, width = places as usize);
        let frac = frac.trim_end_matches('0');
        if !frac.is_empty() {
            s.push('.');
            s.push_str(frac);
        }
    }
    (s, rounded)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bracket {
    None,
    Start,
    Stop,
    Both,
}

/// Tuplet brackets for a voice: a run of consecutive beats with the same ratio is a
/// whole group when its length equals `normal` times its smallest plain value (three
/// triplet eighths fill two eighths). Beats that are in no whole group keep their
/// ratio without a bracket; the second value counts them.
fn brackets(beats: &[Beat]) -> Res<(Vec<Bracket>, usize)> {
    let mut marks = vec![Bracket::None; beats.len()];
    let mut loose = 0;
    let mut i = 0;
    while i < beats.len() {
        let Some(ratio) = beats[i].value.tuplet() else {
            i += 1;
            continue;
        };
        let mut sum = Rational::ZERO;
        let mut unit: Option<Rational> = None;
        let mut closed = None;
        for (j, b) in beats.iter().enumerate().skip(i) {
            if b.value.tuplet() != Some(ratio) {
                break;
            }
            sum = add(sum, b.value.length())?;
            let plain = NoteValue::plain(b.value.base()).length();
            let u = unit.map_or(plain, |u| u.min(plain));
            unit = Some(u);
            let target = Rational::from_int(ratio.normal() as i64)
                .checked_mul(u)
                .ok_or(OVERFLOW)?;
            if sum == target {
                closed = Some(j);
                break;
            }
        }
        match closed {
            Some(j) if j == i => {
                marks[i] = Bracket::Both;
                i += 1;
            }
            Some(j) => {
                marks[i] = Bracket::Start;
                marks[j] = Bracket::Stop;
                i = j + 1;
            }
            None => {
                loose += 1;
                i += 1;
            }
        }
    }
    Ok((marks, loose))
}

/// The beat that follows beat `k` of voice `v` in bar `m`, in written order (the
/// inverse of the H-06 "preceding beat").
fn next_beat(track: &Track, m: usize, v: usize, k: usize) -> Option<&Beat> {
    let voice = track.bar(m)?.voices.get(v)?;
    if k + 1 < voice.beats.len() {
        return voice.beats.get(k + 1);
    }
    track.bar(m + 1)?.voices.get(v)?.beats.first()
}

/// True when `note` is the start of a tie: the next beat has a tied note on the same
/// string and fret (H-06).
fn starts_tie(note: &Note, next: Option<&Beat>) -> bool {
    next.is_some_and(|b| {
        b.notes
            .iter()
            .any(|n| n.tie && n.position() == note.position())
    })
}

/// Length of the measure the voices are padded to: the time signature, or for a pickup
/// the longest content of that bar in any track (its nominal length is zero).
fn measure_length(score: &Score, m: usize, bar: &MasterBar) -> Res<Rational> {
    if bar.pickup {
        score.effective_bar_duration(m).map_err(|_| OVERFLOW)
    } else {
        Ok(bar.time_signature.length())
    }
}

fn voice_length(voice: &Voice) -> Res<Rational> {
    voice
        .beats
        .iter()
        .try_fold(Rational::ZERO, |s, b| add(s, b.value.length()))
}

fn divisions(score: &Score) -> Res<Divisions> {
    let mut div = Divisions::new();
    for (m, bar) in score.master_bars().iter().enumerate() {
        div.include(bar.time_signature.length())?;
        div.include(measure_length(score, m, bar)?)?;
    }
    for t in score.tempo_map() {
        div.include(t.offset)?;
    }
    for track in score.tracks() {
        for bar in track.bars() {
            for voice in &bar.voices {
                for beat in &voice.beats {
                    div.include(beat.value.length())?;
                }
            }
        }
    }
    Ok(div)
}

/// Export `score` as MusicXML 4.0 partwise. Returns the document and a report.
pub fn export_musicxml(score: &Score) -> Result<(String, MusicXmlReport), MusicXmlError> {
    let div = divisions(score)?;
    let mut report = MusicXmlReport {
        parts: score.tracks().len(),
        measures: score.master_bar_count(),
        ..MusicXmlReport::default()
    };
    let mut x = Xml::new();
    x.open("score-partwise", &[("version", "4.0")]);
    x.open("identification", &[]);
    x.open("encoding", &[]);
    x.text("software", &[], "GuitarInspireAI");
    x.close();
    x.close();
    x.open("part-list", &[]);
    for (i, track) in score.tracks().enumerate() {
        let id = format!("P{}", i + 1);
        x.open("score-part", &[("id", &id)]);
        x.text("part-name", &[], track.name());
        x.close();
    }
    x.close();
    for (i, track) in score.tracks().enumerate() {
        let id = format!("P{}", i + 1);
        x.open("part", &[("id", &id)]);
        for m in 0..score.master_bar_count() {
            write_measure(&mut x, score, track, i, m, &div, &mut report)?;
        }
        x.close();
    }
    x.close();
    let (out, removed) = x.finish();
    report.characters_removed = removed;
    Ok((out, report))
}

fn write_measure(
    x: &mut Xml,
    score: &Score,
    track: &Track,
    part: usize,
    m: usize,
    div: &Divisions,
    report: &mut MusicXmlReport,
) -> Res<()> {
    let bars = score.master_bars();
    let bar = &bars[m];
    let prev = m.checked_sub(1).map(|p| &bars[p]);
    let next = bars.get(m + 1);
    let first_is_pickup = bars.first().is_some_and(|b| b.pickup);
    let number = if first_is_pickup { m } else { m + 1 }.to_string();
    let mut attrs: Vec<(&str, &str)> = vec![("number", &number)];
    if bar.pickup {
        attrs.push(("implicit", "yes"));
    }
    x.open("measure", &attrs);

    // Left barline: repeat start, ending start.
    let ending_text = bar
        .alternate_endings
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let ending_starts = !bar.alternate_endings.is_empty()
        && prev.is_none_or(|p| p.alternate_endings != bar.alternate_endings);
    let ending_stops = !bar.alternate_endings.is_empty()
        && next.is_none_or(|n| n.alternate_endings != bar.alternate_endings);
    if bar.repeat_start || ending_starts {
        x.open("barline", &[("location", "left")]);
        if bar.repeat_start {
            x.text("bar-style", &[], "heavy-light");
        }
        if ending_starts {
            x.empty("ending", &[("number", &ending_text), ("type", "start")]);
        }
        if bar.repeat_start {
            x.empty("repeat", &[("direction", "forward")]);
        }
        x.close();
    }

    // Attributes at the start and on each change.
    let key_changed = prev.is_none_or(|p| p.key != bar.key);
    let time_changed = prev.is_none_or(|p| p.time_signature != bar.time_signature);
    if m == 0 || key_changed || time_changed {
        x.open("attributes", &[]);
        if m == 0 {
            x.text("divisions", &[], &div.per_quarter.to_string());
        }
        if key_changed {
            write_key(x, bar.key);
        }
        if time_changed {
            x.open("time", &[]);
            x.text("beats", &[], &bar.time_signature.numerator().to_string());
            x.text(
                "beat-type",
                &[],
                &bar.time_signature.denominator().to_string(),
            );
            x.close();
        }
        if m == 0 {
            write_tab_staff(x, track);
        }
        x.close();
    }

    // Song-level directions in the first part only: marker and tempo changes.
    if part == 0 {
        if let Some(marker) = &bar.marker {
            x.open("direction", &[("placement", "above")]);
            x.open("direction-type", &[]);
            x.text("rehearsal", &[], marker);
            x.close();
            x.close();
        }
        for t in score.tempo_map().iter().filter(|t| t.bar == m) {
            let (bpm, rounded) = decimal(t.tempo.bpm());
            if rounded {
                report.tempo_rounded += 1;
            }
            x.open("direction", &[("placement", "above")]);
            x.open("direction-type", &[]);
            x.open("metronome", &[]);
            x.text("beat-unit", &[], "quarter");
            x.text("per-minute", &[], &bpm);
            x.close();
            x.close();
            let offset = div.of(t.offset)?;
            if offset > 0 {
                x.text("offset", &[("sound", "yes")], &offset.to_string());
            }
            x.empty("sound", &[("tempo", &bpm)]);
            x.close();
        }
    }

    // Voices.
    let content = track.bar(m);
    let length = measure_length(score, m, bar)?;
    let mut written = 0i64; // duration written in the current voice, for <backup>
    let mut any_voice = false;
    let mut overfull = false;
    let voices: &[Voice] = content.map_or(&[], |c| c.voices.as_slice());
    for (v, voice) in voices.iter().enumerate() {
        if v > 0 && voice.beats.is_empty() {
            continue;
        }
        if any_voice && written > 0 {
            x.open("backup", &[]);
            x.text("duration", &[], &written.to_string());
            x.close();
        }
        any_voice = true;
        written = 0;
        let voice_number = (v + 1).to_string();
        if voice.beats.is_empty() {
            let d = div.of(length)?;
            if d > 0 {
                x.open("note", &[]);
                x.empty("rest", &[("measure", "yes")]);
                x.text("duration", &[], &d.to_string());
                x.text("voice", &[], &voice_number);
                x.close();
                written = d;
            }
            continue;
        }
        let (marks, loose) = brackets(&voice.beats)?;
        report.tuplets_without_bracket += loose;
        for (k, beat) in voice.beats.iter().enumerate() {
            let next = next_beat(track, m, v, k);
            let d = div.of(beat.value.length())?;
            write_beat(
                x,
                track,
                part,
                m,
                bar.key,
                beat,
                d,
                &voice_number,
                marks[k],
                next,
                report,
            )?;
            written += d;
        }
        let used = voice_length(voice)?;
        if used < length {
            let pad = div.of(length.checked_sub(used).ok_or(OVERFLOW)?)?;
            x.open("note", &[]);
            x.empty("rest", &[]);
            x.text("duration", &[], &pad.to_string());
            x.text("voice", &[], &voice_number);
            x.close();
            written += pad;
            report.padded_voices += 1;
        } else if used > length && !bar.pickup {
            overfull = true;
        }
    }
    if overfull {
        report.overfull_measures += 1;
    }

    // Right barline: ending stop, repeat end.
    if bar.repeat_end.is_some() || ending_stops {
        x.open("barline", &[("location", "right")]);
        if bar.repeat_end.is_some() {
            x.text("bar-style", &[], "light-heavy");
        }
        if ending_stops {
            let kind = if bar.repeat_end.is_some() {
                "stop"
            } else {
                "discontinue"
            };
            x.empty("ending", &[("number", &ending_text), ("type", kind)]);
        }
        if let Some(times) = bar.repeat_end {
            x.empty(
                "repeat",
                &[("direction", "backward"), ("times", &times.to_string())],
            );
        }
        x.close();
    }
    x.close();
    Ok(())
}

fn write_key(x: &mut Xml, key: KeySignature) {
    x.open("key", &[]);
    x.text("fifths", &[], &key.fifths().to_string());
    let mode = match key.mode() {
        Mode::Major => "major",
        Mode::Minor => "minor",
    };
    x.text("mode", &[], mode);
    x.close();
}

/// TAB clef and staff details: one line per string, the open-string tuning from the
/// lowest line (the lowest string) up, and the capo.
fn write_tab_staff(x: &mut Xml, track: &Track) {
    let f = track.fretted();
    let strings = f.tuning.strings();
    x.open("clef", &[]);
    x.text("sign", &[], "TAB");
    x.text("line", &[], "5");
    x.close();
    x.open("staff-details", &[]);
    x.text("staff-lines", &[], &strings.len().to_string());
    // String 1 is the highest and is listed first; line 1 is the bottom line.
    for (line, pitch) in strings.iter().rev().enumerate() {
        let line = (line + 1).to_string();
        x.open("staff-tuning", &[("line", &line)]);
        x.text("tuning-step", &[], step(pitch.letter()));
        if pitch.alter().semitones() != 0 {
            x.text("tuning-alter", &[], &pitch.alter().semitones().to_string());
        }
        x.text("tuning-octave", &[], &pitch.octave().to_string());
        x.close();
    }
    if f.capo.0 > 0 {
        x.text("capo", &[], &f.capo.0.to_string());
    }
    x.close();
}

fn write_time_modification(x: &mut Xml, t: Tuplet) {
    x.open("time-modification", &[]);
    x.text("actual-notes", &[], &t.actual().to_string());
    x.text("normal-notes", &[], &t.normal().to_string());
    x.close();
}

fn write_tuplet_marks(x: &mut Xml, mark: Bracket) {
    match mark {
        Bracket::None => {}
        Bracket::Start => x.empty("tuplet", &[("type", "start")]),
        Bracket::Stop => x.empty("tuplet", &[("type", "stop")]),
        Bracket::Both => {
            x.empty("tuplet", &[("type", "start")]);
            x.empty("tuplet", &[("type", "stop")]);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn write_beat(
    x: &mut Xml,
    track: &Track,
    part: usize,
    m: usize,
    key: KeySignature,
    beat: &Beat,
    duration: i64,
    voice: &str,
    mark: Bracket,
    next: Option<&Beat>,
    report: &mut MusicXmlReport,
) -> Res<()> {
    let value = beat.value;
    let d = duration.to_string();
    let common = |x: &mut Xml| {
        x.text("voice", &[], voice);
        x.text("type", &[], type_name(value));
        for _ in 0..value.dots() {
            x.empty("dot", &[]);
        }
        if let Some(t) = value.tuplet() {
            write_time_modification(x, t);
        }
    };
    if beat.notes.is_empty() {
        x.open("note", &[]);
        x.empty("rest", &[]);
        x.text("duration", &[], &d);
        common(x);
        if mark != Bracket::None {
            x.open("notations", &[]);
            write_tuplet_marks(x, mark);
            x.close();
        }
        x.close();
        return Ok(());
    }
    let f = track.fretted();
    for (i, note) in beat.notes.iter().enumerate() {
        let position = note.position();
        let semitone = pitch_at(&f.tuning, f.capo, position).ok_or(MusicXmlError::NoPitch {
            track: part,
            bar: m,
        })?;
        let pitch = key.spell(semitone);
        let tie_start = starts_tie(note, next);
        x.open("note", &[]);
        if i > 0 {
            x.empty("chord", &[]);
        }
        x.open("pitch", &[]);
        x.text("step", &[], step(pitch.letter()));
        if pitch.alter().semitones() != 0 {
            x.text("alter", &[], &pitch.alter().semitones().to_string());
        }
        x.text("octave", &[], &pitch.octave().to_string());
        x.close();
        x.text("duration", &[], &d);
        if note.tie {
            x.empty("tie", &[("type", "stop")]);
        }
        if tie_start {
            x.empty("tie", &[("type", "start")]);
        }
        common(x);
        if note.effects.dead {
            x.text("notehead", &[], "x");
        }
        x.open("notations", &[]);
        if note.tie {
            x.empty("tied", &[("type", "stop")]);
        }
        if tie_start {
            x.empty("tied", &[("type", "start")]);
        }
        if note.effects.let_ring {
            x.empty("tied", &[("type", "let-ring")]);
        }
        if i == 0 {
            write_tuplet_marks(x, mark);
        }
        x.open("technical", &[]);
        x.text("string", &[], &position.string.get().to_string());
        x.text("fret", &[], &position.fret.0.to_string());
        x.close();
        x.close();
        x.close();
        report.notes += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: i64) -> Rational {
        Rational::new(n, d).unwrap()
    }

    #[test]
    fn tempo_decimals_are_exact_or_rounded() {
        assert_eq!(decimal(r(120, 1)), ("120".into(), false));
        assert_eq!(decimal(r(241, 2)), ("120.5".into(), false));
        assert_eq!(decimal(r(193, 8)), ("24.125".into(), false));
        assert_eq!(decimal(r(100, 3)), ("33.333".into(), true));
        assert_eq!(decimal(r(200, 3)), ("66.667".into(), true));
    }

    #[test]
    fn divisions_cover_dots_and_tuplets() {
        let mut d = Divisions::new();
        d.include(r(3, 8)).unwrap(); // dotted quarter: 1.5 quarters
        d.include(r(1, 12)).unwrap(); // triplet eighth: 1/3 quarter
        assert_eq!(d.per_quarter, 6);
        assert_eq!(d.of(r(1, 12)).unwrap(), 2);
        assert_eq!(d.of(r(3, 8)).unwrap(), 9);
    }
}
