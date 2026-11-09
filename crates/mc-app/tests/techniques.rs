//! ADR-035 TQ-1 and TQ-2: every technique flag and the stroke can be set, cleared,
//! undone and redone through session commands; each change is one revision and one
//! undo step and changes nothing else; copy and paste keep every flag and the stroke.

mod common;
use common::*;
use mc_app::types::{NoteEffects, Stroke};
use mc_app::{BeatAddress, ClipBeat, Command, ScoreCommand};
use mc_score::BeatRef;

fn set_note(beat: BeatAddress, string: u8, fret: u8) -> Command {
    Command::Score(ScoreCommand::SetNote { beat, string, fret })
}

fn beat_ref(b: BeatAddress) -> BeatRef {
    BeatRef {
        track: b.track,
        bar: b.bar,
        voice: b.voice,
        beat: b.beat,
    }
}

/// All 32 combinations of the five note flags.
fn all_effects() -> Vec<NoteEffects> {
    (0u8..32)
        .map(|m| NoteEffects {
            let_ring: m & 1 != 0,
            dead: m & 2 != 0,
            accent: m & 4 != 0,
            palm_mute: m & 8 != 0,
            string_percussive: m & 16 != 0,
        })
        .collect()
}

/// A session with two chord beats in bar 0 (strings 1 and 2) and returns the first
/// beat and its first note.
fn chord_session() -> (mc_app::DocumentSession, BeatAddress, mc_app::types::NoteId) {
    let mut s = session(2);
    let t = first_track(&s);
    let b = add_beat(&mut s, t, 0);
    s.execute(cmd(1, set_note(b, 1, 3))).unwrap();
    s.execute(cmd(2, set_note(b, 2, 1))).unwrap();
    let other = add_beat(&mut s, t, 0);
    s.execute(cmd(3, set_note(other, 3, 2))).unwrap();
    let note = s.score().track(t).unwrap().bar(0).unwrap().voices[0].beats[0].notes[0].id;
    (s, b, note)
}

#[test]
fn tq1_every_flag_combination_is_one_step_and_changes_only_that_note() {
    let (mut s, b, note) = chord_session();
    for (i, effects) in all_effects().into_iter().enumerate() {
        let before = s.snapshot();
        let current =
            before.track(b.track).unwrap().bar(0).unwrap().voices[0].beats[0].notes[0].effects;
        if current == effects {
            continue; // the first combination is the starting state
        }
        let (rev, depth) = (s.revision(), s.undo_depth());
        s.execute(cmd(
            100 + i as u64,
            Command::Score(ScoreCommand::SetNoteEffects {
                beat: b,
                note,
                effects,
            }),
        ))
        .unwrap();
        assert_eq!(s.revision().0, rev.0 + 1, "{effects:?}: one revision");
        assert_eq!(s.undo_depth(), depth + 1, "{effects:?}: one undo step");
        // Nothing else changes: the result equals the old score with only this note's
        // effects set through the aggregate.
        let mut expected = before.clone();
        expected
            .set_note_effects(beat_ref(b), note, effects)
            .unwrap();
        assert_eq!(s.score(), &expected, "{effects:?}");
        s.undo().unwrap();
        assert_eq!(s.score(), &before, "{effects:?}: undo");
        s.redo().unwrap();
        assert_eq!(s.score(), &expected, "{effects:?}: redo");
    }
    // Clearing every flag is a change like any other.
    let before = s.snapshot();
    s.execute(cmd(
        999,
        Command::Score(ScoreCommand::SetNoteEffects {
            beat: b,
            note,
            effects: NoteEffects::default(),
        }),
    ))
    .unwrap();
    let mut expected = before.clone();
    expected
        .set_note_effects(beat_ref(b), note, NoteEffects::default())
        .unwrap();
    assert_eq!(s.score(), &expected);
}

#[test]
fn tq1_stroke_set_change_clear_undo_redo() {
    let (mut s, b, _) = chord_session();
    let stroke_of = |s: &mc_app::DocumentSession| {
        s.score().track(b.track).unwrap().bar(0).unwrap().voices[0].beats[0].stroke
    };
    let mut history = vec![s.snapshot()];
    for (i, stroke) in [Some(Stroke::Down), Some(Stroke::Up), None]
        .into_iter()
        .enumerate()
    {
        let before = s.snapshot();
        let (rev, depth) = (s.revision(), s.undo_depth());
        s.execute(cmd(
            200 + i as u64,
            Command::Score(ScoreCommand::SetBeatStroke { beat: b, stroke }),
        ))
        .unwrap();
        assert_eq!(stroke_of(&s), stroke);
        assert_eq!(s.revision().0, rev.0 + 1);
        assert_eq!(s.undo_depth(), depth + 1);
        let mut expected = before.clone();
        expected.set_beat_stroke(beat_ref(b), stroke).unwrap();
        assert_eq!(s.score(), &expected, "only the stroke changes");
        history.push(s.snapshot());
    }
    // Undo walks back through every state, redo forward again.
    for state in history.iter().rev().skip(1) {
        s.undo().unwrap();
        assert_eq!(s.score(), state);
    }
    for state in history.iter().skip(1) {
        s.redo().unwrap();
        assert_eq!(s.score(), state);
    }
}

#[test]
fn tq1_a_stroke_on_a_rest_is_allowed_and_a_missing_beat_is_rejected() {
    let mut s = session(1);
    let t = first_track(&s);
    let rest = add_beat(&mut s, t, 0);
    s.execute(cmd(
        1,
        Command::Score(ScoreCommand::SetBeatStroke {
            beat: rest,
            stroke: Some(Stroke::Down),
        }),
    ))
    .unwrap();
    let before = s.snapshot();
    let rev = s.revision();
    let missing = BeatAddress {
        beat: mc_app::types::BeatId::from_raw(u64::MAX),
        ..rest
    };
    assert!(
        s.execute(cmd(
            2,
            Command::Score(ScoreCommand::SetBeatStroke {
                beat: missing,
                stroke: Some(Stroke::Up),
            }),
        ))
        .is_err()
    );
    assert_eq!(s.score(), &before);
    assert_eq!(s.revision(), rev);
}

#[test]
fn tq2_copy_and_paste_keep_every_flag_and_the_stroke() {
    let mut s = session(8);
    let t = first_track(&s);
    // 32 quarter beats, four per bar: one per flag combination, strokes cycling
    // none/down/up so every pair of flag set and stroke value occurs.
    let strokes = [None, Some(Stroke::Down), Some(Stroke::Up)];
    for (i, effects) in all_effects().into_iter().enumerate() {
        let b = add_beat(&mut s, t, i / 4);
        s.execute(cmd(10, set_note(b, 1 + (i % 6) as u8, 2)))
            .unwrap();
        let note = s.score().track(t).unwrap().bar(b.bar).unwrap().voices[0]
            .beats
            .last()
            .unwrap()
            .notes[0]
            .id;
        s.execute(cmd(
            11,
            Command::Score(ScoreCommand::SetNoteEffects {
                beat: b,
                note,
                effects,
            }),
        ))
        .unwrap();
        s.execute(cmd(
            12,
            Command::Score(ScoreCommand::SetBeatStroke {
                beat: b,
                stroke: strokes[i % 3],
            }),
        ))
        .unwrap();
    }
    let facts = |f: &[ClipBeat]| -> Vec<(Option<Stroke>, Vec<NoteEffects>)> {
        f.iter()
            .map(|b| (b.stroke, b.notes.iter().map(|n| n.effects).collect()))
            .collect()
    };
    let frag: Vec<ClipBeat> = s.copy_beats(t, 0, (0, 0), (7, 3)).unwrap();
    let expected: Vec<_> = all_effects()
        .into_iter()
        .enumerate()
        .map(|(i, e)| (strokes[i % 3], vec![e]))
        .collect();
    assert_eq!(facts(&frag), expected, "copy carries the stored values");
    // Paste into the same bars of a second guitar track and copy back.
    s.execute(cmd(
        20,
        Command::Track(mc_app::TrackCommand::Add {
            name: "Copy".into(),
            instrument: mc_app::InstrumentSpecInput::standard_guitar(),
        }),
    ))
    .unwrap();
    let t2 = s.score().track_at(1).unwrap().id();
    let rev = s.revision();
    s.execute(cmd(
        21,
        Command::Score(ScoreCommand::PasteBeats {
            track: t2,
            bar: 0,
            voice: 0,
            index: 0,
            beats: frag.clone(),
        }),
    ))
    .unwrap();
    assert_eq!(s.revision().0, rev.0 + 1, "one paste, one revision");
    let n = s.score().track(t2).unwrap().bar(0).unwrap().voices[0]
        .beats
        .len();
    let back = s.copy_beats(t2, 0, (0, 0), (0, n - 1)).unwrap();
    assert_eq!(
        facts(&back),
        expected,
        "paste keeps every flag and the stroke"
    );
}
