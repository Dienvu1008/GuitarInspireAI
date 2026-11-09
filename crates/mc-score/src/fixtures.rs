//! Deterministic reference scores (score-model-and-undo.md section 4) for tests,
//! benchmarks and the CLI. Built through `Score::from_parts`, so they are validated.

use crate::model::{
    BarContent, Beat, FrettedInstrument, InstrumentSpec, MasterBar, Note, NoteEffects, NoteKind,
    TempoChange, TrackData, Voice,
};
use crate::{BeatId, BeatRef, NoteId, Score, ScoreParts, TrackId};
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex};
use mc_theory::{BaseValue, KeySignature, Mode, NoteValue, Rational, Tempo, TimeSignature, Tuplet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub tracks: usize,
    pub bars: usize,
    /// Beats per bar; every bar of 4/4 is filled exactly (8 eighths or 16 sixteenths).
    pub beats: usize,
    pub notes_per_beat: usize,
}

/// 1 track x 50 bars x 8 beats x 2 notes = 800 notes.
pub const SMALL: Shape = Shape {
    tracks: 1,
    bars: 50,
    beats: 8,
    notes_per_beat: 2,
};
/// 4 tracks x 150 bars x 8 beats x 2 notes = 9 600 notes.
pub const TYPICAL: Shape = Shape {
    tracks: 4,
    bars: 150,
    beats: 8,
    notes_per_beat: 2,
};
/// 20 tracks x 300 bars x 16 beats x 3 notes = 288 000 notes.
pub const LARGE: Shape = Shape {
    tracks: 20,
    bars: 300,
    beats: 16,
    notes_per_beat: 3,
};

pub fn reference(shape: Shape) -> Score {
    let value = match shape.beats {
        8 => NoteValue::plain(BaseValue::EIGHTH),
        16 => NoteValue::plain(BaseValue::SIXTEENTH),
        _ => NoteValue::plain(BaseValue::QUARTER),
    };
    let mut next = 0u64;
    let mut id = || {
        next += 1;
        next
    };
    let tracks = (0..shape.tracks)
        .map(|t| {
            let track_id = TrackId::from_raw(id());
            let bars = (0..shape.bars)
                .map(|b| {
                    let beats = (0..shape.beats)
                        .map(|k| {
                            let first = (b + k + t) % 6;
                            let notes = (0..shape.notes_per_beat.min(6))
                                .map(|n| {
                                    let string = ((first + n) % 6) as u8 + 1;
                                    let fret = ((b * 7 + k * 3 + n + t) % 13) as u8;
                                    Note {
                                        id: NoteId::from_raw(id()),
                                        kind: NoteKind::Fretted(FretPosition {
                                            string: StringIndex::new(string).expect("1..=6"),
                                            fret: Fret(fret),
                                        }),
                                        tie: false,
                                        effects: NoteEffects {
                                            let_ring: (b + k) % 17 == 0,
                                            ..NoteEffects::default()
                                        },
                                    }
                                })
                                .collect();
                            Beat {
                                id: BeatId::from_raw(id()),
                                value,
                                notes,
                                stroke: None,
                            }
                        })
                        .collect();
                    BarContent {
                        voices: vec![Voice { beats }],
                    }
                })
                .collect();
            TrackData {
                id: track_id,
                name: format!("Track {}", t + 1),
                instrument: InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
                bars,
            }
        })
        .collect();
    let mut master_bars = vec![MasterBar::default(); shape.bars];
    if shape.bars >= 8 {
        master_bars[0].marker = Some("Intro".into());
        master_bars[4].repeat_start = true;
        master_bars[7].repeat_end = Some(2);
    }
    let parts = ScoreParts {
        master_bars,
        tracks,
        tempo_map: vec![TempoChange {
            bar: 0,
            offset: Rational::ZERO,
            tempo: Tempo::from_bpm(120).expect("positive"),
        }],
    };
    Score::from_parts(parts).expect("reference scores are valid")
}

/// A score that exercises every v1.0 field: pickup, repeats, alternate endings, marker,
/// key and time changes, tempo changes, tuplets, dots, two voices, ties, effects,
/// a second track with a drop-D tuning and a capo. Built with aggregate operations,
/// so ids follow allocation order.
pub fn feature_rich() -> Score {
    let mut s = crate::Score::new(
        "Lead",
        InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
        MasterBar {
            pickup: true,
            ..MasterBar::default()
        },
        Tempo::from_bpm(96).unwrap(),
    )
    .unwrap();
    s.insert_master_bars(1, 4, MasterBar::default()).unwrap();
    s.set_master_bar(
        1,
        MasterBar {
            repeat_start: true,
            marker: Some("Verse".into()),
            ..MasterBar::default()
        },
    )
    .unwrap();
    s.set_master_bar(
        2,
        MasterBar {
            alternate_endings: vec![1],
            repeat_end: Some(2),
            ..MasterBar::default()
        },
    )
    .unwrap();
    s.set_master_bar(
        3,
        MasterBar {
            alternate_endings: vec![2],
            ..MasterBar::default()
        },
    )
    .unwrap();
    s.set_master_bar(
        4,
        MasterBar::new(
            TimeSignature::new(6, 8).unwrap(),
            KeySignature::new(-3, Mode::Minor).unwrap(),
        ),
    )
    .unwrap();
    s.set_tempo(
        4,
        Rational::new(3, 8).unwrap(),
        Tempo::new(Rational::new(241, 2).unwrap()).unwrap(),
    )
    .unwrap();

    let lead = s.track_at(0).unwrap().id();
    let drop_d = mc_theory::fretted::Tuning::new(
        ["E4", "B3", "G3", "D3", "A2", "D2"]
            .map(|p| p.parse().unwrap())
            .to_vec(),
    )
    .unwrap();
    let (rhythm, _) = s
        .add_track(
            "Rhythm",
            InstrumentSpec::Fretted(FrettedInstrument {
                tuning: drop_d,
                fret_count: 22,
                capo: Capo(2),
            }),
        )
        .unwrap();

    let q = NoteValue::plain(BaseValue::QUARTER);
    let dotted = NoteValue::new(BaseValue::QUARTER, 1, None).unwrap();
    let trip = NoteValue::new(BaseValue::EIGHTH, 0, Some(Tuplet::TRIPLET)).unwrap();
    let pos = |string: u8, fret: u8| FretPosition {
        string: StringIndex::new(string).unwrap(),
        fret: Fret(fret),
    };

    // Pickup: one quarter.
    let (b, _) = s.insert_beat(lead, 0, 0, 0, q).unwrap();
    let r0 = BeatRef {
        track: lead,
        bar: 0,
        voice: 0,
        beat: b,
    };
    s.set_note(r0, pos(1, 3)).unwrap();
    // Bar 1: dotted quarter, eighth triplet x3, rest, tie from the pickup.
    let (b1, _) = s.insert_beat(lead, 1, 0, 0, dotted).unwrap();
    let r1 = BeatRef {
        track: lead,
        bar: 1,
        voice: 0,
        beat: b1,
    };
    let (tied, _) = s.set_note(r1, pos(1, 3)).unwrap();
    s.set_tie(r1, tied, true).unwrap();
    for i in 0..3 {
        let (bt, _) = s.insert_beat(lead, 1, 0, 1 + i, trip).unwrap();
        let (n, _) = s
            .set_note(
                BeatRef {
                    track: lead,
                    bar: 1,
                    voice: 0,
                    beat: bt,
                },
                pos(2, 5 + i as u8),
            )
            .unwrap();
        if i == 2 {
            s.set_note_effects(
                BeatRef {
                    track: lead,
                    bar: 1,
                    voice: 0,
                    beat: bt,
                },
                n,
                NoteEffects {
                    let_ring: true,
                    ..NoteEffects::default()
                },
            )
            .unwrap();
        }
    }
    s.insert_beat(lead, 1, 0, 4, q).unwrap();
    // Second voice in bar 1 on the same string as voice 0 (allowed, D-03 later).
    let (v1, _) = s.add_voice(lead, 1).unwrap();
    let (bv, _) = s
        .insert_beat(lead, 1, v1, 0, NoteValue::plain(BaseValue::WHOLE))
        .unwrap();
    let (dead, _) = s
        .set_note(
            BeatRef {
                track: lead,
                bar: 1,
                voice: v1,
                beat: bv,
            },
            pos(6, 0),
        )
        .unwrap();
    s.set_note_effects(
        BeatRef {
            track: lead,
            bar: 1,
            voice: v1,
            beat: bv,
        },
        dead,
        NoteEffects {
            dead: true,
            ..NoteEffects::default()
        },
    )
    .unwrap();
    // Rhythm track: a chord in bar 2 and an overfull bar 3 (D-01 is allowed in files).
    let (rb, _) = s
        .insert_beat(rhythm, 2, 0, 0, NoteValue::plain(BaseValue::WHOLE))
        .unwrap();
    for (st, fr) in [(6, 0), (5, 2), (4, 2), (3, 2)] {
        s.set_note(
            BeatRef {
                track: rhythm,
                bar: 2,
                voice: 0,
                beat: rb,
            },
            pos(st, fr),
        )
        .unwrap();
    }
    for i in 0..5 {
        s.insert_beat(rhythm, 3, 0, i, q).unwrap();
    }
    assert!(s.validate().is_empty());
    s
}

/// A score that uses every playing technique of ADR-035: accent, palm mute (whole and
/// partial chords), string-percussive, stroke directions (also on a rest), and the
/// precedence combinations (dead with let ring, palm mute with let ring, string-
/// percussive with dead). One standard-tuned track, two bars of 4/4, open C shape
/// (`x32010`). Built with aggregate operations, so ids follow allocation order.
pub fn techniques() -> Score {
    use crate::Stroke;
    let mut s = crate::Score::new(
        "Techniques",
        InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
        MasterBar::default(),
        Tempo::from_bpm(100).unwrap(),
    )
    .unwrap();
    s.insert_master_bars(1, 1, MasterBar::default()).unwrap();
    let t = s.track_at(0).unwrap().id();
    let q = NoteValue::plain(BaseValue::QUARTER);
    let e = NoteValue::plain(BaseValue::EIGHTH);
    let pos = |string: u8, fret: u8| FretPosition {
        string: StringIndex::new(string).unwrap(),
        fret: Fret(fret),
    };
    let chord = [(1, 0), (2, 1), (3, 0), (4, 2), (5, 3)];
    let fx = |accent, palm_mute, string_percussive, dead, let_ring| NoteEffects {
        accent,
        palm_mute,
        string_percussive,
        dead,
        let_ring,
    };
    // One beat: notes with their effects, an optional stroke.
    let put = |s: &mut Score,
               bar: usize,
               index: usize,
               value: NoteValue,
               notes: &[((u8, u8), NoteEffects)],
               stroke: Option<Stroke>| {
        let (b, _) = s.insert_beat(t, bar, 0, index, value).unwrap();
        let r = BeatRef {
            track: t,
            bar,
            voice: 0,
            beat: b,
        };
        for &((string, fret), effects) in notes {
            let (n, _) = s.set_note(r, pos(string, fret)).unwrap();
            s.set_note_effects(r, n, effects).unwrap();
        }
        s.set_beat_stroke(r, stroke).unwrap();
    };
    let all = |e: NoteEffects| chord.map(|p| (p, e));
    // Bar 0: accented down-stroke chord; two palm-muted eighths (the second accented);
    // accented string-percussive up-stroke chord; dead + let ring (dead wins).
    put(
        &mut s,
        0,
        0,
        q,
        &all(fx(true, false, false, false, false)),
        Some(Stroke::Down),
    );
    put(
        &mut s,
        0,
        1,
        e,
        &[((5, 3), fx(false, true, false, false, false))],
        None,
    );
    put(
        &mut s,
        0,
        2,
        e,
        &[((5, 3), fx(true, true, false, false, false))],
        None,
    );
    put(
        &mut s,
        0,
        3,
        q,
        &all(fx(true, false, true, false, false)),
        Some(Stroke::Up),
    );
    put(
        &mut s,
        0,
        4,
        q,
        &[((1, 0), fx(false, false, false, true, true))],
        None,
    );
    // Bar 1: palm mute + let ring (palm mute wins); a down-stroke on a rest; a partly
    // palm-muted chord; string-percussive + dead (string-percussive wins); let ring.
    put(
        &mut s,
        1,
        0,
        q,
        &[((5, 3), fx(false, true, false, false, true))],
        None,
    );
    put(&mut s, 1, 1, e, &[], Some(Stroke::Down));
    put(
        &mut s,
        1,
        2,
        e,
        &[
            ((4, 2), fx(false, true, false, false, false)),
            ((5, 3), fx(false, false, false, false, false)),
        ],
        Some(Stroke::Down),
    );
    put(
        &mut s,
        1,
        3,
        q,
        &[((2, 1), fx(false, false, true, true, false))],
        None,
    );
    put(
        &mut s,
        1,
        4,
        q,
        &[((3, 0), fx(false, false, false, false, true))],
        None,
    );
    assert!(s.validate().is_empty());
    s
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_shapes_have_the_documented_sizes() {
        let s = reference(SMALL);
        assert_eq!(s.note_count(), 800);
        assert!(s.diagnostics(0..s.master_bar_count()).unwrap().is_empty());
        assert_eq!(reference(TYPICAL).note_count(), 9_600);
    }
}
