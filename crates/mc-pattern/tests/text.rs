//! ADR-034 PE-10: the text front end. `parse(print(ir)) == ir` for random valid IR;
//! every confirmed corpus pattern parses to its IR; a step without a value and without
//! a `unit` is an error with a span; keywords are case-insensitive; a bar check off a
//! bar line is an error.

mod common;
use common::*;
use mc_pattern::text::{self, ParseError, ParseErrorKind as K};
use mc_pattern::*;
use mc_theory::{BaseValue, NoteValue, Rational, TimeSignature, Tuplet};
use proptest::prelude::*;

use StringSelector::{Alt, Bass, Strum};

const TREBLE: StringSelector = StringSelector::TREBLE;
const D: Direction = Direction::Down;
const U: Direction = Direction::Up;

fn parse(text: &str) -> Result<PatternParts, ParseError> {
    text::parse(text, &PatternLibrary::new())
}

fn ok(text: &str) -> PatternParts {
    parse(text).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

/// The error of `text`, with the text its span covers.
fn bad(text: &str) -> (K, String) {
    let e = parse(text).expect_err(text);
    (e.kind.clone(), text[e.span.clone()].to_string())
}

// ----------------------------------------------------------------- the corpus

const CORPUS: [(&str, &str); 9] = [
    ("1", include_str!("corpus/p01-basic-fingerpicking.pattern")),
    ("2", include_str!("corpus/p02-travis-picking.pattern")),
    ("3", include_str!("corpus/p03-boom-chick.pattern")),
    ("4", include_str!("corpus/p04-folk-strumming.pattern")),
    ("5", include_str!("corpus/p05-down-up-eighths.pattern")),
    ("6", include_str!("corpus/p06-waltz.pattern")),
    ("8", include_str!("corpus/p08-palm-muted-bass.pattern")),
    (
        "9",
        include_str!("corpus/p09-string-percussive-boom-chuck.pattern"),
    ),
    ("10", include_str!("corpus/p10-users-example.pattern")),
];

fn k(targets: &[StringSelector], dir: Option<Direction>) -> Action {
    Action::Strike(Strike {
        direction: dir,
        ..Strike::on(targets.to_vec())
    })
}

fn tech(a: Action, f: impl FnOnce(&mut TechniqueSet)) -> Action {
    match a {
        Action::Strike(mut s) => {
            f(&mut s.techniques);
            Action::Strike(s)
        }
        other => other,
    }
}

fn named(name: &str, length: Rational, lanes: Lanes) -> PatternParts {
    PatternParts {
        name: name.into(),
        ..pattern(length, lanes)
    }
}

/// The IR each confirmed corpus pattern means, written by hand from the corpus
/// document (pattern-corpus-proposed.md), not from the parser. Rests written with `-`
/// are explicit rest steps.
fn expected(id: &str) -> PatternParts {
    let treble4 = StringSelector::Treble(4);
    let eighths = |items: Vec<Action>| run(items.into_iter().map(|a| (e(), a)).collect());
    match id {
        "1" => named(
            "Basic fingerpicking",
            Rational::ONE,
            Lanes::Strum(eighths(
                [Bass, s(3), s(2), s(3), s(1), s(3), s(2), s(3)]
                    .iter()
                    .map(|t| k(&[*t], None))
                    .collect(),
            )),
        ),
        "2" => named(
            "Travis-style picking",
            Rational::ONE,
            Lanes::ThumbFingers {
                thumb: run(vec![
                    (q(), k(&[Bass], None)),
                    (q(), k(&[Alt], None)),
                    (q(), k(&[Bass], None)),
                    (q(), k(&[Alt], None)),
                ]),
                fingers: eighths(vec![
                    k(&[s(1)], None),
                    k(&[s(2)], None),
                    Action::Rest,
                    k(&[s(1)], None),
                    Action::Rest,
                    k(&[s(2)], None),
                    Action::Rest,
                    k(&[s(1)], None),
                ]),
            },
        ),
        "3" => named(
            "Boom-chick",
            Rational::ONE,
            Lanes::Strum(run(vec![
                (q(), k(&[Bass], None)),
                (q(), k(&[TREBLE], Some(D))),
                (q(), k(&[Alt], None)),
                (q(), k(&[TREBLE], Some(D))),
            ])),
        ),
        "4" => named(
            "Folk strumming",
            Rational::ONE,
            Lanes::Strum(run(vec![
                (q(), k(&[Strum], Some(D))),
                (e(), k(&[Strum], Some(D))),
                (e(), k(&[treble4], Some(U))),
                (e(), Action::Rest),
                (e(), k(&[treble4], Some(U))),
                (e(), k(&[Strum], Some(D))),
                (e(), k(&[treble4], Some(U))),
            ])),
        ),
        "5" => named(
            "Down and up strokes on eighths",
            Rational::ONE,
            Lanes::Strum(eighths(
                (0..8)
                    .map(|i| {
                        if i % 2 == 0 {
                            k(&[Strum], Some(D))
                        } else {
                            k(&[treble4], Some(U))
                        }
                    })
                    .collect(),
            )),
        ),
        "6" => PatternParts {
            meter: Some(TimeSignature::new(3, 4).unwrap()),
            ..named(
                "Waltz",
                r(3, 4),
                Lanes::Strum(run(vec![
                    (q(), k(&[Bass], None)),
                    (q(), k(&[TREBLE], Some(D))),
                    (q(), k(&[TREBLE], Some(D))),
                ])),
            )
        },
        "8" => PatternParts {
            spans: vec![TechniqueSpan {
                lane: LaneId::Strum,
                first: 0,
                last: 7,
                technique: Technique::PalmMute,
            }],
            ..named(
                "Palm-muted bass ostinato",
                Rational::ONE,
                Lanes::Strum(eighths(
                    (0..8)
                        .map(|i| tech(k(&[Bass], None), |t| t.accent = i % 4 == 0))
                        .collect(),
                )),
            )
        },
        "9" => named(
            "String-percussive boom-chuck",
            Rational::ONE,
            Lanes::Strum(run(vec![
                (q(), k(&[Strum], Some(D))),
                (
                    q(),
                    tech(k(&[Strum], Some(D)), |t| {
                        t.accent = true;
                        t.string_percussive = true;
                    }),
                ),
                (e(), k(&[Strum], Some(D))),
                (e(), k(&[treble4], Some(U))),
                (
                    q(),
                    tech(k(&[Strum], Some(D)), |t| {
                        t.accent = true;
                        t.string_percussive = true;
                    }),
                ),
            ])),
        ),
        "10" => PatternParts {
            meter: Some(TimeSignature::new(2, 4).unwrap()),
            ..named(
                "Bass treb | Bass treb treb",
                Rational::ONE,
                Lanes::Strum(run(vec![
                    (q(), k(&[Bass], None)),
                    (q(), k(&[TREBLE], None)),
                    (q(), k(&[Bass], None)),
                    (e(), k(&[TREBLE], None)),
                    (e(), k(&[TREBLE], None)),
                ])),
            )
        },
        other => panic!("not a confirmed corpus pattern: {other}"),
    }
}

#[test]
fn pe10_every_confirmed_corpus_pattern_parses_to_its_ir() {
    for (id, text) in CORPUS {
        assert_eq!(ok(text), expected(id), "corpus pattern {id}");
    }
}

#[test]
fn pe10_the_corpus_is_the_confirmed_set_without_pattern_7() {
    let ids: Vec<&str> = CORPUS.iter().map(|c| c.0).collect();
    assert_eq!(ids, ["1", "2", "3", "4", "5", "6", "8", "9", "10"]);
    assert!(
        !ids.contains(&"7"),
        "pattern 7 is unconfirmed, not a golden (PE-11)"
    );
}

#[test]
fn pe10_corpus_patterns_are_valid_and_print_back_to_the_same_ir() {
    let mut lib = PatternLibrary::new();
    for (id, text) in CORPUS {
        let parts = ok(text);
        assert_eq!(lib.validate_pattern(&parts), vec![], "pattern {id}");
        let printed = text::print(&parts, &lib).unwrap();
        assert_eq!(ok(&printed), parts, "pattern {id}:\n{printed}");
        assert_eq!(
            text::print(&ok(&printed), &lib).unwrap(),
            printed,
            "canonical"
        );
        lib.define_pattern_text(text)
            .unwrap_or_else(|e| panic!("{id}: {e:?}"));
    }
    assert_eq!(lib.patterns().count(), 9);
}

#[test]
fn the_printer_is_canonical_on_the_corpus() {
    let lib = PatternLibrary::new();
    let printed = |id: &str| text::print(&expected(id), &lib).unwrap();
    assert_eq!(
        printed("3"),
        "\"name\": \"Boom-chick\"\n\"length\": 1\nstrum: Bass:4 D Treble:4 Alt:4 D Treble:4\n"
    );
    // A bar check where a bar starts, written in the index form for spans.
    assert_eq!(
        printed("10"),
        "\"name\": \"Bass treb | Bass treb treb\"\n\"length\": 1\nmeter 2/4\n\
         strum: Bass:4 Treble:4 | Bass:4 Treble:8 Treble:8\n"
    );
    assert_eq!(
        printed("8"),
        "\"name\": \"Palm-muted bass ostinato\"\n\"length\": 1\n\
         strum: >Bass:8 Bass:8 Bass:8 Bass:8 >Bass:8 Bass:8 Bass:8 Bass:8\nstrum pm#0..7\n"
    );
    assert_eq!(
        printed("9"),
        "\"name\": \"String-percussive boom-chuck\"\n\"length\": 1\n\
         strum: D Strum:4 D >sp(Strum):4 D Strum:8 U Treble(4):8 D >sp(Strum):4\n"
    );
}

// ------------------------------------------------------- syntax: what it means

#[test]
fn keywords_are_case_insensitive() {
    let a = ok("\"length\": 3/4\nmeter 3/4\nstrum: Bass:4 D Treble(4):4 U pm(sp(Alt)):4");
    let b = ok("\"LENGTH\": 3/4\nMETER 3/4\nSTRUM: BASS:4 d TREBLE(4):4 u PM(SP(ALT)):4");
    assert_eq!(a, b);
    let c = ok("\"Length\": 1\nUnit 4\nSTRUM: Strum(6..4) bass+TREBLE+3");
    let Lanes::Strum(l) = &c.lanes else { panic!() };
    assert_eq!(l.steps.len(), 2);
}

#[test]
fn a_step_takes_its_duration_from_the_unit_in_force() {
    let p = ok("\"length\": 1\nunit 8\nthumb: Bass Alt:4 Bass\nunit 4.\nfingers: 1 2:8t 3");
    let Lanes::ThumbFingers { thumb, fingers } = &p.lanes else {
        panic!()
    };
    let dur = |l: &Lane| l.steps.iter().map(|s| s.duration).collect::<Vec<_>>();
    assert_eq!(dur(thumb), vec![e(), q(), e()]);
    let triplet = NoteValue::new(BaseValue::EIGHTH, 0, Some(Tuplet::TRIPLET)).unwrap();
    assert_eq!(dur(fingers), vec![dotted_q(), triplet, dotted_q()]);
    assert_eq!(thumb.steps[2].onset, r(3, 8));
    // The unit is never printed: the text is explicit.
    let out = text::print(&p, &PatternLibrary::new()).unwrap();
    assert!(!out.to_lowercase().contains("unit"), "{out}");
}

#[test]
fn onsets_default_to_the_end_of_the_previous_step_and_can_be_set() {
    let p = ok("\"length\": 1\nstrum: @1/4 Bass:4 Treble:4 @3/4 Alt:8 -:8");
    let Lanes::Strum(l) = &p.lanes else { panic!() };
    let onsets: Vec<_> = l.steps.iter().map(|s| s.onset).collect();
    assert_eq!(onsets, vec![r(1, 4), r(1, 2), r(3, 4), r(7, 8)]);
    assert!(
        matches!(l.steps[3].action, Action::Rest),
        "- is an explicit rest step"
    );
}

#[test]
fn a_gap_and_an_explicit_rest_are_different_ir_and_print_differently() {
    let gap = ok("\"length\": 1\nstrum: Bass:4 @1/2 Bass:4");
    let rest = ok("\"length\": 1\nstrum: Bass:4 -:4 Bass:4");
    assert_ne!(gap, rest);
    let lib = PatternLibrary::new();
    assert!(text::print(&gap, &lib).unwrap().contains("@1/2"));
    assert!(text::print(&rest, &lib).unwrap().contains("-:4"));
}

#[test]
fn technique_marks_wrappers_holds_and_selectors() {
    let p = ok(
        "\"length\": 1\nstrum: D >!~pm(sp(Bass+3+Strum(6..4)+Treble(5))):4 _(Bass+3):4 _:4 U 2:4",
    );
    let Lanes::Strum(l) = &p.lanes else { panic!() };
    let Action::Strike(strike) = &l.steps[0].action else {
        panic!()
    };
    assert_eq!(strike.direction, Some(D));
    assert_eq!(
        strike.techniques,
        TechniqueSet {
            accent: true,
            dead: true,
            let_ring: true,
            palm_mute: true,
            string_percussive: true
        }
    );
    assert_eq!(
        strike.targets,
        vec![
            Bass,
            s(3),
            StringSelector::StrumRange(
                mc_theory::fretted::StringIndex::new(6).unwrap(),
                mc_theory::fretted::StringIndex::new(4).unwrap()
            ),
            StringSelector::Treble(5)
        ]
    );
    assert!(matches!(&l.steps[1].action, Action::Hold(Some(v)) if *v == vec![Bass, s(3)]));
    assert!(matches!(l.steps[2].action, Action::Hold(None)));
}

#[test]
fn durations_cover_dots_triplets_and_other_tuplets() {
    let p = ok(
        "\"length\": 4\nstrum: Bass:1 Bass:2. Bass:4.. Bass:8... Bass:16(5:4) Bass:8.t Bass:256",
    );
    let Lanes::Strum(l) = &p.lanes else { panic!() };
    let d: Vec<NoteValue> = l.steps.iter().map(|s| s.duration).collect();
    let nv =
        |b: u8, dots, t| NoteValue::new(BaseValue::from_exponent(b).unwrap(), dots, t).unwrap();
    assert_eq!(
        d,
        vec![
            nv(0, 0, None),
            nv(1, 1, None),
            nv(2, 2, None),
            nv(3, 3, None),
            nv(4, 0, Some(Tuplet::new(5, 4).unwrap())),
            nv(3, 1, Some(Tuplet::TRIPLET)),
            nv(8, 0, None),
        ]
    );
}

#[test]
fn gestures_are_written_by_name_without_case_and_resolve_to_ids() {
    let (lib, g) = gesture_library();
    let p = text::parse("\"length\": 1\nstrum: Bùm:4 kịch:4 xl:4 BÙM:4", &lib).unwrap();
    let Lanes::Strum(l) = &p.lanes else { panic!() };
    let ids: Vec<_> = l
        .steps
        .iter()
        .map(|s| match s.action {
            Action::Gesture(id) => id,
            _ => panic!(),
        })
        .collect();
    assert_eq!(ids, vec![g[0].0, g[1].0, g[2].0, g[0].0]);
    // Printed with the gesture's current name, so a rename changes the text only.
    let out = text::print(&p, &lib).unwrap();
    assert!(out.contains("strum: Bùm:4 Kịch:4 XL:4 Bùm:4"), "{out}");
}

#[test]
fn spans_in_bracket_form_and_index_form_mean_the_same() {
    let bracket = ok("\"length\": 1\nstrum: Bass:8 [pm Bass:8 [> Treble:8 Treble:8] Bass:8] Alt:8");
    let index = ok(
        "\"length\": 1\nstrum: Bass:8 Bass:8 Treble:8 Treble:8 Bass:8 Alt:8\nstrum >#2..3\nstrum pm#1..4",
    );
    let sort = |mut p: PatternParts| {
        p.spans.sort_by_key(|s| (s.first, s.last));
        p
    };
    assert_eq!(sort(bracket.clone()), sort(index));
    assert_eq!(bracket.spans.len(), 2);
    // The index form keeps spans exactly: overlapping, crossing and repeated.
    let crossing = ok(
        "\"length\": 1\nthumb: Bass:4 Bass:4 Bass:4 Bass:4\nfingers: 1:4\nthumb pm#0..2\nfingers >#0\nthumb !#1..3\nthumb pm#0..2",
    );
    assert_eq!(crossing.spans.len(), 4);
    assert_eq!(
        crossing
            .spans
            .iter()
            .map(|s| (s.lane, s.technique))
            .collect::<Vec<_>>(),
        vec![
            (LaneId::Thumb, Technique::PalmMute),
            (LaneId::Fingers, Technique::Accent),
            (LaneId::Thumb, Technique::Dead),
            (LaneId::Thumb, Technique::PalmMute),
        ]
    );
}

#[test]
fn headers_carry_the_meta_exactly() {
    let p = ok(
        "\"Name\": \"A \\\"quoted\\\" \\\\ name\\n\\u{1}\"\n\"length\": 2/3\n\"description\": \"d\"\n\
         \"tags\": \"a\", \"\" ,\"c d\"\n\"origin\": \"TAP\"\nstrum:",
    );
    assert_eq!(p.name, "A \"quoted\" \\ name\n\u{1}");
    assert_eq!(p.length, r(2, 3));
    assert_eq!(p.meta.description, "d");
    assert_eq!(
        p.meta.tags,
        vec!["a".to_string(), String::new(), "c d".into()]
    );
    assert_eq!(p.meta.origin, Some(Origin::Tap));
    let lib = PatternLibrary::new();
    assert_eq!(ok(&text::print(&p, &lib).unwrap()), p);
    // Comments and white space are free.
    assert_eq!(
        ok("// c\n\"length\": 1 // x\n\n  strum: // y\n  Bass:4"),
        ok("\"length\": 1\nstrum: Bass:4")
    );
}

// ---------------------------------------------------------------- bar checks

#[test]
fn bar_checks_need_a_meter_and_a_bar_line() {
    let good = ok("\"length\": 3/2\nmeter 3/4\nstrum: | Bass:4 Bass:4 Bass:4 | Bass:2. |");
    let Lanes::Strum(l) = &good.lanes else {
        panic!()
    };
    assert_eq!(l.steps.len(), 4);
    // The meter may be declared after the lane.
    ok("\"length\": 1\nstrum: Bass:2 | Bass:2\nmeter 2/4");
    assert_eq!(
        bad("\"length\": 1\nmeter 4/4\nstrum: Bass:4 | Bass:4"),
        (
            K::BarCheckOffBarLine {
                position: r(1, 4),
                bar_length: Rational::ONE
            },
            "|".into()
        )
    );
    assert_eq!(
        bad("\"length\": 1\nstrum: Bass:4 | Bass:4"),
        (K::BarCheckWithoutMeter, "|".into())
    );
    // The position counts gaps and explicit onsets.
    ok("\"length\": 1\nmeter 4/4\nstrum: @1/2 Bass:2 |");
    assert!(matches!(
        bad("\"length\": 1\nmeter 4/4\nstrum: @1/2 Bass:4 |").0,
        K::BarCheckOffBarLine { .. }
    ));
}

#[test]
fn the_printer_writes_a_bar_check_only_where_it_parses_back() {
    let lib = PatternLibrary::new();
    // A gap before the step at the bar line: no check, because the position there is
    // not the bar line.
    let p = ok("\"length\": 2\nmeter 4/4\nstrum: Bass:1 @5/4 Bass:4");
    let out = text::print(&p, &lib).unwrap();
    assert!(!out.contains('|'), "{out}");
    assert_eq!(ok(&out), p);
    let q = ok("\"length\": 2\nmeter 4/4\nstrum: Bass:1 Bass:1");
    let out = text::print(&q, &lib).unwrap();
    assert!(out.contains("strum: Bass:1 | Bass:1"), "{out}");
}

// ------------------------------------------------------------------- errors

#[test]
fn a_step_without_a_value_and_without_a_unit_is_an_error_with_a_span() {
    assert_eq!(
        bad("\"length\": 1\nstrum: Bass Treble:4"),
        (K::NoValue, "Bass".into())
    );
    assert_eq!(
        bad("\"length\": 1\nstrum: Bass:4 D >sp(Strum)"),
        (K::NoValue, "D >sp(Strum)".into())
    );
    assert_eq!(
        bad("\"length\": 1\nstrum: Bass:4 -"),
        (K::NoValue, "-".into())
    );
    // The unit must come before the step in the text.
    assert_eq!(
        bad("\"length\": 1\nstrum: Bass unit 4"),
        (K::NoValue, "Bass".into())
    );
    // The line and the column point at the step.
    let e = parse("\"length\": 1\n\nstrum: Bass:4\n   Treble").unwrap_err();
    assert_eq!((e.kind, e.line, e.column), (K::NoValue, 4, 4));
}

#[test]
fn syntax_errors_name_their_kind_and_span() {
    let l = "\"length\": 1\n";
    for (text, kind, span) in [
        ("strum: Bass:4", K::MissingLength, ""),
        (
            "\"length\": 1\n\"length\": 2\nstrum:",
            K::DuplicateHeader("length"),
            "\"length\"",
        ),
        (
            "\"length\": 1\n\"color\": \"x\"\nstrum:",
            K::UnknownKey("color".into()),
            "\"color\"",
        ),
        (
            "\"length\": 1\n\"origin\": \"drawn\"\nstrum:",
            K::BadOrigin("drawn".into()),
            "\"drawn\"",
        ),
        (
            "\"length\": 1\n\"name\": \"open",
            K::UnterminatedString,
            "\"open",
        ),
        (
            "\"length\": 1\n\"name\": \"a\\qb\"\nstrum:",
            K::BadEscape,
            "\\q",
        ),
        (
            "\"length\": 1\n\"name\": \"\\u{110000}\"\nstrum:",
            K::BadEscape,
            "\\u{110000}",
        ),
        ("\"length\": 1/0\nstrum:", K::BadRational, "1/0"),
        ("\"length\": 1\nmeter 0/4\nstrum:", K::BadMeter, "0/4"),
        (
            "\"length\": 1\nBass:4",
            K::StepOutsideLane,
            "Bass:4"[..1].into(),
        ),
    ] {
        let e = parse(text).expect_err(text);
        assert_eq!(e.kind, kind, "{text}");
        assert_eq!(&text[e.span.clone()], span, "{text}");
    }
    for (body, kind, span) in [
        ("strum: 0:4", K::InvalidString(0), "0"),
        ("strum: 256:4", K::InvalidString(256), "256"),
        ("strum: Strum(3..9999):4", K::InvalidString(9999), "9999"),
        ("strum: Bass:3", K::BadDuration, "3"),
        ("strum: Bass:512", K::BadDuration, "512"),
        ("strum: Bass:8(2:2)", K::BadDuration, "8(2:2)"),
        ("strum: Bass:8(300:2)", K::BadTuplet, "8(300:2)"),
        ("strum: Bass:4....", K::BadDuration, "4...."),
        ("strum: D -:4", K::MarksOnNonStrike, "D -"),
        ("strum: >_:4", K::MarksOnNonStrike, ">_"),
        ("strum: Zed:4", K::UnknownGesture("Zed".into()), "Zed"),
        (
            "strum: thumb:4",
            K::KeywordNotAllowed("thumb".into()),
            "thumb",
        ),
        ("strum: [pm ] Bass:4", K::EmptyBracket, "[pm ]"),
        ("strum: [pm Bass:4", K::UnclosedBracket, "["),
        ("strum: [Bass:4]", K::Expected("a technique after '['"), "B"),
        ("strum: Bass:", K::Expected("a note value"), ""),
        ("strum: Bass:4 ?", K::Expected("a step"), "?"),
        ("strum: Treble(x):4", K::Expected("a string count"), "x"),
        ("strum: Strum(3):4", K::Expected("'..'"), ")"),
        ("strum: 4:", K::Expected("a note value"), ""),
    ] {
        let text = format!("{l}{body}");
        let e = parse(&text).expect_err(&text);
        assert_eq!(e.kind, kind, "{body}");
        assert_eq!(&text[e.span.clone()], span, "{body}");
    }
}

#[test]
fn lane_structure_errors() {
    let l = "\"length\": 1\n";
    for (body, kind) in [
        ("", K::NoLane),
        ("strum: Bass:4\nthumb: Bass:4", K::LaneMix),
        ("thumb: Bass:4\nstrum: Bass:4", K::LaneMix),
        ("strum: Bass:4\nstrum: Bass:4", K::DuplicateLane),
        ("thumb: Bass:4\nthumb: Bass:4\nfingers:", K::DuplicateLane),
        ("thumb: Bass:4", K::MissingFingers),
        ("fingers: 1:4", K::MissingThumb),
    ] {
        let e = parse(&format!("{l}{body}")).expect_err(body);
        assert_eq!(e.kind, kind, "{body}");
    }
    // Either order of the two lanes is the same IR.
    assert_eq!(
        ok(&format!("{l}thumb: Bass:4\nfingers: 1:4")),
        ok(&format!("{l}fingers: 1:4\nthumb: Bass:4"))
    );
}

#[test]
fn lane_headers_are_recognised_only_at_the_start_of_a_line_with_a_colon_and_space() {
    // `Strum:4` is a step, even at the start of a line.
    let p = ok("\"length\": 1\nstrum: Bass:4\nStrum:4 Strum:4");
    let Lanes::Strum(l) = &p.lanes else { panic!() };
    assert_eq!(l.steps.len(), 3);
    // Indented headers are headers.
    let p = ok("\"length\": 1\n   thumb:  Bass:4\n\tfingers:\n");
    assert!(matches!(p.lanes, Lanes::ThumbFingers { .. }));
}

#[test]
fn syntax_valid_but_musically_invalid_text_is_rejected_by_the_library_not_the_parser() {
    let mut lib = PatternLibrary::new();
    // Overlapping steps, a hold with no strike, a span past the end.
    let text = "\"length\": 1\nstrum: Bass:2 @1/4 Treble:4 _:4\nstrum pm#0..9";
    assert!(text::parse(text, &lib).is_ok());
    let Err(text::TextError::Invalid(errors)) = lib.define_pattern_text(text) else {
        panic!()
    };
    assert!(
        errors
            .iter()
            .any(|x| matches!(x, PatternError::Overlap { .. }))
    );
    assert!(
        errors
            .iter()
            .any(|x| matches!(x, PatternError::SpanOutOfRange { .. }))
    );
    assert_eq!(lib.patterns().count(), 0, "nothing changed");
    assert!(matches!(
        lib.define_pattern_text("strum: Bass:4"),
        Err(text::TextError::Parse(ParseError {
            kind: K::MissingLength,
            ..
        }))
    ));
}

// ----------------------------------------------------------- the printer's own

#[test]
fn the_printer_refuses_what_it_cannot_write() {
    let lib = PatternLibrary::new();
    let ghost = GestureId::from_raw(77);
    let p = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![(q(), Action::Gesture(ghost))])),
    );
    assert_eq!(
        text::print(&p, &lib),
        Err(text::PrintError::UnknownGesture(ghost))
    );
    let p = pattern(Rational::ONE, Lanes::Strum(run(vec![(q(), strike(&[]))])));
    assert_eq!(
        text::print(&p, &lib),
        Err(text::PrintError::NoTargets {
            lane: LaneId::Strum,
            step: 0
        })
    );
    let p = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![(q(), Action::Hold(Some(vec![])))])),
    );
    assert!(matches!(
        text::print(&p, &lib),
        Err(text::PrintError::NoTargets { .. })
    ));
}

// ------------------------------------------------------------ property tests

fn meta_strategy() -> impl Strategy<Value = (String, String, Vec<String>, Option<Origin>)> {
    (
        any::<String>(),
        any::<String>(),
        proptest::collection::vec(any::<String>(), 0..3),
        prop_oneof![
            Just(None),
            Just(Some(Origin::Grid)),
            Just(Some(Origin::Text)),
            Just(Some(Origin::Tap))
        ],
    )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 500, .. ProptestConfig::default() })]

    /// PE-10: `parse(print(ir)) == ir` for every valid IR (gestures, holds, rests, gaps,
    /// every kind of tuplet and dot, crossing spans, any strings in the meta), and the
    /// printer is canonical: printing the parsed text gives the same text.
    #[test]
    fn pe10_parse_of_print_is_the_identity(seed in pattern_seeds(), ext in any::<bool>(), meta in meta_strategy()) {
        let (lib, g) = gesture_library();
        let values = if ext { ext_values() } else { values() };
        let mut p = build_pattern_with(&values, &seed, &g);
        (p.name, p.meta.description, p.meta.tags, p.meta.origin) = meta;
        prop_assert!(lib.validate_pattern(&p).is_empty());
        let printed = text::print(&p, &lib).unwrap();
        let back = text::parse(&printed, &lib);
        prop_assert!(back.is_ok(), "{:?}\n{}", back.err(), printed);
        let back = back.unwrap();
        prop_assert_eq!(&back, &p, "\n{}", printed);
        prop_assert_eq!(text::print(&back, &lib).unwrap(), printed);
        // And the library accepts it as a pattern.
        let mut lib = lib;
        prop_assert!(lib.define_pattern_text(&text::print(&p, &lib).unwrap()).is_ok());
    }

    /// Any text the parser accepts and the library validates is also fixed by
    /// print/parse: the first print is the canonical form.
    #[test]
    fn pe10_the_canonical_form_is_stable(seed in pattern_seeds()) {
        let (lib, g) = gesture_library();
        let p = build_pattern_with(&ext_values(), &seed, &g);
        let once = text::print(&p, &lib).unwrap();
        let twice = text::print(&text::parse(&once, &lib).unwrap(), &lib).unwrap();
        prop_assert_eq!(once, twice);
    }

    /// The parser never panics and always reports a span inside the input, whatever
    /// the bytes (also not on cut-off or mutated pattern text).
    #[test]
    fn pe10_the_parser_never_panics_and_its_spans_stay_inside_the_text(
        seed in pattern_seeds(), cut in any::<usize>(), junk in any::<String>(), at in any::<usize>(),
    ) {
        let (lib, g) = gesture_library();
        let printed = text::print(&build_pattern_with(&ext_values(), &seed, &g), &lib).unwrap();
        let mut chars: Vec<char> = printed.chars().collect();
        chars.truncate(cut % (chars.len() + 1));
        let cutoff: String = chars.iter().collect();
        let mut mutated = printed.clone();
        let idx = mutated.char_indices().map(|(i, _)| i).nth(at % (printed.chars().count() + 1)).unwrap_or(mutated.len());
        mutated.insert_str(idx, &junk);
        for text in [cutoff, mutated, junk] {
            if let Err(e) = text::parse(&text, &lib) {
                prop_assert!(e.span.start <= e.span.end && e.span.end <= text.len());
                prop_assert!(text.is_char_boundary(e.span.start) && text.is_char_boundary(e.span.end));
                prop_assert!(e.line >= 1 && e.column >= 1);
            }
        }
    }
}

proptest! {
    /// Patterns that run over several bars of their meter: the printer writes bar
    /// checks at the bar lines, and they parse back to the same IR.
    #[test]
    fn pe10_bar_checks_at_bar_lines_round_trip(
        bits in proptest::collection::vec(0u8..3, 16), three_four in any::<bool>(),
    ) {
        let (lib, _) = gesture_library();
        let (meter, length, count) = if three_four {
            (TimeSignature::new(3, 4).unwrap(), Rational::new(3, 2).unwrap(), 12)
        } else {
            (TimeSignature::COMMON, Rational::new(2, 1).unwrap(), 16)
        };
        let items: Vec<(NoteValue, Action)> = bits
            .iter()
            .take(count)
            .map(|b| (e(), if *b == 0 { Action::Rest } else { k(&[Bass], None) }))
            .collect();
        let mut p = pattern(length, Lanes::Strum(run(items)));
        p.meter = Some(meter);
        prop_assert!(lib.validate_pattern(&p).is_empty());
        let printed = text::print(&p, &lib).unwrap();
        prop_assert!(printed.contains('|'), "{printed}");
        prop_assert_eq!(&text::parse(&printed, &lib).unwrap(), &p, "\n{}", printed);
    }
}

/// The round-trip property is not vacuous: its inputs include every feature the syntax
/// has to carry.
#[test]
fn the_round_trip_property_covers_every_syntax_feature() {
    use proptest::strategy::ValueTree;
    let (lib, g) = gesture_library();
    let mut runner = proptest::test_runner::TestRunner::deterministic();
    let mut seen = std::collections::BTreeMap::<&str, usize>::new();
    for _ in 0..500 {
        let seed = pattern_seeds().new_tree(&mut runner).unwrap().current();
        let p = build_pattern_with(&ext_values(), &seed, &g);
        let out = text::print(&p, &lib).unwrap();
        for (key, needle) in [
            ("gap", "@"),
            ("rest", "-:"),
            ("hold all", "_:"),
            ("hold some", "_("),
            ("bar check", "|"),
            ("span", "#"),
            ("dead", "!"),
            ("let ring", "~"),
            ("palm mute", "pm("),
            ("string-percussive", "sp("),
            ("accent", ">"),
            ("up", "U "),
            ("down", "D "),
            ("range", "Strum("),
            ("treble n", "Treble("),
            ("dotted", "."),
            ("triplet", "t "),
            ("quintuplet", "(5:4)"),
            ("septuplet", "(7:4)"),
            ("gesture", "Bùm"),
            ("two lanes", "thumb:"),
        ] {
            if out.contains(needle) {
                *seen.entry(key).or_default() += 1;
            }
        }
        if p.meter.is_some() {
            *seen.entry("meter").or_default() += 1;
        }
    }
    println!("{seen:?}");
    for key in [
        "gap",
        "rest",
        "hold all",
        "hold some",
        "span",
        "dead",
        "let ring",
        "palm mute",
        "string-percussive",
        "accent",
        "up",
        "down",
        "range",
        "treble n",
        "dotted",
        "triplet",
        "quintuplet",
        "septuplet",
        "gesture",
        "two lanes",
        "meter",
    ] {
        assert!(
            seen.get(key).copied().unwrap_or(0) > 0,
            "{key} never printed: {seen:?}"
        );
    }
}
