#![allow(dead_code)]
//! Builders and a generator of random valid Pattern IR, shared by the tests.

use mc_pattern::*;
use mc_theory::fretted::StringIndex;
use mc_theory::{BaseValue, NoteValue, Rational, TimeSignature, Tuplet};
use proptest::prelude::*;

pub fn r(n: i64, d: i64) -> Rational {
    Rational::new(n, d).unwrap()
}

pub fn v(base: BaseValue) -> NoteValue {
    NoteValue::plain(base)
}

pub fn q() -> NoteValue {
    v(BaseValue::QUARTER)
}

pub fn e() -> NoteValue {
    v(BaseValue::EIGHTH)
}

pub fn dotted_q() -> NoteValue {
    NoteValue::new(BaseValue::QUARTER, 1, None).unwrap()
}

pub fn s(n: u8) -> StringSelector {
    StringSelector::Absolute(StringIndex::new(n).unwrap())
}

pub fn strike(targets: &[StringSelector]) -> Action {
    Action::Strike(Strike::on(targets.to_vec()))
}

pub fn stroke(dir: Direction, targets: &[StringSelector]) -> Action {
    Action::Strike(Strike {
        direction: Some(dir),
        ..Strike::on(targets.to_vec())
    })
}

pub fn step(onset: Rational, duration: NoteValue, action: Action) -> Step {
    Step {
        onset,
        duration,
        action,
    }
}

/// Consecutive steps from 0, each starting where the previous one ends.
pub fn run(items: Vec<(NoteValue, Action)>) -> Lane {
    let mut t = Rational::ZERO;
    let mut steps = Vec::new();
    for (value, action) in items {
        steps.push(step(t, value, action));
        t = t.checked_add(value.length()).unwrap();
    }
    Lane { steps }
}

pub fn pattern(length: Rational, lanes: Lanes) -> PatternParts {
    PatternParts {
        name: "P".into(),
        length,
        meter: None,
        lanes,
        spans: Vec::new(),
        meta: PatternMeta::default(),
    }
}

pub fn errors(lib: &PatternLibrary, p: &PatternParts) -> Vec<PatternError> {
    lib.validate_pattern(p)
}

// ------------------------------------------------------------- random valid IR

/// Note values used by the generator: plain, dotted and triplet values.
pub fn values() -> Vec<NoteValue> {
    let mut out = Vec::new();
    for b in [
        BaseValue::HALF,
        BaseValue::QUARTER,
        BaseValue::EIGHTH,
        BaseValue::SIXTEENTH,
    ] {
        out.push(NoteValue::plain(b));
        out.push(NoteValue::new(b, 1, None).unwrap());
        out.push(NoteValue::new(b, 0, Some(Tuplet::TRIPLET)).unwrap());
    }
    out
}

pub fn selector() -> impl Strategy<Value = StringSelector> {
    prop_oneof![
        Just(StringSelector::Bass),
        Just(StringSelector::Alt),
        (1u8..=5).prop_map(StringSelector::Treble),
        (1u8..=6).prop_map(s),
        Just(StringSelector::Strum),
        (1u8..=6, 1u8..=6).prop_map(|(a, b)| StringSelector::StrumRange(
            StringIndex::new(a).unwrap(),
            StringIndex::new(b).unwrap()
        )),
    ]
}

pub fn techniques() -> impl Strategy<Value = TechniqueSet> {
    (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(a, b, c, d, f)| TechniqueSet {
            accent: a,
            palm_mute: b,
            dead: c,
            let_ring: d,
            string_percussive: f,
        })
}

pub fn strike_strategy() -> impl Strategy<Value = Strike> {
    (
        proptest::collection::vec(selector(), 1..4),
        prop_oneof![
            Just(None),
            Just(Some(Direction::Down)),
            Just(Some(Direction::Up))
        ],
        techniques(),
    )
        .prop_map(|(targets, direction, techniques)| Strike {
            targets,
            direction,
            techniques,
        })
}

/// One raw decision per step: value index, gap kind, action kind, and a seed.
pub type StepSeed = (usize, u8, u8, u64, Strike);

pub fn step_seeds() -> impl Strategy<Value = Vec<StepSeed>> {
    proptest::collection::vec(
        (0usize..64, 0u8..4, 0u8..10, any::<u64>(), strike_strategy()),
        0..14,
    )
}

/// Builds a valid lane inside `length` from seeds. Holds come only directly after a
/// strike, gesture or hold, with selectors taken from what is still held, so PT-05
/// holds by construction; gestures come from `gestures` (id, targets).
pub fn build_lane(
    length: Rational,
    seeds: &[StepSeed],
    gestures: &[(GestureId, Vec<StringSelector>)],
) -> Lane {
    let values = values();
    let mut t = Rational::ZERO;
    let mut steps: Vec<Step> = Vec::new();
    let mut held: Option<Vec<StringSelector>> = None;
    for (vi, gap, kind, seed, strike) in seeds {
        let value = values[vi % values.len()];
        let gap_len = match gap {
            0 | 1 => Rational::ZERO,
            2 => r(1, 16),
            _ => r(1, 8),
        };
        let onset = t.checked_add(gap_len).unwrap();
        let end = onset.checked_add(value.length()).unwrap();
        if end > length {
            break;
        }
        if !gap_len.is_zero() {
            held = None; // a gap is a rest (PT-09)
        }
        let action = match (kind, &held) {
            (0..=1, Some(h)) => {
                if seed % 2 == 0 {
                    Action::Hold(None)
                } else {
                    // A non-empty subset of the held selectors.
                    let pick: Vec<_> = h
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| (seed >> (i + 1)) & 1 == 1)
                        .map(|(_, x)| *x)
                        .collect();
                    let pick = if pick.is_empty() { vec![h[0]] } else { pick };
                    Action::Hold(Some(pick))
                }
            }
            (2, _) => Action::Rest,
            (3..=4, _) if !gestures.is_empty() => {
                Action::Gesture(gestures[(*seed as usize) % gestures.len()].0)
            }
            _ => Action::Strike(strike.clone()),
        };
        held = match &action {
            Action::Strike(k) => Some(k.targets.clone()),
            Action::Gesture(g) => gestures
                .iter()
                .find(|(id, _)| id == g)
                .map(|(_, t)| t.clone()),
            Action::Hold(None) => held.clone(),
            Action::Hold(Some(sel)) => Some(sel.clone()),
            Action::Rest => None,
        };
        steps.push(step(onset, value, action));
        t = end;
    }
    Lane { steps }
}

pub fn lengths() -> Vec<Rational> {
    vec![r(1, 2), r(3, 4), Rational::ONE, r(2, 1)]
}

pub type PatternSeed = (
    usize,
    bool,
    Vec<StepSeed>,
    Vec<StepSeed>,
    Vec<(u8, usize, usize, u8)>,
    u8,
);

pub fn pattern_seeds() -> impl Strategy<Value = PatternSeed> {
    (
        0usize..4,
        any::<bool>(),
        step_seeds(),
        step_seeds(),
        proptest::collection::vec((0u8..3, 0usize..16, 0usize..16, 0u8..5), 0..4),
        0u8..3,
    )
}

/// A random valid pattern from seeds (spans only over existing steps of a lane).
pub fn build_pattern(
    seed: &PatternSeed,
    gestures: &[(GestureId, Vec<StringSelector>)],
) -> PatternParts {
    let (li, two, a, b, spans, meter) = seed;
    let length = lengths()[li % 4];
    let lanes = if *two {
        Lanes::ThumbFingers {
            thumb: build_lane(length, a, gestures),
            fingers: build_lane(length, b, gestures),
        }
    } else {
        Lanes::Strum(build_lane(length, a, gestures))
    };
    let mut out = Vec::new();
    for (lane_kind, x, y, tech) in spans {
        let lane = match (two, lane_kind) {
            (false, _) => LaneId::Strum,
            (true, 0) => LaneId::Thumb,
            (true, _) => LaneId::Fingers,
        };
        let n = lanes.get(lane).unwrap().steps.len();
        if n == 0 {
            continue;
        }
        let (first, last) = (x % n, y % n);
        out.push(TechniqueSpan {
            lane,
            first: first.min(last),
            last: first.max(last),
            technique: [
                Technique::Accent,
                Technique::PalmMute,
                Technique::Dead,
                Technique::LetRing,
                Technique::StringPercussive,
            ][*tech as usize % 5],
        });
    }
    PatternParts {
        name: format!("Random {li}"),
        length,
        meter: [
            None,
            Some(TimeSignature::COMMON),
            Some(TimeSignature::new(3, 4).unwrap()),
        ][*meter as usize % 3],
        lanes,
        spans: out,
        meta: PatternMeta::default(),
    }
}

/// A library with three gestures (Bùm, Kịch, XL), as in the user's examples.
pub fn gesture_library() -> (PatternLibrary, Vec<(GestureId, Vec<StringSelector>)>) {
    let mut lib = PatternLibrary::new();
    let mut out = Vec::new();
    let defs: [(&str, Strike); 3] = [
        (
            "Bùm",
            Strike {
                techniques: TechniqueSet {
                    palm_mute: true,
                    accent: true,
                    ..TechniqueSet::default()
                },
                ..Strike::on(vec![StringSelector::Bass])
            },
        ),
        (
            "Kịch",
            Strike {
                direction: Some(Direction::Down),
                techniques: TechniqueSet {
                    string_percussive: true,
                    ..TechniqueSet::default()
                },
                ..Strike::on(vec![StringSelector::Strum])
            },
        ),
        (
            "XL",
            Strike::on(vec![StringSelector::Bass, StringSelector::TREBLE]),
        ),
    ];
    for (name, strike) in defs {
        let targets = strike.targets.clone();
        let id = lib.define_gesture(name, strike).unwrap();
        out.push((id, targets));
    }
    (lib, out)
}
