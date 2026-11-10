//! ADR-034 PE-3 (binding follows section 5), PE-6 (every technique, direction and
//! accent reaches each note exactly once), PT-02, and the bind part of PE-9.

mod common;
use common::*;
use mc_guitar::Voicing;
use mc_pattern::*;
use mc_score::{FrettedInstrument, NoteEffects};
use mc_theory::fretted::{Capo, StringIndex, Tuning};
use mc_theory::{Chord, Rational, chord_symbol};
use proptest::prelude::*;

use StringSelector::{Alt, Bass, Strum};

fn voicing(shape: &str) -> Voicing {
    shape.parse().unwrap_or_else(|e| panic!("{shape}: {e:?}"))
}

fn chord(symbol: &str) -> Chord {
    chord_symbol::parse(symbol).unwrap()
}

fn tuning(pitches: [&str; 6]) -> Tuning {
    Tuning::new(pitches.map(|p| p.parse().unwrap()).to_vec()).unwrap()
}

fn standard(capo: u8) -> FrettedInstrument {
    FrettedInstrument {
        capo: Capo(capo),
        ..FrettedInstrument::standard_guitar()
    }
}

fn drop_d(capo: u8) -> FrettedInstrument {
    FrettedInstrument {
        tuning: tuning(["E4", "B3", "G3", "D3", "A2", "D2"]),
        fret_count: 24,
        capo: Capo(capo),
    }
}

fn dadgad(capo: u8) -> FrettedInstrument {
    FrettedInstrument {
        tuning: tuning(["D4", "A3", "G3", "D3", "A2", "D2"]),
        fret_count: 24,
        capo: Capo(capo),
    }
}

/// Binds a one-step Strum pattern striking `targets`; the strings of its notes.
fn strings_of(
    targets: &[StringSelector],
    v: &Voicing,
    c: &Chord,
    i: &FrettedInstrument,
) -> Result<Vec<u8>, Vec<BindError>> {
    let mut lib = PatternLibrary::new();
    let id = lib
        .define_pattern(pattern(
            Rational::ONE,
            Lanes::Strum(run(vec![(q(), strike(targets))])),
        ))
        .unwrap();
    match lib.bind(id, v, c, i) {
        Ok(f) => Ok(f.lanes[0].beats[0]
            .notes
            .iter()
            .map(|n| n.position.string.get())
            .collect()),
        Err(BindFailure::Bind(e)) => Err(e),
        Err(other) => panic!("{other:?}"),
    }
}

fn at0() -> StepRef {
    StepRef {
        lane: LaneId::Strum,
        index: 0,
        onset: Rational::ZERO,
    }
}

// ------------------------------------------------------------- PE-3: Alt

#[test]
fn pe3_the_four_alt_fixtures_give_the_stated_results() {
    let i = standard(0);
    for (shape, symbol, expected) in [("x32010", "C", 4), ("320003", "G", 4), ("x02210", "Am", 4)] {
        assert_eq!(
            strings_of(&[Alt], &voicing(shape), &chord(symbol), &i),
            Ok(vec![expected]),
            "{symbol} {shape}"
        );
    }
    assert_eq!(
        strings_of(&[Alt], &voicing("xx0232"), &chord("D"), &i),
        Err(vec![BindError::NoAlternateBass { step: at0() }])
    );
}

#[test]
fn pe3_alt_prefers_the_fifth_then_the_third_then_the_lowest_candidate() {
    let i = standard(0);
    // G 320003: candidates 5 (B, the third) and 4 (D, the fifth): the fifth wins.
    assert_eq!(
        strings_of(&[Alt], &voicing("320003"), &chord("G"), &i),
        Ok(vec![4])
    );
    // 322003 (6 to 1) has candidates 5 (B, the third) and 4 (E, not a G chord tone):
    // with no fifth among them, the third wins.
    assert_eq!(
        strings_of(&[Alt], &voicing("322003"), &chord("G"), &i),
        Ok(vec![5])
    );
    // Gsus4 has no third and its fifth is not among the candidates 5 (B) and 4 (E):
    // the lowest candidate, string 5.
    assert_eq!(
        strings_of(&[Alt], &voicing("322003"), &chord("Gsus4"), &i),
        Ok(vec![5])
    );
}

// ------------------------------------------------- PE-3: the selector table

/// Expected strings, string numbers in ascending order (string 1 first), per selector:
/// Bass, Treble(1..=5), Strum.
struct Row {
    name: &'static str,
    instrument: FrettedInstrument,
    shape: &'static str,
    symbol: &'static str,
    bass: u8,
    treble: [&'static [u8]; 5],
    alt: Option<u8>,
}

fn table() -> Vec<Row> {
    let row = |name, instrument, shape, symbol, bass, treble, alt| Row {
        name,
        instrument,
        shape,
        symbol,
        bass,
        treble,
        alt,
    };
    let t3 = [
        &[1u8][..],
        &[1, 2],
        &[1, 2, 3],
        &[1, 2, 3, 4],
        &[1, 2, 3, 4],
    ];
    let t6 = [
        &[1u8][..],
        &[1, 2],
        &[1, 2, 3],
        &[1, 2, 3, 4],
        &[1, 2, 3, 4, 5],
    ];
    vec![
        row("C standard", standard(0), "x32010", "C", 5, t3, Some(4)),
        row("G standard", standard(0), "320003", "G", 6, t6, Some(4)),
        row("Am standard", standard(0), "x02210", "Am", 5, t3, Some(4)),
        row(
            "D standard",
            standard(0),
            "xx0232",
            "D",
            4,
            [&[1], &[1, 2], &[1, 2, 3], &[1, 2, 3], &[1, 2, 3]],
            None,
        ),
        // E 022100: S = 6..1; candidates 5 (B, fifth) and 4 (E, root): string 5.
        row("E standard", standard(0), "022100", "E", 6, t6, Some(5)),
        // F barre 133211: candidates 5 (C, fifth) and 4 (F): string 5.
        row("F barre", standard(0), "133211", "F", 6, t6, Some(5)),
        // C/G 332010: the bass is G on string 6 (decision 13.3); candidates 5 (C) and
        // 4 (E, third); the fifth G is not a candidate: string 4.
        row("C/G slash", standard(0), "332010", "C/G", 6, t6, Some(4)),
        // G/B x20003: bass B on string 5; candidate 4 (D, fifth).
        row(
            "G/B inversion",
            standard(0),
            "x20003",
            "G/B",
            5,
            t3,
            Some(4),
        ),
        // Capo 2, shape C x32010 sounds D: candidate 4 sounds F#, the third.
        row(
            "C shape capo 2 (D)",
            standard(2),
            "x32010",
            "D",
            5,
            t3,
            Some(4),
        ),
        // Capo 3, shape Am sounds Cm: candidate 4 sounds G, the fifth.
        row(
            "Am shape capo 3 (Cm)",
            standard(3),
            "x02210",
            "Cm",
            5,
            t3,
            Some(4),
        ),
        // Drop D, D 000232: S = 6..1; candidates 5 (A, fifth) and 4 (D): string 5.
        row("D drop D", drop_d(0), "000232", "D", 6, t6, Some(5)),
        // Drop D, G 550433 (6 to 1): bass G on string 6; candidates 5 and 4 both sound
        // D, the fifth: the lowest of them, string 5.
        row("G drop D", drop_d(0), "550433", "G", 6, t6, Some(5)),
        // Drop D capo 2, D shape 000232 sounds E: candidate 5 sounds B, the fifth.
        row(
            "D shape drop D capo 2 (E)",
            drop_d(2),
            "000232",
            "E",
            6,
            t6,
            Some(5),
        ),
        // DADGAD open: Dsus4 000000; candidates 5 (A, fifth) and 4 (D): string 5.
        row("Dsus4 DADGAD", dadgad(0), "000000", "Dsus4", 6, t6, Some(5)),
        // DADGAD capo 2, xx0000 sounds Esus4 on four strings: bass on string 4, the upper
        // region takes the other three, so there is no Alt candidate.
        row(
            "Dsus4 four strings DADGAD capo 2 (Esus4)",
            dadgad(2),
            "xx0000",
            "Esus4",
            4,
            [&[1], &[1, 2], &[1, 2, 3], &[1, 2, 3], &[1, 2, 3]],
            None,
        ),
    ]
}

#[test]
fn pe3_bass_treble_alt_strum_and_absolute_on_fourteen_voicings() {
    let rows = table();
    assert!(rows.len() >= 12);
    for row in &rows {
        let v = voicing(row.shape);
        let c = chord(row.symbol);
        let i = &row.instrument;
        let sounded: Vec<u8> = (1..=6)
            .filter(|s| v.fret(StringIndex::new(*s).unwrap()).is_some())
            .collect();
        assert_eq!(
            strings_of(&[Bass], &v, &c, i),
            Ok(vec![row.bass]),
            "{} Bass",
            row.name
        );
        for n in 1..=5u8 {
            assert_eq!(
                strings_of(&[StringSelector::Treble(n)], &v, &c, i),
                Ok(row.treble[n as usize - 1].to_vec()),
                "{} Treble({n})",
                row.name
            );
        }
        assert_eq!(
            strings_of(&[Strum], &v, &c, i),
            Ok(sounded.clone()),
            "{} Strum",
            row.name
        );
        match row.alt {
            Some(a) => assert_eq!(
                strings_of(&[Alt], &v, &c, i),
                Ok(vec![a]),
                "{} Alt",
                row.name
            ),
            None => assert_eq!(
                strings_of(&[Alt], &v, &c, i),
                Err(vec![BindError::NoAlternateBass { step: at0() }]),
                "{} Alt",
                row.name
            ),
        }
        for string in 1..=6u8 {
            let r = strings_of(&[s(string)], &v, &c, i);
            if sounded.contains(&string) {
                assert_eq!(r, Ok(vec![string]), "{} {string}", row.name);
            } else {
                assert_eq!(
                    r,
                    Err(vec![BindError::StringNotInVoicing {
                        step: at0(),
                        string
                    }]),
                    "{} {string}",
                    row.name
                );
            }
        }
        // Every note is on a sounded string, at the voicing's fret.
        let mut lib = PatternLibrary::new();
        let id = lib
            .define_pattern(pattern(
                Rational::ONE,
                Lanes::Strum(run(vec![(q(), strike(&[Strum]))])),
            ))
            .unwrap();
        let f = lib.bind(id, &v, &c, i).unwrap();
        for n in &f.lanes[0].beats[0].notes {
            assert_eq!(
                Some(n.position.fret),
                v.fret(n.position.string),
                "{}",
                row.name
            );
        }
    }
}

#[test]
fn pe3_strum_range_and_the_union_of_targets() {
    let (v, c, i) = (voicing("x32010"), chord("C"), standard(0));
    let range = |a: u8, b: u8| {
        StringSelector::StrumRange(StringIndex::new(a).unwrap(), StringIndex::new(b).unwrap())
    };
    assert_eq!(
        strings_of(&[range(6, 4)], &v, &c, &i),
        Ok(vec![4, 5]),
        "string 6 is muted"
    );
    assert_eq!(strings_of(&[range(1, 3)], &v, &c, &i), Ok(vec![1, 2, 3]));
    assert_eq!(
        strings_of(&[range(3, 1)], &v, &c, &i),
        Ok(vec![1, 2, 3]),
        "either order"
    );
    // "+": the union, each string once.
    assert_eq!(
        strings_of(&[Bass, StringSelector::TREBLE, s(1)], &v, &c, &i),
        Ok(vec![1, 2, 3, 5])
    );
    assert_eq!(
        strings_of(&[Strum, Bass], &v, &c, &i),
        Ok(vec![1, 2, 3, 4, 5])
    );
}

// ------------------------------------------------- PT-02: the typed failures

#[test]
fn pt02_each_bind_failure_is_a_typed_error() {
    let i = standard(0);
    let (c, cv) = (chord("C"), voicing("x32010"));
    assert_eq!(
        strings_of(&[s(7)], &cv, &c, &i),
        Err(vec![BindError::StringOutOfRange {
            step: at0(),
            string: 7,
            strings: 6
        }])
    );
    assert_eq!(
        strings_of(&[s(6)], &cv, &c, &i),
        Err(vec![BindError::StringNotInVoicing {
            step: at0(),
            string: 6
        }])
    );
    assert_eq!(
        strings_of(&[Alt], &voicing("xx0232"), &chord("D"), &i),
        Err(vec![BindError::NoAlternateBass { step: at0() }])
    );
    // A single sounded string: nothing above the bass.
    assert_eq!(
        strings_of(
            &[StringSelector::TREBLE],
            &voicing("xxxxx0"),
            &chord("E"),
            &i
        ),
        Err(vec![BindError::NoTrebleStrings { step: at0() }])
    );
    let range =
        StringSelector::StrumRange(StringIndex::new(6).unwrap(), StringIndex::new(5).unwrap());
    assert_eq!(
        strings_of(&[range], &voicing("xx0232"), &chord("D"), &i),
        Err(vec![BindError::EmptyStrike { step: at0() }])
    );
    // Instrument-level failures.
    assert_eq!(
        strings_of(&[Bass], &voicing("x3201"), &c, &i),
        Err(vec![BindError::VoicingStringCount {
            voicing: 5,
            instrument: 6
        }])
    );
    assert_eq!(
        strings_of(&[Bass], &voicing("xxxxxx"), &c, &i),
        Err(vec![BindError::NoSoundedStrings])
    );
    let short = FrettedInstrument {
        fret_count: 2,
        ..standard(0)
    };
    assert_eq!(
        strings_of(&[Bass], &cv, &c, &short),
        Err(vec![BindError::FretOutOfRange {
            string: 5,
            fret: 3,
            fret_count: 2
        }])
    );
}

// ------------------------------------------------- holds and spans (PE-6)

#[test]
fn pe6_a_hold_ties_only_the_held_strings_and_keeps_only_let_ring() {
    let (v, c, i) = (voicing("x32010"), chord("C"), standard(0));
    let mut k = Strike::on(vec![Bass, StringSelector::TREBLE]);
    k.techniques = NoteEffects {
        accent: true,
        let_ring: true,
        ..NoteEffects::default()
    };
    k.direction = Some(Direction::Down);
    let mut lib = PatternLibrary::new();
    let id = lib
        .define_pattern(pattern(
            Rational::ONE,
            Lanes::Strum(run(vec![
                (q(), Action::Strike(k)),
                (q(), Action::Hold(None)),
                (q(), Action::Hold(Some(vec![Bass]))),
                (q(), Action::Rest),
            ])),
        ))
        .unwrap();
    let f = lib.bind(id, &v, &c, &i).unwrap();
    let beats = &f.lanes[0].beats;
    assert_eq!(beats.len(), 3, "the rest is not a beat (PT-09)");
    let strings = |b: &BoundBeat| {
        b.notes
            .iter()
            .map(|n| n.position.string.get())
            .collect::<Vec<_>>()
    };
    assert_eq!(strings(&beats[0]), vec![1, 2, 3, 5]);
    assert_eq!(strings(&beats[1]), vec![1, 2, 3, 5]);
    assert_eq!(strings(&beats[2]), vec![5]);
    assert_eq!(beats[0].stroke, Some(Direction::Down));
    assert!(beats[1..].iter().all(|b| b.stroke.is_none()));
    for b in &beats[1..] {
        for n in &b.notes {
            assert!(n.tie);
            assert_eq!(
                n.effects,
                NoteEffects {
                    let_ring: true,
                    ..NoteEffects::default()
                }
            );
        }
    }
    assert!(beats[0].notes.iter().all(|n| !n.tie && n.effects.accent));
}

#[test]
fn a_hold_of_a_short_note_is_rejected() {
    let (v, c, i) = (voicing("x32010"), chord("C"), standard(0));
    for t in [
        Technique::Dead,
        Technique::PalmMute,
        Technique::StringPercussive,
    ] {
        let mut lib = PatternLibrary::new();
        let mut p = pattern(
            Rational::ONE,
            Lanes::Strum(run(vec![(q(), strike(&[Bass])), (q(), Action::Hold(None))])),
        );
        p.spans.push(TechniqueSpan {
            lane: LaneId::Strum,
            first: 0,
            last: 0,
            technique: t,
        });
        let id = lib.define_pattern(p).unwrap();
        assert_eq!(
            lib.bind(id, &v, &c, &i).unwrap_err(),
            BindFailure::Bind(vec![BindError::HoldOfShortNote {
                step: StepRef {
                    lane: LaneId::Strum,
                    index: 1,
                    onset: r(1, 4)
                },
                string: 5,
            }]),
            "{t:?}"
        );
    }
}

#[test]
fn a_span_that_reaches_no_attack_is_rejected() {
    let (v, c, i) = (voicing("x32010"), chord("C"), standard(0));
    let mut lib = PatternLibrary::new();
    let mut p = pattern(
        Rational::ONE,
        Lanes::Strum(run(vec![
            (q(), strike(&[Bass])),
            (q(), Action::Hold(None)),
            (q(), Action::Rest),
        ])),
    );
    p.spans.push(TechniqueSpan {
        lane: LaneId::Strum,
        first: 1,
        last: 2,
        technique: Technique::Accent,
    });
    p.spans.push(TechniqueSpan {
        lane: LaneId::Strum,
        first: 0,
        last: 2,
        technique: Technique::Accent,
    });
    let id = lib.define_pattern(p).unwrap();
    assert_eq!(
        lib.bind(id, &v, &c, &i).unwrap_err(),
        BindFailure::Bind(vec![BindError::SpanReachesNoAttack { span: 0 }])
    );
}

// ------------------------------------------------- PE-9, bind part

#[test]
fn pe9_a_gesture_binds_identically_in_every_pattern_and_after_a_rename() {
    let (mut lib, g) = gesture_library();
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
                thumb: run(vec![(q(), Action::Gesture(bum)), (q(), strike(&[Bass]))]),
                fingers: run(vec![(e(), Action::Rest), (e(), Action::Gesture(kich))]),
            },
        ))
        .unwrap();
    let (v, c, i) = (voicing("x32010"), chord("C"), standard(0));
    let bound = |lib: &PatternLibrary, id| lib.bind(id, &v, &c, &i).unwrap();
    let (fa, fb) = (bound(&lib, a), bound(&lib, b));
    // The notes and stroke of a gesture step, by gesture.
    let of = |f: &PatternFragment,
              lib: &PatternLibrary,
              id: PatternId|
     -> Vec<(GestureId, Vec<BoundNote>, Option<Direction>)> {
        let p = lib.pattern(id).unwrap().parts();
        let mut out = Vec::new();
        for ((_, lane), bl) in p.lanes.iter().zip(&f.lanes) {
            for beat in &bl.beats {
                if let Action::Gesture(gid) = lane.steps[beat.step].action {
                    out.push((gid, beat.notes.clone(), beat.stroke));
                }
            }
        }
        out
    };
    let in_a = of(&fa, &lib, a);
    let in_b = of(&fb, &lib, b);
    assert_eq!(in_a.len(), 4);
    assert_eq!(in_b.len(), 2);
    for (ga, na, sa) in &in_a {
        for (gb, nb, sb) in &in_b {
            if ga == gb {
                assert_eq!((na, sa), (nb, sb), "gesture {ga:?} binds identically");
            }
        }
    }
    // Bùm = Bass + palm mute + accent on C: string 5, fret 3, both flags.
    let bum_notes = &in_a[0].1;
    assert_eq!(bum_notes.len(), 1);
    assert_eq!(bum_notes[0].position.string.get(), 5);
    assert!(bum_notes[0].effects.palm_mute && bum_notes[0].effects.accent);
    // A rename changes no bound result.
    lib.rename_gesture(bum, "Boom").unwrap();
    lib.rename_gesture(kich, "Chuck").unwrap();
    assert_eq!(bound(&lib, a), fa);
    assert_eq!(bound(&lib, b), fb);
}

// ------------------------------------------------- property (PE-3, PE-6)

fn voicing_strategy() -> impl Strategy<Value = Voicing> {
    proptest::collection::vec(prop_oneof![Just(None), (0u8..13).prop_map(Some)], 6)
        .prop_filter("at least one sounded string", |v| {
            v.iter().any(|f| f.is_some())
        })
        .prop_map(|frets| {
            Voicing::new(
                frets
                    .into_iter()
                    .map(|f| f.map(mc_theory::fretted::Fret))
                    .collect(),
            )
            .unwrap()
        })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 400, .. ProptestConfig::default() })]

    /// Every bound note is on a sounded string at the voicing's fret; every attack
    /// carries exactly the strike's techniques plus its spans' and the strike's
    /// direction; holds are tied continuations; every failure is typed and names a
    /// step of the pattern.
    #[test]
    fn pe3_pe6_binding_random_patterns_to_random_voicings(
        seed in pattern_seeds(),
        v in voicing_strategy(),
        tuning_kind in 0u8..3,
        capo in 0u8..4,
        root in 0usize..7,
    ) {
        let (mut lib, g) = gesture_library();
        let p = build_pattern(&seed, &g);
        let id = lib.define_pattern(p.clone()).unwrap();
        let i = match tuning_kind { 0 => standard(capo), 1 => drop_d(capo), _ => dadgad(capo) };
        let c = chord(["C", "D", "E", "F", "G", "A", "B"][root]);
        let resolved = lib.resolve(id).unwrap();
        match lib.bind(id, &v, &c, &i) {
            Ok(f) => {
                prop_assert_eq!(f.lanes.len(), resolved.lanes.len());
                for ((lane, steps), bl) in resolved.lanes.iter().zip(&f.lanes) {
                    prop_assert_eq!(*lane, bl.lane);
                    // One beat per strike or hold step, in order; none for rests.
                    let expect: Vec<usize> = steps.iter().enumerate()
                        .filter(|(_, s)| !matches!(s.action, ResolvedAction::Rest))
                        .map(|(k, _)| k).collect();
                    prop_assert_eq!(bl.beats.iter().map(|b| b.step).collect::<Vec<_>>(), expect);
                    let mut prev: Option<&BoundBeat> = None;
                    for beat in &bl.beats {
                        let step = &steps[beat.step];
                        prop_assert_eq!((beat.onset, beat.duration), (step.onset, step.duration));
                        let mut strings: Vec<u8> = beat.notes.iter().map(|n| n.position.string.get()).collect();
                        let sorted = { let mut s = strings.clone(); s.sort(); s.dedup(); s };
                        prop_assert_eq!(&strings, &sorted, "string 1 first, each once");
                        for n in &beat.notes {
                            prop_assert_eq!(Some(n.position.fret), v.fret(n.position.string));
                        }
                        match &step.action {
                            ResolvedAction::Strike(k) => {
                                let mut want = k.techniques;
                                for sp in resolved.spans.iter().filter(|s| s.lane == *lane && s.first <= beat.step && beat.step <= s.last) {
                                    match sp.technique {
                                        Technique::Accent => want.accent = true,
                                        Technique::PalmMute => want.palm_mute = true,
                                        Technique::Dead => want.dead = true,
                                        Technique::LetRing => want.let_ring = true,
                                        Technique::StringPercussive => want.string_percussive = true,
                                    }
                                }
                                prop_assert!(!beat.notes.is_empty());
                                for n in &beat.notes {
                                    prop_assert!(!n.tie);
                                    prop_assert_eq!(n.effects, want);
                                }
                                prop_assert_eq!(beat.stroke, k.direction);
                            }
                            ResolvedAction::Hold(_) => {
                                let before = prev.expect("a hold follows a beat");
                                prop_assert_eq!(beat.onset, before.onset.checked_add(before.duration.length()).unwrap());
                                prop_assert!(beat.stroke.is_none());
                                for n in &beat.notes {
                                    prop_assert!(n.tie);
                                    let src = before.notes.iter().find(|m| m.position == n.position);
                                    prop_assert!(src.is_some(), "held from the previous beat");
                                    prop_assert_eq!(n.effects, NoteEffects { let_ring: src.unwrap().effects.let_ring, ..NoteEffects::default() });
                                }
                                strings.clear();
                            }
                            ResolvedAction::Rest => unreachable!(),
                        }
                        prev = Some(beat);
                    }
                }
            }
            Err(BindFailure::Bind(errors)) => {
                prop_assert!(!errors.is_empty());
                for e in &errors {
                    let step = match e {
                        BindError::StringOutOfRange { step, .. }
                        | BindError::StringNotInVoicing { step, .. }
                        | BindError::NoAlternateBass { step }
                        | BindError::NoTrebleStrings { step }
                        | BindError::EmptyStrike { step }
                        | BindError::HoldOfShortNote { step, .. } => Some(*step),
                        BindError::SpanReachesNoAttack { span } => {
                            prop_assert!(*span < resolved.spans.len());
                            None
                        }
                        other => return Err(TestCaseError::fail(format!("unexpected {other:?}"))),
                    };
                    if let Some(step) = step {
                        let lane = resolved.lanes.iter().find(|(l, _)| *l == step.lane).unwrap();
                        prop_assert_eq!(lane.1[step.index].onset, step.onset);
                    }
                }
            }
            Err(other) => return Err(TestCaseError::fail(format!("{other:?}"))),
        }
    }
}

/// The binding property above is not vacuous: many random cases bind, and every typed
/// error kind occurs.
#[test]
fn the_binding_property_covers_success_and_every_error_kind() {
    use proptest::strategy::ValueTree;
    let (mut lib, g) = gesture_library();
    let mut runner = proptest::test_runner::TestRunner::deterministic();
    let mut seen = std::collections::BTreeMap::<&str, usize>::new();
    for k in 0..600 {
        let seed = pattern_seeds().new_tree(&mut runner).unwrap().current();
        let v = voicing_strategy().new_tree(&mut runner).unwrap().current();
        let id = lib.define_pattern(build_pattern(&seed, &g)).unwrap();
        let i = match k % 3 {
            0 => standard(0),
            1 => drop_d(1),
            _ => dadgad(2),
        };
        match lib.bind(id, &v, &chord("G"), &i) {
            Ok(f) => {
                *seen.entry("ok").or_default() += 1;
                if f.lanes
                    .iter()
                    .any(|l| l.beats.iter().any(|b| b.notes.iter().any(|n| n.tie)))
                {
                    *seen.entry("ok with holds").or_default() += 1;
                }
            }
            Err(BindFailure::Bind(e)) => {
                for x in e {
                    *seen
                        .entry(match x {
                            BindError::StringNotInVoicing { .. } => "not in voicing",
                            BindError::NoAlternateBass { .. } => "no alt",
                            BindError::NoTrebleStrings { .. } => "no treble",
                            BindError::EmptyStrike { .. } => "empty",
                            BindError::HoldOfShortNote { .. } => "hold of short",
                            BindError::SpanReachesNoAttack { .. } => "span without attack",
                            _ => "other",
                        })
                        .or_default() += 1;
                }
            }
            Err(other) => panic!("{other:?}"),
        }
    }
    println!("{seen:?}");
    for key in [
        "ok",
        "ok with holds",
        "not in voicing",
        "no alt",
        "no treble",
        "empty",
        "hold of short",
        "span without attack",
    ] {
        assert!(
            seen.get(key).copied().unwrap_or(0) > 0,
            "{key} never seen: {seen:?}"
        );
    }
    assert!(seen["ok"] >= 60, "at least 10% bind: {seen:?}");
}

/// Clears the short techniques (dead, palm mute, string-percussive) from every strike
/// and span, so every hold can bind.
fn without_short(p: &mut PatternParts) {
    let clear = |lane: &mut Lane| {
        for st in &mut lane.steps {
            if let Action::Strike(k) = &mut st.action {
                k.techniques.dead = false;
                k.techniques.palm_mute = false;
                k.techniques.string_percussive = false;
            }
        }
    };
    match &mut p.lanes {
        Lanes::Strum(l) => clear(l),
        Lanes::ThumbFingers { thumb, fingers } => {
            clear(thumb);
            clear(fingers);
        }
    }
    p.spans
        .retain(|s| matches!(s.technique, Technique::Accent | Technique::LetRing));
    // Keep only spans that reach an attack (a span over holds and rests is rejected
    // by binding, tested above).
    let lanes = p.lanes.clone();
    p.spans.retain(|s| {
        let lane = lanes.get(s.lane).unwrap();
        (s.first..=s.last).any(|k| matches!(lane.steps[k].action, Action::Strike(_)))
    });
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 300, .. ProptestConfig::default() })]

    /// PE-6 for holds: with every string sounded and no short technique, every pattern
    /// binds, and each hold is a tied continuation of exactly the held strings of the
    /// beat before it, on the same frets, keeping only let ring.
    #[test]
    fn pe6_holds_bind_as_tied_continuations(
        seed in pattern_seeds(),
        frets in proptest::collection::vec(0u8..13, 6),
    ) {
        let mut p = build_pattern(&seed, &[]);
        without_short(&mut p);
        let mut lib = PatternLibrary::new();
        let id = lib.define_pattern(p.clone()).unwrap();
        let v = Voicing::new(frets.into_iter().map(|f| Some(mc_theory::fretted::Fret(f))).collect()).unwrap();
        let f = lib.bind(id, &v, &chord("C"), &standard(0)).expect("all strings sounded");
        for ((_, lane), bl) in p.lanes.iter().zip(&f.lanes) {
            for (k, beat) in bl.beats.iter().enumerate() {
                if let Action::Hold(sel) = &lane.steps[beat.step].action {
                    let before = &bl.beats[k - 1];
                    prop_assert_eq!(before.step, beat.step - 1, "directly after its source");
                    let held: Vec<_> = before.notes.iter().map(|n| n.position).collect();
                    let now: Vec<_> = beat.notes.iter().map(|n| n.position).collect();
                    if sel.is_none() {
                        prop_assert_eq!(&now, &held, "hold all keeps every string");
                    }
                    prop_assert!(!now.is_empty());
                    for n in &beat.notes {
                        prop_assert!(n.tie && held.contains(&n.position));
                        let src = before.notes.iter().find(|m| m.position == n.position).unwrap();
                        prop_assert_eq!(n.effects, NoteEffects { let_ring: src.effects.let_ring, ..NoteEffects::default() });
                    }
                }
            }
        }
    }
}
