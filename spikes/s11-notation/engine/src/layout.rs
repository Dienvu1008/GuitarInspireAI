//! The spike's engine: one bar plan per bar for every staff of every track (README
//! 4.3), duration spacing, greedy line breaking with justification, then emission of
//! a notation staff (treble 8vb) and a TAB staff per track.
//!
//! Engraving rules used (general practice, not copied from any engraver):
//! - stem direction by the middle-line rule; one direction per beam group; with two
//!   voices in a bar, voice 1 up and voice 2 down;
//! - stems 3.5 staff spaces from the outermost head, at least 2.5 when beamed;
//! - a second in a chord puts the upper head right of an up stem, the lower head left
//!   of a down stem;
//! - accidentals in columns left of the heads, top to bottom, closest column first;
//! - dots in a space; ledger lines beyond the head by `legerLineExtension`;
//! - beams by beat (a quarter, or a dotted quarter in compound meters), flat.

use crate::model::*;
use crate::pitch::{self, AccidentalState};
use crate::smufl::Font;
use mc_score::{Beat, MasterBar, Score, Track};
use mc_theory::fretted::pitch_at;
use mc_theory::{Rational, SpelledPitch};
use std::collections::{BTreeMap, BTreeSet};

thread_local! {
    /// Only for the "no-bar-reset" breakage: the state carried from the previous bar.
    static CARRY: std::cell::RefCell<Option<AccidentalState>> = const { std::cell::RefCell::new(None) };
}

const STEM: f32 = 3.5;
const STEM_BEAMED_MIN: f32 = 2.5;
const ACC_CLEAR: f32 = 0.2;
const ACC_COLUMN_GAP: f32 = 0.12;
const DOT_GAP: f32 = 0.35;
const DOT_STEP: f32 = 0.5;
const TAB_STRING_GAP: f32 = 1.5;
const TAB_BELOW: f32 = 1.5;
/// Above the first string: room for half a fret number's knockout (0.59 sp) and a gap.
const TAB_ABOVE: f32 = 1.0;
const TRACK_GAP: f32 = 2.0;
const SYSTEM_GAP: f32 = 3.0;
const FRET_SIZE: f32 = 1.3;
const KEY_GAP: f32 = 0.15;
const TUPLET_GAP: f32 = 0.5;
/// Clearance between the notation content and the staff box edge, and the minimum
/// box margin above the top line and below the bottom line, in staff spaces.
const BOX_PAD: f32 = 1.0;
const BOX_MIN: f32 = 2.5;

/// Deliberate breakages, to show the checks catch real faults (S11_BREAK=name).
fn broken(name: &str) -> bool {
    std::env::var("S11_BREAK").is_ok_and(|v| v == name)
}

fn f(r: Rational) -> f32 {
    (r.numerator() as f64 / r.denominator() as f64) as f32
}

/// One note before placement, with its spelling and accidental decided.
#[derive(Clone, Debug)]
struct PNote {
    string: u8,
    fret: u8,
    written: SpelledPitch,
    position: i32,
    accidental: Option<i8>,
    tied: bool,
    displaced: bool,
    acc_column: usize,
}

#[derive(Clone, Debug)]
struct PBeat {
    voice: usize,
    beat: usize,
    onset: Rational,
    exponent: u8,
    dots: u8,
    tuplet: Option<u8>,
    notes: Vec<PNote>,
    stem_up: bool,
    group: Option<usize>,
}

/// A tuplet: its beats (indices into `beats`), the number, the side (the stem side of
/// its first note) and the number's baseline in staff spaces above the bottom line.
#[derive(Clone, Debug)]
struct PTuplet {
    members: Vec<usize>,
    digit: u8,
    up: bool,
    baseline: f32,
}

/// One track's prepared bar.
#[derive(Clone, Debug)]
struct TrackBar {
    beats: Vec<PBeat>,
    voices: usize,
    /// Beam groups: beats (indices into `beats`) and direction.
    groups: Vec<(Vec<usize>, bool)>,
    tuplets: Vec<PTuplet>,
    /// How far the notation content reaches above the top line and below the bottom
    /// line, in staff spaces (a per-bar estimate; N3 checks the result stays inside
    /// the staff box).
    above: f32,
    below: f32,
}

/// One column of a bar: an onset shared by every staff of every track.
#[derive(Clone, Copy, Debug)]
struct Column {
    onset: Rational,
    /// Natural flexible width (scaled by justification).
    flex: f32,
    /// Space reserved left of the anchor, and right of it (not scaled).
    left: f32,
    right: f32,
}

#[derive(Clone, Debug)]
struct BarPlan {
    tracks: Vec<TrackBar>,
    columns: Vec<Column>,
    fixed_left: f32,
    fixed_right: f32,
    show_time: bool,
    show_key: bool,
    /// The part of `fixed_left` reserved for the key change (0 without one).
    key_reserve: f32,
}

impl BarPlan {
    fn natural(&self) -> f32 {
        self.fixed_left
            + self.fixed_right
            + self
                .columns
                .iter()
                .map(|c| c.left + c.right + c.flex)
                .sum::<f32>()
            + if self.columns.is_empty() { 6.0 } else { 0.0 }
    }
}

/// Cached bar plans (opaque outside the engine).
#[derive(Clone)]
pub struct Plans(Vec<BarPlan>);

pub struct Engine<'a> {
    pub score: &'a Score,
    pub font: &'a Font,
    pub cfg: &'a Config,
    tracks: Vec<&'a Track>,
}

impl<'a> Engine<'a> {
    pub fn new(score: &'a Score, font: &'a Font, cfg: &'a Config) -> Self {
        Engine {
            score,
            font,
            cfg,
            tracks: score.tracks().collect(),
        }
    }

    fn sp(&self) -> f32 {
        self.cfg.staff_space
    }

    // ------------------------------------------------------------------ preparation

    /// Spelling, positions, accidentals, stems, beam groups and chord displacement of
    /// one track's bar (README 4.1, 4.2).
    fn prepare(&self, t: usize, b: usize) -> TrackBar {
        let track = self.tracks[t];
        let master = &self.score.master_bars()[b];
        let key = master.key;
        let inst = track.fretted();
        let content = track.bar(b).expect("H-02");
        let mut beats = Vec::new();
        for (v, voice) in content.voices.iter().enumerate() {
            let mut onset = Rational::ZERO;
            for (k, beat) in voice.beats.iter().enumerate() {
                beats.push(self.pbeat(t, b, v, k, onset, beat, inst));
                onset = onset.checked_add(beat.value.length()).unwrap_or(onset);
            }
        }
        // Accidentals: onset order across the staff's voices, then low to high.
        let mut order: Vec<(Rational, usize, usize)> = Vec::new();
        for (i, pb) in beats.iter().enumerate() {
            for (j, _) in pb.notes.iter().enumerate() {
                order.push((pb.onset, i, j));
            }
        }
        order.sort_by(|a, c| {
            a.0.cmp(&c.0)
                .then(beats[a.1].notes[a.2].position.cmp(&beats[c.1].notes[c.2].position))
        });
        let mut state = if broken("no-bar-reset") {
            CARRY.with(|c| c.borrow().clone()).filter(|_| b > 0).unwrap_or_else(|| AccidentalState::new_bar(key))
        } else {
            AccidentalState::new_bar(key)
        };
        for (_, i, j) in order {
            let n = &beats[i].notes[j];
            let across = n.tied && beats[i].beat == 0;
            let acc = state.note(n.written, n.tied, across);
            beats[i].notes[j].accidental = acc;
        }
        if broken("no-bar-reset") {
            CARRY.with(|c| *c.borrow_mut() = Some(state.clone()));
        }
        let voices = content.voices.iter().filter(|v| !v.beats.is_empty()).count();
        // Beam groups by beat window.
        let ts = master.time_signature;
        let (num, den) = (ts.numerator() as i64, ts.denominator() as i64);
        let window = if den >= 8 && num > 3 && num % 3 == 0 {
            Rational::new(3, 8).unwrap()
        } else {
            Rational::new(1, 4).unwrap()
        };
        // Tuplets: consecutive beats of a voice with the same tuplet, `actual` beats
        // each (the spike handles equal-value tuplets only).
        let mut tuplets: Vec<PTuplet> = Vec::new();
        let mut tuplet_of: BTreeMap<usize, usize> = BTreeMap::new();
        for v in 0..content.voices.len() {
            let mut run: Vec<usize> = Vec::new();
            let mut digit = 0u8;
            let mut flush = |run: &mut Vec<usize>, digit: u8, tuplets: &mut Vec<PTuplet>| {
                if !run.is_empty() {
                    for i in run.iter() {
                        tuplet_of.insert(*i, tuplets.len());
                    }
                    tuplets.push(PTuplet { members: std::mem::take(run), digit, up: true, baseline: 0.0 });
                }
            };
            for i in (0..beats.len()).filter(|i| beats[*i].voice == v) {
                match beats[i].tuplet {
                    Some(a) if run.is_empty() || a == digit => {
                        digit = a;
                        run.push(i);
                    }
                    other => {
                        flush(&mut run, digit, &mut tuplets);
                        if let Some(a) = other {
                            digit = a;
                            run.push(i);
                        }
                    }
                }
                if !run.is_empty() && run.len() == digit as usize {
                    flush(&mut run, digit, &mut tuplets);
                }
            }
            flush(&mut run, digit, &mut tuplets);
        }
        let mut groups: Vec<(Vec<usize>, bool)> = Vec::new();
        for v in 0..content.voices.len() {
            let mut current: Vec<usize> = Vec::new();
            let mut current_win: i64 = -1;
            let idx: Vec<usize> = (0..beats.len()).filter(|i| beats[*i].voice == v).collect();
            for &i in &idx {
                let pb = &beats[i];
                // A tuplet is its own beam unit; other beats group by beat window.
                let win = match tuplet_of.get(&i) {
                    Some(r) if !broken("tuplet-by-window") => -10 - *r as i64,
                    _ => pb.onset.checked_div(window).map(|q| q.numerator() / q.denominator()).unwrap_or(0),
                };
                let beamable = pb.exponent >= 3 && !pb.notes.is_empty();
                if beamable && win == current_win {
                    current.push(i);
                } else {
                    if current.len() > 1 {
                        groups.push((std::mem::take(&mut current), true));
                    }
                    current.clear();
                    if beamable {
                        current.push(i);
                        current_win = win;
                    } else {
                        current_win = -1;
                    }
                }
            }
            if current.len() > 1 {
                groups.push((current, true));
            }
        }
        // Stem directions.
        let dir_of = |notes: &[PNote]| -> bool {
            let hi = notes.iter().map(|n| n.position).max().unwrap_or(4);
            let lo = notes.iter().map(|n| n.position).min().unwrap_or(4);
            // Middle line is position 4: the note farther from it decides; a note on the
            // middle line, or a tie in distance, takes a down stem.
            if broken("stem-rule") { (hi - 4).abs() <= (4 - lo).abs() } else { (4 - lo) > (hi - 4) }
        };
        for pb in beats.iter_mut() {
            pb.stem_up = if voices > 1 { pb.voice == 0 } else { dir_of(&pb.notes) };
        }
        for (gi, (members, up)) in groups.iter_mut().enumerate() {
            let all: Vec<PNote> = members.iter().flat_map(|i| beats[*i].notes.clone()).collect();
            *up = if voices > 1 { beats[members[0]].voice == 0 } else { dir_of(&all) };
            for &i in members.iter() {
                beats[i].stem_up = *up;
                beats[i].group = Some(gi);
            }
        }
        // Seconds and accidental columns.
        for pb in beats.iter_mut() {
            pb.notes.sort_by_key(|n| n.position);
            let up = pb.stem_up;
            let n = if broken("no-seconds") { 0 } else { pb.notes.len() };
            if up {
                for j in 1..n {
                    let prev = pb.notes[j - 1].position;
                    if pb.notes[j].position - prev == 1 && !pb.notes[j - 1].displaced {
                        pb.notes[j].displaced = true;
                    }
                }
            } else {
                for j in (0..n.saturating_sub(1)).rev() {
                    let next = pb.notes[j + 1].position;
                    if next - pb.notes[j].position == 1 && !pb.notes[j + 1].displaced {
                        pb.notes[j].displaced = true;
                    }
                }
            }
            // Accidental columns, top to bottom: the closest column whose accidentals
            // are at least 6 positions (3 spaces) away, else a new column.
            let mut columns: Vec<Vec<i32>> = Vec::new();
            let mut tops: Vec<usize> = (0..n).filter(|j| pb.notes[*j].accidental.is_some()).collect();
            tops.sort_by_key(|j| std::cmp::Reverse(pb.notes[*j].position));
            for j in tops {
                let p = pb.notes[j].position;
                let c = columns
                    .iter()
                    .position(|col| col.iter().all(|q| (q - p).abs() >= 6))
                    .unwrap_or_else(|| {
                        columns.push(Vec::new());
                        columns.len() - 1
                    });
                columns[c].push(p);
                pb.notes[j].acc_column = c;
            }
        }
        let mut tuplets = tuplets;
        let (above, below) = self.extent(&beats, &mut tuplets, voices);
        TrackBar {
            beats,
            voices,
            groups,
            tuplets,
            above,
            below,
        }
    }

    /// Vertical reach of a bar's notation, in staff spaces above the top line and below
    /// the bottom line. The same rules as the emission, applied to positions only.
    /// Also places the tuplet numbers: TUPLET_GAP beyond the farthest reach of their
    /// beats on the stem side.
    fn extent(&self, beats: &[PBeat], tuplets: &mut [PTuplet], voices: usize) -> (f32, f32) {
        let font = self.font;
        // u: staff spaces above the bottom line.
        let (mut top, mut bottom) = (4.0f32, 0.0f32);
        let mut push = |a: f32, b: f32| {
            top = top.max(a);
            bottom = bottom.min(b);
        };
        if beats.is_empty() {
            let b = font.bbox("restWhole");
            push(3.0 + b.ne.1, 3.0 + b.sw.1);
        }
        // Per beat: the reach on the stem side (for a tuplet number) and overall.
        let mut reach: Vec<(f32, f32)> = Vec::with_capacity(beats.len());
        for pb in beats {
            if pb.notes.is_empty() {
                let b = font.bbox(rest_glyph(pb.exponent));
                let base = if pb.exponent == 0 { 6 } else { 4 };
                let shift = if voices > 1 { if pb.voice == 0 { 4 } else { -4 } } else { 0 };
                let u = (base + shift) as f32 / 2.0;
                push(u + b.ne.1, u + b.sw.1);
                reach.push((u + b.ne.1, u + b.sw.1));
                continue;
            }
            let hi = pb.notes.iter().map(|n| n.position).max().unwrap() as f32 / 2.0;
            let lo = pb.notes.iter().map(|n| n.position).min().unwrap() as f32 / 2.0;
            let hb = font.bbox(head_glyph(pb.exponent));
            let (mut a, mut b) = (hi + hb.ne.1, lo + hb.sw.1);
            for n in &pb.notes {
                if let Some(acc) = n.accidental {
                    let ab = font.bbox(acc_glyph(acc));
                    let u = n.position as f32 / 2.0;
                    a = a.max(u + ab.ne.1);
                    b = b.min(u + ab.sw.1);
                }
            }
            if pb.notes.iter().any(|n| n.tied) {
                // A tie sits 0.6 sp off the head and bulges 0.7 sp away from it.
                a = a.max(hi + 1.4);
                b = b.min(lo - 1.4);
            }
            if pb.exponent >= 1 {
                if pb.stem_up {
                    let end = (hi + STEM).max(2.0);
                    a = a.max(end);
                    if pb.group.is_none() && pb.exponent >= 3 {
                        let fb = font.bbox(flag_glyph(pb.exponent, true));
                        a = a.max(end + fb.ne.1);
                        b = b.min(end + fb.sw.1);
                    }
                } else {
                    let end = (lo - STEM).min(2.0);
                    b = b.min(end);
                    if pb.group.is_none() && pb.exponent >= 3 {
                        let fb = font.bbox(flag_glyph(pb.exponent, false));
                        a = a.max(end + fb.ne.1);
                        b = b.min(end + fb.sw.1);
                    }
                }
            }
            push(a, b);
            reach.push((a, b));
        }
        for t in tuplets.iter_mut() {
            t.up = t.members.iter().map(|i| &beats[*i]).find(|pb| !pb.notes.is_empty()).is_none_or(|pb| pb.stem_up);
            let g = font.bbox(tuplet_glyph(t.digit));
            // Without the gap (a breakage), the number sits on the farthest item.
            let gap = if broken("tuplet-touches") { -0.5 } else { TUPLET_GAP };
            if t.up {
                let a = t.members.iter().map(|i| reach[*i].0).fold(4.0f32, f32::max);
                t.baseline = a + gap - g.sw.1;
                push(t.baseline + g.ne.1, 0.0);
            } else {
                let b = t.members.iter().map(|i| reach[*i].1).fold(0.0f32, f32::min);
                t.baseline = b - gap - g.ne.1;
                push(4.0, t.baseline + g.sw.1);
            }
        }
        (top - 4.0, -bottom)
    }

    #[allow(clippy::too_many_arguments)]
    fn pbeat(
        &self,
        t: usize,
        b: usize,
        v: usize,
        k: usize,
        onset: Rational,
        beat: &Beat,
        inst: &mc_score::FrettedInstrument,
    ) -> PBeat {
        let key = self.score.master_bars()[b].key;
        let notes = beat
            .notes
            .iter()
            .map(|n| {
                let pos = n.position();
                let sounding = pitch_at(&inst.tuning, inst.capo, pos).expect("valid string");
                // A tied note keeps the spelling of the note it continues (4.2 rule 3).
                let spelled = if n.tie {
                    self.tie_source(t, b, v, k, pos.string.get())
                        .filter(|s| s.semitone() == sounding)
                        .unwrap_or_else(|| pitch::spell(sounding, key))
                } else {
                    pitch::spell(sounding, key)
                };
                let written = pitch::written(spelled, self.cfg.staves.transposition);
                PNote {
                    string: pos.string.get(),
                    fret: pos.fret.0,
                    written,
                    position: pitch::treble_position(written),
                    accidental: None,
                    tied: n.tie,
                    displaced: false,
                    acc_column: 0,
                }
            })
            .collect();
        PBeat {
            voice: v,
            beat: k,
            onset,
            exponent: beat.value.base().exponent(),
            dots: beat.value.dots(),
            tuplet: beat.value.tuplet().map(|t| t.actual()),
            notes,
            stem_up: true,
            group: None,
        }
    }

    /// Sounding spelling of the note a tie continues: the same string in the previous
    /// beat of the voice (H-06), following the chain back.
    fn tie_source(&self, t: usize, b: usize, v: usize, k: usize, string: u8) -> Option<SpelledPitch> {
        let track = self.tracks[t];
        let (pb, pk) = if k > 0 {
            (b, k - 1)
        } else {
            let pb = b.checked_sub(1)?;
            let len = track.bar(pb)?.voices.get(v)?.beats.len();
            (pb, len.checked_sub(1)?)
        };
        let beat = track.bar(pb)?.voices.get(v)?.beats.get(pk)?;
        let note = beat.notes.iter().find(|n| n.position().string.get() == string)?;
        let inst = track.fretted();
        let sounding = pitch_at(&inst.tuning, inst.capo, note.position())?;
        if note.tie {
            if let Some(s) = self.tie_source(t, pb, v, pk, string) {
                if s.semitone() == sounding {
                    return Some(s);
                }
            }
        }
        Some(pitch::spell(sounding, self.score.master_bars()[pb].key))
    }

    // ------------------------------------------------------------------ bar plans

    fn head_width(&self, exponent: u8) -> f32 {
        self.font.bbox(head_glyph(exponent)).width()
    }

    fn acc_width(&self, alter: i8) -> f32 {
        self.font.bbox(acc_glyph(alter)).width()
    }

    fn plan(&self, b: usize) -> BarPlan {
        let master = &self.score.master_bars()[b];
        let prev: Option<&MasterBar> = b.checked_sub(1).map(|p| &self.score.master_bars()[p]);
        let tracks: Vec<TrackBar> = (0..self.tracks.len()).map(|t| self.prepare(t, b)).collect();
        let mut onsets: BTreeSet<Rational> = BTreeSet::new();
        for tb in &tracks {
            for pb in &tb.beats {
                onsets.insert(pb.onset);
            }
        }
        let onsets: Vec<Rational> = onsets.into_iter().collect();
        let bar_end = master.time_signature.length();
        let content_end = tracks
            .iter()
            .flat_map(|tb| tb.beats.iter())
            .map(|pb| pb.onset)
            .max()
            .unwrap_or(Rational::ZERO);
        let ext = self.font.leger_extension;
        let columns = onsets
            .iter()
            .enumerate()
            .map(|(i, &o)| {
                let next = onsets.get(i + 1).copied().unwrap_or(if bar_end > content_end {
                    bar_end
                } else {
                    content_end.checked_add(Rational::new(1, 4).unwrap()).unwrap()
                });
                let gap = f(next.checked_sub(o).unwrap_or(Rational::new(1, 4).unwrap())).max(1.0 / 64.0);
                let flex = 2.4 * (1.0 + 0.45 * (gap * 16.0).log2()).max(0.55);
                // Reserves from every staff's content at this onset.
                let mut left: f32 = 0.0;
                let mut right: f32 = 0.0;
                for tb in &tracks {
                    for pb in tb.beats.iter().filter(|pb| pb.onset == o) {
                        let hw = if pb.notes.is_empty() {
                            self.font.bbox(rest_glyph(pb.exponent)).width()
                        } else {
                            self.head_width(pb.exponent)
                        };
                        let mut l = hw / 2.0;
                        let mut r = hw / 2.0;
                        let ledgers = pb.notes.iter().any(|n| n.position <= -2 || n.position >= 10);
                        if ledgers {
                            l = l.max(hw / 2.0 + ext);
                            r = r.max(hw / 2.0 + ext);
                        }
                        if pb.notes.iter().any(|n| n.displaced) {
                            if pb.stem_up {
                                r += hw;
                            } else {
                                l += hw;
                            }
                        }
                        let mut acc_cols: BTreeMap<usize, f32> = BTreeMap::new();
                        for n in &pb.notes {
                            if let Some(a) = n.accidental {
                                let w = acc_cols.entry(n.acc_column).or_insert(0.0);
                                *w = w.max(self.acc_width(a));
                            }
                        }
                        if !acc_cols.is_empty() {
                            l += ACC_CLEAR + acc_cols.values().sum::<f32>() + ACC_COLUMN_GAP * (acc_cols.len() as f32 - 1.0);
                        }
                        if pb.dots > 0 {
                            r += DOT_GAP + DOT_STEP * pb.dots as f32;
                        }
                        if pb.group.is_none() && pb.exponent >= 3 && !pb.notes.is_empty() && pb.stem_up {
                            r = r.max(hw / 2.0 + self.font.bbox(flag_glyph(pb.exponent, true)).width());
                        }
                        // TAB fret numbers.
                        for n in &pb.notes {
                            let w = fret_width(n.fret, n.tied) / 2.0;
                            l = l.max(w);
                            r = r.max(w);
                        }
                        left = left.max(l);
                        right = right.max(r);
                    }
                }
                Column {
                    onset: o,
                    flex,
                    left: left + 0.25,
                    right: right + 0.25,
                }
            })
            .collect();
        let show_time = prev.is_none_or(|p| p.time_signature != master.time_signature);
        let show_key = prev.is_some_and(|p| p.key != master.key);
        let mut fixed_left = 1.0;
        if show_time {
            fixed_left += self.time_width(master) + 1.0;
        }
        let key_reserve = if show_key { self.key_width(master) + 0.8 } else { 0.0 };
        fixed_left += key_reserve;
        BarPlan {
            tracks,
            columns,
            fixed_left,
            fixed_right: 0.8,
            show_time,
            show_key,
            key_reserve,
        }
    }

    fn time_width(&self, m: &MasterBar) -> f32 {
        let digits = |v: u8| -> f32 {
            v.to_string()
                .chars()
                .map(|c| self.font.bbox(time_digit(c)).width())
                .sum()
        };
        digits(m.time_signature.numerator()).max(digits(m.time_signature.denominator()))
    }

    fn key_width(&self, m: &MasterBar) -> f32 {
        let f = m.key.fifths();
        let glyph = if f > 0 { "accidentalSharp" } else { "accidentalFlat" };
        f.unsigned_abs() as f32 * (self.font.bbox(glyph).width() + KEY_GAP)
    }

    /// A key change in a system's first bar is shown by the header, so the bar neither
    /// draws it nor keeps its space (the cached plan still reserves it).
    fn header_key_saving(&self, pi: usize, p: &BarPlan) -> f32 {
        if pi == 0 && p.show_key && !broken("double-key") {
            p.key_reserve
        } else {
            0.0
        }
    }

    fn header_width(&self, first_bar: usize) -> f32 {
        let m = &self.score.master_bars()[first_bar];
        let clef = self.font.bbox("gClef8vb").width();
        1.0 + clef + 1.0 + self.key_width(m) + if m.key.fifths() != 0 { 0.8 } else { 0.0 }
    }

    // ------------------------------------------------------------------ public

    /// The whole layout (`systems` = None) or only the given systems (the in-view
    /// path; line breaking still covers the whole score, as ADR-031's outline does).
    pub fn layout(&self, systems: Option<std::ops::Range<usize>>) -> Layout {
        self.layout_with(&self.plans(), systems)
    }

    /// Every bar's plan (the cacheable part of the layout).
    pub fn plans(&self) -> Plans {
        Plans((0..self.score.master_bar_count()).map(|b| self.plan(b)).collect())
    }

    /// After an edit of `bar`: its plan and the next bar's (which reads this bar for
    /// key and time display and for tie spelling) are rebuilt; the others are kept.
    pub fn replan(&self, plans: &mut Plans, bar: usize) {
        for b in bar..(bar + 2).min(plans.0.len()) {
            plans.0[b] = self.plan(b);
        }
    }

    /// Line breaking, stacks and emission from cached plans.
    pub fn layout_with(&self, plans: &Plans, systems: Option<std::ops::Range<usize>>) -> Layout {
        let sp = self.sp();
        let count = plans.0.len();
        let plans = &plans.0;
        // Line breaking in staff spaces.
        let width_sp = self.cfg.width / sp;
        let mut breaks: Vec<(usize, usize)> = Vec::new();
        let mut start = 0;
        while start < count {
            let available = width_sp - self.header_width(start);
            let mut end = start + 1;
            let mut used = plans[start].natural();
            while end < count && used + plans[end].natural() <= available {
                used += plans[end].natural();
                end += 1;
            }
            breaks.push((start, end));
            start = end;
        }
        // Every system's staff stack comes from its bars' plans (cheap), so the y of a
        // system in view is known without emitting the systems before it.
        let stacks: Vec<Vec<(StaffKind, usize, f32, f32)>> = breaks.iter().map(|&(a, z)| self.staff_stack(&plans[a..z])).collect();
        let heights: Vec<f32> = stacks
            .iter()
            .map(|st| st.iter().map(|s| s.3).sum::<f32>() * sp + TRACK_GAP * sp * (self.tracks.len().saturating_sub(1)) as f32)
            .collect();
        let wanted = systems.unwrap_or(0..breaks.len());
        let mut out = Vec::new();
        let mut y = 0.0f32;
        for (i, &(a, z)) in breaks.iter().enumerate() {
            if wanted.contains(&i) {
                out.push(self.emit_system(i, y, heights[i], a, &stacks[i], &plans[a..z], i + 1 == breaks.len()));
            }
            y += heights[i] + SYSTEM_GAP * sp;
        }
        let height = (y - SYSTEM_GAP * sp).max(0.0);
        Layout {
            font: format!("{} {}", self.font.name, self.font.version),
            config: self.cfg.clone(),
            width: self.cfg.width,
            height,
            systems: out,
        }
    }

    /// (kind, track, lines-top offset, height) of each staff in a system, in staff
    /// spaces, top to bottom.
    /// The notation box follows the content of the system's bars (BOX_PAD beyond the
    /// farthest item, at least BOX_MIN); the "no-headroom" breakage uses BOX_MIN only.
    fn staff_stack(&self, plans: &[BarPlan]) -> Vec<(StaffKind, usize, f32, f32)> {
        let mut v = Vec::new();
        for (t, track) in self.tracks.iter().enumerate() {
            let fixed = broken("no-headroom");
            // The header's clef (on the second line, position 2) reaches below the staff
            // with its 8.
            let clef = self.font.bbox("gClef8vb");
            let above = plans.iter().map(|p| p.tracks[t].above).fold((1.0 + clef.ne.1 - 4.0).max(0.0), f32::max);
            let below = plans.iter().map(|p| p.tracks[t].below).fold((-(1.0 + clef.sw.1)).max(0.0), f32::max);
            let above = if fixed { BOX_MIN } else { (above + BOX_PAD).max(BOX_MIN) };
            let below = if fixed { BOX_MIN } else { (below + BOX_PAD).max(BOX_MIN) };
            for kind in self.cfg.staves.kinds() {
                match kind {
                    StaffKind::Notation => v.push((kind, t, above, above + 4.0 + below)),
                    StaffKind::Tab => {
                        let strings = track.fretted().tuning.string_count() as f32;
                        v.push((kind, t, TAB_ABOVE, TAB_ABOVE + (strings - 1.0) * TAB_STRING_GAP + TAB_BELOW + 1.0))
                    }
                }
            }
        }
        v
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_system(
        &self,
        index: usize,
        y: f32,
        height: f32,
        first: usize,
        stack: &[(StaffKind, usize, f32, f32)],
        plans: &[BarPlan],
        last: bool,
    ) -> System {
        let sp = self.sp();
        let header = self.header_width(first);
        let available = self.cfg.width / sp - header;
        let fixed: f32 = plans
            .iter()
            .enumerate()
            .map(|(pi, p)| p.fixed_left - self.header_key_saving(pi, p) + p.fixed_right + p.columns.iter().map(|c| c.left + c.right).sum::<f32>()
                + if p.columns.is_empty() { 6.0 } else { 0.0 })
            .sum();
        let flex: f32 = plans.iter().map(|p| p.columns.iter().map(|c| c.flex).sum::<f32>()).sum();
        let mut scale = if flex > 0.0 { (available - fixed) / flex } else { 1.0 };
        if last && scale > 4.0 / 3.0 {
            scale = 1.0;
        }
        // Bar x and column anchors, shared by every staff (README 4.3).
        let mut bars: Vec<(usize, f32, f32, Vec<f32>)> = Vec::new(); // bar, x, width, anchors
        let mut x = header;
        for (pi, p) in plans.iter().enumerate() {
            let bx = x;
            let mut cx = x + p.fixed_left - self.header_key_saving(pi, p);
            let mut anchors = Vec::new();
            for c in &p.columns {
                anchors.push(cx + c.left);
                cx += c.left + c.right + c.flex * scale;
            }
            if p.columns.is_empty() {
                cx += 6.0;
            }
            cx += p.fixed_right;
            bars.push((plans.len(), bx, cx - bx, anchors));
            x = cx;
        }
        let mut staves = Vec::new();
        let mut top = 0.0f32;
        let mut prev_track = None;
        for (si, &(kind, t, lines_top, h)) in stack.iter().enumerate() {
            if prev_track.is_some_and(|p| p != t) {
                top += TRACK_GAP;
            }
            prev_track = Some(t);
            let staff_index = stack[..si].iter().filter(|s| s.1 == t).count();
            let pos = StaffPosition {
                track: t,
                staff: staff_index,
                top: top * sp,
                height: h * sp,
                lines_top: lines_top * sp,
            };
            let staff = match kind {
                StaffKind::Notation => self.emit_notation(pos, first, plans, &bars),
                StaffKind::Tab => self.emit_tab(pos, first, plans, &bars),
            };
            staves.push(staff);
            top += h;
        }
        System {
            index,
            y,
            height,
            first_bar: first,
            end_bar: first + plans.len(),
            staves,
        }
    }

    fn new_staff(kind: StaffKind, pos: StaffPosition) -> Staff {
        Staff {
            kind,
            position: pos,
            lines: Vec::new(),
            glyphs: Vec::new(),
            texts: Vec::new(),
            beams: Vec::new(),
            ties: Vec::new(),
            barlines: Vec::new(),
            anchors: Vec::new(),
            notes: Vec::new(),
            stems: Vec::new(),
            rests: Vec::new(),
            beam_facts: Vec::new(),
            tuplets: Vec::new(),
            bar_rests: Vec::new(),
        }
    }

    // ------------------------------------------------------------------ notation

    fn emit_notation(&self, pos: StaffPosition, first: usize, plans: &[BarPlan], bars: &[(usize, f32, f32, Vec<f32>)]) -> Staff {
        let sp = self.sp();
        let font = self.font;
        let mut st = Self::new_staff(StaffKind::Notation, pos);
        let top = pos.lines_top;
        let bottom = top + 4.0 * sp;
        let y_of = |p: i32| bottom - p as f32 * sp / 2.0;
        let end_x = bars.last().map(|b| (b.1 + b.2) * sp).unwrap_or(0.0);
        for l in 0..5 {
            let ly = top + l as f32 * sp;
            st.lines.push(Line { kind: LineKind::StaffLine, x1: 0.0, y1: ly, x2: end_x, y2: ly, thickness: font.staff_line * sp });
        }
        // Header: clef and key signature.
        st.glyphs.push(Glyph { name: "gClef8vb", x: 1.0 * sp, y: y_of(2) });
        let master = &self.score.master_bars()[first];
        let kx = (1.0 + font.bbox("gClef8vb").width() + 1.0) * sp;
        self.emit_key(&mut st, master.key.fifths(), kx, &y_of);
        let t = pos.track;
        for (pi, p) in plans.iter().enumerate() {
            let (_, bx, bw, anchors) = &bars[pi];
            let bar = first + pi;
            let m = &self.score.master_bars()[bar];
            let mut fx = bx + 1.0;
            if p.show_key && (pi > 0 || broken("double-key")) {
                self.emit_key(&mut st, m.key.fifths(), fx * sp, &y_of);
                fx += self.key_width(m) + 0.8;
            }
            if p.show_time {
                self.emit_time(&mut st, m, fx * sp, &y_of);
            }
            // The bar line's right edge on the bar's end, so the last one stays on the page.
            let bar_end = (bx + bw - font.thin_barline / 2.0) * sp;
            st.lines.push(Line { kind: LineKind::Barline, x1: bar_end, y1: top, x2: bar_end, y2: bottom, thickness: font.thin_barline * sp });
            st.barlines.push(bar_end);
            let tb = &p.tracks[t];
            if tb.beats.is_empty() && !broken("no-bar-rest") {
                // A whole-bar rest, hanging from the fourth line, centred in the bar's
                // space after its key and time signature.
                let b = font.bbox("restWhole");
                let cx = (bx + p.fixed_left + bx + bw - p.fixed_right) / 2.0 * sp;
                let x = cx - (b.sw.0 + b.width() / 2.0) * sp;
                let y = y_of(6);
                st.glyphs.push(Glyph { name: "restWhole", x, y });
                let rect = Rect { x: x + b.sw.0 * sp, y: y - b.ne.1 * sp, w: b.width() * sp, h: (b.ne.1 - b.sw.1) * sp };
                st.bar_rests.push(RestFact { bar, voice: 0, beat: 0, rect });
            }
            let anchor_of = |onset: Rational| -> f32 {
                let c = p.columns.iter().position(|c| c.onset == onset).expect("column");
                anchors[c] * sp
            };
            let mut stem_ends: BTreeMap<usize, (f32, f32)> = BTreeMap::new(); // beat idx -> (x, natural end y)
            for (i, pb) in tb.beats.iter().enumerate() {
                let ax = anchor_of(pb.onset);
                st.anchors.push(Anchor { bar, voice: pb.voice, beat: pb.beat, x: ax });
                if pb.notes.is_empty() {
                    self.emit_rest(&mut st, bar, pb, ax, tb.voices, &y_of);
                    continue;
                }
                let hw = self.head_width(pb.exponent) * sp;
                let head = head_glyph(pb.exponent);
                let up = pb.stem_up;
                // Stem x: right edge for up stems, left edge for down stems.
                let normal_left = ax - hw / 2.0;
                let mut accidental_x: BTreeMap<usize, f32> = BTreeMap::new();
                let mut acc_widths: BTreeMap<usize, f32> = BTreeMap::new();
                for n in &pb.notes {
                    if let Some(a) = n.accidental {
                        let w = acc_widths.entry(n.acc_column).or_insert(0.0);
                        *w = w.max(self.acc_width(a) * sp);
                    }
                }
                let leftmost_head = if !up && pb.notes.iter().any(|n| n.displaced) { normal_left - hw } else { normal_left };
                let clear = if broken("accidental-touches") { 0.0 } else { ACC_CLEAR };
                let mut ax_cursor = leftmost_head - clear * sp;
                for (c, w) in &acc_widths {
                    ax_cursor -= w;
                    accidental_x.insert(*c, ax_cursor);
                    ax_cursor -= ACC_COLUMN_GAP * sp;
                }
                for n in &pb.notes {
                    let hx = if n.displaced { if up { normal_left + hw } else { normal_left - hw } } else { normal_left };
                    let hy = y_of(n.position);
                    st.glyphs.push(Glyph { name: head, x: hx, y: hy });
                    let hb = font.bbox(head);
                    let head_rect = Rect { x: hx + hb.sw.0 * sp, y: hy - hb.ne.1 * sp, w: hb.width() * sp, h: (hb.ne.1 - hb.sw.1) * sp };
                    let mut acc_box = None;
                    if let Some(a) = n.accidental {
                        let g = acc_glyph(a);
                        let gx = accidental_x[&n.acc_column];
                        st.glyphs.push(Glyph { name: g, x: gx, y: hy });
                        let b = font.bbox(g);
                        acc_box = Some(Rect { x: gx + b.sw.0 * sp, y: hy - b.ne.1 * sp, w: b.width() * sp, h: (b.ne.1 - b.sw.1) * sp });
                    }
                    // Ledger lines.
                    let mut ledgers = Vec::new();
                    let ext = if broken("short-ledger") { 0.1 } else { font.leger_extension };
                    let (lx1, lx2) = (hx - ext * sp, hx + hw + ext * sp);
                    let mut lp = -2;
                    while lp >= n.position {
                        ledgers.push(Line { kind: LineKind::Ledger, x1: lx1, y1: y_of(lp), x2: lx2, y2: y_of(lp), thickness: font.leger_line * sp });
                        lp -= 2;
                    }
                    let mut lp = 10;
                    while lp <= n.position {
                        ledgers.push(Line { kind: LineKind::Ledger, x1: lx1, y1: y_of(lp), x2: lx2, y2: y_of(lp), thickness: font.leger_line * sp });
                        lp += 2;
                    }
                    st.lines.extend(ledgers.iter().copied());
                    // Dots: right of the rightmost head, in a space.
                    let mut dots = Vec::new();
                    let rightmost = if up && pb.notes.iter().any(|m| m.displaced) { normal_left + 2.0 * hw } else { normal_left + hw };
                    for d in 0..pb.dots {
                        let dy = if n.position.rem_euclid(2) == 0 && !broken("dot-on-line") { y_of(n.position + 1) } else { hy };
                        let dx = rightmost + (DOT_GAP + d as f32 * DOT_STEP) * sp;
                        st.glyphs.push(Glyph { name: "augmentationDot", x: dx, y: dy });
                        dots.push((dx, dy));
                    }
                    st.notes.push(NoteFact {
                        bar,
                        voice: pb.voice,
                        beat: pb.beat,
                        string: n.string,
                        written: (pitch::letter_index(n.written.letter()), n.written.alter().semitones(), n.written.octave()),
                        position: n.position,
                        accidental: n.accidental,
                        tied: n.tied,
                        head: head_rect,
                        displaced: n.displaced,
                        accidental_box: acc_box,
                        stem_up: (pb.exponent >= 1).then_some(up),
                        ledgers,
                        dots,
                    });
                    if n.tied {
                        self.emit_tie_into(&mut st, bar, pb, n, hx, hy, up);
                    }
                }
                if pb.exponent >= 1 {
                    let hi = pb.notes.iter().map(|n| n.position).max().unwrap();
                    let lo = pb.notes.iter().map(|n| n.position).min().unwrap();
                    let (sx, start_y, end_y) = if up {
                        let (axr, ayr) = font.anchor(head, "stemUpSE");
                        (normal_left + axr * sp, y_of(lo) - ayr * sp, y_of(hi) - STEM * sp)
                    } else {
                        let (axl, ayl) = font.anchor(head, "stemDownNW");
                        (normal_left + axl * sp, y_of(hi) - ayl * sp, y_of(lo) + STEM * sp)
                    };
                    // The stem reaches at least the middle line.
                    let end_y = if up { end_y.min(y_of(4)) } else { end_y.max(y_of(4)) };
                    stem_ends.insert(i, (sx, end_y));
                    let _ = start_y;
                }
            }
            // Beams (flat), then stems to the beam; flags for unbeamed short notes.
            let mut beam_y: BTreeMap<usize, f32> = BTreeMap::new();
            for (gi, (members, up)) in tb.groups.iter().enumerate() {
                let ends: Vec<f32> = members.iter().filter_map(|i| stem_ends.get(i).map(|e| e.1)).collect();
                if ends.is_empty() {
                    continue;
                }
                let y = if *up { ends.iter().cloned().fold(f32::MAX, f32::min) } else { ends.iter().cloned().fold(f32::MIN, f32::max) };
                // Keep beamed stems at least STEM_BEAMED_MIN from their outer heads.
                let mut y = y;
                for &i in members {
                    let pb = &tb.beats[i];
                    let hi = pb.notes.iter().map(|n| n.position).max().unwrap();
                    let lo = pb.notes.iter().map(|n| n.position).min().unwrap();
                    if *up {
                        y = y.min(y_of(hi) - STEM_BEAMED_MIN * sp);
                    } else {
                        y = y.max(y_of(lo) + STEM_BEAMED_MIN * sp);
                    }
                }
                beam_y.insert(gi, y);
                let levels_of = |i: usize| tb.beats[i].exponent.saturating_sub(2) as usize;
                let thick = font.beam * sp;
                let step = (font.beam + font.beam_spacing) * sp;
                let first_x = stem_ends[&members[0]].0;
                let last_x = stem_ends[members.last().unwrap()].0;
                let dir = if *up { 1.0 } else { -1.0 };
                let band_y = y + dir * thick / 2.0;
                st.beams.push(Beam { x1: first_x, y1: band_y, x2: last_x, y2: band_y, thickness: thick });
                // Higher levels: runs of members that have the level; a lone member gets
                // a stub toward its neighbour.
                let max_level = members.iter().map(|i| levels_of(*i)).max().unwrap_or(1);
                for level in 2..=max_level {
                    let ly = band_y + dir * step * (level as f32 - 1.0);
                    let mut run: Vec<usize> = Vec::new();
                    let flush = |run: &mut Vec<usize>, st: &mut Staff| {
                        if run.is_empty() {
                            return;
                        }
                        let x1 = stem_ends[&run[0]].0;
                        let x2 = stem_ends[run.last().unwrap()].0;
                        let (x1, x2) = if run.len() == 1 {
                            let stub = 1.1 * sp;
                            let pos = members.iter().position(|m| *m == run[0]).unwrap();
                            if pos + 1 < members.len() { (x1, x1 + stub) } else { (x1 - stub, x1) }
                        } else {
                            (x1, x2)
                        };
                        st.beams.push(Beam { x1, y1: ly, x2, y2: ly, thickness: thick });
                        run.clear();
                    };
                    for &i in members {
                        if levels_of(i) >= level {
                            run.push(i);
                        } else {
                            flush(&mut run, &mut st);
                        }
                    }
                    flush(&mut run, &mut st);
                }
                st.beam_facts.push(BeamFact {
                    bar,
                    voice: tb.beats[members[0]].voice,
                    up: *up,
                    beats: members.iter().map(|i| tb.beats[*i].beat).collect(),
                    band: Rect {
                        x: first_x,
                        y: if *up { y } else { y - thick * max_level as f32 - font.beam_spacing * sp * (max_level as f32 - 1.0) },
                        w: last_x - first_x,
                        h: thick * max_level as f32 + font.beam_spacing * sp * (max_level as f32 - 1.0),
                    },
                });
            }
            for (i, pb) in tb.beats.iter().enumerate() {
                let Some(&(sx, natural_end)) = stem_ends.get(&i) else { continue };
                let up = pb.stem_up;
                let end = pb.group.and_then(|g| beam_y.get(&g).copied()).unwrap_or(natural_end);
                let hi = pb.notes.iter().map(|n| n.position).max().unwrap();
                let lo = pb.notes.iter().map(|n| n.position).min().unwrap();
                let head = head_glyph(pb.exponent);
                let start = if up {
                    y_of(lo) - font.anchor(head, "stemUpSE").1 * sp
                } else {
                    y_of(hi) - font.anchor(head, "stemDownNW").1 * sp
                };
                st.lines.push(Line { kind: LineKind::Stem, x1: sx, y1: start, x2: sx, y2: end, thickness: font.stem * sp });
                let outer = if up { y_of(lo) } else { y_of(hi) };
                st.stems.push(StemFact { bar, voice: pb.voice, beat: pb.beat, up, x: sx, length_from_outer: (end - outer).abs() / sp, beamed: pb.group.is_some() });
                if pb.group.is_none() && pb.exponent >= 3 {
                    let g = flag_glyph(pb.exponent, up);
                    st.glyphs.push(Glyph { name: g, x: sx - if up { 0.0 } else { 0.0 }, y: end });
                }
            }
            // Tuplet numbers, centred over the outer heads' anchors, at the baseline the
            // plan decided.
            for tp in &tb.tuplets {
                if broken("no-tuplet-number") {
                    break;
                }
                let g = tuplet_glyph(tp.digit);
                let b = font.bbox(g);
                let xs: Vec<f32> = tp.members.iter().map(|i| anchor_of(tb.beats[*i].onset)).collect();
                let cx = (xs[0] + xs[xs.len() - 1]) / 2.0;
                let x = cx - (b.sw.0 + b.width() / 2.0) * sp;
                let y = bottom - tp.baseline * sp;
                st.glyphs.push(Glyph { name: g, x, y });
                st.tuplets.push(TupletFact {
                    bar,
                    voice: tb.beats[tp.members[0]].voice,
                    beats: tp.members.iter().map(|i| tb.beats[*i].beat).collect(),
                    number: tp.digit,
                    rect: Rect { x: x + b.sw.0 * sp, y: y - b.ne.1 * sp, w: b.width() * sp, h: (b.ne.1 - b.sw.1) * sp },
                });
            }
        }
        let _ = t;
        st
    }

    fn emit_key(&self, st: &mut Staff, fifths: i8, x: f32, y_of: &dyn Fn(i32) -> f32) {
        const SHARPS: [i32; 7] = [8, 5, 9, 6, 3, 7, 4];
        const FLATS: [i32; 7] = [4, 7, 3, 6, 2, 5, 1];
        let sp = self.sp();
        let (glyph, positions) = if fifths > 0 { ("accidentalSharp", &SHARPS) } else { ("accidentalFlat", &FLATS) };
        // Each sign one glyph width plus a small gap from the previous one.
        let step = self.font.bbox(glyph).width() + KEY_GAP;
        for i in 0..fifths.unsigned_abs() as usize {
            st.glyphs.push(Glyph { name: glyph, x: x + i as f32 * step * sp, y: y_of(positions[i]) });
        }
    }

    fn emit_time(&self, st: &mut Staff, m: &MasterBar, x: f32, y_of: &dyn Fn(i32) -> f32) {
        let sp = self.sp();
        let w = self.time_width(m);
        for (value, position) in [(m.time_signature.numerator(), 6), (m.time_signature.denominator(), 2)] {
            let s = value.to_string();
            let sw: f32 = s.chars().map(|c| self.font.bbox(time_digit(c)).width()).sum();
            let mut cx = x + (w - sw) / 2.0 * sp;
            for c in s.chars() {
                st.glyphs.push(Glyph { name: time_digit(c), x: cx, y: y_of(position) });
                cx += self.font.bbox(time_digit(c)).width() * sp;
            }
        }
    }

    fn emit_rest(&self, st: &mut Staff, bar: usize, pb: &PBeat, ax: f32, voices: usize, y_of: &dyn Fn(i32) -> f32) {
        let sp = self.sp();
        let g = rest_glyph(pb.exponent);
        let b = self.font.bbox(g);
        // Standard positions: the whole rest hangs from the fourth line, the half rest
        // sits on the middle line, the others are centred on it. With two voices,
        // voice 1 moves up and voice 2 down by two spaces.
        let base = match pb.exponent {
            0 => 6,
            _ => 4,
        };
        let shift = if voices > 1 { if pb.voice == 0 { 4 } else { -4 } } else { 0 };
        let y = y_of(base + shift);
        let x = ax - b.width() * sp / 2.0 - b.sw.0 * sp;
        st.glyphs.push(Glyph { name: g, x, y });
        let rect = Rect { x: x + b.sw.0 * sp, y: y - b.ne.1 * sp, w: b.width() * sp, h: (b.ne.1 - b.sw.1) * sp };
        st.rests.push(RestFact { bar, voice: pb.voice, beat: pb.beat, rect });
        for d in 0..pb.dots {
            let dx = rect.right() + (DOT_GAP + d as f32 * DOT_STEP) * sp;
            let pos = base + shift + 1;
            st.glyphs.push(Glyph { name: "augmentationDot", x: dx, y: y_of(pos) });
        }
    }

    /// A tie into this note from the previous head on the same staff position (in this
    /// system); at a system start, a short tie from the left edge.
    #[allow(clippy::too_many_arguments)]
    fn emit_tie_into(&self, st: &mut Staff, bar: usize, pb: &PBeat, n: &PNote, hx: f32, hy: f32, up: bool) {
        let sp = self.sp();
        let source = st
            .notes
            .iter()
            .rev()
            .find(|f| f.voice == pb.voice && f.string == n.string && (f.bar < bar || f.beat < pb.beat) && f.position == n.position);
        let x1 = source.map(|s| s.head.right() + 0.1 * sp).unwrap_or(hx - 2.0 * sp);
        let y = if up { hy + 0.6 * sp } else { hy - 0.6 * sp };
        st.ties.push(Tie { x1, x2: hx - 0.1 * sp, y, up: !up });
    }

    // ------------------------------------------------------------------ TAB

    fn emit_tab(&self, pos: StaffPosition, first: usize, plans: &[BarPlan], bars: &[(usize, f32, f32, Vec<f32>)]) -> Staff {
        let sp = self.sp();
        let mut st = Self::new_staff(StaffKind::Tab, pos);
        let track = self.tracks[pos.track];
        let strings = track.fretted().tuning.string_count();
        let top = pos.lines_top;
        let y_of = |s: u8| top + (s as f32 - 1.0) * TAB_STRING_GAP * sp;
        let bottom = y_of(strings);
        let end_x = bars.last().map(|b| (b.1 + b.2) * sp).unwrap_or(0.0);
        for s in 1..=strings {
            st.lines.push(Line { kind: LineKind::TabString, x1: 0.0, y1: y_of(s), x2: end_x, y2: y_of(s), thickness: self.font.staff_line * sp });
        }
        st.texts.push(text("TAB".into(), 2.0 * sp, (top + bottom) / 2.0, 1.4 * sp));
        for (pi, p) in plans.iter().enumerate() {
            let (_, bx, bw, anchors) = &bars[pi];
            let bar = first + pi;
            let bar_end = (bx + bw - self.font.thin_barline / 2.0) * sp;
            st.lines.push(Line { kind: LineKind::Barline, x1: bar_end, y1: top, x2: bar_end, y2: bottom, thickness: self.font.thin_barline * sp });
            st.barlines.push(bar_end);
            let tb = &p.tracks[pos.track];
            for pb in &tb.beats {
                let c = p.columns.iter().position(|c| c.onset == pb.onset).expect("column");
                let ax = anchors[c] * sp;
                st.anchors.push(Anchor { bar, voice: pb.voice, beat: pb.beat, x: ax });
                for n in &pb.notes {
                    let text_of = if n.tied { format!("({})", n.fret) } else { n.fret.to_string() };
                    st.texts.push(text(text_of, ax, y_of(n.string), FRET_SIZE * sp));
                }
            }
        }
        st
    }
}

/// A TAB text centred at (x, y) with the white box that clears the line behind it
/// (monospace digits at 0.6 em, 0.9 em tall).
fn text(text: String, x: f32, y: f32, size: f32) -> Text {
    let w = text.chars().count() as f32 * size * 0.6;
    let knockout = Some(Rect { x: x - w / 2.0, y: y - size * 0.45, w, h: size * 0.9 });
    Text { text, x, y, size, knockout }
}

pub fn tuplet_glyph(d: u8) -> &'static str {
    ["tuplet0", "tuplet1", "tuplet2", "tuplet3", "tuplet4", "tuplet5", "tuplet6", "tuplet7", "tuplet8", "tuplet9"][(d % 10) as usize]
}

pub fn head_glyph(exponent: u8) -> &'static str {
    match exponent {
        0 => "noteheadWhole",
        1 => "noteheadHalf",
        _ => "noteheadBlack",
    }
}

pub fn acc_glyph(alter: i8) -> &'static str {
    match alter {
        1 => "accidentalSharp",
        -1 => "accidentalFlat",
        _ => "accidentalNatural",
    }
}

pub fn rest_glyph(exponent: u8) -> &'static str {
    match exponent {
        0 => "restWhole",
        1 => "restHalf",
        2 => "restQuarter",
        3 => "rest8th",
        4 => "rest16th",
        5 => "rest32nd",
        _ => "rest64th",
    }
}

pub fn flag_glyph(exponent: u8, up: bool) -> &'static str {
    match (exponent, up) {
        (3, true) => "flag8thUp",
        (3, false) => "flag8thDown",
        (4, true) => "flag16thUp",
        (4, false) => "flag16thDown",
        (5, true) => "flag32ndUp",
        (5, false) => "flag32ndDown",
        (_, true) => "flag64thUp",
        (_, false) => "flag64thDown",
    }
}

pub fn time_digit(c: char) -> &'static str {
    match c {
        '0' => "timeSig0",
        '1' => "timeSig1",
        '2' => "timeSig2",
        '3' => "timeSig3",
        '4' => "timeSig4",
        '5' => "timeSig5",
        '6' => "timeSig6",
        '7' => "timeSig7",
        '8' => "timeSig8",
        _ => "timeSig9",
    }
}

/// Width of a TAB fret number in staff spaces (monospace digits, 0.55 em at FRET_SIZE).
pub fn fret_width(fret: u8, tied: bool) -> f32 {
    let chars = fret.to_string().len() + if tied { 2 } else { 0 };
    chars as f32 * 0.6 * FRET_SIZE + 0.3
}
