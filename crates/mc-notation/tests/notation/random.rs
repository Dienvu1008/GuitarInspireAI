//! Seeded random scores for L2 to L4 and L9, built through the validated aggregate
//! operations; an operation the score rejects is skipped. They vary key and time
//! signature per bar, tuning capo, chords with seconds, two voices, ties, triplets,
//! dotted values and rests.

use mc_score::{BeatRef, FrettedInstrument, InstrumentSpec, MasterBar, Score};
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex};
use mc_theory::{BaseValue, KeySignature, Mode, NoteValue, Tempo, TimeSignature, Tuplet};

/// xorshift64*.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

pub fn random_score(seed: u64) -> Score {
    let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    let sigs = [(4, 4), (3, 4), (6, 8), (2, 4), (12, 8)];
    let bars = 1 + rng.below(6) as usize;
    let master = |rng: &mut Rng| {
        let (n, d) = sigs[rng.below(sigs.len() as u64) as usize];
        MasterBar::new(
            TimeSignature::new(n, d).unwrap(),
            KeySignature::new(rng.below(15) as i8 - 7, Mode::Major).unwrap(),
        )
    };
    let mut inst = FrettedInstrument::standard_guitar();
    inst.capo = Capo(if rng.below(3) == 0 {
        rng.below(5) as u8
    } else {
        0
    });
    let first = master(&mut rng);
    let mut s = Score::new(
        "Random",
        InstrumentSpec::Fretted(inst),
        first,
        Tempo::from_bpm(100).unwrap(),
    )
    .unwrap();
    if bars > 1 {
        s.insert_master_bars(1, bars - 1, MasterBar::default())
            .unwrap();
    }
    for b in 1..bars {
        if rng.below(3) == 0 {
            let m = master(&mut rng);
            let _ = s.set_master_bar(b, m);
        }
    }
    let t = s.track_at(0).unwrap().id();
    let trip = NoteValue::new(BaseValue::EIGHTH, 0, Some(Tuplet::TRIPLET)).unwrap();
    for bar in 0..bars {
        let voices = if rng.below(4) == 0 { 2 } else { 1 };
        for voice in 0..voices {
            if voice == 1 && s.add_voice(t, bar).is_err() {
                continue;
            }
            let count = rng.below(8);
            let mut k = 0;
            while k < count {
                let value = match rng.below(9) {
                    0 => NoteValue::plain(BaseValue::HALF),
                    1 => NoteValue::new(BaseValue::QUARTER, 1, None).unwrap(),
                    2 | 3 => NoteValue::plain(BaseValue::QUARTER),
                    4 | 5 => NoteValue::plain(BaseValue::EIGHTH),
                    6 => NoteValue::plain(BaseValue::from_exponent(4).unwrap()),
                    7 => trip,
                    _ => NoteValue::new(BaseValue::EIGHTH, 1, None).unwrap(),
                };
                let reps = if value == trip { 3 } else { 1 };
                for _ in 0..reps {
                    let index = s.track(t).unwrap().bar(bar).unwrap().voices[voice]
                        .beats
                        .len();
                    let Ok((beat, _)) = s.insert_beat(t, bar, voice, index, value) else {
                        break;
                    };
                    let r = BeatRef {
                        track: t,
                        bar,
                        voice,
                        beat,
                    };
                    // 0 to 3 notes; a second on neighbouring strings is common.
                    let notes = rng.below(4);
                    let base = 1 + rng.below(5) as u8;
                    for j in 0..notes as u8 {
                        let string = (base + j).min(6);
                        let fret = rng.below(13) as u8;
                        let p = FretPosition {
                            string: StringIndex::new(string).unwrap(),
                            fret: Fret(fret),
                        };
                        if let Ok((id, _)) = s.set_note(r, p)
                            && rng.below(5) == 0
                        {
                            let _ = s.set_tie(r, id, true);
                        }
                    }
                }
                k += reps;
            }
        }
    }
    s
}
