//! Layout of the notation staff (ADR-038, ADR-041). The input is the semantic layer's
//! notated beats; this module decides only geometry: staff positions, stem direction,
//! beams, columns of accidentals, dots, rests, ties and tuplet numbers.
//!
//! Units: every distance is in **staff spaces** (named constants below) and is turned
//! into layout units by multiplying by `LayoutMetrics::staff_space()` (one place per
//! value). Staff positions count half staff spaces above the bottom line; the written
//! E4 is position 0 and the top line position 8 (treble staff). y grows downward in
//! stave coordinates: `y(p) = bottom_line_y - p / 2 * staff_space`.
//!
//! Engraving rules (general practice, as in the S11 spike; SMuFL for glyph data):
//! - stems: middle-line rule (the note farther from the middle line decides; a tie
//!   goes down); one direction per beam group; two voices: voice 1 up, voice 2 down;
//! - stem length 3.5 staff spaces from the outermost head, at least 2.5 when beamed,
//!   and at least to the middle line;
//! - a second in a chord puts the upper head right of an up stem, the lower head left
//!   of a down stem;
//! - accidentals in columns left of the heads, from the top, the closest column whose
//!   accidentals are at least 3 staff spaces away;
//! - dots in a space; ledger lines beyond the head by the font's leger extension;
//! - beams by beat window (a dotted quarter in compound meters), a tuplet beamed as
//!   one unit; beams are flat;
//! - tuplet number on the stem side, clear of the beats it spans;
//! - a whole-bar rest in a bar without beats.

use crate::display::*;
use crate::semantic::NotatedBeat;
use crate::smufl::{Anchor, DEFAULTS, SmuflGlyph};
use mc_score::MasterBar;
use mc_theory::Alter;
use mc_theory::notation::TieRole;
use std::collections::BTreeMap;

// ------------------------------------------------------------------ constants (sp)

/// Position of the written E4 (the bottom line of the treble staff) as a diatonic
/// step (C4 = 28).
const BOTTOM_LINE_STEP: i32 = 30;
/// Position of the middle line.
const MIDDLE_LINE: i32 = 4;
/// Position of the top line.
const TOP_LINE: i32 = 8;
/// Unbeamed stem length from the outermost head.
pub(crate) const STEM_SP: f32 = 3.5;
/// Shortest stem in a beam group.
pub(crate) const STEM_BEAMED_MIN_SP: f32 = 2.5;
/// Gap between an accidental and the heads.
pub(crate) const ACCIDENTAL_CLEARANCE_SP: f32 = 0.2;
/// Gap between two accidental columns.
const ACCIDENTAL_COLUMN_GAP_SP: f32 = 0.12;
/// Accidentals in one column are at least this many positions (3 staff spaces) apart.
const ACCIDENTAL_COLUMN_SPAN: i32 = 6;
/// Gap from the rightmost head to the first dot, and from dot to dot.
const DOT_GAP_SP: f32 = 0.35;
const DOT_STEP_SP: f32 = 0.5;
/// Space kept on each side of a column's content.
const COLUMN_PAD_SP: f32 = 0.25;
/// Tie: offset of its ends from the head centre, and its height.
const TIE_OFFSET_SP: f32 = 0.6;
const TIE_HEIGHT_SP: f32 = 0.7;
/// Tie at a system start: starts this far left of the head.
const TIE_LEAD_SP: f32 = 2.0;
/// Gap between a tuplet number and the farthest item of its beats.
pub(crate) const TUPLET_GAP_SP: f32 = 0.5;
/// Beam stub length for a lone shorter value.
const BEAM_STUB_SP: f32 = 1.1;
/// Two voices: rests move this many positions up (voice 1) or down (voice 2).
const REST_VOICE_SHIFT: i32 = 4;
/// Two voices: least gap between a rest and a head of the other voice.
const REST_CLEARANCE_SP: f32 = 0.25;
/// Half the height of a notehead (SMuFL noteheads are one staff space tall).
const HEAD_HALF_SP: f32 = 0.5;
/// Pad between the notation content and the staff box, and the least box margin.
pub(crate) const BOX_PAD_SP: f32 = 1.0;
pub(crate) const BOX_MIN_SP: f32 = 2.5;
/// Header: space before the clef, between clef and key signature, after the key.
const HEADER_BEFORE_CLEF_SP: f32 = 1.0;
const HEADER_AFTER_CLEF_SP: f32 = 1.0;
const KEY_AFTER_SP: f32 = 0.8;
/// Gap between key-signature signs.
const KEY_SIGN_GAP_SP: f32 = 0.15;
/// Space before a key change inside a bar.
const KEY_BEFORE_SP: f32 = 0.5;
/// Space around a time signature (both sides together).
const TIME_SIGNATURE_PAD_SP: f32 = 1.0;
/// Clef position: on the second line (G4 written, position 2).
const CLEF_POSITION: i32 = 2;
/// Time signature positions: numerator around the fourth line, denominator around the
/// second.
const TIME_TOP_POSITION: i32 = 6;
const TIME_BOTTOM_POSITION: i32 = 2;
/// Repeat dots in the two middle spaces.
const REPEAT_DOT_POSITIONS: [i32; 2] = [3, 5];
/// Key-signature staff positions on the treble staff, in the order the signs are
/// written (an engraving convention: where signs are drawn, not which letters).
const KEY_SHARP_POSITIONS: [i32; 7] = [8, 5, 9, 6, 3, 7, 4];
const KEY_FLAT_POSITIONS: [i32; 7] = [4, 7, 3, 6, 2, 5, 1];

/// Staff position of a written pitch.
pub(crate) fn staff_position(written: mc_theory::SpelledPitch) -> i32 {
    written.diatonic_step() - BOTTOM_LINE_STEP
}

fn glyph_width(g: SmuflGlyph) -> f32 {
    g.metrics().bbox.width()
}

// ------------------------------------------------------------------ plan

#[derive(Clone, Debug)]
pub(crate) struct PNote {
    pub string: u8,
    pub written: mc_theory::SpelledPitch,
    pub position: i32,
    pub accidental: Option<Alter>,
    pub tie: TieRole,
    pub displaced: bool,
    pub acc_column: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct PBeat {
    pub voice: usize,
    pub beat: usize,
    pub onset: mc_theory::Rational,
    pub exponent: u8,
    pub dots: u8,
    pub tuplet: Option<u8>,
    pub notes: Vec<PNote>,
    pub stem_up: bool,
    pub group: Option<usize>,
    /// Staff position of a rest (rests only).
    pub rest_position: i32,
}

#[derive(Clone, Debug)]
pub(crate) struct PTuplet {
    pub members: Vec<usize>,
    pub digit: u8,
    pub up: bool,
    /// Baseline in staff spaces above the bottom line.
    pub baseline: f32,
}

/// One track's notation in one bar, before horizontal placement.
#[derive(Clone, Debug)]
pub(crate) struct NotationPlan {
    pub beats: Vec<PBeat>,
    /// Beam groups: beats (indices into `beats`) and direction (up).
    pub groups: Vec<(Vec<usize>, bool)>,
    pub tuplets: Vec<PTuplet>,
    /// Reach above the top line and below the bottom line, in staff spaces.
    pub above: f32,
    pub below: f32,
}

/// Beam window: a dotted quarter in compound meters, else a quarter (as on TAB).
fn beam_window(master: &MasterBar) -> mc_theory::Rational {
    let ts = master.time_signature;
    let (n, d) = (ts.numerator(), ts.denominator());
    if d >= 8 && n > 3 && n % 3 == 0 {
        mc_theory::Rational::new(3, d as i64).expect("non-zero")
    } else {
        mc_theory::Rational::new(1, 4).expect("non-zero")
    }
}

fn window_index(onset: mc_theory::Rational, window: mc_theory::Rational) -> i64 {
    let q = onset
        .checked_div(window)
        .unwrap_or(mc_theory::Rational::ZERO);
    q.numerator().div_euclid(q.denominator())
}

/// Stem direction of a set of notes by the middle-line rule.
fn stem_up_for(positions: impl Iterator<Item = i32> + Clone) -> bool {
    let hi = positions.clone().max().unwrap_or(MIDDLE_LINE);
    let lo = positions.min().unwrap_or(MIDDLE_LINE);
    (MIDDLE_LINE - lo) > (hi - MIDDLE_LINE)
}

pub(crate) fn plan(notated: Vec<NotatedBeat>, master: &MasterBar) -> NotationPlan {
    let mut beats: Vec<PBeat> = notated
        .into_iter()
        .map(|b| PBeat {
            voice: b.voice,
            beat: b.beat,
            onset: b.onset,
            exponent: b.value.base().exponent(),
            dots: b.value.dots(),
            tuplet: b.value.tuplet().map(|t| t.actual()),
            notes: b
                .notes
                .into_iter()
                .map(|n| PNote {
                    string: n.string,
                    written: n.written,
                    position: staff_position(n.written),
                    accidental: n.accidental,
                    tie: n.tie,
                    displaced: false,
                    acc_column: 0,
                })
                .collect(),
            stem_up: true,
            group: None,
            rest_position: 0,
        })
        .collect();
    let voice_count = beats.iter().map(|b| b.voice + 1).max().unwrap_or(0);
    let voices = (0..voice_count)
        .filter(|v| beats.iter().any(|b| b.voice == *v))
        .count();

    // Tuplets: consecutive beats of a voice with the same number, `actual` beats each.
    let mut tuplets: Vec<PTuplet> = Vec::new();
    let mut tuplet_of: BTreeMap<usize, usize> = BTreeMap::new();
    for v in 0..voice_count {
        let mut run: Vec<usize> = Vec::new();
        let mut digit = 0u8;
        let mut flush = |run: &mut Vec<usize>, digit: u8, tuplets: &mut Vec<PTuplet>| {
            if !run.is_empty() {
                for i in run.iter() {
                    tuplet_of.insert(*i, tuplets.len());
                }
                tuplets.push(PTuplet {
                    members: std::mem::take(run),
                    digit,
                    up: true,
                    baseline: 0.0,
                });
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

    // Beam groups: beamable beats (eighth or shorter, with notes) in one window, or in
    // one tuplet.
    let window = beam_window(master);
    let mut groups: Vec<(Vec<usize>, bool)> = Vec::new();
    for v in 0..voice_count {
        let mut current: Vec<usize> = Vec::new();
        let mut current_win: Option<i64> = None;
        for i in (0..beats.len()).filter(|i| beats[*i].voice == v) {
            let pb = &beats[i];
            let win = match tuplet_of.get(&i) {
                Some(t) => -10 - *t as i64,
                None => window_index(pb.onset, window),
            };
            let beamable = pb.exponent >= 3 && !pb.notes.is_empty();
            if beamable && current_win == Some(win) {
                current.push(i);
            } else {
                if current.len() > 1 {
                    groups.push((std::mem::take(&mut current), true));
                }
                current.clear();
                current_win = None;
                if beamable {
                    current.push(i);
                    current_win = Some(win);
                }
            }
        }
        if current.len() > 1 {
            groups.push((current, true));
        }
    }

    // Stem directions.
    for pb in beats.iter_mut() {
        pb.stem_up = if voices > 1 {
            pb.voice == 0
        } else {
            stem_up_for(pb.notes.iter().map(|n| n.position))
        };
    }
    for (gi, (members, up)) in groups.iter_mut().enumerate() {
        *up = if voices > 1 {
            beats[members[0]].voice == 0
        } else {
            stem_up_for(
                members
                    .iter()
                    .flat_map(|i| beats[*i].notes.iter().map(|n| n.position)),
            )
        };
        for &i in members.iter() {
            beats[i].stem_up = *up;
            beats[i].group = Some(gi);
        }
    }

    // Seconds and accidental columns.
    for pb in beats.iter_mut() {
        pb.notes.sort_by_key(|n| (n.position, n.string));
        let n = pb.notes.len();
        if pb.stem_up {
            for j in 1..n {
                if pb.notes[j].position - pb.notes[j - 1].position == 1
                    && !pb.notes[j - 1].displaced
                {
                    pb.notes[j].displaced = true;
                }
            }
        } else {
            for j in (0..n.saturating_sub(1)).rev() {
                if pb.notes[j + 1].position - pb.notes[j].position == 1
                    && !pb.notes[j + 1].displaced
                {
                    pb.notes[j].displaced = true;
                }
            }
        }
        let mut columns: Vec<Vec<i32>> = Vec::new();
        let mut order: Vec<usize> = (0..n)
            .filter(|j| pb.notes[*j].accidental.is_some())
            .collect();
        order.sort_by_key(|j| std::cmp::Reverse(pb.notes[*j].position));
        for j in order {
            let p = pb.notes[j].position;
            let c = columns
                .iter()
                .position(|col| col.iter().all(|q| (q - p).abs() >= ACCIDENTAL_COLUMN_SPAN))
                .unwrap_or_else(|| {
                    columns.push(Vec::new());
                    columns.len() - 1
                });
            columns[c].push(p);
            pb.notes[j].acc_column = c;
        }
    }

    // Rests: the standard position, moved away from the other voice's heads at the
    // same onset, a staff space at a time (voice 1 up, voice 2 down).
    for i in 0..beats.len() {
        if !beats[i].notes.is_empty() {
            continue;
        }
        let pb = &beats[i];
        let mut p = rest_position(pb.exponent, pb.voice, voices);
        if voices > 1 {
            let b = SmuflGlyph::rest(pb.exponent).metrics().bbox;
            let others: Vec<i32> = beats
                .iter()
                .filter(|o| o.voice != pb.voice && o.onset == pb.onset)
                .flat_map(|o| o.notes.iter().map(|n| n.position))
                .collect();
            let up = pb.voice == 0;
            let clear = |p: i32| {
                let (lo, hi) = (p as f32 / 2.0 + b.sw.1, p as f32 / 2.0 + b.ne.1);
                others.iter().all(|q| {
                    let (hl, hh) = (
                        *q as f32 / 2.0 - HEAD_HALF_SP,
                        *q as f32 / 2.0 + HEAD_HALF_SP,
                    );
                    hi + REST_CLEARANCE_SP <= hl || lo - REST_CLEARANCE_SP >= hh
                })
            };
            while !clear(p) {
                p += if up { 2 } else { -2 };
            }
        }
        beats[i].rest_position = p;
    }

    let mut plan = NotationPlan {
        beats,
        groups,
        tuplets,
        above: 0.0,
        below: 0.0,
    };
    plan.extent();
    plan
}

/// Rest position (before the voice shift): the whole rest hangs from the fourth line,
/// the others sit on or centre on the middle line.
fn rest_position(exponent: u8, voice: usize, voices: usize) -> i32 {
    let base = if exponent == 0 { 6 } else { MIDDLE_LINE };
    let shift = if voices > 1 {
        if voice == 0 {
            REST_VOICE_SHIFT
        } else {
            -REST_VOICE_SHIFT
        }
    } else {
        0
    };
    base + shift
}

impl NotationPlan {
    /// Vertical reach (staff spaces) of the bar, and the tuplet baselines. Uses the same
    /// rules as the emission, on positions only.
    fn extent(&mut self) {
        let (mut top, mut bottom) = (4.0f32, 0.0f32);
        let mut push = |a: f32, b: f32| {
            top = top.max(a);
            bottom = bottom.min(b);
        };
        if self.beats.is_empty() {
            let b = SmuflGlyph::RestWhole.metrics().bbox;
            let u = 6.0 / 2.0;
            push(u + b.ne.1, u + b.sw.1);
        }
        let mut reach: Vec<(f32, f32)> = Vec::with_capacity(self.beats.len());
        for pb in &self.beats {
            if pb.notes.is_empty() {
                let b = SmuflGlyph::rest(pb.exponent).metrics().bbox;
                let u = pb.rest_position as f32 / 2.0;
                push(u + b.ne.1, u + b.sw.1);
                reach.push((u + b.ne.1, u + b.sw.1));
                continue;
            }
            let hi = pb.notes.iter().map(|n| n.position).max().unwrap() as f32 / 2.0;
            let lo = pb.notes.iter().map(|n| n.position).min().unwrap() as f32 / 2.0;
            let hb = SmuflGlyph::notehead(pb.exponent).metrics().bbox;
            let (mut a, mut b) = (hi + hb.ne.1, lo + hb.sw.1);
            for n in &pb.notes {
                if let Some(acc) = n.accidental {
                    let ab = SmuflGlyph::accidental(acc.semitones()).metrics().bbox;
                    let u = n.position as f32 / 2.0;
                    a = a.max(u + ab.ne.1);
                    b = b.min(u + ab.sw.1);
                }
            }
            if pb.notes.iter().any(|n| n.tie != TieRole::NotTied) {
                a = a.max(hi + TIE_OFFSET_SP + TIE_HEIGHT_SP + 0.1);
                b = b.min(lo - TIE_OFFSET_SP - TIE_HEIGHT_SP - 0.1);
            }
            if pb.exponent >= 1 {
                let middle = MIDDLE_LINE as f32 / 2.0;
                if pb.stem_up {
                    let end = (hi + STEM_SP).max(middle);
                    a = a.max(end);
                    if pb.group.is_none() && pb.exponent >= 3 {
                        let g = SmuflGlyph::flag(pb.exponent, true);
                        let fb = g.metrics().bbox;
                        let ay = g.metrics().anchor(Anchor::StemUpNW).expect("required").1;
                        a = a.max(end - ay + fb.ne.1);
                        b = b.min(end - ay + fb.sw.1);
                    }
                } else {
                    let end = (lo - STEM_SP).min(middle);
                    b = b.min(end);
                    if pb.group.is_none() && pb.exponent >= 3 {
                        let g = SmuflGlyph::flag(pb.exponent, false);
                        let fb = g.metrics().bbox;
                        let ay = g.metrics().anchor(Anchor::StemDownSW).expect("required").1;
                        a = a.max(end - ay + fb.ne.1);
                        b = b.min(end - ay + fb.sw.1);
                    }
                }
            }
            // Dots sit at most half a space above their head.
            if pb.dots > 0 {
                a = a.max(hi + 0.5 + 0.2);
            }
            push(a, b);
            reach.push((a, b));
        }
        for t in self.tuplets.iter_mut() {
            t.up = t
                .members
                .iter()
                .map(|i| &self.beats[*i])
                .find(|pb| !pb.notes.is_empty())
                .is_none_or(|pb| pb.stem_up);
            let g = SmuflGlyph::tuplet_digit(t.digit).metrics().bbox;
            // Everything under the number: the beats of any voice from the tuplet's
            // first onset to its last.
            let first = self.beats[t.members[0]].onset;
            let last = self.beats[*t.members.last().unwrap()].onset;
            let span: Vec<usize> = (0..self.beats.len())
                .filter(|i| (first..=last).contains(&self.beats[*i].onset))
                .collect();
            if t.up {
                let a = span.iter().map(|i| reach[*i].0).fold(4.0f32, f32::max);
                t.baseline = a + TUPLET_GAP_SP - g.sw.1;
                push(t.baseline + g.ne.1, 0.0);
            } else {
                let b = span.iter().map(|i| reach[*i].1).fold(0.0f32, f32::min);
                t.baseline = b - TUPLET_GAP_SP - g.ne.1;
                push(4.0, t.baseline + g.sw.1);
            }
        }
        self.above = top - 4.0;
        self.below = -bottom;
    }

    /// Half the width a beat's column needs, in staff spaces, so the content stays
    /// within the column when it is centred on the column's anchor (as TAB is).
    pub(crate) fn half_reserve(&self, i: usize) -> f32 {
        let pb = &self.beats[i];
        let ext = DEFAULTS.leger_line_extension;
        let hw = if pb.notes.is_empty() {
            glyph_width(SmuflGlyph::rest(pb.exponent))
        } else {
            glyph_width(SmuflGlyph::notehead(pb.exponent))
        };
        let mut l = hw / 2.0;
        let mut r = hw / 2.0;
        if pb
            .notes
            .iter()
            .any(|n| n.position <= -2 || n.position >= TOP_LINE + 2)
        {
            l += ext;
            r += ext;
        }
        if pb.notes.iter().any(|n| n.displaced) {
            if pb.stem_up {
                r += hw;
            } else {
                l += hw;
            }
        }
        let mut cols: BTreeMap<usize, f32> = BTreeMap::new();
        for n in &pb.notes {
            if let Some(a) = n.accidental {
                let w = cols.entry(n.acc_column).or_insert(0.0);
                *w = w.max(glyph_width(SmuflGlyph::accidental(a.semitones())));
            }
        }
        if !cols.is_empty() {
            l += ACCIDENTAL_CLEARANCE_SP
                + cols.values().sum::<f32>()
                + ACCIDENTAL_COLUMN_GAP_SP * (cols.len() as f32 - 1.0);
        }
        if pb.dots > 0 {
            r += DOT_GAP_SP + DOT_STEP_SP * pb.dots as f32;
        }
        if pb.group.is_none() && pb.exponent >= 3 && !pb.notes.is_empty() && pb.stem_up {
            r = r.max(hw + glyph_width(SmuflGlyph::flag(pb.exponent, true)) - hw / 2.0);
        }
        l.max(r) + COLUMN_PAD_SP
    }
}

// ------------------------------------------------------------------ widths (sp)

/// Width of a key signature of `fifths` signs.
pub(crate) fn key_width_sp(fifths: i8) -> f32 {
    let g = if fifths > 0 {
        SmuflGlyph::AccidentalSharp
    } else {
        SmuflGlyph::AccidentalFlat
    };
    fifths.unsigned_abs() as f32 * (glyph_width(g) + KEY_SIGN_GAP_SP)
}

/// Header width in staff spaces: clef and key signature.
pub(crate) fn header_sp(fifths: i8) -> f32 {
    HEADER_BEFORE_CLEF_SP
        + glyph_width(SmuflGlyph::GClef8vb)
        + HEADER_AFTER_CLEF_SP
        + key_width_sp(fifths)
        + if fifths != 0 { KEY_AFTER_SP } else { 0.0 }
}

/// Room for a key change inside a bar.
pub(crate) fn key_change_sp(fifths: i8) -> f32 {
    KEY_BEFORE_SP + key_width_sp(fifths) + KEY_AFTER_SP
}

fn digits_width(n: u8) -> f32 {
    n.to_string()
        .bytes()
        .map(|d| glyph_width(SmuflGlyph::time_digit(d - b'0')))
        .sum()
}

/// Width a notation time signature needs, padding included.
pub(crate) fn time_signature_sp(master: &MasterBar) -> f32 {
    let ts = master.time_signature;
    digits_width(ts.numerator()).max(digits_width(ts.denominator())) + TIME_SIGNATURE_PAD_SP
}

/// Reach of the header clef (above the top line, below the bottom line), staff spaces.
pub(crate) fn clef_reach() -> (f32, f32) {
    let b = SmuflGlyph::GClef8vb.metrics().bbox;
    let u = CLEF_POSITION as f32 / 2.0;
    ((u + b.ne.1 - 4.0).max(0.0), (-(u + b.sw.1)).max(0.0))
}

// ------------------------------------------------------------------ emission

/// Where one bar sits, shared with the TAB stave (computed in `layout`).
pub(crate) struct BarGeometry {
    pub bar: usize,
    pub content_start: f32,
    pub content_end: f32,
    /// Column start and width, by column index.
    pub columns: Vec<(f32, f32)>,
    /// Column centres (the beat anchors), by column index.
    pub centres: Vec<f32>,
    pub time_signature_centre: Option<f32>,
    /// x where a key change inside the bar starts.
    pub key_x: Option<f32>,
    /// Bar lines drawn, as on TAB: (x, thick).
    pub barlines: Vec<(f32, bool)>,
    /// Repeat dots centres.
    pub repeat_dots: Vec<f32>,
    pub last_in_system: bool,
}

/// One notation stave of one system.
pub(crate) struct Stave<'a> {
    pub sp: f32,
    pub lines_top: f32,
    pub header_x: f32,
    pub key: mc_theory::KeySignature,
    pub plans: Vec<(&'a NotationPlan, &'a [usize], BarGeometry, &'a MasterBar)>,
}

impl Stave<'_> {
    fn y_of(&self, p: i32) -> f32 {
        self.lines_top + (TOP_LINE - p) as f32 / 2.0 * self.sp
    }

    fn rect(&self, g: SmuflGlyph, x: f32, y: f32) -> Rect {
        let b = g.metrics().bbox;
        Rect {
            x: x + b.sw.0 * self.sp,
            y: y - b.ne.1 * self.sp,
            w: b.width() * self.sp,
            h: b.height() * self.sp,
        }
    }

    pub(crate) fn emit(&self, end_x: f32) -> NotationStaff {
        let sp = self.sp;
        let mut st = NotationStaff {
            staff_space: sp,
            lines_top: self.lines_top,
            ..NotationStaff::default()
        };
        for l in 0..5 {
            let y = self.lines_top + l as f32 * sp;
            st.lines.push(EngravedLine {
                kind: EngravedLineKind::StaffLine,
                x1: self.header_x,
                y1: y,
                x2: end_x,
                y2: y,
                thickness: DEFAULTS.staff_line_thickness * sp,
            });
        }
        // Header: clef and key signature.
        let clef_x = self.header_x + HEADER_BEFORE_CLEF_SP * sp;
        st.glyphs.push(MusicGlyph {
            glyph: SmuflGlyph::GClef8vb,
            x: clef_x,
            y: self.y_of(CLEF_POSITION),
        });
        let key_x = clef_x + (glyph_width(SmuflGlyph::GClef8vb) + HEADER_AFTER_CLEF_SP) * sp;
        self.emit_key(&mut st, self.key.fifths(), key_x);
        for (plan, cols, geo, master) in &self.plans {
            self.emit_bar(&mut st, plan, cols, geo, master);
        }
        st
    }

    fn emit_key(&self, st: &mut NotationStaff, fifths: i8, x: f32) {
        let (g, positions) = if fifths > 0 {
            (SmuflGlyph::AccidentalSharp, &KEY_SHARP_POSITIONS)
        } else {
            (SmuflGlyph::AccidentalFlat, &KEY_FLAT_POSITIONS)
        };
        let step = (glyph_width(g) + KEY_SIGN_GAP_SP) * self.sp;
        for (i, p) in positions
            .iter()
            .take(fifths.unsigned_abs() as usize)
            .enumerate()
        {
            st.glyphs.push(MusicGlyph {
                glyph: g,
                x: x + i as f32 * step,
                y: self.y_of(*p),
            });
            st.facts.key_signs += 1;
        }
    }

    fn emit_bar(
        &self,
        st: &mut NotationStaff,
        plan: &NotationPlan,
        cols: &[usize],
        geo: &BarGeometry,
        master: &MasterBar,
    ) {
        let sp = self.sp;
        let top = self.lines_top;
        let bottom = self.y_of(0);
        for &(x, thick) in &geo.barlines {
            st.lines.push(EngravedLine {
                kind: if thick {
                    EngravedLineKind::ThickBarLine
                } else {
                    EngravedLineKind::BarLine
                },
                x1: x,
                y1: top,
                x2: x,
                y2: bottom,
                thickness: if thick {
                    DEFAULTS.thick_barline_thickness
                } else {
                    DEFAULTS.thin_barline_thickness
                } * sp,
            });
            st.facts.barlines.push(x);
        }
        for &cx in &geo.repeat_dots {
            let b = SmuflGlyph::RepeatDot.metrics().bbox;
            for p in REPEAT_DOT_POSITIONS {
                st.glyphs.push(MusicGlyph {
                    glyph: SmuflGlyph::RepeatDot,
                    x: cx - (b.sw.0 + b.width() / 2.0) * sp,
                    y: self.y_of(p),
                });
            }
        }
        if let Some(kx) = geo.key_x {
            self.emit_key(st, master.key.fifths(), kx + KEY_BEFORE_SP * sp);
        }
        if let Some(cx) = geo.time_signature_centre {
            let ts = master.time_signature;
            for (value, p) in [
                (ts.numerator(), TIME_TOP_POSITION),
                (ts.denominator(), TIME_BOTTOM_POSITION),
            ] {
                let mut x = cx - digits_width(value) / 2.0 * sp;
                for d in value.to_string().bytes() {
                    let g = SmuflGlyph::time_digit(d - b'0');
                    st.glyphs.push(MusicGlyph {
                        glyph: g,
                        x,
                        y: self.y_of(p),
                    });
                    x += glyph_width(g) * sp;
                }
            }
        }
        if plan.beats.is_empty() {
            let g = SmuflGlyph::RestWhole;
            let b = g.metrics().bbox;
            let cx = (geo.content_start + geo.content_end) / 2.0;
            let x = cx - (b.sw.0 + b.width() / 2.0) * sp;
            let y = self.y_of(6);
            st.glyphs.push(MusicGlyph { glyph: g, x, y });
            st.facts.bar_rests.push(RestFact {
                bar: geo.bar,
                voice: 0,
                beat: 0,
                rect: self.rect(g, x, y),
            });
            return;
        }
        let anchor = |i: usize| geo.centres[cols[i]];
        let mut stem_ends: BTreeMap<usize, (f32, f32)> = BTreeMap::new();
        for (i, pb) in plan.beats.iter().enumerate() {
            let ax = anchor(i);
            st.facts.anchors.push(AnchorFact {
                bar: geo.bar,
                voice: pb.voice,
                beat: pb.beat,
                x: ax,
            });
            if pb.notes.is_empty() {
                self.emit_rest(st, geo.bar, pb, ax);
                continue;
            }
            self.emit_notes(st, geo.bar, pb, ax);
            if pb.exponent >= 1 {
                let head = SmuflGlyph::notehead(pb.exponent).metrics();
                let hw = head.bbox.width() * sp;
                let left = ax - hw / 2.0;
                let hi = pb.notes.iter().map(|n| n.position).max().unwrap();
                let lo = pb.notes.iter().map(|n| n.position).min().unwrap();
                let st_half = DEFAULTS.stem_thickness * sp / 2.0;
                let (sx, end) = if pb.stem_up {
                    let a = head.anchor(Anchor::StemUpSE).expect("required");
                    let end = (self.y_of(hi) - STEM_SP * sp).min(self.y_of(MIDDLE_LINE));
                    (left + a.0 * sp - st_half, end)
                } else {
                    let a = head.anchor(Anchor::StemDownNW).expect("required");
                    let end = (self.y_of(lo) + STEM_SP * sp).max(self.y_of(MIDDLE_LINE));
                    (left + a.0 * sp + st_half, end)
                };
                stem_ends.insert(i, (sx, end));
            }
        }
        // Beams (flat) at the most extreme natural end, kept STEM_BEAMED_MIN from the
        // heads.
        let mut beam_y: BTreeMap<usize, f32> = BTreeMap::new();
        let thick = DEFAULTS.beam_thickness * sp;
        let step = (DEFAULTS.beam_thickness + DEFAULTS.beam_spacing) * sp;
        for (gi, (members, up)) in plan.groups.iter().enumerate() {
            let ends: Vec<f32> = members
                .iter()
                .filter_map(|i| stem_ends.get(i).map(|e| e.1))
                .collect();
            if ends.is_empty() {
                continue;
            }
            let mut y = if *up {
                ends.iter().copied().fold(f32::MAX, f32::min)
            } else {
                ends.iter().copied().fold(f32::MIN, f32::max)
            };
            for &i in members {
                let pb = &plan.beats[i];
                let hi = pb.notes.iter().map(|n| n.position).max().unwrap();
                let lo = pb.notes.iter().map(|n| n.position).min().unwrap();
                if *up {
                    y = y.min(self.y_of(hi) - STEM_BEAMED_MIN_SP * sp);
                } else {
                    y = y.max(self.y_of(lo) + STEM_BEAMED_MIN_SP * sp);
                }
            }
            beam_y.insert(gi, y);
            let levels_of = |i: usize| plan.beats[i].exponent.saturating_sub(2) as usize;
            let first_x = stem_ends[&members[0]].0;
            let last_x = stem_ends[members.last().unwrap()].0;
            let dir = if *up { 1.0 } else { -1.0 };
            let band_y = y + dir * thick / 2.0;
            st.beams.push(Beam {
                x1: first_x,
                y1: band_y,
                x2: last_x,
                y2: band_y,
                thickness: thick,
            });
            let max_level = members.iter().map(|i| levels_of(*i)).max().unwrap_or(1);
            for level in 2..=max_level {
                let ly = band_y + dir * step * (level as f32 - 1.0);
                let mut run: Vec<usize> = Vec::new();
                let flush = |run: &mut Vec<usize>, st: &mut NotationStaff| {
                    if run.is_empty() {
                        return;
                    }
                    let x1 = stem_ends[&run[0]].0;
                    let x2 = stem_ends[run.last().unwrap()].0;
                    let (x1, x2) = if run.len() == 1 {
                        let stub = BEAM_STUB_SP * sp;
                        let pos = members.iter().position(|m| *m == run[0]).unwrap();
                        if pos + 1 < members.len() {
                            (x1, x1 + stub)
                        } else {
                            (x1 - stub, x1)
                        }
                    } else {
                        (x1, x2)
                    };
                    st.beams.push(Beam {
                        x1,
                        y1: ly,
                        x2,
                        y2: ly,
                        thickness: thick,
                    });
                    run.clear();
                };
                for &i in members {
                    if levels_of(i) >= level {
                        run.push(i);
                    } else {
                        flush(&mut run, st);
                    }
                }
                flush(&mut run, st);
            }
            let depth =
                thick * max_level as f32 + DEFAULTS.beam_spacing * sp * (max_level as f32 - 1.0);
            st.facts.beams.push(BeamFact {
                bar: geo.bar,
                voice: plan.beats[members[0]].voice,
                up: *up,
                beats: members.iter().map(|i| plan.beats[*i].beat).collect(),
                band: Rect {
                    x: first_x,
                    y: if *up { y } else { y - depth },
                    w: last_x - first_x,
                    h: depth,
                },
            });
        }
        // Stems and flags.
        for (i, pb) in plan.beats.iter().enumerate() {
            let Some(&(sx, natural)) = stem_ends.get(&i) else {
                continue;
            };
            let end = pb
                .group
                .and_then(|g| beam_y.get(&g).copied())
                .unwrap_or(natural);
            let head = SmuflGlyph::notehead(pb.exponent).metrics();
            let hi = pb.notes.iter().map(|n| n.position).max().unwrap();
            let lo = pb.notes.iter().map(|n| n.position).min().unwrap();
            let start = if pb.stem_up {
                self.y_of(lo) - head.anchor(Anchor::StemUpSE).expect("required").1 * sp
            } else {
                self.y_of(hi) - head.anchor(Anchor::StemDownNW).expect("required").1 * sp
            };
            st.lines.push(EngravedLine {
                kind: EngravedLineKind::Stem,
                x1: sx,
                y1: start,
                x2: sx,
                y2: end,
                thickness: DEFAULTS.stem_thickness * sp,
            });
            let outer = if pb.stem_up {
                self.y_of(lo)
            } else {
                self.y_of(hi)
            };
            st.facts.stems.push(StemFact {
                bar: geo.bar,
                voice: pb.voice,
                beat: pb.beat,
                up: pb.stem_up,
                x: sx,
                length_sp: (end - outer).abs() / sp,
                beamed: pb.group.is_some(),
            });
            if pb.group.is_none() && pb.exponent >= 3 {
                let g = SmuflGlyph::flag(pb.exponent, pb.stem_up);
                let a = g
                    .metrics()
                    .anchor(if pb.stem_up {
                        Anchor::StemUpNW
                    } else {
                        Anchor::StemDownSW
                    })
                    .expect("required");
                // The flag's stem anchor sits on the stem's left edge (SMuFL flags are
                // drawn from the stem's left edge, up or down).
                let edge = sx - DEFAULTS.stem_thickness * sp / 2.0;
                st.glyphs.push(MusicGlyph {
                    glyph: g,
                    x: edge - a.0 * sp,
                    y: end + a.1 * sp,
                });
            }
        }
        // Tuplet numbers.
        for t in &plan.tuplets {
            let g = SmuflGlyph::tuplet_digit(t.digit);
            let b = g.metrics().bbox;
            let first = anchor(t.members[0]);
            let last = anchor(*t.members.last().unwrap());
            let cx = (first + last) / 2.0;
            let x = cx - (b.sw.0 + b.width() / 2.0) * sp;
            let y = self.y_of(0) - t.baseline * sp;
            st.glyphs.push(MusicGlyph { glyph: g, x, y });
            st.facts.tuplets.push(TupletFact {
                bar: geo.bar,
                voice: plan.beats[t.members[0]].voice,
                beats: t.members.iter().map(|i| plan.beats[*i].beat).collect(),
                number: t.digit,
                rect: self.rect(g, x, y),
            });
        }
    }

    fn emit_rest(&self, st: &mut NotationStaff, bar: usize, pb: &PBeat, ax: f32) {
        let sp = self.sp;
        let g = SmuflGlyph::rest(pb.exponent);
        let b = g.metrics().bbox;
        let p = pb.rest_position;
        let y = self.y_of(p);
        let x = ax - (b.sw.0 + b.width() / 2.0) * sp;
        st.glyphs.push(MusicGlyph { glyph: g, x, y });
        let rect = self.rect(g, x, y);
        st.facts.rests.push(RestFact {
            bar,
            voice: pb.voice,
            beat: pb.beat,
            rect,
        });
        for d in 0..pb.dots {
            let dx = rect.right() + (DOT_GAP_SP + d as f32 * DOT_STEP_SP) * sp;
            st.glyphs.push(MusicGlyph {
                glyph: SmuflGlyph::AugmentationDot,
                x: dx,
                y: self.y_of(p + 1),
            });
        }
    }

    fn emit_notes(&self, st: &mut NotationStaff, bar: usize, pb: &PBeat, ax: f32) {
        let sp = self.sp;
        let head = SmuflGlyph::notehead(pb.exponent);
        let hw = glyph_width(head) * sp;
        let up = pb.stem_up;
        let normal_left = ax - hw / 2.0;
        let displaced_any = pb.notes.iter().any(|n| n.displaced);
        // Accidental columns, right to left from the leftmost head.
        let mut widths: BTreeMap<usize, f32> = BTreeMap::new();
        for n in &pb.notes {
            if let Some(a) = n.accidental {
                let w = widths.entry(n.acc_column).or_insert(0.0);
                *w = w.max(glyph_width(SmuflGlyph::accidental(a.semitones())) * sp);
            }
        }
        let leftmost = if !up && displaced_any {
            normal_left - hw
        } else {
            normal_left
        };
        let mut cursor = leftmost - ACCIDENTAL_CLEARANCE_SP * sp;
        let mut column_x: BTreeMap<usize, f32> = BTreeMap::new();
        for (c, w) in &widths {
            cursor -= w;
            column_x.insert(*c, cursor);
            cursor -= ACCIDENTAL_COLUMN_GAP_SP * sp;
        }
        let rightmost = if up && displaced_any {
            normal_left + 2.0 * hw
        } else {
            normal_left + hw
        };
        for n in &pb.notes {
            let hx = if n.displaced {
                if up {
                    normal_left + hw
                } else {
                    normal_left - hw
                }
            } else {
                normal_left
            };
            let hy = self.y_of(n.position);
            st.glyphs.push(MusicGlyph {
                glyph: head,
                x: hx,
                y: hy,
            });
            let head_rect = self.rect(head, hx, hy);
            let mut accidental_box = None;
            if let Some(a) = n.accidental {
                let g = SmuflGlyph::accidental(a.semitones());
                // Right edge of the glyph box on the column's right edge.
                let w = widths[&n.acc_column];
                let gx = column_x[&n.acc_column] + w - (g.metrics().bbox.ne.0) * sp;
                st.glyphs.push(MusicGlyph {
                    glyph: g,
                    x: gx,
                    y: hy,
                });
                accidental_box = Some(self.rect(g, gx, hy));
            }
            let ext = DEFAULTS.leger_line_extension * sp;
            let mut ledgers = Vec::new();
            let mut ledger = |p: i32| {
                ledgers.push(EngravedLine {
                    kind: EngravedLineKind::LedgerLine,
                    x1: hx - ext,
                    y1: self.y_of(p),
                    x2: hx + hw + ext,
                    y2: self.y_of(p),
                    thickness: DEFAULTS.leger_line_thickness * sp,
                });
            };
            let mut p = -2;
            while p >= n.position {
                ledger(p);
                p -= 2;
            }
            let mut p = TOP_LINE + 2;
            while p <= n.position {
                ledger(p);
                p += 2;
            }
            st.lines.extend(ledgers.iter().copied());
            let mut dots = Vec::new();
            for d in 0..pb.dots {
                let dy = if n.position.rem_euclid(2) == 0 {
                    self.y_of(n.position + 1)
                } else {
                    hy
                };
                let dx = rightmost + (DOT_GAP_SP + d as f32 * DOT_STEP_SP) * sp;
                st.glyphs.push(MusicGlyph {
                    glyph: SmuflGlyph::AugmentationDot,
                    x: dx,
                    y: dy,
                });
                let b = SmuflGlyph::AugmentationDot.metrics().bbox;
                dots.push((dx + (b.sw.0 + b.width() / 2.0) * sp, dy));
            }
            if n.tie != TieRole::NotTied {
                let source = st.facts.notes.iter().rev().find(|f| {
                    f.voice == pb.voice
                        && f.string == n.string
                        && (f.bar < bar || f.beat < pb.beat)
                        && f.position == n.position
                });
                let x1 = source
                    .map(|s| s.head.right() + 0.1 * sp)
                    .unwrap_or(hx - TIE_LEAD_SP * sp);
                let y = if up {
                    hy + TIE_OFFSET_SP * sp
                } else {
                    hy - TIE_OFFSET_SP * sp
                };
                st.ties.push(Tie {
                    x1,
                    x2: hx - 0.1 * sp,
                    y,
                    up: !up,
                    height: TIE_HEIGHT_SP * sp,
                    end_thickness: DEFAULTS.tie_endpoint_thickness * sp,
                    mid_thickness: DEFAULTS.tie_midpoint_thickness * sp,
                });
            }
            st.facts.notes.push(NoteFact {
                bar,
                voice: pb.voice,
                beat: pb.beat,
                string: n.string,
                written: n.written,
                position: n.position,
                accidental: n.accidental,
                tie: n.tie,
                head: head_rect,
                displaced: n.displaced,
                accidental_box,
                ledgers,
                dots,
            });
        }
    }
}
