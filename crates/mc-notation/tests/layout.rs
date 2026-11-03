//! TAB layout goldens and consistency checks (ADR-018, bounded-contexts section 7).
//!
//! Goldens live in `golden/*.txt` (embedded with `include_str!`; no file I/O in domain
//! tests, FF-03). On a mismatch the test prints the full actual text between markers;
//! review it and replace the golden file only if the change is intended.

use mc_notation::{GlyphKind, HitTarget, LayoutMetrics, LineKind, TabLayout, layout_tab};
use mc_score::fixtures::{SMALL, TYPICAL, feature_rich, reference};
use mc_score::{
    BeatRef, FrettedInstrument, InstrumentSpec, MasterBar, NoteEffects, Score, TrackId,
};
use mc_theory::fretted::{Fret, FretPosition, StringIndex};
use mc_theory::{BaseValue, NoteValue, Tempo, TimeSignature, Tuplet};
use std::fmt::Write;

fn v(exponent: u8) -> NoteValue {
    NoteValue::plain(BaseValue::from_exponent(exponent).unwrap())
}

fn pos(string: u8, fret: u8) -> FretPosition {
    FretPosition {
        string: StringIndex::new(string).unwrap(),
        fret: Fret(fret),
    }
}

fn new_score(bars: usize) -> (Score, TrackId) {
    let mut s = Score::new(
        "Guitar",
        InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
        MasterBar::default(),
        Tempo::from_bpm(120).unwrap(),
    )
    .unwrap();
    if bars > 1 {
        s.insert_master_bars(1, bars - 1, MasterBar::default())
            .unwrap();
    }
    let t = s.track_at(0).unwrap().id();
    (s, t)
}

/// Appends a beat with notes `(string, fret)` and returns its reference.
fn beat(
    s: &mut Score,
    t: TrackId,
    bar: usize,
    voice: usize,
    value: NoteValue,
    notes: &[(u8, u8)],
) -> BeatRef {
    let index = s.track(t).unwrap().bar(bar).unwrap().voices[voice]
        .beats
        .len();
    let (id, _) = s.insert_beat(t, bar, voice, index, value).unwrap();
    let r = BeatRef {
        track: t,
        bar,
        voice,
        beat: id,
    };
    for (string, fret) in notes {
        s.set_note(r, pos(*string, *fret)).unwrap();
    }
    r
}

fn r1(x: f32) -> String {
    format!("{:.1}", x)
}

/// Deterministic text form of a layout. Hit regions are listed for string 1 only;
/// the consistency test checks that every string has the same columns.
fn dump(l: &TabLayout) -> String {
    let mut out = String::new();
    writeln!(
        out,
        "layout strings={} width={} height={}",
        l.strings,
        r1(l.width),
        r1(l.height)
    )
    .unwrap();
    for s in &l.systems {
        let bars: Vec<String> = s
            .bars
            .iter()
            .map(|b| {
                format!(
                    "{}@{}+{}{}",
                    b.bar,
                    r1(b.x),
                    r1(b.width),
                    if b.diagnostic { "!" } else { "" }
                )
            })
            .collect();
        writeln!(
            out,
            "system {} y={} h={} staff_top={} bars=[{}]",
            s.index,
            r1(s.y),
            r1(s.height),
            r1(s.staff_top),
            bars.join(" ")
        )
        .unwrap();
        for line in &s.lines {
            writeln!(
                out,
                "  line {:?} {},{} -> {},{}",
                line.kind,
                r1(line.x1),
                r1(line.y1),
                r1(line.x2),
                r1(line.y2)
            )
            .unwrap();
        }
        for g in &s.glyphs {
            writeln!(
                out,
                "  glyph {:?} {},{} w={}",
                g.kind,
                r1(g.x),
                r1(g.y),
                r1(g.width)
            )
            .unwrap();
        }
        for h in s.hits.iter().filter(|h| h.string == 1) {
            let target = match h.target {
                HitTarget::Beat { voice, beat, .. } => format!("beat v{voice} #{beat}"),
                HitTarget::Append { voice } => format!("append v{voice}"),
            };
            writeln!(
                out,
                "  hit bar={} {} x={} w={}",
                h.bar,
                target,
                r1(h.rect.x),
                r1(h.rect.w)
            )
            .unwrap();
        }
    }
    out
}

fn check_golden(name: &str, golden: &str, layout: &TabLayout) {
    let actual = dump(layout);
    let expected = golden.replace("\r\n", "\n");
    if actual != expected {
        println!("----- BEGIN {name} -----\n{actual}----- END {name} -----");
        panic!("layout golden '{name}' differs; actual text printed above");
    }
}

fn metrics() -> LayoutMetrics {
    LayoutMetrics::default()
}

#[test]
fn golden_empty_score() {
    let (s, t) = new_score(4);
    let l = layout_tab(&s, t, &metrics()).unwrap();
    check_golden("empty", include_str!("golden/empty.txt"), &l);
}

#[test]
fn golden_rhythm_and_note_kinds() {
    let (mut s, t) = new_score(2);
    // Bar 0: quarter, two beamed eighths, four sixteenths, dotted eighth + sixteenth.
    beat(&mut s, t, 0, 0, v(2), &[(6, 3), (5, 2)]);
    beat(&mut s, t, 0, 0, v(3), &[(1, 0)]);
    beat(&mut s, t, 0, 0, v(3), &[(2, 1)]);
    for f in [5, 7, 8, 10] {
        beat(&mut s, t, 0, 0, v(4), &[(3, f)]);
    }
    beat(
        &mut s,
        t,
        0,
        0,
        NoteValue::new(BaseValue::from_exponent(3).unwrap(), 1, None).unwrap(),
        &[(4, 12)],
    );
    beat(&mut s, t, 0, 0, v(4), &[(4, 12)]);
    // Bar 1: a tied eighth, a dead eighth, a sixteenth triplet, an eighth rest, a half note.
    let tied = beat(&mut s, t, 1, 0, v(3), &[(4, 12)]);
    let tied_note = s.track(t).unwrap().bar(1).unwrap().voices[0].beats[0].notes[0].id;
    s.set_tie(tied, tied_note, true).unwrap();
    let dead = beat(&mut s, t, 1, 0, v(3), &[(6, 0)]);
    let dead_note = s.track(t).unwrap().bar(1).unwrap().voices[0].beats[1].notes[0].id;
    s.set_note_effects(
        dead,
        dead_note,
        NoteEffects {
            dead: true,
            let_ring: false,
        },
    )
    .unwrap();
    let triplet = NoteValue::new(
        BaseValue::from_exponent(4).unwrap(),
        0,
        Some(Tuplet::new(3, 2).unwrap()),
    )
    .unwrap();
    for f in [0, 2, 3] {
        beat(&mut s, t, 1, 0, triplet, &[(1, f)]);
    }
    beat(
        &mut s,
        t,
        1,
        0,
        NoteValue::new(BaseValue::from_exponent(3).unwrap(), 0, None).unwrap(),
        &[],
    );
    beat(&mut s, t, 1, 0, v(1), &[(2, 3)]);
    // Second voice in bar 1: two halves (stems above the staff).
    let (v1, _) = s.add_voice(t, 1).unwrap();
    beat(&mut s, t, 1, v1, v(1), &[(5, 3)]);
    beat(&mut s, t, 1, v1, v(1), &[(5, 5)]);
    let l = layout_tab(&s, t, &metrics()).unwrap();
    check_golden("rhythm", include_str!("golden/rhythm.txt"), &l);
}

#[test]
fn golden_structure_signs() {
    let (mut s, t) = new_score(5);
    let mut bars: Vec<MasterBar> = s.master_bars().to_vec();
    bars[0].pickup = true;
    bars[1].repeat_start = true;
    bars[1].marker = Some("Intro".into());
    bars[2].alternate_endings = vec![1];
    bars[2].repeat_end = Some(2);
    bars[3].alternate_endings = vec![2];
    bars[4].time_signature = TimeSignature::new(3, 4).unwrap();
    for (i, b) in bars.into_iter().enumerate() {
        s.set_master_bar(i, b).unwrap();
    }
    beat(&mut s, t, 0, 0, v(2), &[(1, 3)]);
    for bar in 1..5 {
        beat(&mut s, t, bar, 0, v(1), &[(3, 2)]);
    }
    let l = layout_tab(&s, t, &metrics()).unwrap();
    check_golden("structure", include_str!("golden/structure.txt"), &l);
}

#[test]
fn golden_system_breaking() {
    let (mut s, t) = new_score(6);
    for bar in 0..6 {
        for k in 0..4u8 {
            beat(&mut s, t, bar, 0, v(2), &[(1 + k % 6, k + bar as u8)]);
        }
    }
    let narrow = LayoutMetrics {
        width: 420.0,
        ..metrics()
    };
    let l = layout_tab(&s, t, &narrow).unwrap();
    assert!(l.systems.len() >= 3, "{}", l.systems.len());
    check_golden("breaking", include_str!("golden/breaking.txt"), &l);
}

/// Layout-wide properties on every track of the reference fixtures.
#[test]
fn consistency_on_reference_scores() {
    let scores = [reference(SMALL), reference(TYPICAL), feature_rich()];
    let m = metrics();
    for score in &scores {
        for track in score.tracks() {
            let l = layout_tab(score, track.id(), &m).unwrap();
            assert_eq!(
                l,
                layout_tab(score, track.id(), &m).unwrap(),
                "deterministic"
            );
            // Every bar appears once, in order, and bars are contiguous in a system.
            let bars: Vec<usize> = l
                .systems
                .iter()
                .flat_map(|s| s.bars.iter().map(|b| b.bar))
                .collect();
            assert_eq!(bars, (0..score.master_bar_count()).collect::<Vec<_>>());
            let mut y = 0.0;
            for s in &l.systems {
                assert!((s.y - y).abs() < 1e-3, "systems stack without overlap");
                y = s.y + s.height + m.system_gap;
                for w in s.bars.windows(2) {
                    assert!((w[0].x + w[0].width - w[1].x).abs() < 1e-3);
                }
                let right = s.bars.last().map(|b| b.x + b.width).unwrap();
                if s.bars.len() > 1 {
                    assert!(
                        right <= m.width - m.margin_right + 0.5,
                        "system {} overflows: {right}",
                        s.index
                    );
                }
                for g in &s.glyphs {
                    assert!(g.x.is_finite() && g.y >= 0.0 && g.y <= s.height, "{g:?}");
                }
                for line in &s.lines {
                    assert!(line.y1 >= 0.0 && line.y2 <= s.height, "{line:?}");
                }
                // Hit regions: every string has the same targets.
                let per_string = |string: u8| -> Vec<_> {
                    s.hits
                        .iter()
                        .filter(|h| h.string == string)
                        .map(|h| (h.bar, h.target, h.rect.x))
                        .collect()
                };
                for string in 2..=l.strings {
                    assert_eq!(per_string(1), per_string(string));
                }
                assert_eq!(
                    s.lines
                        .iter()
                        .filter(|x| x.kind == LineKind::String)
                        .count(),
                    l.strings as usize
                );
            }
            // One fret glyph per note, and each note's glyph centre hits its own beat.
            let notes: usize = track
                .bars()
                .flat_map(|b| b.voices.iter())
                .flat_map(|v| v.beats.iter())
                .map(|b| b.notes.len())
                .sum();
            let frets: usize = l
                .systems
                .iter()
                .flat_map(|s| s.glyphs.iter())
                .filter(|g| matches!(g.kind, GlyphKind::Fret { .. }))
                .count();
            assert_eq!(notes, frets);
            for (bar, content) in track.bars().enumerate() {
                for (vi, voice) in content.voices.iter().enumerate() {
                    for (k, b) in voice.beats.iter().enumerate() {
                        for n in &b.notes {
                            let string = n.position().string.get();
                            let (sys, rect) = l
                                .cursor_rect(
                                    bar,
                                    string,
                                    HitTarget::Beat {
                                        voice: vi,
                                        beat: k,
                                        beat_id: b.id,
                                    },
                                )
                                .expect("beat has a hit region");
                            let hit = l.systems[sys]
                                .hit_test(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0, vi)
                                .unwrap();
                            assert_eq!(hit.bar, bar);
                            assert_eq!(hit.string, string);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn unknown_track_and_bad_metrics_are_errors() {
    let (s, _) = new_score(1);
    assert!(layout_tab(&s, TrackId::from_raw(999), &metrics()).is_err());
    let (s, t) = new_score(1);
    let bad = LayoutMetrics {
        width: 50.0,
        ..metrics()
    };
    assert!(layout_tab(&s, t, &bad).is_err());
}
