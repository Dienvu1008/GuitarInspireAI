//! `Score::replace_voices` (used by ADR-034 `ApplyPattern`): all or nothing, adds a
//! voice only at the next index, keeps every other voice and bar, normalizes ties once.

use mc_score::{
    BeatRef, DomainEvent, FragmentBeat, FragmentNote, FrettedInstrument, InstrumentSpec, MasterBar,
    NoteEffects, Score,
};
use mc_theory::fretted::{Fret, FretPosition, StringIndex};
use mc_theory::{BaseValue, NoteValue, Tempo};

fn q() -> NoteValue {
    NoteValue::plain(BaseValue::QUARTER)
}

fn note(string: u8, fret: u8, tie: bool) -> FragmentNote {
    FragmentNote {
        position: FretPosition {
            string: StringIndex::new(string).unwrap(),
            fret: Fret(fret),
        },
        tie,
        effects: NoteEffects::default(),
    }
}

fn beat(notes: Vec<FragmentNote>) -> FragmentBeat {
    FragmentBeat {
        value: q(),
        notes,
        stroke: None,
    }
}

fn bar_of(notes: &[(u8, u8, bool)]) -> Vec<FragmentBeat> {
    notes
        .iter()
        .map(|&(s, f, t)| beat(vec![note(s, f, t)]))
        .collect()
}

fn score(bars: usize) -> Score {
    let mut s = Score::new(
        "T",
        InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
        MasterBar::default(),
        Tempo::from_bpm(100).unwrap(),
    )
    .unwrap();
    if bars > 1 {
        s.insert_master_bars(1, bars - 1, MasterBar::default())
            .unwrap();
    }
    s
}

#[test]
fn replaces_voices_adds_the_next_voice_and_keeps_the_rest() {
    let mut s = score(4);
    let t = s.track_at(0).unwrap().id();
    // Bar 1 has an existing voice 1 and voice 2 (index 2): keep index 2.
    s.add_voice(t, 1).unwrap();
    s.add_voice(t, 1).unwrap();
    let (b, _) = s.insert_beat(t, 1, 2, 0, q()).unwrap();
    s.set_note(
        BeatRef {
            track: t,
            bar: 1,
            voice: 2,
            beat: b,
        },
        note(6, 0, false).position,
    )
    .unwrap();
    let kept_bar3 = s.track(t).unwrap().bar(3).unwrap().clone();
    let kept_voice2 = s.track(t).unwrap().bar(1).unwrap().voices[2].clone();
    s.replace_voices(
        t,
        0,
        &[
            vec![
                (0, bar_of(&[(1, 0, false); 4])),
                (1, bar_of(&[(5, 3, false); 4])),
            ],
            vec![
                (0, bar_of(&[(2, 1, false); 4])),
                (1, bar_of(&[(5, 3, false); 4])),
            ],
        ],
    )
    .unwrap();
    let tr = s.track(t).unwrap();
    assert_eq!(tr.bar(0).unwrap().voices.len(), 2, "voice 1 added in bar 0");
    assert_eq!(tr.bar(1).unwrap().voices.len(), 3);
    assert_eq!(
        tr.bar(1).unwrap().voices[2],
        kept_voice2,
        "voice index 2 kept"
    );
    assert_eq!(tr.bar(3).unwrap(), &kept_bar3, "bar outside the range kept");
    assert_eq!(tr.bar(0).unwrap().voices[0].beats.len(), 4);
    assert!(s.validate().is_empty());
}

#[test]
fn an_invalid_note_anywhere_changes_nothing() {
    let mut s = score(3);
    let t = s.track_at(0).unwrap().id();
    let before = s.clone();
    for bad in [
        vec![
            vec![(0, bar_of(&[(1, 0, false)]))],
            vec![(0, bar_of(&[(7, 0, false)]))],
        ],
        vec![
            vec![(0, bar_of(&[(1, 0, false)]))],
            vec![(0, bar_of(&[(1, 30, false)]))],
        ],
        vec![vec![(
            0,
            vec![beat(vec![note(1, 0, false), note(1, 2, false)])],
        )]],
        vec![vec![(2, bar_of(&[(1, 0, false)]))]],
        vec![vec![(0, vec![]), (0, vec![])]],
    ] {
        assert!(s.replace_voices(t, 0, &bad).is_err(), "{bad:?}");
        assert_eq!(s, before, "nothing changed");
    }
    assert!(
        s.replace_voices(t, 2, &[vec![], vec![]]).is_err(),
        "past the last bar"
    );
    assert!(s.replace_voices(t, 0, &[]).is_err());
    assert_eq!(s, before);
}

#[test]
fn ties_are_normalized_once_and_a_tie_into_the_range_end_is_reported() {
    let mut s = score(3);
    let t = s.track_at(0).unwrap().id();
    // Bar 2 starts with a note tied to bar 1's last note (string 1 fret 0).
    s.replace_voices(
        t,
        1,
        &[
            vec![(0, bar_of(&[(1, 0, false)]))],
            vec![(0, bar_of(&[(1, 0, true)]))],
        ],
    )
    .unwrap();
    assert!(s.track(t).unwrap().bar(2).unwrap().voices[0].beats[0].notes[0].tie);
    // Replacing bar 1 with another fret breaks that tie: it is cleared and reported.
    let events = s
        .replace_voices(t, 1, &[vec![(0, bar_of(&[(1, 5, false)]))]])
        .unwrap();
    assert!(!s.track(t).unwrap().bar(2).unwrap().voices[0].beats[0].notes[0].tie);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DomainEvent::TiesRemoved { notes, .. } if notes.len() == 1))
    );
    // A tie inside the written range, across its bar line, is kept.
    let events = s
        .replace_voices(
            t,
            0,
            &[
                vec![(0, bar_of(&[(3, 2, false)]))],
                vec![(0, bar_of(&[(3, 2, true)]))],
            ],
        )
        .unwrap();
    assert!(s.track(t).unwrap().bar(1).unwrap().voices[0].beats[0].notes[0].tie);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, DomainEvent::TiesRemoved { .. }))
    );
}
