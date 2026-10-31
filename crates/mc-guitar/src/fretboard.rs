//! Fretboard position queries.

use mc_theory::PitchClass;
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning, pitch_at};
use std::ops::RangeInclusive;

/// Every position in the fret window that sounds `pc`, string 1 first, then by fret.
pub fn positions_of(
    tuning: &Tuning,
    capo: Capo,
    pc: PitchClass,
    frets: RangeInclusive<u8>,
) -> Vec<FretPosition> {
    let mut out = Vec::new();
    for s in 1..=tuning.string_count() {
        let string = StringIndex::new(s).expect("1-based");
        for f in frets.clone() {
            let pos = FretPosition {
                string,
                fret: Fret(f),
            };
            if pitch_at(tuning, capo, pos).map(PitchClass::from_semitone) == Some(pc) {
                out.push(pos);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notes_in_the_first_position() {
        let t = Tuning::standard_guitar();
        let a = PitchClass::from_semitone(9);
        let found: Vec<(u8, u8)> = positions_of(&t, Capo(0), a, 0..=5)
            .iter()
            .map(|p| (p.string.get(), p.fret.0))
            .collect();
        assert_eq!(found, [(1, 5), (3, 2), (5, 0), (6, 5)]);
        // A capo moves the sounding pitch up, so the same note sits lower.
        let with_capo: Vec<(u8, u8)> = positions_of(&t, Capo(2), a, 0..=5)
            .iter()
            .map(|p| (p.string.get(), p.fret.0))
            .collect();
        assert_eq!(with_capo, [(1, 3), (3, 0), (4, 5), (6, 3)]);
    }
}
