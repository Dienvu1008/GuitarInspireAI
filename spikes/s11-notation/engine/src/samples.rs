//! The spike's inputs (README section 3): six hand-built scores and the reference
//! fixtures, all built through the validated aggregate operations.

use mc_score::fixtures::{Shape, feature_rich, reference, techniques};
use mc_score::{BeatRef, FrettedInstrument, InstrumentSpec, MasterBar, Score};
use mc_theory::fretted::{Fret, FretPosition, StringIndex, Tuning, pitch_at};
use mc_theory::{BaseValue, KeySignature, Mode, NoteValue, Tempo, TimeSignature, Tuplet};

fn new_score(name: &str, first: MasterBar, bars: usize) -> Score {
    let mut s = Score::new(
        name,
        InstrumentSpec::Fretted(FrettedInstrument::standard_guitar()),
        first,
        Tempo::from_bpm(100).unwrap(),
    )
    .unwrap();
    if bars > 1 {
        s.insert_master_bars(1, bars - 1, MasterBar::default()).unwrap();
    }
    s
}

fn bar(ts: (u8, u8), fifths: i8) -> MasterBar {
    MasterBar::new(
        TimeSignature::new(ts.0, ts.1).unwrap(),
        KeySignature::new(fifths, Mode::Major).unwrap(),
    )
}

fn v(exp: u8) -> NoteValue {
    NoteValue::plain(BaseValue::from_exponent(exp).unwrap())
}

fn pos(string: u8, fret: u8) -> FretPosition {
    FretPosition {
        string: StringIndex::new(string).unwrap(),
        fret: Fret(fret),
    }
}

/// The lowest-fret position of a sounding semitone on a standard guitar (frets 0..=15).
fn at(semitone: i32) -> FretPosition {
    let t = Tuning::standard_guitar();
    (1..=6u8)
        .filter_map(|s| {
            let open = pitch_at(&t, mc_theory::fretted::Capo(0), pos(s, 0)).unwrap();
            let fret = semitone - open;
            (0..=15).contains(&fret).then_some((fret, s))
        })
        .min()
        .map(|(fret, s)| pos(s, fret as u8))
        .unwrap_or_else(|| panic!("semitone {semitone} not playable"))
}

/// Appends a beat with these notes (none = a rest); `tie` ties every note.
fn put(s: &mut Score, bar: usize, voice: usize, value: NoteValue, notes: &[FretPosition], tie: bool) {
    let track = s.track_at(0).unwrap().id();
    let index = s.track(track).unwrap().bar(bar).unwrap().voices[voice].beats.len();
    let (beat, _) = s.insert_beat(track, bar, voice, index, value).unwrap();
    let r = BeatRef { track, bar, voice, beat };
    for &p in notes {
        let (id, _) = s.set_note(r, p).unwrap();
        if tie {
            s.set_tie(r, id, true).unwrap();
        }
    }
}

/// 1. A C major scale, two octaves up from the low E, in quarters (ledger lines).
pub fn scale() -> Score {
    let degrees = [40, 41, 43, 45, 47, 48, 50, 52, 53, 55, 57, 59, 60, 62, 64, 65, 67, 69, 71, 72];
    let mut s = new_score("Scale", bar((4, 4), 0), degrees.len() / 4);
    for (i, d) in degrees.iter().enumerate() {
        put(&mut s, i / 4, 0, v(2), &[at(*d)], false);
    }
    s
}

/// 2. Keys and accidentals: G, F, B-flat and E major, with chromatic notes, a repeat
/// of an altered note in the bar, a reset at the bar line, and a sharp tied into the
/// next bar followed by the same sharp again.
pub fn keys() -> Score {
    let mut s = new_score("Keys", bar((4, 4), 1), 6);
    for (b, f) in [(1, 1), (2, -1), (3, -2), (4, 4), (5, 4)] {
        let _ = s.set_master_bar(b, bar((4, 4), f));
    }
    // G major: F#4 (in key), F4 natural, F4 again (no sign), C#5.
    for d in [66, 65, 65, 73] {
        put(&mut s, 0, 0, v(2), &[at(d)], false);
    }
    // G major again: F4 (the natural sign again after the bar line), E4, a half G#4.
    put(&mut s, 1, 0, v(2), &[at(65)], false);
    put(&mut s, 1, 0, v(2), &[at(64)], false);
    put(&mut s, 1, 0, v(1), &[at(68)], false); // G#4
    // F major: Bb4 (key), B4 natural, Bb4 (flat sign again), E-flat 5.
    for d in [70, 71, 70, 75] {
        put(&mut s, 2, 0, v(2), &[at(d)], false);
    }
    // B-flat major: Eb4 (key), E4 natural, A-flat 4, A4 natural.
    for d in [63, 64, 68, 69] {
        put(&mut s, 3, 0, v(2), &[at(d)], false);
    }
    // E major: D#5 (key), D5 natural, then a half D#5 tied into the next bar.
    put(&mut s, 4, 0, v(2), &[at(75)], false);
    put(&mut s, 4, 0, v(2), &[at(74)], false);
    put(&mut s, 4, 0, v(1), &[at(75)], false);
    // Next bar: the tied D#5 (no sign), D5 (natural sign), D#5 (sharp sign: the natural
    // is in force), D5 (natural sign again).
    let d = at(75);
    put(&mut s, 5, 0, v(2), &[d], true);
    put(&mut s, 5, 0, v(2), &[at(74)], false);
    put(&mut s, 5, 0, v(2), &[at(75)], false);
    put(&mut s, 5, 0, v(2), &[at(74)], false);
    s
}

/// 3. Chords: the open C shape x32010, a second in a chord with an up stem, a second
/// with a down stem, and a chord with two accidentals.
pub fn chords() -> Score {
    let mut s = new_score("Chords", bar((4, 4), 0), 2);
    put(&mut s, 0, 0, v(1), &[pos(5, 3), pos(4, 2), pos(3, 0), pos(2, 1), pos(1, 0)], false);
    // G3 and A3: a second, low on the staff (stem up).
    put(&mut s, 0, 0, v(1), &[pos(4, 5), pos(3, 2)], false);
    // E5 and F5 written (E4 and F4 sounding): a second high on the staff (stem down).
    put(&mut s, 1, 0, v(1), &[pos(1, 0), pos(2, 6)], false);
    // F#4 and C#5 written (F#3, C#4 sounding): two sharps in one chord.
    put(&mut s, 1, 0, v(1), &[pos(4, 4), pos(3, 6)], false);
    s
}

/// 4. Rhythm: eighths and sixteenths in 4/4, then eighths in 6/8.
pub fn rhythm() -> Score {
    let mut s = new_score("Rhythm", bar((4, 4), 0), 2);
    let _ = s.set_master_bar(1, bar((6, 8), 0));
    // 4 eighths (2/4) and 6 sixteenths (3/8), then an eighth: exactly 4/4, with a
    // sixteenth-sixteenth-eighth group in the last beat.
    let line = [64, 62, 60, 59, 57, 59, 60, 62, 64, 65];
    for (i, d) in line.iter().enumerate() {
        put(&mut s, 0, 0, v(if i < 4 { 3 } else { 4 }), &[at(*d)], false);
    }
    put(&mut s, 0, 0, v(3), &[at(67)], false);
    for d in [52, 55, 59, 64, 59, 55] {
        put(&mut s, 1, 0, v(3), &[at(d)], false);
    }
    s
}

/// 5. A triplet and dotted rhythms.
pub fn tuplets() -> Score {
    let mut s = new_score("Tuplets", bar((4, 4), 0), 1);
    let trip = NoteValue::new(BaseValue::EIGHTH, 0, Some(Tuplet::TRIPLET)).unwrap();
    for d in [60, 62, 64] {
        put(&mut s, 0, 0, trip, &[at(d)], false);
    }
    put(&mut s, 0, 0, NoteValue::new(BaseValue::QUARTER, 1, None).unwrap(), &[at(65)], false);
    put(&mut s, 0, 0, v(3), &[at(67)], false);
    put(&mut s, 0, 0, NoteValue::new(BaseValue::EIGHTH, 1, None).unwrap(), &[at(69)], false);
    put(&mut s, 0, 0, v(4), &[at(71)], false);
    s
}

/// 6. Every rest value.
pub fn rests() -> Score {
    let mut s = new_score("Rests", bar((4, 4), 0), 2);
    put(&mut s, 0, 0, v(0), &[], false);
    for e in [1, 2, 3, 4, 5, 6] {
        put(&mut s, 1, 0, v(e), &[], false);
    }
    s
}

pub fn small() -> Score {
    reference(Shape {
        tracks: 1,
        bars: 8,
        beats: 8,
        notes_per_beat: 2,
    })
}

/// All inputs: name, score, hand-built (zero N3 violations required).
pub fn all() -> Vec<(&'static str, Score, bool)> {
    vec![
        ("scale", scale(), true),
        ("keys", keys(), true),
        ("chords", chords(), true),
        ("rhythm", rhythm(), true),
        ("tuplets", tuplets(), true),
        ("rests", rests(), true),
        ("feature_rich", feature_rich(), false),
        ("techniques", techniques(), false),
        ("small", small(), false),
    ]
}
