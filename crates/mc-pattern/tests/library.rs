//! ADR-034 PE-9 (gestures), PT-06 and PT-08 (library rules) and PE-1 (the IR
//! round-trips through its plain-data form).

mod common;
use common::*;
use mc_pattern::*;
use mc_theory::Rational;
use proptest::prelude::*;

use StringSelector::{Bass, Strum};

fn name_error(lib: &mut PatternLibrary, name: &str) -> Vec<PatternError> {
    lib.define_gesture(name, Strike::on(vec![Bass]))
        .unwrap_err()
}

// ------------------------------------------------------------------- PT-08

#[test]
fn pt08_names_are_unique_without_case() {
    let (mut lib, g) = gesture_library();
    let (bum, kich, xl) = (g[0].0, g[1].0, g[2].0);
    for (name, existing) in [
        ("bùm", bum),
        ("BÙM", bum),
        ("KỊCH", kich),
        ("xl", xl),
        ("Xl", xl),
    ] {
        assert_eq!(
            name_error(&mut lib, name),
            vec![PatternError::DuplicateGestureName {
                name: name.into(),
                existing,
            }],
            "{name}"
        );
    }
    // Renaming to another gesture's name is a duplicate too; to its own name in
    // another case is not.
    assert_eq!(
        lib.rename_gesture(kich, "bùm").unwrap_err(),
        vec![PatternError::DuplicateGestureName {
            name: "bùm".into(),
            existing: bum,
        }]
    );
    lib.rename_gesture(kich, "KỊCH").unwrap();
    assert_eq!(lib.gesture(kich).unwrap().name(), "KỊCH");
    assert_eq!(lib.gesture_by_name("kịch").unwrap().id(), kich);
}

#[test]
fn pt08_a_name_equal_to_a_keyword_is_rejected() {
    let mut lib = PatternLibrary::new();
    for kw in KEYWORDS {
        for name in [kw.to_string(), kw.to_uppercase()] {
            assert_eq!(
                name_error(&mut lib, &name),
                vec![PatternError::InvalidGestureName {
                    name: name.clone(),
                    problem: NameProblem::Keyword(kw),
                }],
                "{name}"
            );
        }
    }
    // Words of the corpus sketch, in the user's spelling.
    for name in ["Bass", "STRUM", "Treble", "Pm", "D"] {
        assert!(matches!(
            name_error(&mut lib, name)[..],
            [PatternError::InvalidGestureName {
                problem: NameProblem::Keyword(_),
                ..
            }]
        ));
    }
}

#[test]
fn pt08_names_are_letters_and_digits_starting_with_a_letter() {
    let mut lib = PatternLibrary::new();
    for name in ["Bùm", "Kịch", "XL", "Chuck2", "Đàn"] {
        lib.define_gesture(name, Strike::on(vec![Bass]))
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    }
    for (name, problem) in [
        ("", NameProblem::Empty),
        ("4x", NameProblem::MustStartWithLetter),
        ("_x", NameProblem::MustStartWithLetter),
        ("Kịch-2", NameProblem::InvalidCharacter('-')),
        ("a b", NameProblem::InvalidCharacter(' ')),
        ("Bùm+", NameProblem::InvalidCharacter('+')),
        // A decomposed spelling: u followed by a combining grave accent.
        ("Bu\u{300}m2", NameProblem::InvalidCharacter('\u{300}')),
    ] {
        assert_eq!(
            name_error(&mut lib, name),
            vec![PatternError::InvalidGestureName {
                name: name.into(),
                problem,
            }],
            "{name:?}"
        );
    }
}

// ------------------------------------------------------------------- PE-9

/// Two patterns that use the same gestures.
fn two_users(
    lib: &mut PatternLibrary,
    g: &[(GestureId, Vec<StringSelector>)],
) -> (PatternId, PatternId) {
    let (bum, kich) = (g[0].0, g[1].0);
    let a = lib
        .define_pattern(pattern(
            Rational::ONE,
            Lanes::Strum(run(vec![
                (q(), Action::Gesture(bum)),
                (q(), Action::Gesture(kich)),
                (q(), Action::Gesture(bum)),
                (q(), Action::Gesture(kich)),
            ])),
        ))
        .unwrap();
    let b = lib
        .define_pattern(pattern(
            r(1, 2),
            Lanes::ThumbFingers {
                thumb: run(vec![(q(), Action::Gesture(bum)), (q(), Action::Hold(None))]),
                fingers: run(vec![(e(), Action::Rest), (e(), Action::Gesture(kich))]),
            },
        ))
        .unwrap();
    (a, b)
}

/// Every expanded gesture step of a resolved pattern: (gesture, strike).
fn expanded(p: &ResolvedPattern) -> Vec<(GestureId, Strike)> {
    p.lanes
        .iter()
        .flat_map(|(_, steps)| steps)
        .filter_map(|s| match (&s.action, s.gesture) {
            (ResolvedAction::Strike(k), Some(g)) => Some((g, k.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn pe9_a_gesture_used_by_several_patterns_expands_identically_in_each() {
    let (mut lib, g) = gesture_library();
    let (a, b) = two_users(&mut lib, &g);
    for id in [a, b] {
        let resolved = lib.resolve(id).unwrap();
        let ex = expanded(&resolved);
        assert!(!ex.is_empty());
        for (gesture, strike) in ex {
            assert_eq!(
                &strike,
                lib.gesture(gesture).unwrap().strike(),
                "pattern {id:?}"
            );
        }
        // Nothing else changes: onsets, values, holds and rests are the pattern's own.
        let parts = lib.pattern(id).unwrap().parts();
        for ((_, lane), (_, steps)) in parts.lanes.iter().zip(&resolved.lanes) {
            assert_eq!(lane.steps.len(), steps.len());
            for (s, r) in lane.steps.iter().zip(steps) {
                assert_eq!((s.onset, s.duration), (r.onset, r.duration));
                match (&s.action, &r.action) {
                    (Action::Gesture(_), ResolvedAction::Strike(_)) => {}
                    (Action::Strike(x), ResolvedAction::Strike(y)) => assert_eq!(x, y),
                    (Action::Hold(x), ResolvedAction::Hold(y)) => assert_eq!(x, y),
                    (Action::Rest, ResolvedAction::Rest) => {}
                    other => panic!("{other:?}"),
                }
            }
        }
    }
    // The same gesture gives the same strike in both patterns.
    let in_a = expanded(&lib.resolve(a).unwrap());
    let in_b = expanded(&lib.resolve(b).unwrap());
    for (ga, sa) in &in_a {
        for (gb, sb) in &in_b {
            if ga == gb {
                assert_eq!(sa, sb);
            }
        }
    }
}

#[test]
fn pe9_a_rename_changes_no_resolved_result() {
    let (mut lib, g) = gesture_library();
    let (a, b) = two_users(&mut lib, &g);
    let before = (lib.resolve(a).unwrap(), lib.resolve(b).unwrap());
    let parts_before = (
        lib.pattern(a).unwrap().clone(),
        lib.pattern(b).unwrap().clone(),
    );
    lib.rename_gesture(g[0].0, "Boom").unwrap();
    lib.rename_gesture(g[1].0, "Chuck").unwrap();
    assert_eq!((lib.resolve(a).unwrap(), lib.resolve(b).unwrap()), before);
    assert_eq!(
        (
            lib.pattern(a).unwrap().clone(),
            lib.pattern(b).unwrap().clone()
        ),
        parts_before
    );
    assert!(lib.gesture_by_name("Bùm").is_none());
    assert_eq!(lib.gesture_by_name("boom").unwrap().id(), g[0].0);
}

#[test]
fn pe9_unknown_and_duplicate_names_and_keywords_are_typed_errors() {
    let (mut lib, g) = gesture_library();
    let ghost = GestureId::from_raw(999);
    assert_eq!(
        lib.rename_gesture(ghost, "Ghost").unwrap_err(),
        vec![PatternError::UnknownGestureId(ghost)]
    );
    assert_eq!(
        lib.change_gesture(ghost, Strike::on(vec![Bass]))
            .unwrap_err(),
        vec![PatternError::UnknownGestureId(ghost)]
    );
    assert_eq!(
        lib.remove_gesture(ghost).unwrap_err(),
        vec![PatternError::UnknownGestureId(ghost)]
    );
    assert!(matches!(
        lib.rename_gesture(g[0].0, "XL").unwrap_err()[..],
        [PatternError::DuplicateGestureName { .. }]
    ));
    assert!(matches!(
        lib.rename_gesture(g[0].0, "Alt").unwrap_err()[..],
        [PatternError::InvalidGestureName {
            problem: NameProblem::Keyword("alt"),
            ..
        }]
    ));
    // A pattern that names a gesture the library does not have (PT-06).
    let p = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![(q(), Action::Gesture(ghost))])),
    );
    assert!(matches!(
        lib.define_pattern(p).unwrap_err()[..],
        [PatternError::UnknownGesture { gesture, .. }] if gesture == ghost
    ));
}

#[test]
fn pt06_a_gesture_in_use_cannot_be_removed() {
    let (mut lib, g) = gesture_library();
    let (a, b) = two_users(&mut lib, &g);
    let before = lib.clone();
    assert_eq!(
        lib.remove_gesture(g[0].0).unwrap_err(),
        vec![PatternError::GestureInUse {
            gesture: g[0].0,
            patterns: vec![a, b],
        }]
    );
    assert_eq!(lib, before, "nothing changed");
    // XL is unused and can go; Bùm can go once its patterns are gone.
    lib.remove_gesture(g[2].0).unwrap();
    lib.remove_pattern(a).unwrap();
    lib.remove_pattern(b).unwrap();
    lib.remove_gesture(g[0].0).unwrap();
}

#[test]
fn a_gesture_change_that_would_break_a_pattern_changes_nothing() {
    let (mut lib, g) = gesture_library();
    let kich = g[1].0; // Strum
    // A hold of string 1 after Kịch is valid while Kịch strums.
    let p = lib
        .define_pattern(pattern(
            Rational::ONE,
            Lanes::Strum(run(vec![
                (q(), Action::Gesture(kich)),
                (q(), Action::Hold(Some(vec![s(1)]))),
            ])),
        ))
        .unwrap();
    let before = lib.clone();
    let err = lib
        .change_gesture(kich, Strike::on(vec![Bass]))
        .unwrap_err();
    assert!(
        matches!(
            &err[..],
            [PatternError::InvalidPattern { pattern, errors }]
                if *pattern == p && matches!(errors[..], [PatternError::HoldNotInStrike { .. }])
        ),
        "{err:?}"
    );
    assert_eq!(lib, before);
    // A change that keeps it valid is applied and seen by the pattern.
    let mut wider = Strike::on(vec![Strum, Bass]);
    wider.techniques.accent = true;
    lib.change_gesture(kich, wider.clone()).unwrap();
    assert!(
        matches!(&lib.resolve(p).unwrap().lanes[0].1[0].action, ResolvedAction::Strike(k) if *k == wider)
    );
    // A gesture's own strike is checked like any strike.
    assert_eq!(
        lib.change_gesture(kich, Strike::on(vec![])).unwrap_err(),
        vec![PatternError::EmptyTargets {
            at: StrikeAt::Gesture {
                name: "Kịch".into()
            },
        }]
    );
}

#[test]
fn ids_are_shared_and_stable() {
    let (mut lib, g) = gesture_library();
    let (a, b) = two_users(&mut lib, &g);
    let mut raw: Vec<u64> = g.iter().map(|x| x.0.raw()).collect();
    raw.extend([a.raw(), b.raw()]);
    assert_eq!(
        raw,
        vec![1, 2, 3, 4, 5],
        "one counter for gestures and patterns"
    );
    lib.remove_pattern(a).unwrap();
    let c = lib
        .define_pattern(pattern(Rational::ONE, Lanes::Strum(Lane::default())))
        .unwrap();
    assert_eq!(c.raw(), 6, "ids are never reused");
    assert_eq!(
        lib.replace_pattern(a, pattern(Rational::ONE, Lanes::Strum(Lane::default())))
            .unwrap_err(),
        vec![PatternError::UnknownPattern(a)]
    );
}

#[test]
fn a_rejected_replace_keeps_the_old_pattern() {
    let mut lib = PatternLibrary::new();
    let good = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![(q(), strike(&[Bass]))])),
    );
    let id = lib.define_pattern(good.clone()).unwrap();
    let bad = pattern(Rational::ZERO, Lanes::Strum(Lane::default()));
    assert!(lib.replace_pattern(id, bad).is_err());
    assert_eq!(lib.pattern(id).unwrap().parts(), &good);
}

// ------------------------------------------------------------------- PE-1

#[test]
fn pe1_library_parts_are_checked() {
    let (mut lib, g) = gesture_library();
    two_users(&mut lib, &g);
    let good = lib.to_parts();
    // A duplicate id, an id at the counter, a bad name and an invalid pattern: all
    // reported.
    let mut bad = good.clone();
    bad.gestures[1].id = bad.gestures[0].id;
    bad.gestures[2].name = "strum".into();
    bad.patterns[0].1.length = Rational::ZERO;
    bad.next_id = bad.patterns[1].0.raw();
    let e = PatternLibrary::from_parts(bad).unwrap_err();
    assert!(
        e.iter().any(|x| matches!(x, PatternError::DuplicateId(1))),
        "{e:?}"
    );
    assert!(
        e.iter()
            .any(|x| matches!(x, PatternError::IdNotBelowCounter { .. })),
        "{e:?}"
    );
    assert!(
        e.iter()
            .any(|x| matches!(x, PatternError::InvalidGestureName { .. })),
        "{e:?}"
    );
    assert!(
        e.iter()
            .any(|x| matches!(x, PatternError::InvalidPattern { .. })),
        "{e:?}"
    );
    assert_eq!(
        PatternLibrary::from_parts(good.clone()).unwrap().to_parts(),
        good
    );
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 300, .. ProptestConfig::default() })]

    /// PE-1: onsets and durations are kept as exact rationals and written values, and
    /// a valid library round-trips through its plain data unchanged: parts -> library
    /// -> parts gives the same parts, and library -> parts -> library the same library.
    #[test]
    fn pe1_the_ir_round_trips_through_its_plain_data(seeds in proptest::collection::vec(pattern_seeds(), 1..5)) {
        let (mut lib, g) = gesture_library();
        let mut defined = Vec::new();
        for seed in &seeds {
            let p = build_pattern(seed, &g);
            let id = lib.define_pattern(p.clone()).unwrap();
            prop_assert_eq!(lib.pattern(id).unwrap().parts(), &p, "stored exactly as given");
            defined.push((id, p));
        }
        let parts = lib.to_parts();
        prop_assert_eq!(&parts.patterns, &defined);
        let again = PatternLibrary::from_parts(parts.clone()).unwrap();
        prop_assert_eq!(&again, &lib);
        prop_assert_eq!(again.to_parts(), parts);
    }
}
