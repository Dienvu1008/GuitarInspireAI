//! ADR-030 acceptance criteria C1 to C5: paste and cut of a beat run are atomic, one
//! undo step each, keep the hard invariants, and copy then paste reproduces the beats.

mod common;
use common::*;
use mc_app::types::DomainEvent;
use mc_app::{
    BeatAddress, ClipBeat, ClipNote, Command, CommandEnvelope, CommandError, DurationSpec,
    EventPayload, InstrumentSpecInput, Revision, ScoreCommand, TrackCommand,
};
use proptest::prelude::*;

fn set_note(beat: BeatAddress, string: u8, fret: u8) -> Command {
    Command::Score(ScoreCommand::SetNote { beat, string, fret })
}

fn paste(track: mc_app::types::TrackId, bar: usize, index: usize, beats: Vec<ClipBeat>) -> Command {
    Command::Score(ScoreCommand::PasteBeats {
        track,
        bar,
        voice: 0,
        index,
        beats,
    })
}

fn clip(string: u8, fret: u8) -> ClipBeat {
    ClipBeat {
        duration: DurationSpec::plain(2),
        notes: vec![ClipNote {
            string,
            fret,
            tie: false,
            effects: Default::default(),
        }],
    }
}

fn four_string_bass(frets: u8) -> Command {
    Command::Track(TrackCommand::Add {
        name: "Bass".into(),
        instrument: InstrumentSpecInput {
            tuning: ["G2", "D2", "A1", "E1"].map(String::from).to_vec(),
            fret_count: frets,
            capo: 0,
        },
    })
}

fn rule_of(e: &CommandError) -> &'static str {
    match e {
        CommandError::Domain(d) => d.rule(),
        _ => "other",
    }
}

/// Two quarter beats in bar 0 of a fresh session: string 2 fret 3, then the same note
/// tied to it. Returns the session and its track.
fn tied_pair() -> (mc_app::DocumentSession, mc_app::types::TrackId) {
    let mut s = session(3);
    let t = first_track(&s);
    let b0 = add_beat(&mut s, t, 0);
    s.execute(cmd(1, set_note(b0, 2, 3))).unwrap();
    let b1 = add_beat(&mut s, t, 0);
    s.execute(cmd(2, set_note(b1, 2, 3))).unwrap();
    let note = s.score().track(t).unwrap().bar(0).unwrap().voices[0].beats[1].notes[0].id;
    s.execute(cmd(
        3,
        Command::Score(ScoreCommand::SetTie {
            beat: b1,
            note,
            tie: true,
        }),
    ))
    .unwrap();
    (s, t)
}

#[test]
fn c3_a_tie_is_kept_when_its_partner_is_copied() {
    let (mut s, t) = tied_pair();
    let frag = s.copy_beats(t, 0, (0, 0), (0, 1)).unwrap();
    assert_eq!(frag.len(), 2);
    assert!(frag[1].notes[0].tie);
    let out = s.execute(cmd(10, paste(t, 1, 0, frag))).unwrap();
    assert!(!out.events.iter().any(|e| matches!(
        e.payload,
        EventPayload::Domain(DomainEvent::TiesRemoved { .. })
    )));
    let back = s.copy_beats(t, 0, (1, 0), (1, 1)).unwrap();
    assert!(!back[0].notes[0].tie);
    assert!(back[1].notes[0].tie, "the tie inside the fragment survives");
}

#[test]
fn c3_a_tie_without_its_partner_is_cleared_and_reported() {
    let (mut s, t) = tied_pair();
    // Copy only the tied beat and paste it at the very start, where nothing precedes it.
    let frag = s.copy_beats(t, 0, (0, 1), (0, 1)).unwrap();
    assert!(frag[0].notes[0].tie);
    let out = s.execute(cmd(10, paste(t, 0, 0, frag))).unwrap();
    let removed = out.events.iter().any(|e| {
        matches!(&e.payload,
            EventPayload::Domain(DomainEvent::TiesRemoved { notes, .. }) if notes.len() == 1)
    });
    assert!(removed, "the cleared tie is reported (H-06)");
    let first = s.copy_beats(t, 0, (0, 0), (0, 0)).unwrap();
    assert!(!first[0].notes[0].tie);
    assert!(s.score().validate().is_empty());
}

#[test]
fn c2_a_failed_paste_changes_nothing() {
    let mut s = session(2);
    let t = first_track(&s);
    s.execute(cmd(1, four_string_bass(12))).unwrap();
    let bass = s.score().track_at(1).unwrap().id();

    let cases: Vec<(&str, Vec<ClipBeat>, usize, &str)> = vec![
        ("string beyond the target", vec![clip(6, 3)], 0, "H-03"),
        ("fret beyond the target", vec![clip(1, 20)], 0, "H-04"),
        (
            "two notes on one string",
            vec![ClipBeat {
                duration: DurationSpec::plain(2),
                notes: vec![clip(1, 1).notes[0], clip(1, 2).notes[0]],
            }],
            0,
            "H-05",
        ),
        ("nothing to paste", vec![], 0, "E-04"),
        ("index beyond the voice", vec![clip(1, 1)], 5, "E-04"),
    ];
    for (name, beats, index, expected) in cases {
        let before = s.snapshot();
        let (rev, depth) = (s.revision(), s.undo_depth());
        let err = s
            .execute(cmd(20, paste(bass, 0, index, beats)))
            .expect_err(name);
        assert_eq!(rule_of(&err), expected, "{name}: {err:?}");
        assert_eq!(s.score(), &before, "{name}");
        assert_eq!((s.revision(), s.undo_depth()), (rev, depth), "{name}");
    }
    // The same fragments are fine on the six-string track.
    s.execute(cmd(30, paste(t, 0, 0, vec![clip(6, 3)])))
        .unwrap();
}

#[test]
fn c2_a_stale_revision_is_rejected_without_change() {
    let mut s = session(2);
    let t = first_track(&s);
    let before = s.snapshot();
    let mut env = CommandEnvelope::new(1, paste(t, 0, 0, vec![clip(1, 1)]));
    env.expected_revision = Some(Revision(99));
    let err = s.execute(env).unwrap_err();
    assert!(matches!(err, CommandError::StaleRevision { .. }), "{err:?}");
    assert_eq!(s.score(), &before);
    assert_eq!(s.undo_depth(), 0);
}

#[test]
fn c5_cut_across_bars_is_one_undo_step() {
    let mut s = session(3);
    let t = first_track(&s);
    for bar in 0..3 {
        for k in 0..2u8 {
            let b = add_beat(&mut s, t, bar);
            s.execute(cmd(100, set_note(b, 1, k + 1))).unwrap();
        }
    }
    let before = s.snapshot();
    let depth = s.undo_depth();
    // Last beat of bar 0 through the first beat of bar 2: 1 + 2 + 1 = 4 beats.
    let out = s.execute(cmd(
        200,
        Command::Score(ScoreCommand::RemoveBeats {
            track: t,
            voice: 0,
            from: (0, 1),
            to: (2, 0),
        }),
    ));
    out.unwrap();
    assert_eq!(s.undo_depth(), depth + 1);
    let lens: Vec<usize> = (0..3)
        .map(|b| {
            s.score().track(t).unwrap().bar(b).unwrap().voices[0]
                .beats
                .len()
        })
        .collect();
    assert_eq!(lens, vec![1, 0, 1]);
    assert!(s.score().validate().is_empty());
    s.undo().unwrap();
    assert_eq!(s.score(), &before);
}

#[test]
fn c5_a_cut_with_a_missing_end_changes_nothing() {
    let mut s = session(2);
    let t = first_track(&s);
    let b = add_beat(&mut s, t, 0);
    s.execute(cmd(1, set_note(b, 1, 1))).unwrap();
    let before = s.snapshot();
    let rev = s.revision();
    for (from, to) in [((0, 0), (0, 5)), ((0, 1), (0, 0)), ((0, 0), (9, 0))] {
        let err = s
            .execute(cmd(
                2,
                Command::Score(ScoreCommand::RemoveBeats {
                    track: t,
                    voice: 0,
                    from,
                    to,
                }),
            ))
            .unwrap_err();
        assert_eq!(rule_of(&err), "E-04", "{from:?}..{to:?}");
        assert_eq!(s.score(), &before);
        assert_eq!(s.revision(), rev);
    }
}

/// Pasted beats equal the copied ones. Only the first pasted beat may lose a tie,
/// because nothing is guaranteed to precede it at the target.
fn reproduces(pasted: &[ClipBeat], copied: &[ClipBeat]) -> bool {
    pasted.len() == copied.len()
        && pasted.iter().zip(copied).enumerate().all(|(k, (p, c))| {
            if k > 0 {
                return p == c;
            }
            p.duration == c.duration
                && p.notes.len() == c.notes.len()
                && p.notes.iter().zip(&c.notes).all(|(pn, cn)| {
                    (pn.string, pn.fret, pn.effects) == (cn.string, cn.fret, cn.effects)
                        && (!pn.tie || cn.tie)
                })
        })
}

/// One note to place: string, fret and whether to try to tie it to the previous beat.
type Seed = (u8, u8, bool);

/// Applies random structure first, then appends seeded beats to the first track, so the
/// properties always have real content to copy (random edits alone rarely leave a beat
/// at a chosen position, which made the first version of these properties vacuous).
fn populated(ops: &[Op], seeds: &[Vec<Seed>]) -> (mc_app::DocumentSession, mc_app::types::TrackId) {
    let mut s = session(4);
    for (i, o) in ops.iter().enumerate() {
        let c = to_command(&s, o);
        let _ = s.execute(cmd(i as u64, c));
    }
    let t = first_track(&s);
    let bars = s.score().master_bar_count();
    for (bar, notes) in seeds.iter().enumerate().take(bars) {
        let mut prev: Option<(u8, u8)> = None;
        for &(string, fret, tie) in notes {
            // A tied note repeats the previous beat's string and fret, so the tie is
            // valid (H-06) and fragments really carry ties.
            let (string, fret) = if tie {
                prev.unwrap_or((string, fret))
            } else {
                (string, fret)
            };
            prev = Some((string, fret));
            let b = add_beat(&mut s, t, bar);
            if s.execute(cmd(7, set_note(b, string, fret))).is_err() {
                continue;
            }
            if tie {
                let last = s.score().track(t).unwrap().bar(bar).unwrap().voices[0]
                    .beats
                    .last()
                    .and_then(|x| x.notes.first())
                    .map(|n| n.id);
                if let Some(note) = last {
                    let _ = s.execute(cmd(
                        8,
                        Command::Score(ScoreCommand::SetTie {
                            beat: b,
                            note,
                            tie: true,
                        }),
                    ));
                }
            }
        }
    }
    (s, t)
}

/// Picks an existing inclusive run in voice 0 from the numbers drawn by proptest.
fn pick_run(
    s: &mc_app::DocumentSession,
    t: mc_app::types::TrackId,
    pick: [usize; 4],
) -> Option<((usize, usize), (usize, usize))> {
    let n = s.score().master_bar_count();
    let len = |bar: usize| {
        s.score()
            .track(t)
            .and_then(|tr| tr.bar(bar))
            .map_or(0, |b| b.voices[0].beats.len())
    };
    let nonempty: Vec<usize> = (0..n).filter(|&b| len(b) > 0).collect();
    let fb = *nonempty.get(pick[0] % nonempty.len().max(1))?;
    let later: Vec<usize> = nonempty.iter().copied().filter(|&b| b >= fb).collect();
    let tb = later[pick[2] % later.len()];
    let mut from = (fb, pick[1] % len(fb));
    let mut to = (tb, pick[3] % len(tb));
    if to < from {
        std::mem::swap(&mut from, &mut to);
    }
    Some((from, to))
}

fn seeds() -> impl Strategy<Value = Vec<Vec<Seed>>> {
    proptest::collection::vec(
        proptest::collection::vec((1u8..7, 0u8..13, any::<bool>()), 0..5),
        4,
    )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// C1, C3, C4: a paste is one undo step, undo restores the score, the hard
    /// invariants hold, and the pasted run equals the copied one.
    #[test]
    fn c1_c3_c4_paste_is_one_undo_step_and_reproduces_the_beats(
        ops in proptest::collection::vec(op(), 0..8),
        seeds in seeds(),
        pick in proptest::array::uniform4(0usize..32),
        dst_bar in 0usize..32, dst_index in 0usize..32,
    ) {
        let (mut s, t) = populated(&ops, &seeds);
        let Some((from, to)) = pick_run(&s, t, pick) else { return Ok(()); };
        let frag = s.copy_beats(t, 0, from, to).expect("a picked run exists");
        let n = s.score().master_bar_count();
        let db = dst_bar % n;
        let dst_len = s.score().track(t).unwrap().bar(db).unwrap().voices[0].beats.len();
        // One past the end is the append slot; two past is deliberately invalid.
        let index = dst_index % (dst_len + 2);
        let before = s.snapshot();
        let (rev, depth) = (s.revision(), s.undo_depth());
        let result = s.execute(cmd(9_000, Command::Score(ScoreCommand::PasteBeats {
            track: t, bar: db, voice: 0, index, beats: frag.clone(),
        })));
        if index > dst_len {
            prop_assert!(result.is_err());
        }
        match result {
            Ok(_) => {
                prop_assert_eq!(s.undo_depth(), depth + 1);
                prop_assert_eq!(s.revision().0, rev.0 + 1);
                let v = s.score().validate();
                prop_assert!(v.is_empty(), "{:?}", v);
                let pasted = s.copy_beats(t, 0, (db, index), (db, index + frag.len() - 1)).unwrap();
                prop_assert!(reproduces(&pasted, &frag), "{:?} vs {:?}", pasted, frag);
                s.undo().unwrap();
                prop_assert_eq!(s.score(), &before);
            }
            Err(_) => {
                prop_assert_eq!(s.score(), &before);
                prop_assert_eq!(s.revision(), rev);
                prop_assert_eq!(s.undo_depth(), depth);
            }
        }
    }

    /// C4, C5: a cut is one undo step, removes exactly the run, and undo restores it.
    #[test]
    fn c4_c5_cut_removes_exactly_the_run_and_undo_restores_it(
        ops in proptest::collection::vec(op(), 0..8),
        seeds in seeds(),
        pick in proptest::array::uniform4(0usize..32),
    ) {
        let (mut s, t) = populated(&ops, &seeds);
        let Some((from, to)) = pick_run(&s, t, pick) else { return Ok(()); };
        let frag = s.copy_beats(t, 0, from, to).expect("a picked run exists");
        let n = s.score().master_bar_count();
        let total = |s: &mc_app::DocumentSession| -> usize {
            (0..n)
                .filter_map(|b| s.score().track(t).unwrap().bar(b))
                .map(|b| b.voices[0].beats.len())
                .sum()
        };
        let (before, before_total, depth) = (s.snapshot(), total(&s), s.undo_depth());
        // Expected run length computed independently of the core's range resolution.
        let bar_len = |b: usize| s.score().track(t).unwrap().bar(b).unwrap().voices[0].beats.len();
        let expected: usize = if from.0 == to.0 {
            to.1 - from.1 + 1
        } else {
            (bar_len(from.0) - from.1) + (from.0 + 1..to.0).map(bar_len).sum::<usize>() + to.1 + 1
        };
        prop_assert_eq!(frag.len(), expected);
        s.execute(cmd(9_001, Command::Score(ScoreCommand::RemoveBeats {
            track: t, voice: 0, from, to,
        }))).unwrap();
        prop_assert_eq!(s.undo_depth(), depth + 1);
        prop_assert_eq!(total(&s), before_total - expected);
        let v = s.score().validate();
        prop_assert!(v.is_empty(), "{:?}", v);
        s.undo().unwrap();
        prop_assert_eq!(s.score(), &before);
    }
}
