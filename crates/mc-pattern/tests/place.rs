//! ADR-034 PE-4 (timing), PE-8 at domain level (range and fit) and PT-09 (gaps are
//! rests): placing a bound fragment over whole master bars.

mod common;
use common::*;
use mc_guitar::Voicing;
use mc_pattern::*;
use mc_score::{FragmentBeat, FrettedInstrument, MasterBar, NoteEffects};
use mc_theory::fretted::Fret;
use mc_theory::{BaseValue, NoteValue, Rational, TimeSignature, Tuplet, chord_symbol};
use proptest::prelude::*;

use StringSelector::{Bass, Strum};

fn bars(sigs: &[(u8, u8)]) -> Vec<MasterBar> {
    sigs.iter()
        .map(|&(n, d)| MasterBar {
            time_signature: TimeSignature::new(n, d).unwrap(),
            ..MasterBar::default()
        })
        .collect()
}

fn four(n: usize) -> Vec<MasterBar> {
    bars(&vec![(4, 4); n])
}

fn bind_c(p: PatternParts) -> PatternFragment {
    let mut lib = PatternLibrary::new();
    let id = lib.define_pattern(p).unwrap();
    lib.bind(
        id,
        &"x32010".parse::<Voicing>().unwrap(),
        &chord_symbol::parse("C").unwrap(),
        &FrettedInstrument::standard_guitar(),
    )
    .unwrap()
}

fn total(beats: &[FragmentBeat]) -> Rational {
    beats.iter().fold(Rational::ZERO, |a, b| {
        a.checked_add(b.value.length()).unwrap()
    })
}

fn triplet(b: BaseValue) -> NoteValue {
    NoteValue::new(b, 0, Some(Tuplet::TRIPLET)).unwrap()
}

// ------------------------------------------------------------------- values

#[test]
fn values_for_writes_dyadic_and_triplet_lengths_exactly() {
    use BaseValue as B;
    let plain = NoteValue::plain;
    for (len, want) in [
        (r(1, 4), vec![plain(B::QUARTER)]),
        (r(3, 8), vec![plain(B::QUARTER), plain(B::EIGHTH)]),
        (r(3, 2), vec![plain(B::WHOLE), plain(B::HALF)]),
        (
            r(5, 2),
            vec![plain(B::WHOLE), plain(B::WHOLE), plain(B::HALF)],
        ),
        (r(1, 256), vec![plain(BaseValue::from_exponent(8).unwrap())]),
        (r(1, 6), vec![triplet(B::QUARTER)]),
        (r(1, 12), vec![triplet(B::EIGHTH)]),
        (r(5, 12), vec![plain(B::QUARTER), triplet(B::QUARTER)]),
        (r(7, 12), vec![plain(B::HALF), triplet(B::EIGHTH)]),
    ] {
        assert_eq!(values_for(len), Some(want), "{len}");
    }
    for len in [
        Rational::ZERO,
        r(-1, 4),
        r(1, 5),
        r(1, 9),
        r(1, 512),
        r(1, 768),
    ] {
        assert_eq!(values_for(len), None, "{len}");
    }
}

proptest! {
    #[test]
    fn values_for_sums_exactly(num in 1i64..2000, k in 0u32..9, three in any::<bool>()) {
        let den = (1i64 << k) * if three { 3 } else { 1 };
        let len = r(num, den);
        if let Some(vs) = values_for(len) {
            let sum = vs.iter().fold(Rational::ZERO, |a, v| a.checked_add(v.length()).unwrap());
            prop_assert_eq!(sum, len);
            prop_assert!(vs.iter().filter(|v| v.tuplet().is_some()).count() <= 1);
        } else {
            // Guaranteed writable: dyadic lengths down to 1/256, and triplet lengths with
            // a denominator up to 3 * 64 (the triplet then has a base of 1/128 or
            // longer). Anything finer may be refused.
            let d = len.denominator();
            let guaranteed = if d % 3 == 0 { d <= 3 * 64 } else { d <= 256 };
            prop_assert!(!guaranteed, "{} should be writable", len);
        }
    }
}

// ------------------------------------------------------------------- PT-09

#[test]
fn pt09_gaps_become_rests_and_every_bar_is_filled_exactly() {
    // Bass on beat 1, Strum on the "and" of 2: gaps before and after.
    let f = bind_c(pattern(
        Rational::ONE,
        Lanes::Strum(Lane {
            steps: vec![
                step(r(0, 1), q(), strike(&[Bass])),
                step(r(3, 8), e(), strike(&[Strum])),
            ],
        }),
    ));
    let p = place(&f, &four(2), None).unwrap();
    assert_eq!(p.report.repetitions, 2);
    for bar in &p.bars {
        let (lane, beats) = &bar.voices[0];
        assert_eq!(*lane, LaneId::Strum);
        assert_eq!(total(beats), Rational::ONE);
        let shape: Vec<(Rational, bool)> = beats
            .iter()
            .map(|b| (b.value.length(), b.notes.is_empty()))
            .collect();
        assert_eq!(
            shape,
            vec![
                (r(1, 4), false),
                (r(1, 8), true),
                (r(1, 8), false),
                (r(1, 2), true)
            ],
            "quarter, eighth rest, eighth, half rest"
        );
    }
    assert_eq!(p.report.notes, 2 * (1 + 5));
}

// ------------------------------------------------------------------- PE-8

#[test]
fn pe8_a_whole_multiple_repeats_with_no_fit() {
    let f = bind_c(pattern(
        r(1, 2),
        Lanes::Strum(run(vec![(q(), strike(&[Bass])), (q(), strike(&[Strum]))])),
    ));
    let p = place(&f, &four(3), None).unwrap();
    assert_eq!(p.report.repetitions, 6);
    assert!(p.report.dropped.is_empty() && p.report.shortened.is_empty());
    assert_eq!(p.bars.len(), 3);
}

#[test]
fn pe8_any_other_length_needs_an_explicit_fit() {
    let f = bind_c(pattern(
        r(3, 4),
        Lanes::Strum(run(vec![
            (v(BaseValue::HALF), strike(&[Bass])),
            (q(), strike(&[Strum])),
        ])),
    ));
    assert_eq!(
        place(&f, &four(1), None).unwrap_err(),
        PlaceError::FitRequired {
            range: Rational::ONE,
            pattern: r(3, 4)
        }
    );
}

#[test]
fn pe8_repeat_and_cut_reports_exactly_what_was_cut() {
    // 3/4 pattern over one 4/4 bar: one repetition, then the second cut after a
    // quarter: its half note is shortened to a quarter, its quarter dropped.
    let f = bind_c(pattern(
        r(3, 4),
        Lanes::Strum(run(vec![
            (v(BaseValue::HALF), strike(&[Bass])),
            (q(), strike(&[Strum])),
        ])),
    ));
    let p = place(&f, &four(1), Some(Fit::RepeatAndCut)).unwrap();
    assert_eq!(p.report.repetitions, 2);
    assert_eq!(
        p.report.shortened,
        vec![StepChange {
            lane: LaneId::Strum,
            repetition: 1,
            step: 0,
            onset: r(0, 1),
            written: v(BaseValue::HALF),
            now: vec![q()]
        }]
    );
    assert_eq!(
        p.report.dropped,
        vec![StepChange {
            lane: LaneId::Strum,
            repetition: 1,
            step: 1,
            onset: r(1, 2),
            written: q(),
            now: vec![]
        }]
    );
    let beats = &p.bars[0].voices[0].1;
    assert_eq!(total(beats), Rational::ONE);
    assert_eq!(
        beats.iter().map(|b| b.value).collect::<Vec<_>>(),
        vec![v(BaseValue::HALF), q(), q()]
    );
}

#[test]
fn pe8_repeat_and_cut_shortens_with_tied_values_and_drops_tuplets() {
    // Pattern of 3/8: a dotted quarter. Over 1/2 (2/4): one repetition, then 1/8 left:
    // the dotted quarter becomes an eighth. Over 7/8 with a triplet group pattern, the
    // triplet that would need shortening is dropped and nothing else moves.
    let f = bind_c(pattern(
        r(3, 8),
        Lanes::Strum(run(vec![(dotted_q(), strike(&[Bass]))])),
    ));
    let p = place(&f, &bars(&[(2, 4)]), Some(Fit::RepeatAndCut)).unwrap();
    assert_eq!(p.report.shortened[0].now, vec![e()]);
    let t = triplet(BaseValue::QUARTER);
    let g = bind_c(pattern(
        r(1, 2),
        Lanes::Strum(run(vec![
            (t, strike(&[Bass])),
            (t, strike(&[Strum])),
            (t, strike(&[Strum])),
        ])),
    ));
    let p = place(&g, &bars(&[(7, 8)]), Some(Fit::RepeatAndCut)).unwrap();
    // Second repetition starts at 1/2; its triplets end at 2/3, 5/6, 1 > 7/8:
    // the first fits (2/3 <= 7/8), the second ends at 5/6 <= 7/8 and fits, the third
    // starts at 5/6 and would end at 1: dropped (a tuplet cannot be cut).
    assert!(p.report.shortened.is_empty());
    assert_eq!(p.report.dropped.len(), 1);
    assert_eq!(
        (p.report.dropped[0].repetition, p.report.dropped[0].step),
        (1, 2)
    );
    let beats = &p.bars[0].voices[0].1;
    assert_eq!(total(beats), r(7, 8));
    assert_eq!(beats.iter().filter(|b| !b.notes.is_empty()).count(), 5);
}

#[test]
fn pe8_whole_repetitions_only_leaves_the_remainder_as_rests() {
    let f = bind_c(pattern(
        r(3, 4),
        Lanes::Strum(run(vec![
            (v(BaseValue::HALF), strike(&[Bass])),
            (q(), strike(&[Strum])),
        ])),
    ));
    let p = place(&f, &four(2), Some(Fit::WholeRepetitionsOnly)).unwrap();
    assert_eq!(p.report.repetitions, 2);
    assert_eq!(p.report.rests_from, Some(r(3, 2)));
    assert!(p.report.dropped.is_empty());
    let bar2 = &p.bars[1].voices[0].1;
    // The second repetition runs from 3/4 to 3/2: its half note crosses the bar line
    // (a quarter tied to a quarter) and its quarter ends at 3/2, which is 1/2 into
    // bar 2. Rests fill the rest of bar 2.
    assert_eq!(total(bar2), Rational::ONE);
    let last_sound_end = {
        let mut t = Rational::ZERO;
        let mut end = Rational::ZERO;
        for b in bar2 {
            t = t.checked_add(b.value.length()).unwrap();
            if !b.notes.is_empty() {
                end = t;
            }
        }
        end
    };
    assert_eq!(
        last_sound_end,
        r(1, 2),
        "nothing sounds after 3/2 in range time"
    );
    // A pattern longer than the range: zero repetitions, all rests.
    let long = bind_c(pattern(
        r(2, 1),
        Lanes::Strum(run(vec![(q(), strike(&[Bass]))])),
    ));
    let p = place(&long, &four(1), Some(Fit::WholeRepetitionsOnly)).unwrap();
    assert_eq!(p.report.repetitions, 0);
    assert!(p.bars[0].voices[0].1.iter().all(|b| b.notes.is_empty()));
}

#[test]
fn pe8_pickup_and_meter_are_checked() {
    let f = bind_c(pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![(q(), strike(&[Bass]))])),
    ));
    let mut b = four(3);
    b[1].pickup = true;
    assert_eq!(
        place(&f, &b, None).unwrap_err(),
        PlaceError::PickupInRange { bar: 1 }
    );
    let mut p = pattern(r(3, 4), Lanes::Strum(run(vec![(q(), strike(&[Bass]))])));
    p.meter = Some(TimeSignature::new(3, 4).unwrap());
    let f = bind_c(p);
    assert_eq!(
        place(&f, &bars(&[(3, 4), (4, 4), (3, 4)]), None).unwrap_err(),
        PlaceError::MeterMismatch {
            bar: 1,
            meter: TimeSignature::new(3, 4).unwrap(),
            found: TimeSignature::COMMON,
        }
    );
    assert!(place(&f, &bars(&[(3, 4), (3, 4)]), None).is_ok());
    assert_eq!(place(&f, &[], None).unwrap_err(), PlaceError::EmptyRange);
}

#[test]
fn pe8_a_pattern_without_meter_runs_on_across_mixed_time_signatures() {
    // Seven quarters over a 4/4 and a 3/4 bar: one repetition, split at the bar line.
    let f = bind_c(pattern(
        r(7, 4),
        Lanes::Strum(run(vec![
            (v(BaseValue::HALF), strike(&[Bass])),
            (v(BaseValue::HALF), strike(&[Strum])),
            (dotted_q(), strike(&[Bass])),
            (e(), strike(&[Bass])),
            (q(), strike(&[Strum])),
        ])),
    ));
    let p = place(&f, &bars(&[(4, 4), (3, 4)]), None).unwrap();
    assert_eq!(p.report.repetitions, 1);
    assert_eq!(total(&p.bars[0].voices[0].1), Rational::ONE);
    assert_eq!(total(&p.bars[1].voices[0].1), r(3, 4));
}

// ------------------------------------------------------------- bar lines

#[test]
fn pe4_a_step_across_a_bar_line_is_tied_and_sums_exactly() {
    // A dotted half from beat 3 in 4/4: a half in bar 1, tied to a quarter in bar 2.
    let mut k = Strike::on(vec![Bass]);
    k.techniques = NoteEffects {
        accent: true,
        let_ring: true,
        ..NoteEffects::default()
    };
    k.direction = Some(Direction::Down);
    let dotted_half = NoteValue::new(BaseValue::HALF, 1, None).unwrap();
    let f = bind_c(pattern(
        r(2, 1),
        Lanes::Strum(Lane {
            steps: vec![step(r(1, 2), dotted_half, Action::Strike(k))],
        }),
    ));
    let p = place(&f, &four(2), None).unwrap();
    let b1 = &p.bars[0].voices[0].1;
    let b2 = &p.bars[1].voices[0].1;
    let first = b1.last().unwrap();
    let cont = b2.iter().find(|b| !b.notes.is_empty()).unwrap();
    assert_eq!((first.value, cont.value), (v(BaseValue::HALF), q()));
    assert!(!first.notes[0].tie && first.notes[0].effects.accent);
    assert_eq!(first.stroke, Some(Direction::Down));
    assert!(cont.notes[0].tie);
    assert_eq!(
        cont.notes[0].effects,
        NoteEffects {
            let_ring: true,
            ..NoteEffects::default()
        }
    );
    assert_eq!(cont.stroke, None);
    assert_eq!(cont.notes[0].position, first.notes[0].position);
}

#[test]
fn pe4_a_tuplet_step_across_a_bar_line_is_rejected() {
    let t = triplet(BaseValue::HALF); // 1/3
    let f = bind_c(pattern(
        r(2, 1),
        Lanes::Strum(Lane {
            steps: vec![step(r(5, 6), t, strike(&[Bass]))],
        }),
    ));
    assert_eq!(
        place(&f, &four(2), None).unwrap_err(),
        PlaceError::TupletCrossesBar {
            lane: LaneId::Strum,
            repetition: 0,
            step: 0,
            bar: 0
        }
    );
}

#[test]
fn a_short_note_across_a_bar_line_is_rejected() {
    let mut k = Strike::on(vec![Bass]);
    k.techniques.palm_mute = true;
    let f = bind_c(pattern(
        r(2, 1),
        Lanes::Strum(Lane {
            steps: vec![step(r(7, 8), q(), Action::Strike(k))],
        }),
    ));
    assert_eq!(
        place(&f, &four(2), None).unwrap_err(),
        PlaceError::ShortNoteNeedsTie {
            lane: LaneId::Strum,
            repetition: 0,
            step: 0,
            bar: 0
        }
    );
}

#[test]
fn a_gap_that_cannot_be_written_is_a_typed_error() {
    // One quintuplet sixteenth (1/20) leaves 19/20: no note values write it.
    let five = NoteValue::new(BaseValue::SIXTEENTH, 0, Some(Tuplet::new(5, 4).unwrap())).unwrap();
    let f = bind_c(pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![(five, strike(&[Bass]))])),
    ));
    assert_eq!(
        place(&f, &four(1), None).unwrap_err(),
        PlaceError::UnwritableLength {
            lane: LaneId::Strum,
            at: r(1, 20),
            length: r(19, 20)
        }
    );
}

#[test]
fn two_lanes_give_two_voices_with_independent_durations() {
    // Travis-style (corpus 2): the thumb's quarters sound under the finger eighths.
    let f = bind_c(pattern(
        Rational::ONE,
        Lanes::ThumbFingers {
            thumb: run(vec![
                (q(), strike(&[Bass])),
                (q(), strike(&[StringSelector::Alt])),
                (q(), strike(&[Bass])),
                (q(), strike(&[StringSelector::Alt])),
            ]),
            fingers: run((0..8)
                .map(|i| (e(), strike(&[s(if i % 2 == 0 { 1 } else { 2 })])))
                .collect()),
        },
    ));
    let p = place(&f, &four(1), None).unwrap();
    let voices = &p.bars[0].voices;
    assert_eq!(
        voices.iter().map(|(l, _)| *l).collect::<Vec<_>>(),
        vec![LaneId::Thumb, LaneId::Fingers]
    );
    assert!(voices[0].1.iter().all(|b| b.value == q()));
    assert!(voices[1].1.iter().all(|b| b.value == e()));
}

// ------------------------------------------------------------ PE-4 property

/// One sounding event read back from placed bars: start, total length, notes of the
/// attack beat, stroke, and whether its first beat is tied (a hold).
#[derive(Debug, PartialEq)]
struct ReadEvent {
    start: Rational,
    length: Rational,
    first: FragmentBeat,
}

/// Reads placed bars back into events. A beat whose notes are all tied, on the same
/// positions as the previous beat, directly after it, continues that event unless it
/// starts a new expected step (`step_starts` tells holds from bar-line continuations).
fn read_back(
    bars: &[PlacedBar],
    voice: usize,
    step_starts: &std::collections::BTreeSet<Rational>,
) -> Vec<ReadEvent> {
    let mut out: Vec<ReadEvent> = Vec::new();
    let mut t = Rational::ZERO;
    let mut prev_end: Option<Rational> = None;
    for bar in bars {
        for b in &bar.voices[voice].1 {
            let len = b.value.length();
            if !b.notes.is_empty() {
                let continues = prev_end == Some(t)
                    && !step_starts.contains(&t)
                    && b.notes.iter().all(|n| n.tie)
                    && out.last().is_some_and(|e| {
                        e.first
                            .notes
                            .iter()
                            .map(|n| n.position)
                            .eq(b.notes.iter().map(|n| n.position))
                    });
                if continues {
                    out.last_mut().unwrap().length =
                        out.last().unwrap().length.checked_add(len).unwrap();
                } else {
                    out.push(ReadEvent {
                        start: t,
                        length: len,
                        first: b.clone(),
                    });
                }
                prev_end = Some(t.checked_add(len).unwrap());
            } else {
                prev_end = None;
            }
            t = t.checked_add(len).unwrap();
        }
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 300, .. ProptestConfig::default() })]

    /// PE-4: over whole repetitions, every note starts at range start + repetition
    /// offset + step onset, lasts its step duration (tied pieces summing exactly), and
    /// keeps its bound notes and stroke; every bar is filled exactly; failures are only
    /// the typed bar-line errors, and only when a step really crosses a bar line.
    #[test]
    fn pe4_placed_timing_matches_the_ir(seed in pattern_seeds(), frets in proptest::collection::vec(0u8..13, 6), extra in 1usize..3) {
        let mut p = build_pattern(&seed, &[]);
        p.meter = None;
        // No short technique, so holds bind (tested in bind.rs); spans kept if they
        // reach an attack.
        let clear = |k: &mut Strike| { k.techniques.dead = false; k.techniques.palm_mute = false; k.techniques.string_percussive = false; };
        match &mut p.lanes {
            Lanes::Strum(l) => l.steps.iter_mut().for_each(|s| if let Action::Strike(k) = &mut s.action { clear(k) }),
            Lanes::ThumbFingers { thumb, fingers } => {
                thumb.steps.iter_mut().chain(fingers.steps.iter_mut()).for_each(|s| if let Action::Strike(k) = &mut s.action { clear(k) })
            }
        }
        p.spans.retain(|s| matches!(s.technique, Technique::Accent | Technique::LetRing));
        let lanes = p.lanes.clone();
        p.spans.retain(|s| (s.first..=s.last).any(|k| matches!(lanes.get(s.lane).unwrap().steps[k].action, Action::Strike(_))));
        let mut lib = PatternLibrary::new();
        let id = lib.define_pattern(p.clone()).unwrap();
        let voicing = Voicing::new(frets.into_iter().map(|f| Some(Fret(f))).collect()).unwrap();
        let f = lib.bind(id, &voicing, &chord_symbol::parse("C").unwrap(), &FrettedInstrument::standard_guitar()).unwrap();
        // Bars of 4/4 such that the range is a whole multiple of the pattern length.
        let len = p.length;
        let per = (len.numerator() * 4 / len.denominator()).max(1); // quarters, >= 1
        let quarters = per * (4 / gcd(per, 4)) * extra as i64; // a multiple of 4 and of per... lcm
        let n_bars = (quarters / 4) as usize;
        let range = r(n_bars as i64, 1);
        let reps = range.checked_div(len).unwrap();
        prop_assume!(reps.denominator() == 1);
        let reps = reps.numerator() as usize;
        match place(&f, &four(n_bars), None) {
            Ok(placed) => {
                prop_assert_eq!(placed.report.repetitions, reps);
                for bar in &placed.bars {
                    for (_, beats) in &bar.voices {
                        prop_assert_eq!(total(beats), Rational::ONE);
                    }
                }
                for (voice, bl) in f.lanes.iter().enumerate() {
                    let mut expected = Vec::new();
                    for rep in 0..reps {
                        let off = len.checked_mul(Rational::from_int(rep as i64)).unwrap();
                        for b in &bl.beats {
                            expected.push((off.checked_add(b.onset).unwrap(), b.duration.length(), b));
                        }
                    }
                    let starts = expected.iter().map(|e| e.0).collect();
                    let got = read_back(&placed.bars, voice, &starts);
                    prop_assert_eq!(got.len(), expected.len());
                    for (g, (start, length, b)) in got.iter().zip(&expected) {
                        prop_assert_eq!(g.start, *start);
                        prop_assert_eq!(g.length, *length);
                        prop_assert_eq!(g.first.stroke, b.stroke);
                        let notes: Vec<_> = g.first.notes.iter().map(|n| (n.position, n.tie, n.effects)).collect();
                        let want: Vec<_> = b.notes.iter().map(|n| (n.position, n.tie, n.effects)).collect();
                        prop_assert_eq!(notes, want);
                    }
                }
            }
            Err(PlaceError::TupletCrossesBar { lane, repetition, step, .. }) => {
                let bl = f.lanes.iter().find(|l| l.lane == lane).unwrap();
                let b = bl.beats.iter().find(|b| b.step == step).unwrap();
                prop_assert!(b.duration.tuplet().is_some());
                let start = len.checked_mul(Rational::from_int(repetition as i64)).unwrap().checked_add(b.onset).unwrap();
                let end = start.checked_add(b.duration.length()).unwrap();
                let bar_end = r(start.numerator() / start.denominator() + 1, 1);
                prop_assert!(end > bar_end, "the tuplet really crosses a bar line");
            }
            Err(PlaceError::UnwritableLength { .. }) => {} // a gap after an incomplete tuplet group
            Err(other) => return Err(TestCaseError::fail(format!("{other:?}"))),
        }
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// The PE-4 property is not vacuous: most cases place, and placed cases include steps
/// split at bar lines and holds.
#[test]
fn the_placement_property_mostly_places_and_covers_splits_and_holds() {
    use proptest::strategy::ValueTree;
    let mut runner = proptest::test_runner::TestRunner::deterministic();
    let mut seen = std::collections::BTreeMap::<&str, usize>::new();
    let voicing: Voicing = "320003".parse().unwrap();
    for _ in 0..400 {
        let seed = pattern_seeds().new_tree(&mut runner).unwrap().current();
        let mut p = build_pattern(&seed, &[]);
        p.meter = None;
        let strip = |l: &mut Lane| {
            for s in &mut l.steps {
                if let Action::Strike(k) = &mut s.action {
                    k.techniques = NoteEffects::default();
                }
            }
        };
        match &mut p.lanes {
            Lanes::Strum(l) => strip(l),
            Lanes::ThumbFingers { thumb, fingers } => {
                strip(thumb);
                strip(fingers);
            }
        }
        p.spans.clear();
        let mut lib = PatternLibrary::new();
        let id = lib.define_pattern(p.clone()).unwrap();
        let f = lib
            .bind(
                id,
                &voicing,
                &chord_symbol::parse("G").unwrap(),
                &FrettedInstrument::standard_guitar(),
            )
            .unwrap();
        let n_bars = match (p.length.numerator(), p.length.denominator()) {
            (3, 4) => 3,
            (2, 1) => 2,
            _ => 1,
        };
        match place(&f, &four(n_bars), None) {
            Ok(placed) => {
                *seen.entry("placed").or_default() += 1;
                let beats: Vec<&FragmentBeat> = placed
                    .bars
                    .iter()
                    .flat_map(|b| b.voices.iter().flat_map(|v| &v.1))
                    .collect();
                let ties = beats
                    .iter()
                    .filter(|b| b.notes.iter().any(|n| n.tie))
                    .count();
                let holds = f
                    .lanes
                    .iter()
                    .flat_map(|l| &l.beats)
                    .filter(|b| b.notes.iter().any(|n| n.tie))
                    .count();
                if ties > holds * placed.report.repetitions {
                    *seen.entry("placed with a bar-line split").or_default() += 1;
                }
                if holds > 0 {
                    *seen.entry("placed with holds").or_default() += 1;
                }
            }
            Err(PlaceError::TupletCrossesBar { .. }) => {
                *seen.entry("tuplet crosses").or_default() += 1
            }
            Err(PlaceError::UnwritableLength { .. }) => *seen.entry("unwritable").or_default() += 1,
            Err(other) => panic!("{other:?}"),
        }
    }
    println!("{seen:?}");
    for key in [
        "placed",
        "placed with a bar-line split",
        "placed with holds",
    ] {
        assert!(
            seen.get(key).copied().unwrap_or(0) > 0,
            "{key} never seen: {seen:?}"
        );
    }
    assert!(seen["placed"] >= 200, "at least half place: {seen:?}");
}

#[test]
fn pe8_repeat_and_cut_drops_a_step_that_starts_exactly_at_the_range_end() {
    // Three quarters (3/4) over one 4/4 bar: the second repetition starts at 3/4; its
    // first quarter fills the bar, its second starts exactly at the end: dropped, as is
    // the third. Nothing is shortened.
    let f = bind_c(pattern(
        r(3, 4),
        Lanes::Strum(run(vec![
            (q(), strike(&[Bass])),
            (q(), strike(&[Strum])),
            (q(), strike(&[Strum])),
        ])),
    ));
    let p = place(&f, &four(1), Some(Fit::RepeatAndCut)).unwrap();
    assert!(p.report.shortened.is_empty());
    assert_eq!(
        p.report
            .dropped
            .iter()
            .map(|d| (d.repetition, d.step, d.onset))
            .collect::<Vec<_>>(),
        vec![(1, 1, r(1, 4)), (1, 2, r(1, 2))]
    );
    assert_eq!(p.bars[0].voices[0].1.len(), 4);
}

#[test]
fn the_note_count_counts_attacks_not_tied_continuations() {
    // A strike of Bass + Treble (4 notes on C), held for a beat, then a dotted half
    // across the bar line: 4 + 1 attacked notes; the hold and the split add none.
    let f = bind_c(pattern(
        r(2, 1),
        Lanes::Strum(Lane {
            steps: vec![
                step(r(0, 1), q(), strike(&[Bass, StringSelector::TREBLE])),
                step(r(1, 4), q(), Action::Hold(None)),
                step(
                    r(1, 2),
                    NoteValue::new(BaseValue::HALF, 1, None).unwrap(),
                    strike(&[Bass]),
                ),
            ],
        }),
    ));
    let p = place(&f, &four(2), None).unwrap();
    assert_eq!(p.report.notes, 5);
}

#[test]
fn a_cut_short_note_that_needs_several_values_is_rejected_like_a_tie() {
    // A palm-muted half note in a 5/8 pattern, over one 4/4 bar: the second repetition
    // starts at 5/8, so the cut leaves 3/8, which needs a quarter plus an eighth. The
    // second value would be a tied second attack of a short note (ADR-024, ADR-035), so
    // the cut is refused, although no bar line is crossed.
    let mut k = Strike::on(vec![Bass]);
    k.techniques.palm_mute = true;
    let f = bind_c(pattern(
        r(5, 8),
        Lanes::Strum(Lane {
            steps: vec![step(r(0, 1), v(BaseValue::HALF), Action::Strike(k))],
        }),
    ));
    assert_eq!(
        place(&f, &four(1), Some(Fit::RepeatAndCut)).unwrap_err(),
        PlaceError::ShortNoteNeedsTie {
            lane: LaneId::Strum,
            repetition: 1,
            step: 0,
            bar: 0
        }
    );
    // The same cut of a normal note is fine: it is written as tied values.
    let f = bind_c(pattern(
        r(5, 8),
        Lanes::Strum(Lane {
            steps: vec![step(r(0, 1), v(BaseValue::HALF), strike(&[Bass]))],
        }),
    ));
    let p = place(&f, &four(1), Some(Fit::RepeatAndCut)).unwrap();
    assert_eq!(p.report.shortened[0].now, vec![q(), e()]);
    // A short note cut to a single value is kept (shortened, not refused).
    let mut k = Strike::on(vec![Bass]);
    k.techniques.palm_mute = true;
    let f = bind_c(pattern(
        r(7, 8),
        Lanes::Strum(Lane {
            steps: vec![step(r(0, 1), v(BaseValue::HALF), Action::Strike(k))],
        }),
    ));
    let p = place(&f, &four(1), Some(Fit::RepeatAndCut)).unwrap();
    assert_eq!(p.report.shortened[0].now, vec![e()]);
}
