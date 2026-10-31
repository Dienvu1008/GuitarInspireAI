//! Guitar commands through the session: retune with keep-pitches and keep-frets.

mod common;
use common::*;
use mc_app::guitar::RetuneMode;
use mc_app::{Command, CommandError, GuitarCommand, InstrumentSpecInput, ScoreCommand};

fn drop_d() -> InstrumentSpecInput {
    InstrumentSpecInput {
        tuning: ["E4", "B3", "G3", "D3", "A2", "D2"]
            .map(String::from)
            .to_vec(),
        ..InstrumentSpecInput::standard_guitar()
    }
}

fn retune(
    track: mc_app::types::TrackId,
    instrument: InstrumentSpecInput,
    mode: RetuneMode,
) -> Command {
    Command::Guitar(GuitarCommand::Retune {
        track,
        instrument,
        mode,
    })
}

fn frets(s: &mc_app::DocumentSession, t: mc_app::types::TrackId) -> Vec<(u8, u8)> {
    s.score().track(t).unwrap().bar(0).unwrap().voices[0].beats[0]
        .notes
        .iter()
        .map(|n| (n.position().string.get(), n.position().fret.0))
        .collect()
}

#[test]
fn retune_keep_pitches_moves_notes_and_undo_restores() {
    let mut s = session(1);
    let t = first_track(&s);
    let b = add_beat(&mut s, t, 0);
    for (string, fret) in [(6, 3), (1, 0)] {
        s.execute(cmd(
            1,
            Command::Score(ScoreCommand::SetNote {
                beat: b,
                string,
                fret,
            }),
        ))
        .unwrap();
    }
    s.execute(cmd(2, retune(t, drop_d(), RetuneMode::KeepPitches)))
        .unwrap();
    assert_eq!(frets(&s, t), [(1, 0), (6, 5)]);
    s.undo().unwrap();
    assert_eq!(frets(&s, t), [(1, 0), (6, 3)]);

    s.execute(cmd(3, retune(t, drop_d(), RetuneMode::KeepFrets)))
        .unwrap();
    assert_eq!(frets(&s, t), [(1, 0), (6, 3)]);
}

#[test]
fn retune_that_would_push_a_note_below_fret_0_is_rejected() {
    let mut s = session(1);
    let t = first_track(&s);
    let b = add_beat(&mut s, t, 0);
    s.execute(cmd(
        1,
        Command::Score(ScoreCommand::SetNote {
            beat: b,
            string: 6,
            fret: 0,
        }),
    ))
    .unwrap();
    let before = s.revision();
    // Standard -> drop D -> standard with keep-pitches: the low D cannot be played on E.
    s.execute(cmd(2, retune(t, drop_d(), RetuneMode::KeepPitches)))
        .unwrap();
    s.execute(cmd(
        3,
        Command::Score(ScoreCommand::SetNote {
            beat: b,
            string: 6,
            fret: 0,
        }),
    ))
    .unwrap();
    let err = s
        .execute(cmd(
            4,
            retune(
                t,
                InstrumentSpecInput::standard_guitar(),
                RetuneMode::KeepPitches,
            ),
        ))
        .unwrap_err();
    assert_eq!(err.rule(), "H-04");
    assert!(s.revision() > before);

    let seven = InstrumentSpecInput {
        tuning: ["E4", "B3", "G3", "D3", "A2", "E2", "B1"]
            .map(String::from)
            .to_vec(),
        ..InstrumentSpecInput::standard_guitar()
    };
    assert!(matches!(
        s.execute(cmd(5, retune(t, seven, RetuneMode::KeepPitches))),
        Err(CommandError::Guitar(_))
    ));
}
