//! The S11 checks as test support (ADR-041 L2 to L4): N1 correctness, N2 alignment,
//! N3 engraving rules and bounds. They take their truth from the score and from
//! `mc-theory` directly, not from the engine's intermediate results. Every violation
//! names its rule, so the breakage tests (L5) can expect a specific one.

use mc_notation::smufl::SmuflGlyph;
use mc_notation::{
    EngravedLine, EngravedLineKind, HitTarget, LineKind, NotationStaff, NoteFact, Rect,
    ScoreLayout, Stave, StaveKind,
};
use mc_score::Score;
use mc_theory::fretted::pitch_at;
use mc_theory::notation::{BarNote, TieRole, bar_accidentals};
use mc_theory::{Rational, TimeSignature};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct Report {
    pub notes: usize,
    pub rule_checks: usize,
    /// Tie halves at system breaks that EP5 checked (coverage).
    pub tie_halves: usize,
    pub violations: Vec<(&'static str, String)>,
}

impl Report {
    fn fail(&mut self, rule: &'static str, msg: String) {
        self.violations.push((rule, msg));
    }

    pub fn has(&self, rule: &str) -> bool {
        self.violations.iter().any(|v| v.0 == rule)
    }

    pub fn summary(&self) -> String {
        self.violations
            .iter()
            .take(8)
            .map(|(r, m)| format!("[{r}] {m}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

type Key = (usize, usize, usize); // bar, voice, beat
/// (bar, voice, beat, string) to (onset, tie role, sounding semitone).
type Truth = BTreeMap<(usize, usize, usize, u8), (Rational, TieRole, i32)>;

/// Onset and tie role of every note of a track, from the score.
fn truth(score: &Score, t: usize) -> Truth {
    let track = score.tracks().nth(t).unwrap();
    let inst = track.fretted();
    let mut out = BTreeMap::new();
    for (b, bar) in track.bars().enumerate() {
        for (v, voice) in bar.voices.iter().enumerate() {
            let mut onset = Rational::ZERO;
            for (k, beat) in voice.beats.iter().enumerate() {
                for n in &beat.notes {
                    let s = pitch_at(&inst.tuning, inst.capo, n.position()).unwrap();
                    let tie = match (n.tie, k) {
                        (false, _) => TieRole::NotTied,
                        (true, 0) => TieRole::TiedAcrossBarLine,
                        (true, _) => TieRole::TiedInBar,
                    };
                    out.insert((b, v, k, n.position().string.get()), (onset, tie, s));
                }
                onset = onset.checked_add(beat.value.length()).unwrap();
            }
        }
    }
    out
}

fn beam_window(ts: TimeSignature) -> Rational {
    let (n, d) = (ts.numerator() as i64, ts.denominator() as i64);
    if d >= 8 && n > 3 && n % 3 == 0 {
        Rational::new(3, d).unwrap()
    } else {
        Rational::new(1, 4).unwrap()
    }
}

/// Tuplets of a voice: runs of beats with the same number, `actual` beats each.
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

fn notation_staves(layout: &ScoreLayout, track: mc_score::TrackId) -> Vec<(usize, &Stave)> {
    layout
        .systems
        .iter()
        .flat_map(|sys| {
            sys.staves
                .iter()
                .filter(move |s| s.track == track && s.kind == StaveKind::Notation)
                .map(move |s| (sys.index, s))
        })
        .collect()
}

pub fn check(score: &Score, layout: &ScoreLayout) -> Report {
    let mut r = Report::default();
    let masters = score.master_bars();
    for (t, track) in score.tracks().enumerate() {
        let staves = notation_staves(layout, track.id());
        if staves.is_empty() {
            continue;
        }
        let truth = truth(score, t);
        let transposition = track.instrument().written_transposition();
        let mut notes: Vec<&NoteFact> = Vec::new();
        for (sys, st) in &staves {
            let n = &st.layout.notation;
            notes.extend(n.facts.notes.iter());
            n3_staff(n, &mut r);
            key_signs(score, layout, *sys, n, &mut r);
        }
        r.notes += notes.len();
        // N1: written pitch, position, ledgers.
        for n in &notes {
            r.rule_checks += 1;
            let Some(&(_, tie, sounding)) = truth.get(&(n.bar, n.voice, n.beat, n.string)) else {
                r.fail(
                    "written-pitch",
                    format!("bar {} beat {}: a note the score lacks", n.bar, n.beat),
                );
                continue;
            };
            if n.written.semitone() != sounding + 12 {
                r.fail(
                    "written-pitch",
                    format!(
                        "bar {} beat {}: written {} for sounding {sounding}",
                        n.bar, n.beat, n.written
                    ),
                );
            }
            if tie == TieRole::NotTied {
                let expected = transposition
                    .written(masters[n.bar].key.spell(sounding))
                    .unwrap();
                if n.written != expected {
                    r.fail(
                        "spelling",
                        format!(
                            "bar {} beat {}: {} spelled, {} expected",
                            n.bar, n.beat, n.written, expected
                        ),
                    );
                }
            }
            if n.position != n.written.diatonic_step() - 30 {
                r.fail(
                    "staff-position",
                    format!("bar {} beat {}: position {}", n.bar, n.beat, n.position),
                );
            }
            let ledgers = if n.position <= -2 {
                (-n.position) / 2
            } else if n.position >= 10 {
                (n.position - 8) / 2
            } else {
                0
            };
            if n.ledgers.len() as i32 != ledgers {
                r.fail(
                    "ledger-count",
                    format!(
                        "bar {} pos {}: {} ledger lines, {ledgers} expected",
                        n.bar,
                        n.position,
                        n.ledgers.len()
                    ),
                );
            }
        }
        // Tied notes keep the written spelling of the note they continue.
        for n in notes.iter().filter(|n| n.tie != TieRole::NotTied) {
            let source = notes
                .iter()
                .filter(|m| {
                    m.voice == n.voice && m.string == n.string && (m.bar, m.beat) < (n.bar, n.beat)
                })
                .max_by_key(|m| (m.bar, m.beat));
            if let Some(src) = source
                && src.written.semitone() == n.written.semitone()
                && (src.written.letter(), src.written.alter())
                    != (n.written.letter(), n.written.alter())
            {
                r.fail(
                    "spelling",
                    format!("bar {} beat {}: tie changed the spelling", n.bar, n.beat),
                );
            }
        }
        // N1: accidentals equal mc-theory's bar_accidentals on the written pitches.
        let mut by_bar: BTreeMap<usize, Vec<&NoteFact>> = BTreeMap::new();
        for n in &notes {
            by_bar.entry(n.bar).or_default().push(n);
        }
        for (bar, list) in by_bar {
            let input: Vec<BarNote> = list
                .iter()
                .map(|n| {
                    let (onset, tie, _) = truth[&(n.bar, n.voice, n.beat, n.string)];
                    BarNote {
                        onset,
                        written: n.written,
                        tie,
                    }
                })
                .collect();
            let expected = bar_accidentals(masters[bar].key, &input);
            for (n, e) in list.iter().zip(expected) {
                r.rule_checks += 1;
                if n.accidental != e {
                    r.fail(
                        "accidental",
                        format!(
                            "bar {bar} beat {} pos {}: {:?}, expected {:?}",
                            n.beat, n.position, n.accidental, e
                        ),
                    );
                }
            }
        }
        // N1: stems by the middle-line rule, beam groups, tuplets, whole-bar rests.
        let stems: Vec<_> = staves
            .iter()
            .flat_map(|(_, s)| s.layout.notation.facts.stems.iter())
            .collect();
        let beams: Vec<_> = staves
            .iter()
            .flat_map(|(_, s)| s.layout.notation.facts.beams.iter())
            .collect();
        let tuplets: Vec<_> = staves
            .iter()
            .flat_map(|(_, s)| s.layout.notation.facts.tuplets.iter())
            .collect();
        let mut bar_rests: Vec<usize> = staves
            .iter()
            .flat_map(|(_, s)| s.layout.notation.facts.bar_rests.iter().map(|x| x.bar))
            .collect();
        let mut beat_positions: BTreeMap<Key, Vec<i32>> = BTreeMap::new();
        for n in &notes {
            beat_positions
                .entry((n.bar, n.voice, n.beat))
                .or_default()
                .push(n.position);
        }
        let group_of: BTreeMap<Key, usize> = beams
            .iter()
            .enumerate()
            .flat_map(|(g, b)| b.beats.iter().map(move |k| ((b.bar, b.voice, *k), g)))
            .collect();
        let voices_in = |b: usize| {
            track
                .bar(b)
                .unwrap()
                .voices
                .iter()
                .filter(|v| !v.beats.is_empty())
                .count()
        };
        for s in &stems {
            r.rule_checks += 1;
            let key = (s.bar, s.voice, s.beat);
            let positions: Vec<i32> = match group_of.get(&key) {
                Some(g) => beams[*g]
                    .beats
                    .iter()
                    .flat_map(|k| beat_positions[&(s.bar, s.voice, *k)].clone())
                    .collect(),
                None => beat_positions[&key].clone(),
            };
            let expected = if voices_in(s.bar) > 1 {
                s.voice == 0
            } else {
                let hi = *positions.iter().max().unwrap();
                let lo = *positions.iter().min().unwrap();
                (4 - lo) > (hi - 4)
            };
            if s.up != expected {
                r.fail(
                    "stem-direction",
                    format!(
                        "bar {} beat {}: stem {}",
                        s.bar,
                        s.beat,
                        if s.up { "up" } else { "down" }
                    ),
                );
            }
        }
        let mut empty_bars = Vec::new();
        let mut expected_tuplets = Vec::new();
        for (b, bar) in track.bars().enumerate() {
            if bar.voices.iter().all(|v| v.beats.is_empty()) {
                empty_bars.push(b);
            }
            let window = beam_window(masters[b].time_signature);
            for (v, voice) in bar.voices.iter().enumerate() {
                let tups = score_tuplets(voice);
                for (beats, n) in &tups {
                    expected_tuplets.push((b, v, beats.clone(), *n));
                }
                let in_tuplet: BTreeMap<usize, i64> = tups
                    .iter()
                    .enumerate()
                    .flat_map(|(i, (ks, _))| ks.iter().map(move |k| (*k, -10 - i as i64)))
                    .collect();
                let mut onset = Rational::ZERO;
                let mut expected: Vec<Vec<usize>> = Vec::new();
                let mut run: Vec<usize> = Vec::new();
                let mut run_win: Option<i64> = None;
                for (k, beat) in voice.beats.iter().enumerate() {
                    let w = match in_tuplet.get(&k) {
                        Some(w) => *w,
                        None => {
                            let q = onset.checked_div(window).unwrap();
                            q.numerator().div_euclid(q.denominator())
                        }
                    };
                    let beamable = beat.value.base().exponent() >= 3 && !beat.notes.is_empty();
                    if beamable && run_win == Some(w) {
                        run.push(k);
                    } else {
                        if run.len() > 1 {
                            expected.push(std::mem::take(&mut run));
                        }
                        run = if beamable { vec![k] } else { vec![] };
                        run_win = beamable.then_some(w);
                    }
                    onset = onset.checked_add(beat.value.length()).unwrap();
                }
                if run.len() > 1 {
                    expected.push(run);
                }
                let mut actual: Vec<Vec<usize>> = beams
                    .iter()
                    .filter(|g| g.bar == b && g.voice == v)
                    .map(|g| g.beats.clone())
                    .collect();
                actual.sort();
                expected.sort();
                r.rule_checks += 1;
                if actual != expected {
                    r.fail(
                        "beam-groups",
                        format!("bar {b} voice {v}: {actual:?}, expected {expected:?}"),
                    );
                }
            }
        }
        let mut actual_tuplets: Vec<_> = tuplets
            .iter()
            .map(|f| (f.bar, f.voice, f.beats.clone(), f.number))
            .collect();
        actual_tuplets.sort();
        expected_tuplets.sort();
        r.rule_checks += 1;
        if actual_tuplets != expected_tuplets {
            r.fail(
                "tuplet-number",
                format!("{actual_tuplets:?}, expected {expected_tuplets:?}"),
            );
        }
        bar_rests.sort();
        r.rule_checks += 1;
        if bar_rests != empty_bars {
            r.fail(
                "bar-rest",
                format!("whole-bar rests in {bar_rests:?}, expected {empty_bars:?}"),
            );
        }
    }
    n2(score, layout, &mut r);
    centred_bar_rests(score, layout, &mut r);
    tie_shapes(score, layout, &mut r);
    bounds(layout, &mut r);
    r
}

/// Key-signature signs on a system's notation staff: the header's key, plus each key
/// change after the first bar of the system.
fn key_signs(score: &Score, layout: &ScoreLayout, sys: usize, n: &NotationStaff, r: &mut Report) {
    let masters = score.master_bars();
    let s = &layout.systems[sys];
    let Some(first) = s.staves.first() else {
        return;
    };
    let (a, z) = (first.layout.first_bar(), first.layout.end_bar());
    let mut expected = masters[a].key.fifths().unsigned_abs() as usize;
    for b in a + 1..z {
        if masters[b].key != masters[b - 1].key {
            expected += masters[b].key.fifths().unsigned_abs() as usize;
        }
    }
    // EP2: the courtesy key at the end, for a key change starting the next system.
    if z < masters.len() && masters[z].key != masters[z - 1].key {
        expected += masters[z].key.fifths().unsigned_abs() as usize;
    }
    key_changes_and_courtesy(score, layout, sys, n, r);
    r.rule_checks += 1;
    let drawn = n
        .glyphs
        .iter()
        .filter(|g| {
            matches!(
                g.glyph,
                SmuflGlyph::AccidentalSharp | SmuflGlyph::AccidentalFlat
            )
        })
        .count()
        - n.facts
            .notes
            .iter()
            .filter(|x| matches!(x.accidental.map(|a| a.semitones()), Some(1 | -1)))
            .count();
    if n.facts.key_signs != expected || drawn != expected {
        r.fail(
            "key-signature",
            format!(
                "system {sys}: {} signs ({drawn} drawn), expected {expected}",
                n.facts.key_signs
            ),
        );
    }
}

/// EP2 and EP3 (ADR-041 section 13) on one notation staff:
/// - every key change drawn (inside the system, or as the courtesy for the next) cancels
///   exactly the letters `mc-theory` gives, and as many naturals are drawn as cancelled
///   signs plus natural accidentals of notes;
/// - a courtesy is drawn exactly when the next system starts with a key or time change,
///   announcing that change, after the system's last bar line;
/// - a double thin bar line precedes every key change (inside the system, and at the
///   system's end before a change in the next).
fn key_changes_and_courtesy(
    score: &Score,
    layout: &ScoreLayout,
    sys: usize,
    n: &NotationStaff,
    r: &mut Report,
) {
    const EPS: f32 = 0.01;
    let masters = score.master_bars();
    let s = &layout.systems[sys];
    let Some(first) = s.staves.first() else {
        return;
    };
    let (a, z) = (first.layout.first_bar(), first.layout.end_bar());
    let changes_at = |b: usize| b > 0 && b < masters.len() && masters[b].key != masters[b - 1].key;
    // Cancellations.
    let mut want: Vec<(usize, Vec<mc_theory::Letter>)> = (a + 1..z)
        .filter(|b| changes_at(*b))
        .map(|b| (b, masters[b - 1].key.cancelled_by(masters[b].key)))
        .collect();
    if changes_at(z) {
        want.push((z, masters[z - 1].key.cancelled_by(masters[z].key)));
    }
    r.rule_checks += 1;
    if n.facts.cancellations != want {
        r.fail(
            "key-cancellation",
            format!(
                "system {sys}: {:?}, expected {want:?}",
                n.facts.cancellations
            ),
        );
    }
    let naturals = n
        .glyphs
        .iter()
        .filter(|g| g.glyph == SmuflGlyph::AccidentalNatural)
        .count();
    let note_naturals = n
        .facts
        .notes
        .iter()
        .filter(|x| x.accidental.map(|a| a.semitones()) == Some(0))
        .count();
    let cancelled: usize = want.iter().map(|w| w.1.len()).sum();
    r.rule_checks += 1;
    if naturals != cancelled + note_naturals {
        r.fail(
            "key-cancellation",
            format!("system {sys}: {naturals} naturals, expected {cancelled} + {note_naturals}"),
        );
    }
    // Courtesy.
    let last_line = n.facts.barlines.iter().copied().fold(f32::MIN, f32::max);
    let time_changes =
        z < masters.len() && masters[z].time_signature != masters[z - 1].time_signature;
    r.rule_checks += 1;
    match (n.facts.courtesy.as_slice(), changes_at(z) || time_changes) {
        ([], false) => {}
        ([c], true) => {
            let key = changes_at(z).then_some(masters[z].key);
            let time = time_changes.then_some(masters[z].time_signature);
            if c.bar != z || c.key != key || c.time != time || c.x < last_line - EPS {
                r.fail("courtesy", format!("system {sys}: {c:?} for bar {z}"));
            }
        }
        (got, expected) => r.fail(
            "courtesy",
            format!("system {sys}: {got:?}, expected one: {expected}"),
        ),
    }
    // Double bar lines: two thin lines 0.56 staff spaces apart before each change.
    let sep = 0.56 * n.staff_space;
    let thin: Vec<f32> = n
        .lines
        .iter()
        .filter(|l| l.kind == EngravedLineKind::BarLine)
        .map(|l| l.x1)
        .collect();
    let pair_at = |x: f32| {
        thin.iter().any(|t| (t - x).abs() <= EPS)
            && thin.iter().any(|t| (t - (x - sep)).abs() <= EPS)
    };
    let bar_x = |b: usize| first.layout.bars.iter().find(|l| l.bar == b).map(|l| l.x);
    for b in (a + 1..z).filter(|b| changes_at(*b)) {
        if masters[b].repeat_start || masters[b - 1].repeat_end.is_some() {
            continue;
        }
        r.rule_checks += 1;
        if !bar_x(b).is_some_and(pair_at) {
            r.fail(
                "double-bar-line",
                format!("system {sys}: none before bar {b}"),
            );
        }
    }
    if changes_at(z) && !masters[z].repeat_start && masters[z - 1].repeat_end.is_none() {
        r.rule_checks += 1;
        if !pair_at(last_line) {
            r.fail(
                "double-bar-line",
                format!("system {sys}: none at the end before bar {z}"),
            );
        }
    }
}

/// EP4 (ADR-041 section 13): a voice whose bar holds one rest as long as the bar shows a
/// whole rest centred in the bar's content, in any time signature; other rests keep
/// their own value.
fn centred_bar_rests(score: &Score, layout: &ScoreLayout, r: &mut Report) {
    const EPS: f32 = 0.01;
    let whole = SmuflGlyph::RestWhole.metrics().bbox.width();
    for sys in &layout.systems {
        for st in sys.staves.iter().filter(|s| s.kind == StaveKind::Notation) {
            let n = &st.layout.notation;
            let track = score.track(st.track).expect("shown track");
            for &(bar, start, end) in &n.facts.contents {
                let length = score.master_bars()[bar].time_signature.length();
                let content = track.bar(bar).expect("bar");
                for (v, voice) in content.voices.iter().enumerate() {
                    let lone = voice.beats.len() == 1
                        && voice.beats[0].notes.is_empty()
                        && voice.beats[0].value.length() == length;
                    let Some(rest) = n
                        .facts
                        .rests
                        .iter()
                        .find(|x| x.bar == bar && x.voice == v && x.beat == 0)
                    else {
                        continue;
                    };
                    r.rule_checks += 1;
                    let is_whole = (rest.rect.w - whole * n.staff_space).abs() <= EPS;
                    let centred =
                        (rest.rect.x + rest.rect.w / 2.0 - (start + end) / 2.0).abs() <= EPS;
                    if lone && !(is_whole && centred) {
                        r.fail(
                            "centred-bar-rest",
                            format!(
                                "bar {bar} voice {v}: whole {is_whole}, centre {:.2} in {start:.2}..{end:.2}",
                                rest.rect.x + rest.rect.w / 2.0
                            ),
                        );
                    }
                }
            }
        }
    }
}

/// EP5 (ADR-041 section 13):
/// - a tie's height is 0.15 of its length, kept within 0.5 to 1.0 staff space;
/// - a tie across a system break is two halves: out of the last note of the earlier
///   system to its end, and into the first note of the next system from its content
///   start.
fn tie_shapes(score: &Score, layout: &ScoreLayout, r: &mut Report) {
    const EPS: f32 = 0.01;
    let mut halves = 0;
    for sys in &layout.systems {
        for st in sys.staves.iter().filter(|s| s.kind == StaveKind::Notation) {
            let n = &st.layout.notation;
            let sp = n.staff_space;
            for t in &n.ties {
                r.rule_checks += 1;
                let want = (0.15 * (t.x2 - t.x1) / sp).clamp(0.5, 1.0) * sp;
                if (t.height - want).abs() > EPS {
                    r.fail(
                        "tie-shape",
                        format!(
                            "system {}: tie height {} for {}",
                            sys.index,
                            t.height,
                            t.x2 - t.x1
                        ),
                    );
                }
            }
            let (Some(first), Some(last)) = (st.layout.bars.first(), st.layout.bars.last()) else {
                continue;
            };
            let system_end = n.facts.barlines.iter().copied().fold(f32::MIN, f32::max);
            let track = score.track(st.track).expect("shown track");
            for f in &n.facts.notes {
                // Into the first note of a system, from a note of the previous system.
                if f.bar == first.bar && f.bar > 0 && f.tie == TieRole::TiedAcrossBarLine {
                    r.rule_checks += 1;
                    halves += 1;
                    let start = n
                        .facts
                        .contents
                        .iter()
                        .find(|c| c.0 == f.bar)
                        .map_or(f32::MAX, |c| c.1);
                    let ok = n.ties.iter().any(|t| {
                        (t.x2 - (f.head.x - 0.1 * sp)).abs() <= EPS
                            && t.x1 <= start + EPS
                            && t.x2 - t.x1 >= 1.0 * sp - EPS
                    });
                    if !ok {
                        r.fail(
                            "tie-across-systems",
                            format!("bar {} voice {}: no tie in from the header", f.bar, f.voice),
                        );
                    }
                }
                // Out of the last note of a system into the next system.
                let next_tied = f.bar == last.bar
                    && track.bar(f.bar + 1).is_some_and(|nb| {
                        nb.voices
                            .get(f.voice)
                            .and_then(|v| v.beats.first())
                            .is_some_and(|b| {
                                b.notes
                                    .iter()
                                    .any(|x| x.tie && x.position().string.get() == f.string)
                            })
                    })
                    && track
                        .bar(f.bar)
                        .is_some_and(|b| b.voices[f.voice].beats.len() == f.beat + 1);
                if next_tied {
                    r.rule_checks += 1;
                    halves += 1;
                    let ok = n.ties.iter().any(|t| {
                        (t.x1 - (f.head.right() + 0.1 * sp)).abs() <= EPS
                            && (t.x2 - system_end.max(t.x1 + sp)).abs() <= EPS
                    });
                    if !ok {
                        r.fail(
                            "tie-across-systems",
                            format!(
                                "bar {} voice {}: no tie out to the system end",
                                f.bar, f.voice
                            ),
                        );
                    }
                }
            }
        }
    }
    r.tie_halves += halves;
}

/// N2: notation anchors equal the TAB beat columns, bar lines equal (0.01 units).
fn n2(score: &Score, layout: &ScoreLayout, r: &mut Report) {
    const EPS: f32 = 0.01;
    for sys in &layout.systems {
        for track in score.tracks() {
            let of = |k: StaveKind| {
                sys.staves
                    .iter()
                    .find(|s| s.track == track.id() && s.kind == k)
            };
            let (Some(n), Some(tab)) = (of(StaveKind::Notation), of(StaveKind::Tab)) else {
                continue;
            };
            // What TAB draws for each beat: its column (the hit regions, one per
            // string) and the x of the fret numbers inside them. Since EP1 a column with
            // notation need not be centred on its anchor, so the column centre is not
            // the TAB position; the fret numbers are (ADR-041 section 13).
            let mut columns: BTreeMap<Key, (f32, f32)> = BTreeMap::new();
            let mut frets: BTreeMap<Key, Vec<f32>> = BTreeMap::new();
            for h in &tab.layout.hits {
                let HitTarget::Beat { voice, beat, .. } = h.target else {
                    continue;
                };
                let k = (h.bar, voice, beat);
                columns.insert(k, (h.rect.x, h.rect.right()));
                for g in &tab.layout.glyphs {
                    if matches!(g.kind, mc_notation::GlyphKind::Fret { .. })
                        && h.rect.contains(g.x, g.y)
                    {
                        frets.entry(k).or_default().push(g.x);
                    }
                }
            }
            for a in &n.layout.notation.facts.anchors {
                r.rule_checks += 1;
                let k = (a.bar, a.voice, a.beat);
                let in_column = columns
                    .get(&k)
                    .is_some_and(|(l, rt)| a.x >= l - EPS && a.x <= rt + EPS);
                let on_frets = frets
                    .get(&k)
                    .is_none_or(|xs| xs.iter().all(|x| (a.x - x).abs() <= EPS));
                if !(in_column && on_frets) {
                    r.fail(
                        "alignment",
                        format!(
                            "bar {} beat {}: notation {} TAB column {:?} frets {:?}",
                            a.bar,
                            a.beat,
                            a.x,
                            columns.get(&k),
                            frets.get(&k)
                        ),
                    );
                }
            }
            let mut tab_lines: Vec<f32> = tab
                .layout
                .lines
                .iter()
                .filter(|l| matches!(l.kind, LineKind::BarLine | LineKind::ThickBarLine))
                .map(|l| l.x1)
                .collect();
            let mut lines = n.layout.notation.facts.barlines.clone();
            tab_lines.sort_by(f32::total_cmp);
            lines.sort_by(f32::total_cmp);
            r.rule_checks += 1;
            if tab_lines.len() != lines.len()
                || tab_lines
                    .iter()
                    .zip(&lines)
                    .any(|(a, b)| (a - b).abs() > EPS)
            {
                r.fail(
                    "alignment",
                    format!(
                        "system {}: bar lines {lines:?} against TAB {tab_lines:?}",
                        sys.index
                    ),
                );
            }
            if n.layout.bars != tab.layout.bars {
                r.fail(
                    "alignment",
                    format!("system {}: bar layouts differ", sys.index),
                );
            }
        }
    }
}

pub fn glyph_rect(g: SmuflGlyph, x: f32, y: f32, sp: f32) -> Rect {
    let b = g.metrics().bbox;
    Rect {
        x: x + b.sw.0 * sp,
        y: y - b.ne.1 * sp,
        w: b.width() * sp,
        h: b.height() * sp,
    }
}

pub fn line_rect(l: &EngravedLine) -> Rect {
    let t = l.thickness / 2.0;
    let (x1, x2) = (l.x1.min(l.x2), l.x1.max(l.x2));
    let (y1, y2) = (l.y1.min(l.y2), l.y1.max(l.y2));
    if (y2 - y1) < (x2 - x1) {
        Rect {
            x: x1,
            y: y1 - t,
            w: x2 - x1,
            h: y2 - y1 + 2.0 * t,
        }
    } else {
        Rect {
            x: x1 - t,
            y: y1,
            w: x2 - x1 + 2.0 * t,
            h: y2 - y1,
        }
    }
}

fn intersects(a: &Rect, b: &Rect) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}

/// Every painted item of a notation staff inside its box; boxes inside their system;
/// systems inside the layout.
fn bounds(layout: &ScoreLayout, r: &mut Report) {
    for sys in &layout.systems {
        let eps = 0.01;
        r.rule_checks += 1;
        if sys.y < -eps || sys.y + sys.height > layout.height + eps {
            r.fail(
                "inside-staff-box",
                format!("system {} outside the layout", sys.index),
            );
        }
        let mut prev = 0.0f32;
        for st in &sys.staves {
            r.rule_checks += 1;
            if st.top < prev - eps || st.top + st.layout.height > sys.height + eps {
                r.fail(
                    "inside-staff-box",
                    format!("system {}: stave box outside the system", sys.index),
                );
            }
            prev = st.top + st.layout.height;
            if st.kind != StaveKind::Notation {
                continue;
            }
            let n = &st.layout.notation;
            let sp = n.staff_space;
            let mut rects: Vec<(String, Rect)> = Vec::new();
            for g in &n.glyphs {
                rects.push((format!("{:?}", g.glyph), glyph_rect(g.glyph, g.x, g.y, sp)));
            }
            for l in &n.lines {
                rects.push((format!("{:?}", l.kind), line_rect(l)));
            }
            for b in &n.beams {
                let t = b.thickness / 2.0;
                rects.push((
                    "beam".into(),
                    Rect {
                        x: b.x1,
                        y: b.y1.min(b.y2) - t,
                        w: b.x2 - b.x1,
                        h: (b.y2 - b.y1).abs() + 2.0 * t,
                    },
                ));
            }
            for tie in &n.ties {
                let y = if tie.up { tie.y - tie.height } else { tie.y };
                rects.push((
                    "tie".into(),
                    Rect {
                        x: tie.x1,
                        y,
                        w: tie.x2 - tie.x1,
                        h: tie.height,
                    },
                ));
            }
            for (what, rc) in rects {
                r.rule_checks += 1;
                if rc.y < -eps
                    || rc.bottom() > st.layout.height + eps
                    || rc.x < -eps
                    || rc.right() > layout.width + eps
                {
                    r.fail(
                        "inside-staff-box",
                        format!(
                            "system {} {what}: y {:.2}..{:.2} sp, box 0..{:.2}",
                            sys.index,
                            rc.y / sp,
                            rc.bottom() / sp,
                            st.layout.height / sp
                        ),
                    );
                }
            }
        }
    }
}

fn dot_rect(x: f32, y: f32, sp: f32) -> Rect {
    let b = SmuflGlyph::AugmentationDot.metrics().bbox;
    Rect {
        x: x - b.width() / 2.0 * sp,
        y: y - b.height() / 2.0 * sp,
        w: b.width() * sp,
        h: b.height() * sp,
    }
}

/// N3: engraving rules on one notation staff (S11 rules a to j).
fn n3_staff(st: &NotationStaff, r: &mut Report) {
    let sp = st.staff_space;
    let eps = 0.01 * sp;
    let ext = mc_notation::smufl::DEFAULTS.leger_line_extension * sp;
    let bottom = st.lines_top + 4.0 * sp;
    let mut by_beat: BTreeMap<Key, Vec<&NoteFact>> = BTreeMap::new();
    for n in &st.facts.notes {
        by_beat.entry((n.bar, n.voice, n.beat)).or_default().push(n);
    }
    for (k, chord) in &by_beat {
        let mut sorted = chord.clone();
        sorted.sort_by_key(|n| n.position);
        // (a) a second puts its heads on both sides of the stem.
        for w in sorted.windows(2) {
            if w[1].position - w[0].position == 1 {
                r.rule_checks += 1;
                if w[0].displaced == w[1].displaced {
                    r.fail(
                        "second-sides",
                        format!("bar {} beat {}: a second on one side", k.0, k.2),
                    );
                }
            }
        }
        // (b) accidentals do not overlap and clear the heads by 0.12 sp.
        let heads: Vec<Rect> = sorted.iter().map(|n| n.head).collect();
        let accs: Vec<Rect> = sorted.iter().filter_map(|n| n.accidental_box).collect();
        let leftmost = heads.iter().map(|h| h.x).fold(f32::MAX, f32::min);
        for (i, a) in accs.iter().enumerate() {
            r.rule_checks += 1;
            if accs[i + 1..].iter().any(|b| intersects(a, b))
                || heads.iter().any(|h| intersects(a, h))
            {
                r.fail(
                    "accidental-clearance",
                    format!("bar {} beat {}: accidentals overlap", k.0, k.2),
                );
            }
            if a.right() > leftmost - 0.12 * sp + eps {
                r.fail(
                    "accidental-clearance",
                    format!(
                        "bar {} beat {}: accidental {:.2} sp from the heads",
                        k.0,
                        k.2,
                        (leftmost - a.right()) / sp
                    ),
                );
            }
        }
        // (c) ledger lines extend beyond the head on both sides.
        for n in &sorted {
            for l in &n.ledgers {
                r.rule_checks += 1;
                if l.x1 > n.head.x - ext + eps || l.x2 < n.head.right() + ext - eps {
                    r.fail(
                        "ledger-extension",
                        format!("bar {} beat {}: ledger too short", k.0, k.2),
                    );
                }
            }
            // (g) dots in a space.
            for &(_, y) in &n.dots {
                r.rule_checks += 1;
                let half = ((bottom - y) / (sp / 2.0)).round() as i32;
                if half.rem_euclid(2) == 0 {
                    r.fail(
                        "dot-in-space",
                        format!("bar {} beat {}: a dot on a line", k.0, k.2),
                    );
                }
            }
        }
    }
    // (c) ledger lines of neighbouring columns at one height do not touch. Beats of two
    // voices at one onset share a column (one anchor) and its ledger lines.
    let anchor: BTreeMap<Key, f32> = st
        .facts
        .anchors
        .iter()
        .map(|a| ((a.bar, a.voice, a.beat), a.x))
        .collect();
    let ledgers: Vec<(Key, &EngravedLine)> = st
        .facts
        .notes
        .iter()
        .flat_map(|n| n.ledgers.iter().map(move |l| ((n.bar, n.voice, n.beat), l)))
        .collect();
    for (i, (ka, a)) in ledgers.iter().enumerate() {
        for (kb, b) in &ledgers[i + 1..] {
            let same_column = (anchor[ka] - anchor[kb]).abs() < eps;
            if !same_column && (a.y1 - b.y1).abs() < eps {
                r.rule_checks += 1;
                let gap = if a.x1 < b.x1 {
                    b.x1 - a.x2
                } else {
                    a.x1 - b.x2
                };
                if gap < 0.1 * sp {
                    r.fail(
                        "ledger-neighbours",
                        format!("bars {}/{}: ledger lines touch", ka.0, kb.0),
                    );
                }
            }
        }
    }
    // (d) stem length.
    for s in &st.facts.stems {
        r.rule_checks += 1;
        let min = if s.beamed { 2.5 } else { 3.5 };
        if s.length_sp < min - 0.01 {
            r.fail(
                "stem-length",
                format!("bar {} beat {}: stem {:.2} sp", s.bar, s.beat, s.length_sp),
            );
        }
    }
    // (e) a beam does not cross a head of its group.
    for b in &st.facts.beams {
        for k in &b.beats {
            for n in by_beat.get(&(b.bar, b.voice, *k)).into_iter().flatten() {
                r.rule_checks += 1;
                if intersects(&b.band, &n.head) {
                    r.fail(
                        "beam-clears-heads",
                        format!("bar {} beat {k}: beam crosses a head", b.bar),
                    );
                }
            }
        }
    }
    // (f) a rest does not overlap a head of another voice.
    for rest in &st.facts.rests {
        for n in st
            .facts
            .notes
            .iter()
            .filter(|n| n.bar == rest.bar && n.voice != rest.voice)
        {
            r.rule_checks += 1;
            if intersects(&rest.rect, &n.head) {
                r.fail(
                    "rest-clears-other-voice",
                    format!("bar {} beat {}: rest on a head", rest.bar, rest.beat),
                );
            }
        }
    }
    // (j) tuplet numbers keep 0.25 sp from heads, accidentals, dots, stems, beams, flags.
    let mut items: Vec<Rect> = Vec::new();
    for n in &st.facts.notes {
        items.push(n.head);
        items.extend(n.accidental_box);
        items.extend(n.dots.iter().map(|&(x, y)| dot_rect(x, y, sp)));
    }
    items.extend(
        st.lines
            .iter()
            .filter(|l| l.kind == EngravedLineKind::Stem)
            .map(line_rect),
    );
    items.extend(st.facts.beams.iter().map(|b| b.band));
    items.extend(st.facts.rests.iter().map(|x| x.rect));
    items.extend(
        st.glyphs
            .iter()
            .filter(|g| g.glyph.name().starts_with("flag"))
            .map(|g| glyph_rect(g.glyph, g.x, g.y, sp)),
    );
    let pad = 0.25 * sp - eps;
    for t in &st.facts.tuplets {
        r.rule_checks += 1;
        let grown = Rect {
            x: t.rect.x - pad,
            y: t.rect.y - pad,
            w: t.rect.w + 2.0 * pad,
            h: t.rect.h + 2.0 * pad,
        };
        if items.iter().any(|i| intersects(i, &grown)) {
            r.fail(
                "tuplet-clearance",
                format!("bar {} beats {:?}: tuplet number too close", t.bar, t.beats),
            );
        }
    }
    // (h) between beats of one voice, heads, accidentals and dots do not overlap.
    let mut boxes: BTreeMap<(usize, usize), Vec<(usize, Rect)>> = BTreeMap::new();
    for n in &st.facts.notes {
        let e = boxes.entry((n.bar, n.voice)).or_default();
        e.push((n.beat, n.head));
        if let Some(a) = n.accidental_box {
            e.push((n.beat, a));
        }
        for &(x, y) in &n.dots {
            e.push((n.beat, dot_rect(x, y, sp)));
        }
    }
    for list in boxes.values() {
        for (i, (ba, a)) in list.iter().enumerate() {
            for (bb, b) in &list[i + 1..] {
                if ba != bb {
                    r.rule_checks += 1;
                    if intersects(a, b) {
                        r.fail("beats-overlap", format!("beats {ba} and {bb} overlap"));
                    }
                }
            }
        }
    }
}
