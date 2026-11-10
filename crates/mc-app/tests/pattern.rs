//! ADR-034 PE-5, PE-7 and PE-8 (session tests): `ApplyPattern` replaces only voices 1
//! and 2 of the track in the range, behind the overwrite safeguard, as one revision
//! and one undo step; fit, pickup and meter rules hold through the session; a thumb
//! note sounds its own duration under the fingers.

mod common;
use common::*;
use mc_app::pattern::{ApplyError, Fit, PatternId, PlaceError};
use mc_app::types::{MasterBar, Score, TimeSignature, TrackId};
use mc_app::{ApplyPattern, Command, CommandError, PatternCommand, ScoreCommand, TrackCommand};
use mc_pattern::{Action, Lane, Lanes, PatternMeta, PatternParts, Step, Strike, StringSelector};
use mc_theory::{BaseValue, NoteValue, Rational};
use proptest::prelude::*;

const C: [Option<u8>; 6] = [Some(0), Some(1), Some(0), Some(2), Some(3), None]; // x32010

fn args(track: TrackId, first: usize, last: usize, pattern: PatternId) -> ApplyPattern {
    ApplyPattern {
        track,
        first,
        last,
        pattern,
        voicing: C.to_vec(),
        chord: "C".into(),
        fit: None,
        overwrite: false,
    }
}

fn apply(a: ApplyPattern) -> Command {
    Command::Pattern(PatternCommand::ApplyPattern(a))
}

fn json(s: &Score) -> Vec<u8> {
    mc_project::score_json(s)
}

fn q() -> NoteValue {
    NoteValue::plain(BaseValue::QUARTER)
}
fn e() -> NoteValue {
    NoteValue::plain(BaseValue::EIGHTH)
}
fn at(n: i64, d: i64) -> Rational {
    Rational::new(n, d).unwrap()
}
fn strike(t: StringSelector) -> Action {
    Action::Strike(Strike::on(vec![t]))
}

/// Corpus pattern 2 (Travis): thumb Bass/Alt quarters, fingers on 1 and 2 with gaps.
fn travis() -> PatternParts {
    let s = |n| StringSelector::Absolute(mc_theory::fretted::StringIndex::new(n).unwrap());
    let thumb = (0..4)
        .map(|k| Step {
            onset: at(k, 4),
            duration: q(),
            action: strike(if k % 2 == 0 {
                StringSelector::Bass
            } else {
                StringSelector::Alt
            }),
        })
        .collect();
    let fingers = [(0, 1), (1, 2), (3, 1), (5, 2), (7, 1)]
        .iter()
        .map(|&(k, n)| Step {
            onset: at(k, 8),
            duration: e(),
            action: strike(s(n)),
        })
        .collect();
    PatternParts {
        name: "Travis".into(),
        length: Rational::ONE,
        meter: None,
        lanes: Lanes::ThumbFingers {
            thumb: Lane { steps: thumb },
            fingers: Lane { steps: fingers },
        },
        spans: Vec::new(),
        meta: PatternMeta::default(),
    }
}

/// A quarter beat with one note in voice `voice` of `bar`.
fn put(s: &mut mc_app::DocumentSession, t: TrackId, bar: usize, voice: usize, string: u8) {
    let index = s.score().track(t).unwrap().bar(bar).unwrap().voices[voice]
        .beats
        .len();
    s.execute(cmd(
        1,
        Command::Score(ScoreCommand::InsertBeat {
            track: t,
            bar,
            voice,
            index,
            duration: mc_app::DurationSpec::plain(2),
        }),
    ))
    .unwrap();
    let beat = s.score().track(t).unwrap().bar(bar).unwrap().voices[voice].beats[index].id;
    s.execute(cmd(
        2,
        Command::Score(ScoreCommand::SetNote {
            beat: mc_app::BeatAddress {
                track: t,
                bar,
                voice,
                beat,
            },
            string,
            fret: 5,
        }),
    ))
    .unwrap();
}

fn add_voice(s: &mut mc_app::DocumentSession, t: TrackId, bar: usize) {
    s.execute(cmd(
        3,
        Command::Score(ScoreCommand::AddVoice { track: t, bar }),
    ))
    .unwrap();
}

// ------------------------------------------------------------------- PE-7

#[test]
fn pe7_the_sample_pattern_fills_an_empty_range_in_one_step() {
    let mut s = session(6);
    let t = first_track(&s);
    let p = s.sample_pattern();
    let (rev, depth) = (s.revision(), s.undo_depth());
    let out = s.execute(cmd(10, apply(args(t, 1, 4, p)))).unwrap();
    assert_eq!(s.revision().0, rev.0 + 1);
    assert_eq!(s.undo_depth(), depth + 1);
    let report = out.pattern.expect("a report");
    assert_eq!((report.first, report.last, report.voices), (1, 4, 1));
    assert!(report.overwritten.is_empty());
    assert_eq!(report.place.repetitions, 4);
    // Bass (1) + Treble (3) + Alt (1) + Treble (3) per bar on C x32010.
    assert_eq!(report.place.notes, 32);
    let tr = s.score().track(t).unwrap();
    for bar in 1..=4 {
        let v = &tr.bar(bar).unwrap().voices[0];
        let strings: Vec<Vec<u8>> = v
            .beats
            .iter()
            .map(|b| b.notes.iter().map(|n| n.position().string.get()).collect())
            .collect();
        assert_eq!(
            strings,
            vec![vec![5], vec![1, 2, 3], vec![4], vec![1, 2, 3]],
            "bar {bar}"
        );
        assert_eq!(v.beats[1].stroke, Some(mc_app::types::Stroke::Down));
    }
    assert!(tr.bar(0).unwrap().voices[0].beats.is_empty());
    assert!(tr.bar(5).unwrap().voices[0].beats.is_empty());
    assert!(s.score().validate().is_empty());
}

#[test]
fn pe7_only_voices_1_and_2_of_the_track_in_the_range_change() {
    let mut s = session(5);
    let t = first_track(&s);
    s.execute(cmd(
        1,
        Command::Track(TrackCommand::Add {
            name: "Other".into(),
            instrument: mc_app::InstrumentSpecInput::standard_guitar(),
        }),
    ))
    .unwrap();
    let other = s.score().track_at(1).unwrap().id();
    put(&mut s, other, 2, 0, 3);
    s.execute(cmd(
        2,
        Command::Score(ScoreCommand::SetTempo {
            bar: 3,
            offset: (0, 1),
            bpm: (90, 1),
        }),
    ))
    .unwrap();
    // Target track: content in voice 1 of bar 0 and bar 4 (outside the range), and in
    // voices 1, 2 and 3 of bar 2 (inside).
    put(&mut s, t, 0, 0, 1);
    put(&mut s, t, 4, 0, 1);
    add_voice(&mut s, t, 2);
    add_voice(&mut s, t, 2);
    put(&mut s, t, 2, 0, 1);
    put(&mut s, t, 2, 1, 2);
    put(&mut s, t, 2, 2, 6);
    let p = s.patterns_mut().define_pattern(travis()).unwrap();
    let before = s.snapshot();
    let mut a = args(t, 1, 3, p);
    a.overwrite = true;
    let out = s.execute(cmd(20, apply(a))).unwrap();
    assert_eq!(out.pattern.unwrap().overwritten, vec![2]);
    let after = s.snapshot();
    // Model content is compared with derived equality (ids included); score_json is a
    // pure function of it, so equal content means identical bytes.
    assert_eq!(
        after.track(other),
        before.track(other),
        "other tracks unchanged"
    );
    assert_eq!(
        after.master_bars(),
        before.master_bars(),
        "master bars unchanged"
    );
    assert_eq!(after.tempo_map(), before.tempo_map(), "tempo unchanged");
    let (tb, ta) = (before.track(t).unwrap(), after.track(t).unwrap());
    for bar in [0, 4] {
        assert_eq!(
            ta.bar(bar),
            tb.bar(bar),
            "bar {bar} outside the range unchanged"
        );
    }
    assert_eq!(
        ta.bar(2).unwrap().voices[2],
        tb.bar(2).unwrap().voices[2],
        "voice 3 unchanged"
    );
    for bar in 1..=3 {
        assert_eq!(
            ta.bar(bar).unwrap().voices.len().min(2),
            2,
            "voice 2 written in bar {bar}"
        );
        assert_ne!(ta.bar(bar).unwrap().voices[1].beats.len(), 0);
    }
    assert_eq!((ta.name(), ta.fretted()), (tb.name(), tb.fretted()));
}

#[test]
fn pe7_existing_content_is_not_overwritten_without_confirmation() {
    let mut s = session(5);
    let t = first_track(&s);
    put(&mut s, t, 1, 0, 1);
    put(&mut s, t, 3, 0, 2);
    let p = s.sample_pattern();
    let before = json(&s.snapshot());
    let (rev, depth) = (s.revision(), s.undo_depth());
    let err = s.execute(cmd(30, apply(args(t, 0, 4, p)))).unwrap_err();
    assert_eq!(
        err,
        CommandError::Pattern(ApplyError::WouldOverwrite { bars: vec![1, 3] })
    );
    assert_eq!(err.rule(), "PE-07");
    assert_eq!(json(&s.snapshot()), before, "nothing changed");
    assert_eq!((s.revision(), s.undo_depth()), (rev, depth));
    // The preview names the same bars; with confirmation it applies.
    let preview = s.preview_pattern(&args(t, 0, 4, p)).unwrap();
    assert_eq!(preview.report.overwritten, vec![1, 3]);
    let mut a = args(t, 0, 4, p);
    a.overwrite = true;
    s.execute(cmd(31, apply(a))).unwrap();
}

#[test]
fn pe7_a_strum_pattern_over_voice_2_only_content_needs_no_confirmation() {
    let mut s = session(3);
    let t = first_track(&s);
    for bar in 0..3 {
        add_voice(&mut s, t, bar);
        put(&mut s, t, bar, 1, 6);
    }
    let voice2: Vec<_> = (0..3)
        .map(|b| s.score().track(t).unwrap().bar(b).unwrap().voices[1].clone())
        .collect();
    let p = s.sample_pattern();
    let out = s.execute(cmd(40, apply(args(t, 0, 2, p)))).unwrap();
    assert!(out.pattern.unwrap().overwritten.is_empty());
    for (b, v) in voice2.iter().enumerate() {
        assert_eq!(
            &s.score().track(t).unwrap().bar(b).unwrap().voices[1],
            v,
            "voice 2 of bar {b} unchanged"
        );
    }
}

#[test]
fn pe7_undo_restores_the_score_byte_for_byte_and_redo_reapplies() {
    let mut s = session(4);
    let t = first_track(&s);
    put(&mut s, t, 1, 0, 3);
    let before = json(&s.snapshot());
    let mut a = args(t, 0, 3, s.sample_pattern());
    a.overwrite = true;
    s.execute(cmd(50, apply(a))).unwrap();
    let applied = json(&s.snapshot());
    assert_ne!(applied, before);
    s.undo().unwrap();
    assert_eq!(json(&s.snapshot()), before);
    s.redo().unwrap();
    assert_eq!(json(&s.snapshot()), applied);
}

#[test]
fn pe7_every_refusal_is_typed_and_changes_nothing() {
    let mut s = session(4);
    let t = first_track(&s);
    let sample = s.sample_pattern();
    let mut pickup = s.score().master_bars()[0].clone();
    pickup.pickup = true;
    s.execute(cmd(
        1,
        Command::Score(ScoreCommand::SetMasterBar {
            index: 0,
            bar: pickup,
        }),
    ))
    .unwrap();
    let three = MasterBar {
        time_signature: TimeSignature::new(3, 4).unwrap(),
        ..MasterBar::default()
    };
    s.execute(cmd(
        2,
        Command::Score(ScoreCommand::SetMasterBar {
            index: 3,
            bar: three,
        }),
    ))
    .unwrap();
    let mut metered = mc_app::pattern::sample_parts();
    metered.meter = Some(TimeSignature::COMMON);
    let metered = s.patterns_mut().define_pattern(metered).unwrap();
    let before = json(&s.snapshot());
    let rev = s.revision();
    let cases: Vec<(ApplyPattern, &str)> = vec![
        (args(t, 1, 2, PatternId::from_raw(999)), "unknown pattern"),
        (
            ApplyPattern {
                voicing: vec![None, None, Some(0), Some(2), Some(3), Some(2)]
                    .into_iter()
                    .rev()
                    .collect(),
                chord: "D".into(),
                ..args(t, 1, 2, sample)
            },
            "no alternate bass",
        ),
        (
            ApplyPattern {
                voicing: vec![Some(0); 5],
                ..args(t, 1, 2, sample)
            },
            "voicing string count",
        ),
        (
            ApplyPattern {
                chord: "not a chord".into(),
                ..args(t, 1, 2, sample)
            },
            "chord symbol",
        ),
        (args(t, 0, 2, sample), "pickup in range"),
        (args(t, 1, 3, sample), "fit required (11/4 range)"),
        (args(t, 2, 3, metered), "meter mismatch"),
        (args(t, 2, 1, sample), "empty range"),
        (args(t, 1, 9, sample), "range past the end"),
        (args(TrackId::from_raw(999), 1, 2, sample), "unknown track"),
    ];
    for (a, name) in cases {
        let err = s.execute(cmd(60, apply(a.clone()))).expect_err(name);
        match name {
            "unknown pattern" => assert_eq!(
                err,
                CommandError::Pattern(ApplyError::UnknownPattern(PatternId::from_raw(999)))
            ),
            "no alternate bass" => assert!(
                matches!(err, CommandError::Pattern(ApplyError::Bind(_))),
                "{err:?}"
            ),
            "voicing string count" => assert!(
                matches!(err, CommandError::Pattern(ApplyError::Bind(_))),
                "{err:?}"
            ),
            "chord symbol" => assert!(matches!(err, CommandError::Value(_)), "{err:?}"),
            "pickup in range" => assert_eq!(
                err,
                CommandError::Pattern(ApplyError::Place(PlaceError::PickupInRange { bar: 0 }))
            ),
            "fit required (11/4 range)" => assert_eq!(
                err,
                CommandError::Pattern(ApplyError::Place(PlaceError::FitRequired {
                    range: at(11, 4),
                    pattern: Rational::ONE
                }))
            ),
            "meter mismatch" => assert!(
                matches!(
                    err,
                    CommandError::Pattern(ApplyError::Place(PlaceError::MeterMismatch {
                        bar: 1,
                        ..
                    }))
                ),
                "{err:?}"
            ),
            _ => assert!(matches!(err, CommandError::Domain(_)), "{name}: {err:?}"),
        }
        assert_eq!(json(&s.snapshot()), before, "{name}: nothing changed");
        assert_eq!(s.revision(), rev, "{name}");
        assert!(
            s.preview_pattern(&a).is_err(),
            "{name}: the preview refuses too"
        );
    }
}

#[test]
fn the_preview_equals_what_is_applied_and_changes_nothing() {
    let mut s = session(3);
    let t = first_track(&s);
    let p = s.patterns_mut().define_pattern(travis()).unwrap();
    let rev = s.revision();
    let preview = s.preview_pattern(&args(t, 0, 2, p)).unwrap();
    assert_eq!(s.revision(), rev);
    let out = s.execute(cmd(70, apply(args(t, 0, 2, p)))).unwrap();
    assert_eq!(out.pattern.unwrap(), preview.report);
    for (k, pb) in preview.bars.iter().enumerate() {
        for (v, (_, beats)) in pb.voices.iter().enumerate() {
            let written = s.copy_beats(t, v, (k, 0), (k, beats.len() - 1)).unwrap();
            let shape: Vec<_> = written
                .iter()
                .map(|b| {
                    (
                        b.duration,
                        b.stroke,
                        b.notes
                            .iter()
                            .map(|n| (n.string, n.fret, n.tie, n.effects))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect();
            let want: Vec<_> = beats
                .iter()
                .map(|b| {
                    (
                        mc_app::DurationSpec::from_value(b.value),
                        b.stroke,
                        b.notes
                            .iter()
                            .map(|n| (n.position.string.get(), n.position.fret.0, n.tie, n.effects))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect();
            assert_eq!(shape, want, "bar {k} voice {v}");
        }
    }
}

// ------------------------------------------------------------------- PE-8

#[test]
fn pe8_fit_through_the_session() {
    let mut s = session(4);
    let t = first_track(&s);
    // A 3/4 pattern (three quarters) over two 4/4 bars: 8/3 repetitions.
    let mut three = mc_app::pattern::sample_parts();
    three.length = at(3, 4);
    let Lanes::Strum(lane) = &mut three.lanes else {
        unreachable!()
    };
    lane.steps.truncate(3);
    let p = s.patterns_mut().define_pattern(three).unwrap();
    assert!(matches!(
        s.execute(cmd(80, apply(args(t, 0, 1, p)))).unwrap_err(),
        CommandError::Pattern(ApplyError::Place(PlaceError::FitRequired { .. }))
    ));
    let cut = s
        .execute(cmd(
            81,
            apply(ApplyPattern {
                fit: Some(Fit::RepeatAndCut),
                ..args(t, 0, 1, p)
            }),
        ))
        .unwrap();
    let r = cut.pattern.unwrap().place;
    assert_eq!(r.repetitions, 3);
    assert_eq!(
        r.dropped
            .iter()
            .map(|d| (d.repetition, d.step))
            .collect::<Vec<_>>(),
        vec![(2, 2)]
    );
    assert!(r.shortened.is_empty());
    s.undo().unwrap();
    let whole = s
        .execute(cmd(
            82,
            apply(ApplyPattern {
                fit: Some(Fit::WholeRepetitionsOnly),
                ..args(t, 0, 1, p)
            }),
        ))
        .unwrap();
    assert_eq!(whole.pattern.unwrap().place.rests_from, Some(at(3, 2)));
    // A whole multiple needs no fit: three 4/4 bars hold exactly four repetitions of
    // the 3/4 pattern.
    s.undo().unwrap();
    let ok = s.execute(cmd(83, apply(args(t, 0, 2, p)))).unwrap();
    assert_eq!(ok.pattern.unwrap().place.repetitions, 4);
}

#[test]
fn pe8_a_pattern_without_meter_runs_on_across_mixed_time_signatures_in_the_score() {
    let mut s = session(2);
    let t = first_track(&s);
    let three = MasterBar {
        time_signature: TimeSignature::new(3, 4).unwrap(),
        ..MasterBar::default()
    };
    s.execute(cmd(
        1,
        Command::Score(ScoreCommand::SetMasterBar {
            index: 1,
            bar: three,
        }),
    ))
    .unwrap();
    // Seven quarters of Bass.
    let steps = (0..7)
        .map(|k| Step {
            onset: at(k, 4),
            duration: q(),
            action: strike(StringSelector::Bass),
        })
        .collect();
    let p = s
        .patterns_mut()
        .define_pattern(PatternParts {
            name: "Seven".into(),
            length: at(7, 4),
            meter: None,
            lanes: Lanes::Strum(Lane { steps }),
            spans: vec![],
            meta: PatternMeta::default(),
        })
        .unwrap();
    s.execute(cmd(84, apply(args(t, 0, 1, p)))).unwrap();
    let tr = s.score().track(t).unwrap();
    assert_eq!(tr.bar(0).unwrap().voices[0].beats.len(), 4);
    assert_eq!(tr.bar(1).unwrap().voices[0].beats.len(), 3);
    assert!(
        s.score().diagnostics(0..2).unwrap().is_empty(),
        "both bars exactly full"
    );
}

// ------------------------------------------------------------------- PE-5

#[test]
fn pe5_a_thumb_note_sounds_its_own_duration_under_the_finger_attacks() {
    use mc_playback::{CompileOptions, PerformanceEvent, compile};
    let mut s = session(1);
    let t = first_track(&s);
    let p = s.patterns_mut().define_pattern(travis()).unwrap();
    s.execute(cmd(90, apply(args(t, 0, 0, p)))).unwrap();
    let tl = compile(s.score(), &CompileOptions::new()).unwrap().timeline;
    let tpw = tl.ticks_per_whole().get() as i64;
    // Bass on C x32010 is string 5 fret 3: C3 = MIDI 48.
    let mut on = None;
    let mut off = None;
    let mut finger_attacks = Vec::new();
    for te in tl.events() {
        match te.event {
            PerformanceEvent::NoteOn { key, .. } if key.get() == 48 && on.is_none() => {
                on = Some(te.tick.0)
            }
            PerformanceEvent::NoteOff { key, .. }
                if key.get() == 48 && off.is_none() && on.is_some() =>
            {
                off = Some(te.tick.0)
            }
            PerformanceEvent::NoteOn { key, .. } if key.get() != 48 && te.tick.0 < tpw / 4 => {
                finger_attacks.push(te.tick.0)
            }
            _ => {}
        }
    }
    assert_eq!(on, Some(0));
    assert_eq!(
        off,
        Some(tpw / 4),
        "the thumb's quarter ends at its own duration"
    );
    assert!(
        finger_attacks.contains(&(tpw / 8)),
        "a finger attacks at 1/8 while the bass sounds: {finger_attacks:?}"
    );
}

// --------------------------------------------------------- PE-7 property

/// Random structure first, then a quarter beat in voice 1 of the first track in each
/// bar whose `fill` flag is set, so the overwrite safeguard has real content to find
/// (random edits alone rarely leave a beat there).
fn populated(ops: &[Op], fill: &[bool]) -> mc_app::DocumentSession {
    let mut s = session(4);
    for (i, o) in ops.iter().enumerate() {
        let c = to_command(&s, o);
        let _ = s.execute(cmd(i as u64, c));
    }
    let t = first_track(&s);
    for (bar, f) in fill.iter().enumerate().take(s.score().master_bar_count()) {
        if *f {
            add_beat(&mut s, t, bar);
        }
    }
    s
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    /// PE-7 over random scores: with `overwrite: false` the command either applies
    /// (when the written voice is empty in every bar of the range) or returns exactly
    /// the bars whose voice 1 holds beats and changes nothing; with `overwrite: true`
    /// only voice 1 of the range changes, and undo restores the bytes.
    #[test]
    fn pe7_random_scores(ops in proptest::collection::vec(op(), 0..10), fill in proptest::collection::vec(any::<bool>(), 8), first in 0usize..6, len in 1usize..4) {
        let mut s = populated(&ops, &fill);
        let n = s.score().master_bar_count();
        let first = first % n;
        let last = (first + len - 1).min(n - 1);
        let t = first_track(&s);
        let before = s.snapshot();
        let bytes = json(&before);
        let p = s.sample_pattern();
        let occupied: Vec<usize> = (first..=last).filter(|&b| !before.track(t).unwrap().bar(b).unwrap().voices[0].beats.is_empty()).collect();
        let pickup = before.master_bars()[first..=last].iter().any(|b| b.pickup);
        let rev = s.revision();
        match s.execute(cmd(500, apply(args(t, first, last, p)))) {
            Err(CommandError::Pattern(ApplyError::WouldOverwrite { bars })) => {
                prop_assert_eq!(&bars, &occupied);
                prop_assert_eq!(json(&s.snapshot()), bytes.clone());
                prop_assert_eq!(s.revision(), rev);
            }
            Err(_) => {
                prop_assert_eq!(json(&s.snapshot()), bytes.clone());
                prop_assert_eq!(s.revision(), rev);
            }
            Ok(_) => {
                prop_assert!(occupied.is_empty());
                prop_assert!(!pickup);
                s.undo().unwrap();
            }
        }
        prop_assert_eq!(json(&s.snapshot()), bytes.clone());
        let mut a = args(t, first, last, p);
        a.overwrite = true;
        a.fit = Some(Fit::RepeatAndCut);
        if s.execute(cmd(501, apply(a))).is_ok() {
            let after = s.snapshot();
            prop_assert_eq!(after.master_bars(), before.master_bars());
            prop_assert_eq!(after.tempo_map(), before.tempo_map());
            for tr in before.tracks().filter(|x| x.id() != t) {
                prop_assert_eq!(after.track(tr.id()), Some(tr));
            }
            let (tb, ta) = (before.track(t).unwrap(), after.track(t).unwrap());
            for b in 0..n {
                let (vb, va) = (&tb.bar(b).unwrap().voices, &ta.bar(b).unwrap().voices);
                if b < first || b > last {
                    // A tie into the range end may be cleared in the bar after it (H-06,
                    // reported); everything else is identical.
                    if b == last + 1 {
                        prop_assert_eq!(vb.len(), va.len());
                    } else {
                        prop_assert_eq!(vb, va, "bar {} outside the range", b);
                    }
                } else {
                    prop_assert_eq!(&vb[1..], &va[1..], "voices 2 and up of bar {}", b);
                }
            }
            prop_assert!(after.validate().is_empty());
            s.undo().unwrap();
            prop_assert_eq!(json(&s.snapshot()), bytes);
        }
    }
}

/// The PE-7 property reaches every branch: applied without confirmation, refused with
/// `WouldOverwrite`, other refusals, and applied with confirmation.
#[test]
fn the_pe7_property_covers_every_branch() {
    use proptest::strategy::{Strategy, ValueTree};
    let mut runner = proptest::test_runner::TestRunner::deterministic();
    let mut seen = std::collections::BTreeMap::<&str, usize>::new();
    let strategy = (
        proptest::collection::vec(op(), 0..10),
        proptest::collection::vec(any::<bool>(), 8),
        0usize..6,
        1usize..4,
    );
    for _ in 0..200 {
        let (ops, fill, first, len) = strategy.new_tree(&mut runner).unwrap().current();
        let mut s = populated(&ops, &fill);
        let n = s.score().master_bar_count();
        let first = first % n;
        let last = (first + len - 1).min(n - 1);
        let t = first_track(&s);
        let p = s.sample_pattern();
        let key = match s.execute(cmd(500, apply(args(t, first, last, p)))) {
            Ok(_) => "applied",
            Err(CommandError::Pattern(ApplyError::WouldOverwrite { .. })) => "would overwrite",
            Err(_) => "other refusal",
        };
        *seen.entry(key).or_default() += 1;
        if key == "would overwrite" {
            let mut a = args(t, first, last, p);
            a.overwrite = true;
            a.fit = Some(Fit::RepeatAndCut);
            if s.execute(cmd(501, apply(a))).is_ok() {
                *seen.entry("applied after confirmation").or_default() += 1;
            }
        }
    }
    println!("{seen:?}");
    for key in [
        "applied",
        "would overwrite",
        "other refusal",
        "applied after confirmation",
    ] {
        assert!(
            seen.get(key).copied().unwrap_or(0) > 0,
            "{key} never seen: {seen:?}"
        );
    }
}

#[test]
fn pe7_a_thumb_and_fingers_pattern_also_checks_voice_2() {
    // Amendment A: the check covers the voices the pattern writes. Content only in
    // voice 2 of bar 1 blocks a Thumb and Fingers pattern, not a Strum pattern.
    let mut s = session(3);
    let t = first_track(&s);
    add_voice(&mut s, t, 1);
    put(&mut s, t, 1, 1, 6);
    let travis = s.patterns_mut().define_pattern(travis()).unwrap();
    let before = json(&s.snapshot());
    assert_eq!(
        s.execute(cmd(95, apply(args(t, 0, 2, travis))))
            .unwrap_err(),
        CommandError::Pattern(ApplyError::WouldOverwrite { bars: vec![1] })
    );
    assert_eq!(json(&s.snapshot()), before);
    let strum = s.sample_pattern();
    assert!(s.execute(cmd(96, apply(args(t, 0, 2, strum)))).is_ok());
}

#[test]
fn refusals_have_rule_ids_and_readable_messages() {
    let mut s = session(4);
    let t = first_track(&s);
    put(&mut s, t, 1, 0, 1);
    put(&mut s, t, 3, 0, 2);
    let p = s.sample_pattern();
    let e = s.execute(cmd(1, apply(args(t, 0, 3, p)))).unwrap_err();
    assert_eq!(e.rule(), "PE-07");
    assert_eq!(
        e.to_string(),
        "PE-07: bars 2, 4 already have content in the voices the pattern writes"
    );
    let mut three = mc_app::pattern::sample_parts();
    three.length = at(3, 4);
    let Lanes::Strum(lane) = &mut three.lanes else {
        unreachable!()
    };
    lane.steps.truncate(3);
    let p3 = s.patterns_mut().define_pattern(three).unwrap();
    let e = s
        .execute(cmd(
            2,
            apply(ApplyPattern {
                overwrite: true,
                ..args(t, 0, 0, p3)
            }),
        ))
        .unwrap_err();
    assert_eq!(e.rule(), "PE-08");
    assert!(e.to_string().contains("choose how to fit it"), "{e}");
    let e = s
        .execute(cmd(
            3,
            apply(ApplyPattern {
                voicing: vec![None, None, Some(2), Some(3), Some(2), Some(0)],
                chord: "D".into(),
                overwrite: true,
                ..args(t, 0, 0, p)
            }),
        ))
        .unwrap_err();
    assert_eq!(e.rule(), "PT-02");
    let e = s
        .execute(cmd(4, apply(args(t, 0, 0, PatternId::from_raw(77)))))
        .unwrap_err();
    assert_eq!(e.rule(), "PT-06");
}

#[test]
fn a_confirmed_overwrite_is_refused_when_the_score_changed_after_the_preview() {
    // The dangerous sequence: the preview finds content to replace, the user is asked,
    // another command changes the same range, then the user confirms. The confirmation
    // belonged to the old content, so the core must refuse (E-01) and change nothing.
    let mut s = session(3);
    let t = first_track(&s);
    put(&mut s, t, 1, 0, 1);
    let p = s.sample_pattern();
    let preview = s.preview_pattern(&args(t, 0, 2, p)).unwrap();
    assert_eq!(preview.report.overwritten, vec![1]);
    assert_eq!(
        preview.revision,
        s.revision(),
        "the preview carries its revision"
    );
    // Another command changes the very bar that was to be overwritten.
    put(&mut s, t, 1, 0, 2);
    let changed = json(&s.snapshot());
    let rev = s.revision();
    let mut confirmed = cmd(
        80,
        apply(ApplyPattern {
            overwrite: true,
            ..args(t, 0, 2, p)
        }),
    );
    confirmed.expected_revision = Some(preview.revision);
    let err = s.execute(confirmed).unwrap_err();
    assert!(matches!(err, CommandError::StaleRevision { .. }), "{err:?}");
    assert_eq!(err.rule(), "E-01");
    assert_eq!(
        json(&s.snapshot()),
        changed,
        "the old confirmation overwrote nothing"
    );
    assert_eq!(s.revision(), rev);
    // A fresh preview and confirmation applies.
    let again = s.preview_pattern(&args(t, 0, 2, p)).unwrap();
    let mut ok = cmd(
        81,
        apply(ApplyPattern {
            overwrite: true,
            ..args(t, 0, 2, p)
        }),
    );
    ok.expected_revision = Some(again.revision);
    s.execute(ok).unwrap();
}

#[test]
fn a_voice_with_only_rests_counts_as_existing_content() {
    // Decision 18.1: any beat in a written voice, rests included, must be confirmed.
    let mut s = session(2);
    let t = first_track(&s);
    add_beat(&mut s, t, 1); // a rest: a beat without notes
    assert!(
        s.score().track(t).unwrap().bar(1).unwrap().voices[0].beats[0]
            .notes
            .is_empty()
    );
    let p = s.sample_pattern();
    let err = s.execute(cmd(90, apply(args(t, 0, 1, p)))).unwrap_err();
    assert_eq!(
        err,
        CommandError::Pattern(ApplyError::WouldOverwrite { bars: vec![1] })
    );
    assert!(err.to_string().contains("content"), "{err}");
}
