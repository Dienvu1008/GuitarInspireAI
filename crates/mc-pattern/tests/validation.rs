//! ADR-034 PE-2: semantic validation accepts every valid IR and rejects each
//! violation of PT-01 and PT-03 to PT-07 with a typed error naming the step. PT-08 and
//! the library are in tests/library.rs.

mod common;
use common::*;
use mc_pattern::*;
use mc_theory::{BaseValue, NoteValue, Rational, TimeSignature, Tuplet};
use proptest::prelude::*;

use StringSelector::{Alt, Bass, Strum};
use proptest::strategy::ValueTree;

const TREBLE: StringSelector = StringSelector::TREBLE;

fn at(lane: LaneId, index: usize, onset: Rational) -> StepRef {
    StepRef { lane, index, onset }
}

fn lib() -> PatternLibrary {
    PatternLibrary::new()
}

// ------------------------------------------------------------- the corpus as IR

/// The confirmed corpus patterns (pattern-corpus-proposed.md), written as IR by hand.
/// They must all be valid. Binding them to TAB is 6a-3 and 6a-4 (PE-11).
fn corpus() -> Vec<(&'static str, PatternParts)> {
    const D: Direction = Direction::Down;
    const U: Direction = Direction::Up;
    let treble4 = StringSelector::Treble(4);
    let accent = |a: Action| match a {
        Action::Strike(mut k) => {
            k.techniques.accent = true;
            Action::Strike(k)
        }
        other => other,
    };
    let sp = |a: Action| match a {
        Action::Strike(mut k) => {
            k.techniques.string_percussive = true;
            Action::Strike(k)
        }
        other => other,
    };
    // 2: fingers with rests as gaps (PT-09), thumb a quarter each.
    let fingers2 = Lane {
        steps: vec![
            step(r(0, 1), e(), strike(&[s(1)])),
            step(r(1, 8), e(), strike(&[s(2)])),
            step(r(3, 8), e(), strike(&[s(1)])),
            step(r(5, 8), e(), strike(&[s(2)])),
            step(r(7, 8), e(), strike(&[s(1)])),
        ],
    };
    // 4: beat 3 is a true rest, written as a gap.
    let strum4 = Lane {
        steps: vec![
            step(r(0, 1), q(), stroke(D, &[Strum])),
            step(r(1, 4), e(), stroke(D, &[Strum])),
            step(r(3, 8), e(), stroke(U, &[treble4])),
            step(r(5, 8), e(), stroke(U, &[treble4])),
            step(r(3, 4), e(), stroke(D, &[Strum])),
            step(r(7, 8), e(), stroke(U, &[treble4])),
        ],
    };
    let mut p8 = pattern(
        Rational::ONE,
        Lanes::Strum(run((0..8)
            .map(|i| {
                let a = strike(&[Bass]);
                (e(), if i % 4 == 0 { accent(a) } else { a })
            })
            .collect())),
    );
    p8.spans.push(TechniqueSpan {
        lane: LaneId::Strum,
        first: 0,
        last: 7,
        technique: Technique::PalmMute,
    });
    let mut p6 = pattern(
        r(3, 4),
        Lanes::Strum(run(vec![
            (q(), strike(&[Bass])),
            (q(), stroke(D, &[TREBLE])),
            (q(), stroke(D, &[TREBLE])),
        ])),
    );
    p6.meter = Some(TimeSignature::new(3, 4).unwrap());
    let mut p10 = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![
            (q(), strike(&[Bass])),
            (q(), strike(&[TREBLE])),
            (q(), strike(&[Bass])),
            (e(), strike(&[TREBLE])),
            (e(), strike(&[TREBLE])),
        ])),
    );
    p10.meter = Some(TimeSignature::new(2, 4).unwrap());
    vec![
        (
            "1",
            pattern(
                Rational::ONE,
                Lanes::Strum(run([Bass, s(3), s(2), s(3), s(1), s(3), s(2), s(3)]
                    .iter()
                    .map(|t| (e(), strike(&[*t])))
                    .collect())),
            ),
        ),
        (
            "2",
            pattern(
                Rational::ONE,
                Lanes::ThumbFingers {
                    thumb: run(vec![
                        (q(), strike(&[Bass])),
                        (q(), strike(&[Alt])),
                        (q(), strike(&[Bass])),
                        (q(), strike(&[Alt])),
                    ]),
                    fingers: fingers2,
                },
            ),
        ),
        (
            "3",
            pattern(
                Rational::ONE,
                Lanes::Strum(run(vec![
                    (q(), strike(&[Bass])),
                    (q(), stroke(D, &[TREBLE])),
                    (q(), strike(&[Alt])),
                    (q(), stroke(D, &[TREBLE])),
                ])),
            ),
        ),
        ("4", pattern(Rational::ONE, Lanes::Strum(strum4))),
        (
            "5",
            pattern(
                Rational::ONE,
                Lanes::Strum(run((0..8)
                    .map(|i| {
                        (
                            e(),
                            if i % 2 == 0 {
                                stroke(D, &[Strum])
                            } else {
                                stroke(U, &[treble4])
                            },
                        )
                    })
                    .collect())),
            ),
        ),
        ("6", p6),
        ("8", p8),
        (
            "9",
            pattern(
                Rational::ONE,
                Lanes::Strum(run(vec![
                    (q(), stroke(D, &[Strum])),
                    (q(), sp(accent(stroke(D, &[Strum])))),
                    (e(), stroke(D, &[Strum])),
                    (e(), stroke(U, &[treble4])),
                    (q(), sp(accent(stroke(D, &[Strum])))),
                ])),
            ),
        ),
        ("10", p10),
    ]
}

#[test]
fn pe2_every_confirmed_corpus_pattern_is_valid_ir() {
    let mut lib = lib();
    for (name, p) in corpus() {
        assert_eq!(errors(&lib, &p), vec![], "corpus pattern {name}");
        lib.define_pattern(p)
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    }
    assert_eq!(lib.patterns().count(), 9);
}

// ------------------------------------------------------------ one test per rule

#[test]
fn pt01_a_step_may_end_exactly_at_the_pattern_end_but_not_after() {
    let ok = pattern(
        r(1, 2),
        Lanes::Strum(run(vec![(q(), strike(&[Bass])), (q(), strike(&[Bass]))])),
    );
    assert_eq!(errors(&lib(), &ok), vec![]);
    let mut late = ok.clone();
    let Lanes::Strum(lane) = &mut late.lanes else {
        unreachable!()
    };
    lane.steps[1].duration = dotted_q();
    assert_eq!(
        errors(&lib(), &late),
        vec![PatternError::OutsidePattern {
            step: at(LaneId::Strum, 1, r(1, 4)),
            end: r(5, 8),
            length: r(1, 2),
        }]
    );
}

#[test]
fn pt01_the_length_must_be_positive() {
    for length in [Rational::ZERO, r(-1, 4)] {
        let p = pattern(length, Lanes::Strum(Lane::default()));
        assert_eq!(
            errors(&lib(), &p),
            vec![PatternError::NonPositiveLength { length }]
        );
    }
}

#[test]
fn pt01_a_multi_bar_pattern_and_a_tuplet_fit_exactly() {
    let t = NoteValue::new(BaseValue::EIGHTH, 0, Some(Tuplet::TRIPLET)).unwrap();
    // Three triplet eighths fill one quarter exactly; a pattern of two bars.
    let p = pattern(
        r(2, 1),
        Lanes::Strum(run(vec![
            (t, strike(&[Bass])),
            (t, strike(&[TREBLE])),
            (t, strike(&[TREBLE])),
            (v(BaseValue::WHOLE), strike(&[Strum])),
        ])),
    );
    assert_eq!(errors(&lib(), &p), vec![]);
}

#[test]
fn pt03_steps_of_a_lane_are_in_onset_order() {
    let p = pattern(
        Rational::ONE,
        Lanes::Strum(Lane {
            steps: vec![
                step(r(1, 2), q(), strike(&[Bass])),
                step(r(1, 4), q(), strike(&[Bass])),
            ],
        }),
    );
    assert_eq!(
        errors(&lib(), &p),
        vec![PatternError::OutOfOrder {
            step: at(LaneId::Strum, 1, r(1, 4)),
            previous_onset: r(1, 2),
        }]
    );
}

#[test]
fn pt03_steps_of_one_lane_do_not_overlap_but_lanes_may() {
    let overlap = pattern(
        Rational::ONE,
        Lanes::Strum(Lane {
            steps: vec![
                step(r(0, 1), q(), strike(&[Bass])),
                step(r(1, 8), e(), strike(&[TREBLE])),
            ],
        }),
    );
    assert_eq!(
        errors(&lib(), &overlap),
        vec![PatternError::Overlap {
            step: at(LaneId::Strum, 1, r(1, 8)),
            previous_end: r(1, 4),
        }]
    );
    // Two steps at the same onset in one lane also overlap.
    let same = pattern(
        Rational::ONE,
        Lanes::Strum(Lane {
            steps: vec![
                step(r(0, 1), e(), strike(&[Bass])),
                step(r(0, 1), e(), strike(&[Bass])),
            ],
        }),
    );
    assert!(matches!(
        errors(&lib(), &same)[..],
        [PatternError::Overlap { .. }]
    ));
    // A half-note bass in the thumb lane under eighths in the fingers lane is fine (Q-03).
    let lanes = pattern(
        r(1, 2),
        Lanes::ThumbFingers {
            thumb: run(vec![(v(BaseValue::HALF), strike(&[Bass]))]),
            fingers: run(vec![
                (e(), strike(&[s(1)])),
                (e(), strike(&[s(2)])),
                (e(), strike(&[s(1)])),
                (e(), strike(&[s(2)])),
            ]),
        },
    );
    assert_eq!(errors(&lib(), &lanes), vec![]);
}

#[test]
fn pt04_onsets_are_non_negative() {
    let p = pattern(
        Rational::ONE,
        Lanes::Strum(Lane {
            steps: vec![step(r(-1, 8), e(), strike(&[Bass]))],
        }),
    );
    assert_eq!(
        errors(&lib(), &p),
        vec![PatternError::NegativeOnset {
            step: at(LaneId::Strum, 0, r(-1, 8)),
        }]
    );
}

#[test]
fn pt05_a_hold_follows_a_strike_or_a_hold_without_a_gap() {
    let hold = |sel: Option<Vec<StringSelector>>| Action::Hold(sel);
    // Strike, hold all, hold one: valid.
    let ok = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![
            (q(), strike(&[Bass, TREBLE])),
            (q(), hold(None)),
            (q(), hold(Some(vec![Bass]))),
        ])),
    );
    assert_eq!(errors(&lib(), &ok), vec![]);
    // First step of the lane.
    let first = pattern(Rational::ONE, Lanes::Strum(run(vec![(q(), hold(None))])));
    assert_eq!(
        errors(&lib(), &first),
        vec![PatternError::HoldWithoutStrike {
            step: at(LaneId::Strum, 0, r(0, 1)),
        }]
    );
    // After a rest.
    let rest = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![
            (q(), strike(&[Bass])),
            (q(), Action::Rest),
            (q(), hold(None)),
        ])),
    );
    assert_eq!(
        errors(&lib(), &rest),
        vec![PatternError::HoldWithoutStrike {
            step: at(LaneId::Strum, 2, r(1, 2)),
        }]
    );
    // After a gap (a gap is a rest, PT-09).
    let gap = pattern(
        Rational::ONE,
        Lanes::Strum(Lane {
            steps: vec![
                step(r(0, 1), q(), strike(&[Bass])),
                step(r(1, 2), q(), hold(None)),
            ],
        }),
    );
    assert_eq!(
        errors(&lib(), &gap),
        vec![PatternError::HoldWithoutStrike {
            step: at(LaneId::Strum, 1, r(1, 2)),
        }]
    );
}

#[test]
fn pt05_a_hold_selects_only_strings_the_held_strike_certainly_plays() {
    let hold = |sel: Vec<StringSelector>| Action::Hold(Some(sel));
    let check = |first: Vec<StringSelector>, held: Vec<StringSelector>| {
        errors(
            &lib(),
            &pattern(
                Rational::ONE,
                Lanes::Strum(run(vec![(q(), strike(&first)), (q(), hold(held))])),
            ),
        )
    };
    // Contained for every voicing.
    assert_eq!(check(vec![Strum], vec![s(1), Bass, Alt]), vec![]);
    assert_eq!(check(vec![StringSelector::Treble(4)], vec![TREBLE]), vec![]);
    assert_eq!(
        check(
            vec![StringSelector::StrumRange(
                mc_theory::fretted::StringIndex::new(4).unwrap(),
                mc_theory::fretted::StringIndex::new(1).unwrap()
            )],
            vec![s(2)]
        ),
        vec![]
    );
    // Not contained, or only for some voicings: rejected, naming the selector.
    for (first, held) in [
        (vec![Bass], TREBLE),
        (vec![TREBLE], s(1)),
        (vec![TREBLE], StringSelector::Treble(4)),
        (vec![s(2)], s(1)),
    ] {
        assert_eq!(
            check(first.clone(), vec![held]),
            vec![PatternError::HoldNotInStrike {
                step: at(LaneId::Strum, 1, r(1, 4)),
                selector: held,
            }],
            "{first:?} then hold {held:?}"
        );
    }
    // A hold narrows the next one: after holding Bass, Treble is no longer held.
    let narrowed = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![
            (q(), strike(&[Bass, TREBLE])),
            (q(), hold(vec![Bass])),
            (q(), hold(vec![TREBLE])),
        ])),
    );
    assert_eq!(
        errors(&lib(), &narrowed),
        vec![PatternError::HoldNotInStrike {
            step: at(LaneId::Strum, 2, r(1, 2)),
            selector: TREBLE,
        }]
    );
    // An empty hold selects nothing.
    assert_eq!(
        check(vec![Bass], vec![]),
        vec![PatternError::EmptyTargets {
            at: StrikeAt::Step(at(LaneId::Strum, 1, r(1, 4))),
        }]
    );
}

#[test]
fn pt05_a_hold_after_a_gesture_uses_the_gesture_targets() {
    let (lib, g) = gesture_library();
    let (xl, _) = g[2]; // Bass + Treble
    let p = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![
            (q(), Action::Gesture(xl)),
            (q(), Action::Hold(Some(vec![TREBLE]))),
            (q(), Action::Hold(Some(vec![s(1)]))),
        ])),
    );
    assert_eq!(
        errors(&lib, &p),
        vec![PatternError::HoldNotInStrike {
            step: at(LaneId::Strum, 2, r(1, 2)),
            selector: s(1),
        }]
    );
}

#[test]
fn pt06_a_gesture_reference_must_resolve() {
    let (lib, _) = gesture_library();
    let ghost = GestureId::from_raw(999);
    let p = pattern(
        Rational::ONE,
        Lanes::ThumbFingers {
            thumb: run(vec![(q(), strike(&[Bass]))]),
            fingers: Lane {
                steps: vec![step(r(1, 8), e(), Action::Gesture(ghost))],
            },
        },
    );
    assert_eq!(
        errors(&lib, &p),
        vec![PatternError::UnknownGesture {
            step: at(LaneId::Fingers, 0, r(1, 8)),
            gesture: ghost,
        }]
    );
    // A hold after an unknown gesture is not reported a second time.
    let hold = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![
            (q(), Action::Gesture(ghost)),
            (q(), Action::Hold(None)),
        ])),
    );
    assert_eq!(errors(&lib, &hold).len(), 1);
}

#[test]
fn pt07_spans_cover_existing_steps_of_one_lane() {
    let base = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![
            (q(), strike(&[Bass])),
            (q(), Action::Rest),
            (q(), strike(&[Strum])),
        ])),
    );
    let with = |lane, first, last| {
        let mut p = base.clone();
        p.spans.push(TechniqueSpan {
            lane,
            first,
            last,
            technique: Technique::PalmMute,
        });
        p
    };
    assert_eq!(errors(&lib(), &with(LaneId::Strum, 0, 2)), vec![]);
    assert_eq!(errors(&lib(), &with(LaneId::Strum, 1, 1)), vec![]);
    assert_eq!(
        errors(&lib(), &with(LaneId::Strum, 1, 3)),
        vec![PatternError::SpanOutOfRange {
            span: 0,
            lane: LaneId::Strum,
            first: 1,
            last: 3,
            steps: 3,
        }]
    );
    assert_eq!(
        errors(&lib(), &with(LaneId::Strum, 2, 1)),
        vec![PatternError::SpanOutOfRange {
            span: 0,
            lane: LaneId::Strum,
            first: 2,
            last: 1,
            steps: 3,
        }]
    );
    assert_eq!(
        errors(&lib(), &with(LaneId::Fingers, 0, 0)),
        vec![PatternError::SpanLaneMissing {
            span: 0,
            lane: LaneId::Fingers,
        }]
    );
}

#[test]
fn pt07_combinations_are_not_rejected_and_every_flag_is_kept() {
    let mut all = Strike::on(vec![Strum]);
    all.techniques = TechniqueSet {
        accent: true,
        palm_mute: true,
        dead: true,
        let_ring: true,
        string_percussive: true,
    };
    let mut p = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![(q(), Action::Strike(all))])),
    );
    for technique in [
        Technique::Accent,
        Technique::PalmMute,
        Technique::Dead,
        Technique::LetRing,
        Technique::StringPercussive,
    ] {
        p.spans.push(TechniqueSpan {
            lane: LaneId::Strum,
            first: 0,
            last: 0,
            technique,
        });
    }
    let mut lib = lib();
    let id = lib.define_pattern(p.clone()).unwrap();
    assert_eq!(lib.pattern(id).unwrap().parts(), &p, "stored unchanged");
}

#[test]
fn strike_structure_needs_a_target_and_treble_at_least_one() {
    let p = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![
            (q(), strike(&[])),
            (q(), strike(&[StringSelector::Treble(0)])),
        ])),
    );
    assert_eq!(
        errors(&lib(), &p),
        vec![
            PatternError::EmptyTargets {
                at: StrikeAt::Step(at(LaneId::Strum, 0, r(0, 1))),
            },
            PatternError::InvalidSelector {
                at: StrikeAt::Step(at(LaneId::Strum, 1, r(1, 4))),
                selector: StringSelector::Treble(0),
            },
        ]
    );
}

#[test]
fn every_violation_is_reported_not_only_the_first() {
    let p = pattern(
        r(1, 2),
        Lanes::ThumbFingers {
            thumb: Lane {
                steps: vec![step(r(-1, 4), q(), strike(&[Bass]))],
            },
            fingers: Lane {
                steps: vec![step(r(1, 4), v(BaseValue::HALF), Action::Hold(None))],
            },
        },
    );
    let e = errors(&lib(), &p);
    assert_eq!(e.len(), 3, "{e:?}");
    assert!(
        e.iter()
            .any(|x| matches!(x, PatternError::NegativeOnset { .. }))
    );
    assert!(
        e.iter()
            .any(|x| matches!(x, PatternError::OutsidePattern { .. }))
    );
    assert!(
        e.iter()
            .any(|x| matches!(x, PatternError::HoldWithoutStrike { .. }))
    );
}

// ------------------------------------------------------------- property tests

/// One way to break a valid pattern, aimed at a lane with at least two steps.
#[derive(Clone, Copy, Debug)]
enum Break {
    PastEnd,
    Negative,
    Swap,
    Overlap,
    HoldFirst,
    UnknownGesture,
    SpanPastEnd,
}

fn lane_mut(lanes: &mut Lanes, id: LaneId) -> &mut Lane {
    match (lanes, id) {
        (Lanes::Strum(l), _) => l,
        (Lanes::ThumbFingers { thumb, .. }, LaneId::Thumb) => thumb,
        (Lanes::ThumbFingers { fingers, .. }, _) => fingers,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 400, .. ProptestConfig::default() })]

    /// PE-2: every generated valid IR is accepted, also with gestures.
    #[test]
    fn pe2_random_valid_ir_is_accepted(seed in pattern_seeds()) {
        let (mut lib, g) = gesture_library();
        let p = build_pattern(&seed, &g);
        prop_assert_eq!(errors(&lib, &p), vec![]);
        prop_assert!(lib.define_pattern(p).is_ok());
    }

    /// PE-2: one injected violation is rejected with its own typed error naming the
    /// step (or span) it was injected at.
    #[test]
    fn pe2_each_injected_violation_is_named(seed in pattern_seeds(), which in 0usize..7, pick in any::<usize>()) {
        let (lib, g) = gesture_library();
        let mut p = build_pattern(&seed, &g);
        let candidates: Vec<LaneId> = p.lanes.iter().filter(|(_, l)| l.steps.len() >= 2).map(|(id, _)| id).collect();
        prop_assume!(!candidates.is_empty());
        let lane_id = candidates[pick % candidates.len()];
        let kind = [Break::PastEnd, Break::Negative, Break::Swap, Break::Overlap,
            Break::HoldFirst, Break::UnknownGesture, Break::SpanPastEnd][which];
        let length = p.length;
        let lane = lane_mut(&mut p.lanes, lane_id);
        let n = lane.steps.len();
        let i = pick % (n - 1); // a step with a successor
        let found = |e: &[PatternError], f: &dyn Fn(&PatternError) -> bool| e.iter().any(f);
        match kind {
            Break::PastEnd => {
                let last = &mut lane.steps[n - 1];
                last.onset = length.checked_sub(last.duration.length()).unwrap()
                    .checked_add(r(1, 64)).unwrap();
                let onset = last.onset;
                let e = errors(&lib, &p);
                prop_assert!(found(&e, &|x| matches!(x, PatternError::OutsidePattern { step, .. }
                    if *step == at(lane_id, n - 1, onset))), "{:?}", e);
            }
            Break::Negative => {
                lane.steps[0].onset = r(-1, 8);
                let e = errors(&lib, &p);
                prop_assert!(found(&e, &|x| matches!(x, PatternError::NegativeOnset { step }
                    if *step == at(lane_id, 0, r(-1, 8)))), "{:?}", e);
            }
            Break::Swap => {
                lane.steps.swap(i, i + 1);
                let onset = lane.steps[i + 1].onset;
                let e = errors(&lib, &p);
                prop_assert!(found(&e, &|x| matches!(x, PatternError::OutOfOrder { step, .. }
                    if *step == at(lane_id, i + 1, onset))), "{:?}", e);
            }
            Break::Overlap => {
                let prev = lane.steps[i].onset;
                let half = lane.steps[i].duration.length().checked_mul(r(1, 2)).unwrap();
                let onset = prev.checked_add(half).unwrap();
                lane.steps[i + 1].onset = onset;
                let e = errors(&lib, &p);
                prop_assert!(found(&e, &|x| matches!(x, PatternError::Overlap { step, .. }
                    if *step == at(lane_id, i + 1, onset))), "{:?}", e);
            }
            Break::HoldFirst => {
                lane.steps[0].action = Action::Hold(None);
                let onset = lane.steps[0].onset;
                let e = errors(&lib, &p);
                prop_assert!(found(&e, &|x| matches!(x, PatternError::HoldWithoutStrike { step }
                    if *step == at(lane_id, 0, onset))), "{:?}", e);
            }
            Break::UnknownGesture => {
                let ghost = GestureId::from_raw(1_000_000);
                lane.steps[i].action = Action::Gesture(ghost);
                let onset = lane.steps[i].onset;
                let e = errors(&lib, &p);
                prop_assert!(found(&e, &|x| matches!(x, PatternError::UnknownGesture { step, gesture }
                    if *step == at(lane_id, i, onset) && *gesture == ghost)), "{:?}", e);
            }
            Break::SpanPastEnd => {
                p.spans.push(TechniqueSpan { lane: lane_id, first: i, last: n, technique: Technique::Accent });
                let span = p.spans.len() - 1;
                let e = errors(&lib, &p);
                prop_assert!(found(&e, &|x| matches!(x, PatternError::SpanOutOfRange { span: s, lane, .. }
                    if *s == span && *lane == lane_id)), "{:?}", e);
            }
        }
    }
}

/// The generator really produces what the properties claim to cover.
#[test]
fn the_generator_covers_every_action_two_lanes_tuplets_meters_and_spans() {
    let (lib, g) = gesture_library();
    let mut runner = proptest::test_runner::TestRunner::deterministic();
    let mut seen = std::collections::BTreeMap::<&str, usize>::new();
    for _ in 0..400 {
        let seed = pattern_seeds().new_tree(&mut runner).unwrap().current();
        let p = build_pattern(&seed, &g);
        assert!(errors(&lib, &p).is_empty());
        *seen
            .entry(if matches!(p.lanes, Lanes::ThumbFingers { .. }) {
                "two lanes"
            } else {
                "one lane"
            })
            .or_default() += 1;
        if p.meter.is_some() {
            *seen.entry("meter").or_default() += 1;
        }
        if !p.spans.is_empty() {
            *seen.entry("spans").or_default() += 1;
        }
        for (_, lane) in p.lanes.iter() {
            for (k, st) in lane.steps.iter().enumerate() {
                if st.duration.tuplet().is_some() {
                    *seen.entry("tuplet").or_default() += 1;
                }
                if k > 0
                    && st.onset
                        > lane.steps[k - 1]
                            .onset
                            .checked_add(lane.steps[k - 1].duration.length())
                            .unwrap()
                {
                    *seen.entry("gap").or_default() += 1;
                }
                *seen
                    .entry(match &st.action {
                        Action::Strike(_) => "strike",
                        Action::Gesture(_) => "gesture",
                        Action::Hold(None) => "hold all",
                        Action::Hold(Some(_)) => "hold some",
                        Action::Rest => "rest",
                    })
                    .or_default() += 1;
            }
        }
    }
    println!("{seen:?}");
    for key in [
        "two lanes",
        "one lane",
        "meter",
        "spans",
        "tuplet",
        "gap",
        "strike",
        "gesture",
        "hold all",
        "hold some",
        "rest",
    ] {
        assert!(
            seen.get(key).copied().unwrap_or(0) > 0,
            "{key} never generated: {seen:?}"
        );
    }
}
