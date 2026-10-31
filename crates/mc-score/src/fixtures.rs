//! Deterministic reference scores (score-model-and-undo.md section 4) for tests,
//! benchmarks and the CLI. Built through `Score::from_parts`, so they are validated.

use crate::model::{
    BarContent, Beat, FrettedInstrument, InstrumentSpec, MasterBar, Note, NoteEffects, NoteKind,
    TempoChange, TrackData, Voice,
};
use crate::{BeatId, NoteId, Score, ScoreParts, TrackId};
use mc_theory::fretted::{Fret, FretPosition, StringIndex};
use mc_theory::{BaseValue, NoteValue, Rational, Tempo};

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
                                            dead: false,
                                        },
                                    }
                                })
                                .collect();
                            Beat {
                                id: BeatId::from_raw(id()),
                                value,
                                notes,
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
