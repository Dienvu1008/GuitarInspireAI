//! Scale positions on the fretboard.

use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning, pitch_at};
use mc_theory::{PitchClass, ScaleFormula};
use std::ops::RangeInclusive;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScaleNote {
    pub position: FretPosition,
    pub pitch_class: PitchClass,
    /// 1-based scale degree.
    pub degree: u8,
}

/// Every scale note in the fret window, string 1 first, then by fret.
pub fn scale_positions(
    tuning: &Tuning,
    capo: Capo,
    root: PitchClass,
    scale: ScaleFormula,
    frets: RangeInclusive<u8>,
) -> Vec<ScaleNote> {
    let mut out = Vec::new();
    for s in 1..=tuning.string_count() {
        let string = StringIndex::new(s).expect("1-based");
        for f in frets.clone() {
            let position = FretPosition {
                string,
                fret: Fret(f),
            };
            let pc = PitchClass::from_semitone(pitch_at(tuning, capo, position).expect("exists"));
            if let Some(degree) = scale.degree_of(root, pc) {
                out.push(ScaleNote {
                    position,
                    pitch_class: pc,
                    degree,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minor_pentatonic_box_one() {
        let t = Tuning::standard_guitar();
        let notes = scale_positions(
            &t,
            Capo(0),
            PitchClass::from_semitone(9),
            ScaleFormula::MinorPentatonic,
            5..=8,
        );
        // Two notes per string in the classic 5th-position box.
        let by_string: Vec<(u8, u8, u8)> = notes
            .iter()
            .map(|n| (n.position.string.get(), n.position.fret.0, n.degree))
            .collect();
        assert_eq!(
            by_string,
            [
                (1, 5, 1),
                (1, 8, 2),
                (2, 5, 4),
                (2, 8, 5),
                (3, 5, 2),
                (3, 7, 3),
                (4, 5, 5),
                (4, 7, 1),
                (5, 5, 3),
                (5, 7, 4),
                (6, 5, 1),
                (6, 8, 2),
            ]
        );
    }
}
