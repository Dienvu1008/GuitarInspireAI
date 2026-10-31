//! Hard invariants and editing constraints (domain-invariants.md sections 2 and 3).
//! Test names start with the rule id (FF-24).

mod common;
use common::*;
use mc_score::{
    BarContent, Beat, BeatId, DomainError, DomainEvent, FrettedInstrument, InstrumentSpec,
    MasterBar, Note, NoteEffects, NoteId, NoteKind, Score, ScoreParts, TempoChange, TrackData,
    TrackId, Voice,
};
use mc_theory::fretted::{Capo, Tuning};
use mc_theory::{Rational, Tempo, TimeSignature};

fn rule(e: DomainError) -> &'static str {
    e.rule()
}

#[test]
fn h01_ids_are_unique_and_duplicates_are_rejected_on_load() {
    let mut s = score(2, 2);
    let t = track(&s, 0);
    let b = fill(&mut s, t, 0, 0, &[q(), q()]);
    let n1 = note(&mut s, b[0], 1, 0);
    let n2 = note(&mut s, b[1], 1, 0);
    assert_ne!(n1, n2);
    assert_ne!(b[0].beat, b[1].beat);
    assert!(s.validate().is_empty());

    let dup = Beat {
        id: BeatId::from_raw(7),
        value: q(),
        notes: vec![],
    };
    let parts = ScoreParts {
        master_bars: vec![MasterBar::default()],
        tracks: vec![TrackData {
            id: TrackId::from_raw(7),
            name: "t".into(),
            instrument: guitar(),
            bars: vec![BarContent {
                voices: vec![Voice { beats: vec![dup] }],
            }],
        }],
        tempo_map: vec![TempoChange {
            bar: 0,
            offset: Rational::ZERO,
            tempo: Tempo::from_bpm(90).unwrap(),
        }],
    };
    let v = Score::from_parts(parts).unwrap_err();
    assert!(v.iter().any(|x| x.rule == "H-01"), "{v:?}");
}

#[test]
fn h01_new_ids_continue_after_loaded_ids() {
    let mut s = score(1, 1);
    let t = track(&s, 0);
    let b = fill(&mut s, t, 0, 0, &[q()]);
    let max_before = b[0].beat.raw();
    let parts = parts_of(&s);
    let mut loaded = Score::from_parts(parts).unwrap();
    let (id, _) = loaded.insert_beat(t, 0, 0, 1, q()).unwrap();
    assert!(id.raw() > max_before);
}

#[test]
fn h02_every_track_stays_aligned_with_master_bars() {
    let mut s = score(3, 2);
    s.insert_master_bars(1, 2, MasterBar::default()).unwrap();
    s.remove_master_bars(0, 1).unwrap();
    for t in s.tracks() {
        assert_eq!(t.bars().len(), s.master_bar_count());
    }
    let (t3, _) = s.add_track("new", guitar()).unwrap();
    assert_eq!(s.track(t3).unwrap().bars().len(), 4);

    let mut parts = parts_of(&s);
    parts.tracks[0].bars.pop();
    assert!(
        Score::from_parts(parts)
            .unwrap_err()
            .iter()
            .any(|v| v.rule == "H-02")
    );
}

#[test]
fn h03_h04_string_and_fret_ranges() {
    let mut s = score(1, 1);
    let t = track(&s, 0);
    let b = fill(&mut s, t, 0, 0, &[q()]);
    assert_eq!(rule(s.set_note(b[0], pos(7, 0)).unwrap_err()), "H-03");
    assert_eq!(rule(s.set_note(b[0], pos(1, 25)).unwrap_err()), "H-04");
    assert!(s.set_note(b[0], pos(6, 24)).is_ok());
}

#[test]
fn h05_one_note_per_string_per_beat_by_construction() {
    let mut s = score(1, 1);
    let t = track(&s, 0);
    let b = fill(&mut s, t, 0, 0, &[q()]);
    let a = note(&mut s, b[0], 6, 3);
    let again = note(&mut s, b[0], 6, 5);
    assert_eq!(
        a, again,
        "same string replaces the fret and keeps the note id"
    );
    let beat = &s.track(t).unwrap().bar(0).unwrap().voices[0].beats[0];
    assert_eq!(beat.notes.len(), 1);
    assert_eq!(beat.notes[0].position().fret.0, 5);

    let mut parts = parts_of(&s);
    let n = parts.tracks[0].bars[0].voices[0].beats[0].notes[0].clone();
    parts.tracks[0].bars[0].voices[0].beats[0].notes.push(Note {
        id: NoteId::from_raw(999),
        ..n
    });
    assert!(
        Score::from_parts(parts)
            .unwrap_err()
            .iter()
            .any(|v| v.rule == "H-05")
    );
}

#[test]
fn h05_two_voices_may_use_the_same_string() {
    let mut s = score(1, 1);
    let t = track(&s, 0);
    let (v1, _) = s.add_voice(t, 0).unwrap();
    let a = fill(&mut s, t, 0, 0, &[q()]);
    let b = fill(&mut s, t, 0, v1, &[q()]);
    note(&mut s, a[0], 3, 2);
    note(&mut s, b[0], 3, 4);
    assert!(s.validate().is_empty());
}

#[test]
fn h06_ties_require_an_equal_preceding_note_and_are_cleared_when_it_changes() {
    let mut s = score(2, 1);
    let t = track(&s, 0);
    let b0 = fill(&mut s, t, 0, 0, &[q()]);
    let b1 = fill(&mut s, t, 1, 0, &[q()]);
    note(&mut s, b0[0], 2, 5);
    let tied = note(&mut s, b1[0], 2, 5);
    s.set_tie(b1[0], tied, true).unwrap();

    let other = note(&mut s, b1[0], 3, 1);
    assert_eq!(rule(s.set_tie(b1[0], other, true).unwrap_err()), "H-06");

    // Changing the preceding note's fret invalidates the tie in the next bar.
    let events = s.set_note(b0[0], pos(2, 7)).unwrap().1;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DomainEvent::TiesRemoved { notes, .. } if notes == &vec![tied])),
        "{events:?}"
    );
    assert!(events.contains(&DomainEvent::BarContentChanged {
        track: t,
        bars: 0..2
    }));
    assert!(s.validate().is_empty());
}

#[test]
fn h06_removing_the_preceding_bar_clears_the_tie() {
    let mut s = score(3, 1);
    let t = track(&s, 0);
    let b1 = fill(&mut s, t, 1, 0, &[q()]);
    let b2 = fill(&mut s, t, 2, 0, &[q()]);
    note(&mut s, b1[0], 1, 3);
    let tied = note(&mut s, b2[0], 1, 3);
    s.set_tie(b2[0], tied, true).unwrap();
    s.remove_master_bars(1, 1).unwrap();
    assert!(s.validate().is_empty());
    assert!(!s.track(t).unwrap().bar(1).unwrap().voices[0].beats[0].notes[0].tie);
}

#[test]
fn h09_e03_last_voice_last_track_and_last_bar_cannot_be_removed() {
    let mut s = score(1, 1);
    let t = track(&s, 0);
    assert_eq!(rule(s.remove_voice(t, 0, 0).unwrap_err()), "H-09");
    assert_eq!(rule(s.remove_track(t).unwrap_err()), "E-03");
    assert_eq!(rule(s.remove_master_bars(0, 1).unwrap_err()), "E-03");
}

/// H-08 (note kind matches the track's instrument kind) holds by construction while
/// only fretted tracks and fretted notes exist: both enums have one variant, so a
/// mismatch cannot be expressed. This test pins that; it must be replaced by a real
/// mismatch test when a second instrument kind is added.
#[test]
fn h08_note_kind_matches_instrument_kind_by_construction() {
    let mut s = score(1, 1);
    let t = track(&s, 0);
    let b = fill(&mut s, t, 0, 0, &[q()]);
    note(&mut s, b[0], 2, 2);
    let tr = s.track(t).unwrap();
    assert!(matches!(tr.instrument(), InstrumentSpec::Fretted(_)));
    let n = &tr.bar(0).unwrap().voices[0].beats[0].notes[0];
    assert!(matches!(n.kind, NoteKind::Fretted(_)));
    assert!(s.validate().is_empty());
}

#[test]
fn h10_capo_plus_fret_within_fret_count() {
    let mut s = score(1, 1);
    let t = track(&s, 0);
    let b = fill(&mut s, t, 0, 0, &[q()]);
    note(&mut s, b[0], 1, 22);
    let with_capo = |c: u8| {
        InstrumentSpec::Fretted(FrettedInstrument {
            capo: Capo(c),
            ..FrettedInstrument::standard_guitar()
        })
    };
    assert_eq!(rule(s.set_instrument(t, with_capo(3)).unwrap_err()), "H-10");
    assert!(s.set_instrument(t, with_capo(2)).is_ok());
    assert_eq!(rule(s.set_note(b[0], pos(2, 23)).unwrap_err()), "H-10");
}

#[test]
fn h03_retune_to_fewer_strings_is_rejected_while_notes_use_them() {
    let mut s = score(1, 1);
    let t = track(&s, 0);
    let b = fill(&mut s, t, 0, 0, &[q()]);
    note(&mut s, b[0], 6, 0);
    let four = Tuning::new(
        ["G2", "D2", "A1", "E1"]
            .map(|p| p.parse().unwrap())
            .to_vec(),
    )
    .unwrap();
    let bass = InstrumentSpec::Fretted(FrettedInstrument {
        tuning: four,
        fret_count: 20,
        capo: Capo(0),
    });
    assert_eq!(rule(s.set_instrument(t, bass).unwrap_err()), "H-03");
}

fn drop_d() -> InstrumentSpec {
    let tuning = Tuning::new(
        ["E4", "B3", "G3", "D3", "A2", "D2"]
            .map(|p| p.parse().unwrap())
            .to_vec(),
    )
    .unwrap();
    InstrumentSpec::Fretted(FrettedInstrument {
        tuning,
        ..FrettedInstrument::standard_guitar()
    })
}

#[test]
fn h04_h06_retune_shifts_frets_per_string_and_keeps_ties() {
    let mut s = score(2, 1);
    let t = track(&s, 0);
    let b0 = fill(&mut s, t, 0, 0, &[q()]);
    let b1 = fill(&mut s, t, 1, 0, &[q()]);
    note(&mut s, b0[0], 6, 3);
    note(&mut s, b0[0], 1, 0);
    let tied = note(&mut s, b1[0], 6, 3);
    s.set_tie(b1[0], tied, true).unwrap();

    let events = s
        .retune_track(t, drop_d(), Some(&[0, 0, 0, 0, 0, 2]))
        .unwrap();
    assert!(events.contains(&DomainEvent::TrackChanged { track: t }));
    let tr = s.track(t).unwrap();
    assert_eq!(tr.instrument(), &drop_d());
    let frets = |bar: usize| -> Vec<(u8, u8)> {
        tr.bar(bar).unwrap().voices[0].beats[0]
            .notes
            .iter()
            .map(|n| (n.position().string.get(), n.position().fret.0))
            .collect()
    };
    assert_eq!(frets(0), [(1, 0), (6, 5)]);
    assert_eq!(frets(1), [(6, 5)]);
    assert!(tr.bar(1).unwrap().voices[0].beats[0].notes[0].tie);
    assert!(s.validate().is_empty());
}

#[test]
fn h04_retune_is_all_or_nothing() {
    let mut s = score(2, 1);
    let t = track(&s, 0);
    let b0 = fill(&mut s, t, 0, 0, &[q()]);
    let b1 = fill(&mut s, t, 1, 0, &[q()]);
    note(&mut s, b0[0], 6, 3);
    note(&mut s, b1[0], 1, 0);
    let before = s.clone();
    // String 1 open would need fret -1.
    let err = s
        .retune_track(t, guitar(), Some(&[-1, 0, 0, 0, 0, 0]))
        .unwrap_err();
    assert_eq!(rule(err), "H-04");
    assert_eq!(s, before);
    // Fret above the fret count.
    let err = s
        .retune_track(t, guitar(), Some(&[0, 0, 0, 0, 0, 22]))
        .unwrap_err();
    assert_eq!(rule(err), "H-04");
    assert_eq!(s, before);
    // A missing delta for a used string.
    let err = s.retune_track(t, guitar(), Some(&[0])).unwrap_err();
    assert_eq!(rule(err), "H-03");
    assert_eq!(s, before);
    // No deltas: frets are kept.
    s.retune_track(t, drop_d(), None).unwrap();
    assert_eq!(
        s.track(t).unwrap().bar(0).unwrap().voices[0].beats[0].notes[0]
            .position()
            .fret
            .0,
        3
    );
}

#[test]
fn h11_tempo_map_rules() {
    let mut s = score(2, 1);
    let t120 = Tempo::from_bpm(120).unwrap();
    assert_eq!(
        rule(s.set_tempo(1, r(1, 1), t120).unwrap_err()),
        "H-11",
        "offset at bar length is outside the bar"
    );
    assert_eq!(rule(s.set_tempo(0, r(-1, 4), t120).unwrap_err()), "H-11");
    s.set_tempo(1, r(1, 2), Tempo::from_bpm(90).unwrap())
        .unwrap();
    s.set_tempo(1, r(1, 4), Tempo::from_bpm(100).unwrap())
        .unwrap();
    let positions: Vec<_> = s.tempo_map().iter().map(|c| (c.bar, c.offset)).collect();
    assert_eq!(positions, [(0, r(0, 1)), (1, r(1, 4)), (1, r(1, 2))]);
    assert_eq!(rule(s.remove_tempo(0, Rational::ZERO).unwrap_err()), "H-11");
    // Shrinking the bar below an existing tempo offset is rejected.
    let three_eight = MasterBar::new(
        TimeSignature::new(3, 8).unwrap(),
        mc_theory::KeySignature::C_MAJOR,
    );
    assert_eq!(rule(s.set_master_bar(1, three_eight).unwrap_err()), "H-11");
    assert!(s.validate().is_empty());
}

#[test]
fn h11_inserting_and_removing_bars_keeps_tempo_positions_meaningful() {
    let mut s = score(4, 1);
    s.set_tempo(2, Rational::ZERO, Tempo::from_bpm(90).unwrap())
        .unwrap();
    s.insert_master_bars(1, 1, MasterBar::default()).unwrap();
    assert_eq!(s.tempo_map()[1].bar, 3, "change moved with its bar");
    s.remove_master_bars(2, 2).unwrap(); // removes bars 2 and 3 (the one holding the change)
    // The tempo heard at the first bar after the removed range is preserved there.
    assert_eq!(
        s.tempo_in_effect(2, Rational::ZERO),
        Some(Tempo::from_bpm(90).unwrap())
    );
    assert!(s.validate().is_empty());
}

#[test]
fn h12_repeat_structure_is_checked_by_master_bar_commands() {
    let mut s = score(3, 1);
    let start = MasterBar {
        repeat_start: true,
        ..MasterBar::default()
    };
    s.set_master_bar(0, start.clone()).unwrap();
    assert_eq!(rule(s.set_master_bar(1, start).unwrap_err()), "H-12");
}

#[test]
fn e02_e04_missing_targets_and_bad_positions() {
    let mut s = score(1, 1);
    let t = track(&s, 0);
    let ghost = beat(t, 0, 0, BeatId::from_raw(4242));
    assert_eq!(rule(s.set_note(ghost, pos(1, 0)).unwrap_err()), "E-02");
    assert_eq!(rule(s.insert_beat(t, 0, 0, 5, q()).unwrap_err()), "E-04");
    assert_eq!(rule(s.insert_beat(t, 3, 0, 0, q()).unwrap_err()), "E-02");
    assert_eq!(
        rule(s.rename_track(TrackId::from_raw(999), "x").unwrap_err()),
        "E-02"
    );
    assert_eq!(
        rule(
            s.insert_master_bars(5, 1, MasterBar::default())
                .unwrap_err()
        ),
        "E-04"
    );
}

#[test]
fn rejected_operations_leave_the_score_unchanged() {
    let mut s = score(2, 2);
    let t = track(&s, 0);
    let b = fill(&mut s, t, 0, 0, &[q()]);
    note(&mut s, b[0], 1, 20);
    let before = s.clone();
    assert!(
        s.shift_frets(t, 0..2, 5).is_err(),
        "fret 25 is out of range, so nothing may move"
    );
    assert!(s.set_note(b[0], pos(9, 0)).is_err());
    assert!(s.remove_master_bars(0, 2).is_err());
    assert_eq!(s, before);
}

#[test]
fn structural_sharing_copies_only_edited_bars() {
    let mut s = score(300, 20);
    let t = track(&s, 5);
    let snapshot = s.clone();
    let b = fill(&mut s, t, 120, 0, &[q()]);
    note(&mut s, b[0], 3, 3);
    assert_eq!(s.unshared_bars(&snapshot), 1);
    let edited = s.track(t).unwrap();
    let old = snapshot.track(t).unwrap();
    assert!(!edited.shares_bar_with(old, 120));
    assert!(edited.shares_bar_with(old, 119));
    assert!(
        s.track_at(0)
            .unwrap()
            .shares_bar_with(snapshot.track_at(0).unwrap(), 120)
    );
}

#[test]
fn note_effects_and_removal() {
    let mut s = score(1, 1);
    let t = track(&s, 0);
    let b = fill(&mut s, t, 0, 0, &[q()]);
    let n = note(&mut s, b[0], 4, 2);
    s.set_note_effects(
        b[0],
        n,
        NoteEffects {
            let_ring: true,
            dead: false,
        },
    )
    .unwrap();
    assert!(
        s.track(t).unwrap().bar(0).unwrap().voices[0].beats[0].notes[0]
            .effects
            .let_ring
    );
    s.remove_note(b[0], n).unwrap();
    assert!(
        s.track(t).unwrap().bar(0).unwrap().voices[0].beats[0]
            .notes
            .is_empty()
    );
    assert!(matches!(
        s.track(t).unwrap().bar(0).unwrap().voices[0].beats[0]
            .notes
            .first()
            .map(|n| n.kind),
        None | Some(NoteKind::Fretted(_))
    ));
}

/// Rebuilds the raw parts of a score (as a loader would produce them).
pub fn parts_of(s: &Score) -> ScoreParts {
    ScoreParts {
        master_bars: s.master_bars().to_vec(),
        tracks: s
            .tracks()
            .map(|t| TrackData {
                id: t.id(),
                name: t.name().into(),
                instrument: t.instrument().clone(),
                bars: t.bars().cloned().collect(),
            })
            .collect(),
        tempo_map: s.tempo_map().to_vec(),
    }
}
