#![allow(dead_code)]

use mc_score::{
    BeatId, BeatRef, FrettedInstrument, InstrumentSpec, MasterBar, NoteId, Score, TrackId,
};
use mc_theory::fretted::{Fret, FretPosition, StringIndex};
use mc_theory::{BaseValue, NoteValue, Rational, Tempo};

pub fn q() -> NoteValue {
    NoteValue::plain(BaseValue::QUARTER)
}

pub fn eighth() -> NoteValue {
    NoteValue::plain(BaseValue::EIGHTH)
}

pub fn half() -> NoteValue {
    NoteValue::plain(BaseValue::HALF)
}

pub fn r(n: i64, d: i64) -> Rational {
    Rational::new(n, d).unwrap()
}

pub fn pos(string: u8, fret: u8) -> FretPosition {
    FretPosition {
        string: StringIndex::new(string).unwrap(),
        fret: Fret(fret),
    }
}

pub fn guitar() -> InstrumentSpec {
    InstrumentSpec::Fretted(FrettedInstrument::standard_guitar())
}

/// Score with `bars` 4/4 master bars and `tracks` standard-guitar tracks, all empty.
pub fn score(bars: usize, tracks: usize) -> Score {
    let mut s = Score::new(
        "Track 1",
        guitar(),
        MasterBar::default(),
        Tempo::from_bpm(120).unwrap(),
    )
    .unwrap();
    if bars > 1 {
        s.insert_master_bars(1, bars - 1, MasterBar::default())
            .unwrap();
    }
    for i in 1..tracks {
        s.add_track(&format!("Track {}", i + 1), guitar()).unwrap();
    }
    s
}

pub fn track(s: &Score, index: usize) -> TrackId {
    s.track_at(index).unwrap().id()
}

/// Appends beats to a voice and returns their refs.
pub fn fill(
    s: &mut Score,
    t: TrackId,
    bar: usize,
    voice: usize,
    values: &[NoteValue],
) -> Vec<BeatRef> {
    values
        .iter()
        .map(|v| {
            let len = s.track(t).unwrap().bar(bar).unwrap().voices[voice]
                .beats
                .len();
            let (id, _) = s.insert_beat(t, bar, voice, len, *v).unwrap();
            beat(t, bar, voice, id)
        })
        .collect()
}

pub fn beat(track: TrackId, bar: usize, voice: usize, id: BeatId) -> BeatRef {
    BeatRef {
        track,
        bar,
        voice,
        beat: id,
    }
}

pub fn note(s: &mut Score, r: BeatRef, string: u8, fret: u8) -> NoteId {
    s.set_note(r, pos(string, fret)).unwrap().0
}
