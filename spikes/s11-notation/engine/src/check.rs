//! N1 (correctness), N2 (alignment) and N3 (collisions by engraving rules). The checks
//! re-derive what they can from the score itself (sounding pitch, onsets, ties, key),
//! not from the engine's intermediate results, so a shared mistake is less likely.

use crate::model::*;
use crate::pitch;
use crate::smufl::Font;
use mc_score::Score;
use mc_theory::fretted::pitch_at;
use mc_theory::Rational;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct Report {
    pub notes: usize,
    pub n1: Vec<String>,
    pub n2: Vec<String>,
    pub n2_max_barline: f32,
    pub n2_max_anchor_sp: f32,
    pub n2_anchors: usize,
    pub n3: Vec<String>,
    pub n3_checks: BTreeMap<&'static str, usize>,
}

type Key = (usize, usize, usize); // bar, voice, beat

/// Onset, tie and sounding pitch of every note of track `t`, by (bar, voice, beat, string).
fn score_notes(score: &Score, t: usize) -> BTreeMap<(usize, usize, usize, u8), (Rational, bool, i32)> {
    let track = score.tracks().nth(t).unwrap();
    let inst = track.fretted();
    let mut out = BTreeMap::new();
    for (b, bar) in track.bars().enumerate() {
        for (v, voice) in bar.voices.iter().enumerate() {
            let mut onset = Rational::ZERO;
            for (k, beat) in voice.beats.iter().enumerate() {
                for n in &beat.notes {
                    let s = pitch_at(&inst.tuning, inst.capo, n.position()).unwrap();
                    out.insert((b, v, k, n.position().string.get()), (onset, n.tie, s));
                }
                onset = onset.checked_add(beat.value.length()).unwrap();
            }
        }
    }
    out
}

fn semitone(w: (i32, i8, i8)) -> i32 {
    (w.2 as i32 + 1) * 12 + pitch::LETTERS[w.0 as usize].natural_semitone() + w.1 as i32
}

pub fn check(score: &Score, layout: &Layout, font: &Font) -> Report {
    let sp = layout.config.staff_space;
    let mut r = Report::default();
    let masters = score.master_bars();
    for (t, _) in score.tracks().enumerate() {
        let truth = score_notes(score, t);
        // Gather this track's notation facts across systems.
        let mut notes: Vec<&NoteFact> = Vec::new();
        let mut stems: Vec<&StemFact> = Vec::new();
        let mut beams: Vec<&BeamFact> = Vec::new();
        let mut tuplets: Vec<&TupletFact> = Vec::new();
        let mut bar_rests: Vec<usize> = Vec::new();
        for sys in &layout.systems {
            for st in sys.staves.iter().filter(|s| s.position.track == t && s.kind == StaffKind::Notation) {
                notes.extend(st.notes.iter());
                stems.extend(st.stems.iter());
                beams.extend(st.beam_facts.iter());
                tuplets.extend(st.tuplets.iter());
                bar_rests.extend(st.bar_rests.iter().map(|r| r.bar));
                n3_staff(st, font, sp, &mut r);
                // N1: key signatures: the header's, plus one per key change that is not
                // the system's first bar (the header shows that one), and no other
                // sharps or flats than the notes' accidentals.
                let key_glyphs = st.glyphs.iter().filter(|g| g.name == "accidentalSharp" || g.name == "accidentalFlat").count();
                let note_accs = st.notes.iter().filter(|n| matches!(n.accidental, Some(1) | Some(-1))).count();
                let mut expected = masters[sys.first_bar].key.fifths().unsigned_abs() as usize;
                for b in sys.first_bar + 1..sys.end_bar {
                    if masters[b].key != masters[b - 1].key {
                        expected += masters[b].key.fifths().unsigned_abs() as usize;
                    }
                }
                if key_glyphs != note_accs + expected {
                    r.n1.push(format!(
                        "system {} track {t}: {} key-signature signs, expected {expected}",
                        sys.index,
                        key_glyphs as isize - note_accs as isize
                    ));
                }
            }
        }
        // N1: one tuplet number per tuplet, with the right number; a whole-bar rest in
        // every bar with no beat in any voice.
        {
            let track = score.tracks().nth(t).unwrap();
            let mut expected: Vec<(usize, usize, Vec<usize>, u8)> = Vec::new();
            let mut empty: Vec<usize> = Vec::new();
            for (b, bar) in track.bars().enumerate() {
                if bar.voices.iter().all(|v| v.beats.is_empty()) {
                    empty.push(b);
                }
                for (v, voice) in bar.voices.iter().enumerate() {
                    for (beats, n) in score_tuplets(voice) {
                        expected.push((b, v, beats, n));
                    }
                }
            }
            let mut actual: Vec<(usize, usize, Vec<usize>, u8)> = tuplets.iter().map(|f| (f.bar, f.voice, f.beats.clone(), f.number)).collect();
            actual.sort();
            expected.sort();
            if actual != expected {
                r.n1.push(format!("track {t}: tuplet numbers {actual:?}, expected {expected:?}"));
            }
            bar_rests.sort();
            if bar_rests != empty {
                r.n1.push(format!("track {t}: whole-bar rests in bars {bar_rests:?}, expected {empty:?}"));
            }
        }
        r.notes += notes.len();
        // N1: written pitch, position, ledger lines.
        for n in &notes {
            let Some(&(_, _, sounding)) = truth.get(&(n.bar, n.voice, n.beat, n.string)) else {
                r.n1.push(format!("track {t} bar {} beat {}: a note the score does not have", n.bar, n.beat));
                continue;
            };
            if semitone(n.written) != sounding + 12 {
                r.n1.push(format!("bar {} beat {}: written semitone {} is not sounding {} + 12", n.bar, n.beat, semitone(n.written), sounding));
            }
            let expected_pos = n.written.2 as i32 * 7 + n.written.0 - 30;
            if n.position != expected_pos {
                r.n1.push(format!("bar {} beat {}: position {} for step {}", n.bar, n.beat, n.position, expected_pos));
            }
            let ledgers = if n.position <= -2 { (-n.position) / 2 } else if n.position >= 10 { (n.position - 8) / 2 } else { 0 };
            if n.ledgers.len() as i32 != ledgers {
                r.n1.push(format!("bar {} beat {} pos {}: {} ledger lines, expected {}", n.bar, n.beat, n.position, n.ledgers.len(), ledgers));
            }
            // A diatonic pitch class is spelled with the key's alteration.
            let key = masters[n.bar].key;
            let pc = sounding.rem_euclid(12);
            let diatonic = pitch::LETTERS.iter().any(|l| (l.natural_semitone() + pitch::key_alter(key, *l) as i32).rem_euclid(12) == pc);
            if diatonic && !n.tied && n.written.1 != pitch::key_alter(key, pitch::LETTERS[n.written.0 as usize]) {
                r.n1.push(format!("bar {} beat {}: a degree of the key spelled against the key", n.bar, n.beat));
            }
        }
        // N1: accidentals, by an oracle written separately from the engine's state.
        let mut by_bar: BTreeMap<usize, Vec<(&NoteFact, Rational, bool)>> = BTreeMap::new();
        for n in &notes {
            if let Some(&(onset, tie, _)) = truth.get(&(n.bar, n.voice, n.beat, n.string)) {
                by_bar.entry(n.bar).or_default().push((n, onset, tie));
            }
        }
        for (bar, mut list) in by_bar {
            list.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.position.cmp(&b.0.position)));
            let key = masters[bar].key;
            let mut seen: BTreeMap<i32, i8> = BTreeMap::new();
            for (n, _, tie) in list {
                let alter = n.written.1;
                let step = n.written.2 as i32 * 7 + n.written.0;
                let current = seen.get(&step).copied().unwrap_or(pitch::key_alter(key, pitch::LETTERS[n.written.0 as usize]));
                let expected = if tie { None } else if alter != current { Some(alter) } else { None };
                if !(tie && n.beat == 0) {
                    seen.insert(step, alter);
                }
                if n.accidental != expected {
                    r.n1.push(format!("bar {bar} beat {} pos {}: accidental {:?}, expected {:?}", n.beat, n.position, n.accidental, expected));
                }
            }
        }
        // N1: stem directions and beam groups.
        let voices_in_bar = |b: usize| score.tracks().nth(t).unwrap().bar(b).unwrap().voices.iter().filter(|v| !v.beats.is_empty()).count();
        let mut beat_notes: BTreeMap<Key, Vec<i32>> = BTreeMap::new();
        for n in &notes {
            beat_notes.entry((n.bar, n.voice, n.beat)).or_default().push(n.position);
        }
        let grouped: BTreeMap<Key, usize> = beams
            .iter()
            .enumerate()
            .flat_map(|(g, bf)| bf.beats.iter().map(move |k| ((bf.bar, bf.voice, *k), g)))
            .collect();
        for s in &stems {
            let key = (s.bar, s.voice, s.beat);
            let positions: Vec<i32> = match grouped.get(&key) {
                Some(g) => beams[*g].beats.iter().flat_map(|k| beat_notes[&(s.bar, s.voice, *k)].clone()).collect(),
                None => beat_notes[&key].clone(),
            };
            let expected = if voices_in_bar(s.bar) > 1 {
                s.voice == 0
            } else {
                let hi = *positions.iter().max().unwrap();
                let lo = *positions.iter().min().unwrap();
                (4 - lo) > (hi - 4)
            };
            if s.up != expected {
                r.n1.push(format!("bar {} beat {}: stem {} expected {}", s.bar, s.beat, if s.up { "up" } else { "down" }, if expected { "up" } else { "down" }));
            }
        }
        // Beam groups: consecutive beamable beats (eighth or shorter, with notes) in the
        // same beat window form one group, and only those.
        let track = score.tracks().nth(t).unwrap();
        for (b, bar) in track.bars().enumerate() {
            let ts = masters[b].time_signature;
            let (num, den) = (ts.numerator() as i64, ts.denominator() as i64);
            let window = if den >= 8 && num > 3 && num % 3 == 0 { Rational::new(3, 8).unwrap() } else { Rational::new(1, 4).unwrap() };
            for (v, voice) in bar.voices.iter().enumerate() {
                let mut onset = Rational::ZERO;
                let mut expected: Vec<Vec<usize>> = Vec::new();
                let mut run: Vec<usize> = Vec::new();
                let mut run_win = -1i64;
                // A tuplet is its own beam unit (general practice: a triplet's eighths
                // are beamed together even across the beat).
                let in_tuplet: BTreeMap<usize, i64> = score_tuplets(voice)
                    .into_iter()
                    .enumerate()
                    .flat_map(|(i, (ks, _))| ks.into_iter().map(move |k| (k, -10 - i as i64)))
                    .collect();
                for (k, beat) in voice.beats.iter().enumerate() {
                    let w = match in_tuplet.get(&k) {
                        Some(w) => *w,
                        None => onset.checked_div(window).map(|q| q.numerator() / q.denominator()).unwrap(),
                    };
                    let beamable = beat.value.base().exponent() >= 3 && !beat.notes.is_empty();
                    if beamable && w == run_win {
                        run.push(k);
                    } else {
                        if run.len() > 1 {
                            expected.push(run.clone());
                        }
                        run = if beamable { vec![k] } else { vec![] };
                        run_win = if beamable { w } else { -1 };
                    }
                    onset = onset.checked_add(beat.value.length()).unwrap();
                }
                if run.len() > 1 {
                    expected.push(run);
                }
                let mut actual: Vec<Vec<usize>> = beams.iter().filter(|g| g.bar == b && g.voice == v).map(|g| g.beats.clone()).collect();
                actual.sort();
                expected.sort();
                if actual != expected {
                    r.n1.push(format!("bar {b} voice {v}: beam groups {actual:?}, expected {expected:?}"));
                }
            }
        }
    }
    n2(layout, sp, &mut r);
    n3_bounds(layout, font, &mut r);
    r
}

/// Tuplets of a voice from the score: runs of consecutive beats with the same tuplet,
/// `actual` beats each (equal-value tuplets, as the spike supports).
fn score_tuplets(voice: &mc_score::Voice) -> Vec<(Vec<usize>, u8)> {
    let mut out = Vec::new();
    let mut run: Vec<usize> = Vec::new();
    let mut n = 0u8;
    for (k, beat) in voice.beats.iter().enumerate() {
        let a = beat.value.tuplet().map(|t| t.actual());
        if a != Some(n) && !run.is_empty() {
            out.push((std::mem::take(&mut run), n));
        }
        if let Some(a) = a {
            n = a;
            run.push(k);
            if run.len() == a as usize {
                out.push((std::mem::take(&mut run), n));
            }
        }
    }
    if !run.is_empty() {
        out.push((run, n));
    }
    out
}

/// The painted box of a horizontal or vertical line (butt caps: thickness across it).
fn line_rect(l: &Line) -> Rect {
    let t = l.thickness / 2.0;
    let (x1, x2) = (l.x1.min(l.x2), l.x1.max(l.x2));
    let (y1, y2) = (l.y1.min(l.y2), l.y1.max(l.y2));
    if (y2 - y1) < (x2 - x1) {
        Rect { x: x1, y: y1 - t, w: x2 - x1, h: y2 - y1 + 2.0 * t }
    } else {
        Rect { x: x1 - t, y: y1, w: x2 - x1 + 2.0 * t, h: y2 - y1 }
    }
}

/// The painted box of a glyph, in staff-local units.
fn glyph_rect(font: &Font, sp: f32, g: &Glyph) -> Rect {
    let b = font.bbox(g.name);
    Rect { x: g.x + b.sw.0 * sp, y: g.y - b.ne.1 * sp, w: b.width() * sp, h: (b.ne.1 - b.sw.1) * sp }
}

/// N3 (i): every painted item of every staff lies inside its staff box (and so cannot
/// collide with the neighbouring staff or leave the page); boxes stay in their system
/// and systems on the page.
fn n3_bounds(layout: &Layout, font: &Font, r: &mut Report) {
    let sp = layout.config.staff_space;
    let eps = 0.01 * sp;
    for sys in &layout.systems {
        if sys.y < -eps || sys.y + sys.height > layout.height + eps {
            r.n3.push(format!("system {} leaves the page", sys.index));
        }
        let mut prev_bottom = 0.0f32;
        for st in &sys.staves {
            let p = st.position;
            *r.n3_checks.entry("staff-box-in-system").or_insert(0) += 1;
            if p.top < prev_bottom - eps || p.top + p.height > sys.height + eps {
                r.n3.push(format!("system {} track {} staff {}: box outside the system or over the previous staff", sys.index, p.track, p.staff));
            }
            prev_bottom = p.top + p.height;
            let mut rects: Vec<(&'static str, Rect)> = Vec::new();
            for g in &st.glyphs {
                rects.push((g.name, glyph_rect(font, sp, g)));
            }
            for l in &st.lines {
                rects.push(("line", line_rect(l)));
            }
            for b in &st.beams {
                let t = b.thickness / 2.0;
                rects.push(("beam", Rect { x: b.x1, y: b.y1.min(b.y2) - t, w: b.x2 - b.x1, h: (b.y2 - b.y1).abs() + 2.0 * t }));
            }
            for tie in &st.ties {
                let bulge = 0.7 * sp;
                let y = if tie.up { tie.y - bulge } else { tie.y };
                rects.push(("tie", Rect { x: tie.x1, y, w: tie.x2 - tie.x1, h: bulge }));
            }
            for t in &st.texts {
                rects.push(("text", t.knockout.unwrap_or(Rect { x: t.x, y: t.y - t.size / 2.0, w: 0.0, h: t.size })));
            }
            for (what, rc) in rects {
                *r.n3_checks.entry("inside-staff-box").or_insert(0) += 1;
                if rc.y < -eps || rc.bottom() > p.height + eps || rc.x < -eps || rc.right() > layout.width + eps {
                    r.n3.push(format!(
                        "system {} track {} {:?}: {what} outside the staff box (x {:.2}..{:.2}, y {:.2}..{:.2} sp; box x 0..{:.2}, y 0..{:.2})",
                        sys.index,
                        p.track,
                        st.kind,
                        rc.x / sp,
                        rc.right() / sp,
                        rc.y / sp,
                        rc.bottom() / sp,
                        layout.width / sp,
                        p.height / sp
                    ));
                }
            }
        }
    }
}

/// N2: bar lines equal, beat anchors within 0.05 staff spaces, notation against TAB.
fn n2(layout: &Layout, sp: f32, r: &mut Report) {
    for sys in &layout.systems {
        let tracks: Vec<usize> = {
            let mut t: Vec<usize> = sys.staves.iter().map(|s| s.position.track).collect();
            t.dedup();
            t
        };
        let first_barlines = &sys.staves[0].barlines;
        for st in &sys.staves {
            if st.barlines.len() != first_barlines.len() {
                r.n2.push(format!("system {}: {} bar lines against {}", sys.index, st.barlines.len(), first_barlines.len()));
                continue;
            }
            for (a, b) in st.barlines.iter().zip(first_barlines) {
                let d = (a - b).abs();
                r.n2_max_barline = r.n2_max_barline.max(d);
                if d > 0.01 {
                    r.n2.push(format!("system {}: bar line off by {d}", sys.index));
                }
            }
        }
        for t in tracks {
            let of = |k: StaffKind| sys.staves.iter().find(|s| s.position.track == t && s.kind == k);
            let (Some(n), Some(tab)) = (of(StaffKind::Notation), of(StaffKind::Tab)) else { continue };
            let tab_x: BTreeMap<Key, f32> = tab.anchors.iter().map(|a| ((a.bar, a.voice, a.beat), a.x)).collect();
            for a in &n.anchors {
                // Rests have no fret number; the TAB anchor of their beat is still kept.
                if let Some(x) = tab_x.get(&(a.bar, a.voice, a.beat)) {
                    let d = (a.x - x).abs() / sp;
                    r.n2_anchors += 1;
                    r.n2_max_anchor_sp = r.n2_max_anchor_sp.max(d);
                    if d > 0.05 {
                        r.n2.push(format!("bar {} beat {}: notation and TAB anchors {d:.3} sp apart", a.bar, a.beat));
                    }
                }
            }
        }
    }
}

fn dot_rect(font: &Font, sp: f32, x: f32, y: f32) -> Rect {
    let b = font.bbox("augmentationDot");
    Rect { x: x + b.sw.0 * sp, y: y - b.ne.1 * sp, w: b.width() * sp, h: (b.ne.1 - b.sw.1) * sp }
}

/// N3: engraving-rule checks on one notation staff.
fn n3_staff(st: &Staff, font: &Font, sp: f32, r: &mut Report) {
    let mut count = |rule: &'static str| *r.n3_checks.entry(rule).or_insert(0) += 1;
    let bottom = st.position.lines_top + 4.0 * sp;
    let mut by_beat: BTreeMap<Key, Vec<&NoteFact>> = BTreeMap::new();
    for n in &st.notes {
        by_beat.entry((n.bar, n.voice, n.beat)).or_default().push(n);
    }
    let eps = 0.01 * sp;
    let mut v = Vec::new();
    for (k, chord) in &by_beat {
        let mut sorted = chord.clone();
        sorted.sort_by_key(|n| n.position);
        // (a) a second puts the two heads on opposite sides of the stem.
        for w in sorted.windows(2) {
            if w[1].position - w[0].position == 1 {
                count("second-opposite-sides");
                if w[0].displaced == w[1].displaced {
                    v.push(format!("bar {} beat {}: a second with both heads on one side", k.0, k.2));
                }
            }
        }
        // (b) accidentals do not overlap each other or any head, and clear their head.
        let heads: Vec<Rect> = sorted.iter().map(|n| n.head).collect();
        let accs: Vec<Rect> = sorted.iter().filter_map(|n| n.accidental_box).collect();
        for (i, a) in accs.iter().enumerate() {
            count("accidental-clearance");
            for b in &accs[i + 1..] {
                if a.intersects(b) {
                    v.push(format!("bar {} beat {}: accidentals overlap", k.0, k.2));
                }
            }
            let leftmost = heads.iter().map(|h| h.x).fold(f32::MAX, f32::min);
            if a.right() > leftmost - 0.12 * sp + eps {
                v.push(format!("bar {} beat {}: accidental {:.2} sp from the heads", k.0, k.2, (leftmost - a.right()) / sp));
            }
            for h in &heads {
                if a.intersects(h) {
                    v.push(format!("bar {} beat {}: accidental overlaps a head", k.0, k.2));
                }
            }
        }
        // (c) ledger lines extend beyond the head on both sides.
        for n in &sorted {
            for l in &n.ledgers {
                count("ledger-extension");
                if l.x1 > n.head.x - font.leger_extension * sp + eps || l.x2 < n.head.right() + font.leger_extension * sp - eps {
                    v.push(format!("bar {} beat {}: ledger line shorter than the head plus extension", k.0, k.2));
                }
            }
        }
        // (g) dots sit in a space.
        for n in &sorted {
            for &(_, y) in &n.dots {
                count("dot-in-space");
                let half = ((bottom - y) / (sp / 2.0)).round() as i32;
                if half.rem_euclid(2) == 0 {
                    v.push(format!("bar {} beat {}: a dot on a line", k.0, k.2));
                }
            }
        }
    }
    // (c) ledger lines of neighbouring beats at one height do not touch.
    let mut ledgers: Vec<(Key, Line)> = Vec::new();
    for n in &st.notes {
        for l in &n.ledgers {
            ledgers.push(((n.bar, n.voice, n.beat), *l));
        }
    }
    for (i, (ka, a)) in ledgers.iter().enumerate() {
        for (kb, b) in &ledgers[i + 1..] {
            if ka != kb && (a.y1 - b.y1).abs() < eps {
                count("ledger-neighbours");
                let gap = if a.x1 < b.x1 { b.x1 - a.x2 } else { a.x1 - b.x2 };
                if gap < 0.1 * sp {
                    v.push(format!("bars {}/{} beats {}/{}: ledger lines of neighbouring beats touch", ka.0, kb.0, ka.2, kb.2));
                }
            }
        }
    }
    // (d) stem length.
    for s in &st.stems {
        count("stem-length");
        let min = if s.beamed { 2.5 } else { 3.5 };
        if s.length_from_outer < min - 0.01 {
            v.push(format!("bar {} beat {}: stem {:.2} sp, at least {min}", s.bar, s.beat, s.length_from_outer));
        }
    }
    // (e) a beam does not cross a head of its own group.
    for b in &st.beam_facts {
        for k in &b.beats {
            for n in by_beat.get(&(b.bar, b.voice, *k)).into_iter().flatten() {
                count("beam-clears-heads");
                if b.band.intersects(&n.head) {
                    v.push(format!("bar {} beat {k}: beam crosses a head", b.bar));
                }
            }
        }
    }
    // (f) a rest does not overlap a head of another voice.
    for rest in &st.rests {
        for n in st.notes.iter().filter(|n| n.bar == rest.bar && n.voice != rest.voice) {
            count("rest-clears-other-voice");
            if rest.rect.intersects(&n.head) {
                v.push(format!("bar {} beat {}: rest overlaps a head of voice {}", rest.bar, rest.beat, n.voice + 1));
            }
        }
    }
    // (j) a tuplet number keeps 0.25 sp from every head, accidental, dot, stem, beam,
    // flag and rest on the staff.
    {
        let mut items: Vec<Rect> = Vec::new();
        for n in &st.notes {
            items.push(n.head);
            items.extend(n.accidental_box);
            items.extend(n.dots.iter().map(|&(x, y)| dot_rect(font, sp, x, y)));
        }
        items.extend(st.lines.iter().filter(|l| l.kind == LineKind::Stem).map(line_rect));
        items.extend(st.beam_facts.iter().map(|b| b.band));
        items.extend(st.rests.iter().map(|r| r.rect));
        items.extend(st.glyphs.iter().filter(|g| g.name.starts_with("flag")).map(|g| glyph_rect(font, sp, g)));
        let pad = 0.25 * sp - eps;
        for t in &st.tuplets {
            count("tuplet-number-clearance");
            let grown = Rect { x: t.rect.x - pad, y: t.rect.y - pad, w: t.rect.w + 2.0 * pad, h: t.rect.h + 2.0 * pad };
            if items.iter().any(|i| i.intersects(&grown)) {
                v.push(format!("bar {} beats {:?}: tuplet number within 0.25 sp of a note, stem or beam", t.bar, t.beats));
            }
        }
    }
    // (h) between beats of one voice: heads, accidentals and dots do not overlap.
    let mut boxes: BTreeMap<(usize, usize), Vec<(usize, Rect)>> = BTreeMap::new();
    for n in &st.notes {
        let e = boxes.entry((n.bar, n.voice)).or_default();
        e.push((n.beat, n.head));
        if let Some(a) = n.accidental_box {
            e.push((n.beat, a));
        }
        for &(x, y) in &n.dots {
            e.push((n.beat, dot_rect(font, sp, x, y)));
        }
    }
    for list in boxes.values() {
        for (i, (ba, a)) in list.iter().enumerate() {
            for (bb, b) in &list[i + 1..] {
                if ba != bb {
                    count("beats-do-not-overlap");
                    if a.intersects(b) {
                        v.push(format!("beats {ba} and {bb} overlap"));
                    }
                }
            }
        }
    }
    r.n3.extend(v);
}
