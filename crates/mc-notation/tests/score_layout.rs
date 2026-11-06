//! Multi-track TAB layout (ADR-031): bar lines and simultaneous beats line up across
//! staves, hit regions stay on their stave's track, diagnostics are per stave, and a
//! golden for a two-of-three-tracks score with different rhythms and a repeat.
//!
//! Randomised cases use a fixed-seed generator (no extra dependency): many cases,
//! reproducible from the seed printed on failure.

use mc_notation::{
    HitTarget, LayoutMetrics, NotationError, ScoreLayout, layout_score, layout_score_systems,
    layout_tab, outline_score,
};
use mc_score::{BeatRef, FrettedInstrument, InstrumentSpec, MasterBar, Score, TrackId};
use mc_theory::fretted::{Fret, FretPosition, StringIndex};
use mc_theory::{BaseValue, NoteValue, Rational, Tempo};
use std::fmt::Write;

fn v(exponent: u8) -> NoteValue {
    NoteValue::plain(BaseValue::from_exponent(exponent).unwrap())
}

fn guitar() -> InstrumentSpec {
    InstrumentSpec::Fretted(FrettedInstrument::standard_guitar())
}

/// A score with `tracks` standard-guitar tracks and `bars` empty 4/4 bars.
fn new_score(tracks: usize, bars: usize) -> (Score, Vec<TrackId>) {
    let mut s = Score::new(
        "T1",
        guitar(),
        MasterBar::default(),
        Tempo::from_bpm(120).unwrap(),
    )
    .unwrap();
    if bars > 1 {
        s.insert_master_bars(1, bars - 1, MasterBar::default())
            .unwrap();
    }
    for i in 1..tracks {
        s.add_track(&format!("T{}", i + 1), guitar()).unwrap();
    }
    let ids = s.tracks().map(|t| t.id()).collect();
    (s, ids)
}

fn beat(s: &mut Score, t: TrackId, bar: usize, value: NoteValue, notes: &[(u8, u8)]) {
    let index = s.track(t).unwrap().bar(bar).unwrap().voices[0].beats.len();
    let (id, _) = s.insert_beat(t, bar, 0, index, value).unwrap();
    let r = BeatRef {
        track: t,
        bar,
        voice: 0,
        beat: id,
    };
    for &(string, fret) in notes {
        s.set_note(
            r,
            FretPosition {
                string: StringIndex::new(string).unwrap(),
                fret: Fret(fret),
            },
        )
        .unwrap();
    }
}

/// xorshift64*: deterministic, good enough to vary test inputs.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// Fills every bar of every track with random rhythms (whole to sixteenth) and notes,
/// sometimes leaving a bar empty, under- or overfull.
fn random_score(seed: u64) -> (Score, Vec<TrackId>) {
    let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    let tracks = 1 + rng.below(4) as usize;
    let bars = 1 + rng.below(8) as usize;
    let (mut s, ids) = new_score(tracks, bars);
    for &t in &ids {
        for bar in 0..bars {
            let beats = rng.below(9);
            for _ in 0..beats {
                let e = 1 + rng.below(4) as u8; // half .. sixteenth
                let notes: Vec<(u8, u8)> = (0..rng.below(3))
                    .map(|k| (1 + k as u8 * 2 + rng.below(2) as u8, rng.below(15) as u8))
                    .collect();
                beat(&mut s, t, bar, v(e), &notes);
            }
        }
    }
    (s, ids)
}

/// A random non-empty subset of `ids`.
fn subset(seed: u64, ids: &[TrackId]) -> Vec<TrackId> {
    let mut rng = Rng(seed ^ 0xdead_beef | 1);
    let mut out: Vec<TrackId> = ids.iter().copied().filter(|_| rng.below(3) > 0).collect();
    if out.is_empty() {
        out.push(ids[rng.below(ids.len() as u64) as usize]);
    }
    out
}

fn metrics_for(seed: u64) -> LayoutMetrics {
    LayoutMetrics {
        width: [420.0, 800.0, 1400.0][(seed % 3) as usize],
        ..LayoutMetrics::default()
    }
}

/// Onset of every beat of voice 0 of `bar` in `track`, in order.
fn onsets(s: &Score, t: TrackId, bar: usize) -> Vec<Rational> {
    let mut at = Rational::ZERO;
    s.track(t).unwrap().bar(bar).unwrap().voices[0]
        .beats
        .iter()
        .map(|b| {
            let o = at;
            at = at.checked_add(b.value.length()).unwrap();
            o
        })
        .collect()
}

const CASES: u64 = 300;

#[test]
fn m2_bar_lines_line_up_in_every_system() {
    let mut staves_checked = 0;
    for seed in 0..CASES {
        let (s, ids) = random_score(seed);
        let shown = subset(seed, &ids);
        let l = layout_score(&s, &shown, &metrics_for(seed)).unwrap();
        for sys in &l.systems {
            let first = &sys.staves[0].layout.bars;
            for st in &sys.staves[1..] {
                let same = st.layout.bars.iter().zip(first).all(|(a, b)| {
                    (a.bar, a.x.to_bits(), a.width.to_bits())
                        == (b.bar, b.x.to_bits(), b.width.to_bits())
                });
                assert!(
                    same && st.layout.bars.len() == first.len(),
                    "seed {seed}: system {} stave bars differ",
                    sys.index
                );
                staves_checked += 1;
            }
        }
        // Every bar appears exactly once, in order, across the systems.
        let bars: Vec<usize> = l
            .systems
            .iter()
            .flat_map(|sys| sys.staves[0].layout.bars.iter().map(|b| b.bar))
            .collect();
        assert_eq!(
            bars,
            (0..s.master_bar_count()).collect::<Vec<_>>(),
            "seed {seed}"
        );
    }
    assert!(
        staves_checked > 300,
        "too few multi-stave systems: {staves_checked}"
    );
}

#[test]
fn m3_beats_that_start_together_share_a_column() {
    let mut pairs = 0;
    for seed in 0..CASES {
        let (s, ids) = random_score(seed);
        let l = layout_score(&s, &ids, &metrics_for(seed)).unwrap();
        for sys in &l.systems {
            // (bar, onset) -> column (x, width) from the first stave that has a beat there.
            let mut seen: Vec<(usize, Rational, f32, f32)> = Vec::new();
            for st in &sys.staves {
                for h in st.layout.hits.iter().filter(|h| h.string == 1) {
                    let HitTarget::Beat { voice: 0, beat, .. } = h.target else {
                        continue;
                    };
                    let onset = onsets(&s, st.track, h.bar)[beat];
                    match seen.iter().find(|e| e.0 == h.bar && e.1 == onset) {
                        Some(&(_, _, x, w)) => {
                            assert_eq!(
                                (x.to_bits(), w.to_bits()),
                                (h.rect.x.to_bits(), h.rect.w.to_bits()),
                                "seed {seed}: bar {} onset {onset:?} not aligned",
                                h.bar
                            );
                            pairs += 1;
                        }
                        None => seen.push((h.bar, onset, h.rect.x, h.rect.w)),
                    }
                }
            }
        }
    }
    assert!(pairs > 1000, "too few aligned pairs compared: {pairs}");
}

#[test]
fn m4_hit_regions_address_their_own_track() {
    for seed in 0..CASES {
        let (s, ids) = random_score(seed);
        let shown = subset(seed, &ids);
        let l = layout_score(&s, &shown, &metrics_for(seed)).unwrap();
        assert_eq!(l.tracks, shown, "seed {seed}: stave order is score order");
        for sys in &l.systems {
            for st in &sys.staves {
                let strings = s.track(st.track).unwrap().fretted().tuning.string_count();
                assert_eq!(st.strings, strings);
                for h in &st.layout.hits {
                    assert!(h.string >= 1 && h.string <= strings);
                    if let HitTarget::Beat {
                        voice,
                        beat,
                        beat_id,
                    } = h.target
                    {
                        let b = &s.track(st.track).unwrap().bar(h.bar).unwrap().voices[voice].beats
                            [beat];
                        assert_eq!(b.id, beat_id, "seed {seed}: hit points into another track");
                    }
                }
            }
        }
    }
}

#[test]
fn staves_stack_without_overlap_and_systems_stack() {
    for seed in 0..CASES {
        let (s, ids) = random_score(seed);
        let l = layout_score(&s, &ids, &metrics_for(seed)).unwrap();
        let mut y = 0.0f32;
        for sys in &l.systems {
            assert!((sys.y - y).abs() < 1e-3, "seed {seed}");
            let mut top = 0.0f32;
            for st in &sys.staves {
                assert!((st.top - top).abs() < 1e-3);
                assert!((st.layout.y - (sys.y + st.top)).abs() < 1e-3);
                top += st.layout.height;
            }
            assert!((sys.height - top).abs() < 1e-3);
            y = sys.y + sys.height + LayoutMetrics::default().system_gap;
        }
    }
}

#[test]
fn m1_a_single_shown_track_is_the_single_track_layout() {
    // layout_tab is built on layout_score; the pre-change output was compared with the
    // new one on every fixture track (ADR-031 implementation notes). This keeps the
    // relation explicit for later changes.
    for seed in 0..60 {
        let (s, ids) = random_score(seed);
        let m = metrics_for(seed);
        for &t in &ids {
            let tab = layout_tab(&s, t, &m).unwrap();
            let score = layout_score(&s, &[t], &m).unwrap();
            assert_eq!(tab.systems.len(), score.systems.len());
            for (a, b) in tab.systems.iter().zip(&score.systems) {
                assert_eq!(b.staves.len(), 1);
                assert_eq!(a, &b.staves[0].layout);
            }
        }
    }
}

#[test]
fn m6_diagnostics_are_per_stave() {
    let (mut s, ids) = new_score(2, 1);
    beat(&mut s, ids[0], 0, v(2), &[(1, 0)]); // underfull
    for _ in 0..4 {
        beat(&mut s, ids[1], 0, v(2), &[(1, 0)]); // exactly full
    }
    let l = layout_score(&s, &ids, &LayoutMetrics::default()).unwrap();
    let flags: Vec<bool> = l.systems[0]
        .staves
        .iter()
        .map(|st| st.layout.bars[0].diagnostic)
        .collect();
    assert_eq!(flags, vec![true, false]);
}

#[test]
fn order_duplicates_and_errors() {
    let (s, ids) = new_score(3, 2);
    let l = layout_score(&s, &[ids[2], ids[0], ids[0]], &LayoutMetrics::default()).unwrap();
    assert_eq!(
        l.tracks,
        vec![ids[0], ids[2]],
        "score order, duplicates ignored"
    );
    assert_eq!(
        layout_score(&s, &[], &LayoutMetrics::default()),
        Err(NotationError::NoTracks)
    );
    let unknown = TrackId::from_raw(9_999);
    assert_eq!(
        layout_score(&s, &[ids[0], unknown], &LayoutMetrics::default()),
        Err(NotationError::UnknownTrack(unknown))
    );
}

fn r1(x: f32) -> String {
    format!("{x:.1}")
}

/// Deterministic text form: per system and stave, bars, fret glyphs and beat hits on
/// string 1 (the single-track goldens cover lines and the other glyph kinds).
fn dump(l: &ScoreLayout) -> String {
    let mut out = String::new();
    let tracks: Vec<u64> = l.tracks.iter().map(|t| t.raw()).collect();
    writeln!(
        out,
        "score tracks={tracks:?} width={} height={}",
        r1(l.width),
        r1(l.height)
    )
    .unwrap();
    for sys in &l.systems {
        writeln!(
            out,
            "system {} y={} h={}",
            sys.index,
            r1(sys.y),
            r1(sys.height)
        )
        .unwrap();
        for st in &sys.staves {
            let bars: Vec<String> = st
                .layout
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
                "  stave track={} top={} h={} bars=[{}]",
                st.track.raw(),
                r1(st.top),
                r1(st.layout.height),
                bars.join(" ")
            )
            .unwrap();
            for g in &st.layout.glyphs {
                if let mc_notation::GlyphKind::Fret { fret, .. } = g.kind {
                    writeln!(out, "    fret {fret} {},{}", r1(g.x), r1(g.y)).unwrap();
                }
            }
            for h in st.layout.hits.iter().filter(|h| h.string == 1) {
                if let HitTarget::Beat { beat, .. } = h.target {
                    writeln!(
                        out,
                        "    beat bar={} #{beat} x={} w={}",
                        h.bar,
                        r1(h.rect.x),
                        r1(h.rect.w)
                    )
                    .unwrap();
                }
            }
        }
    }
    out
}

#[test]
fn m7_golden_two_of_three_tracks_with_a_repeat() {
    let (mut s, ids) = new_score(3, 3);
    let mut bars: Vec<MasterBar> = s.master_bars().to_vec();
    bars[1].repeat_start = true;
    bars[2].repeat_end = Some(2);
    for (i, b) in bars.into_iter().enumerate() {
        s.set_master_bar(i, b).unwrap();
    }
    for bar in 0..3 {
        for k in 0..4u8 {
            beat(&mut s, ids[0], bar, v(2), &[(6, k)]); // quarters
        }
        for k in 0..8u8 {
            beat(&mut s, ids[2], bar, v(3), &[(1, k + 10)]); // eighths, two-digit frets
        }
        beat(&mut s, ids[1], bar, v(0), &[(3, 5)]); // hidden track: a whole note
    }
    let l = layout_score(&s, &[ids[0], ids[2]], &LayoutMetrics::default()).unwrap();
    let actual = dump(&l);
    let expected = include_str!("golden/score_two_tracks.txt").replace("\r\n", "\n");
    if actual != expected {
        println!("----- BEGIN score_two_tracks -----\n{actual}----- END score_two_tracks -----");
        panic!("score layout golden differs; actual text printed above");
    }
    let again = dump(&layout_score(&s, &[ids[0], ids[2]], &LayoutMetrics::default()).unwrap());
    assert_eq!(actual, again, "deterministic");
}

/// ADR-031 fallback 2: the outline sizes every system exactly as the full layout does,
/// and laying out any range of systems gives exactly those systems of the full layout.
#[test]
fn windowed_layout_equals_the_full_layout() {
    let mut ranges = 0;
    for seed in 0..CASES {
        let (s, ids) = random_score(seed);
        let shown = subset(seed, &ids);
        let m = metrics_for(seed);
        let full = layout_score(&s, &shown, &m).unwrap();
        let outline = outline_score(&s, &shown, &m).unwrap();
        assert_eq!(outline.tracks, full.tracks);
        assert_eq!(
            outline.height.to_bits(),
            full.height.to_bits(),
            "seed {seed}"
        );
        assert_eq!(outline.systems.len(), full.systems.len());
        for (o, f) in outline.systems.iter().zip(&full.systems) {
            assert_eq!(
                (o.index, o.y.to_bits()),
                (f.index, f.y.to_bits()),
                "seed {seed}"
            );
            assert!((o.height - f.height).abs() < 1e-3, "seed {seed}");
            let bars = &f.staves[0].layout.bars;
            assert_eq!(o.first_bar, bars.first().unwrap().bar);
            assert_eq!(o.end_bar, bars.last().unwrap().bar + 1);
        }
        let n = full.systems.len();
        let mut rng = Rng(seed + 7);
        for _ in 0..3 {
            let a = rng.below(n as u64 + 1) as usize;
            let b = a + rng.below(4) as usize;
            let part = layout_score_systems(&s, &outline, a..b, &m).unwrap();
            let expected = &full.systems[a.min(n)..b.min(n)];
            assert_eq!(part, expected, "seed {seed} range {a}..{b}");
            ranges += 1;
        }
    }
    assert!(ranges >= 900);
}

#[test]
fn a_stale_outline_is_an_error() {
    let (mut s, ids) = new_score(2, 4);
    let m = LayoutMetrics::default();
    let outline = outline_score(&s, &ids, &m).unwrap();
    s.remove_master_bars(2, 2).unwrap();
    assert_eq!(
        layout_score_systems(&s, &outline, 0..10, &m),
        Err(NotationError::StaleOutline)
    );
    let (mut s, ids) = new_score(2, 4);
    let outline = outline_score(&s, &ids, &m).unwrap();
    s.remove_track(ids[1]).unwrap();
    assert_eq!(
        layout_score_systems(&s, &outline, 0..1, &m),
        Err(NotationError::UnknownTrack(ids[1]))
    );
}
