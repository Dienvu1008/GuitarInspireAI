//! Chord qualities as interval structures (Phase 2 vocabulary). This module knows
//! nothing about symbols or display; see `chord_symbol` for notation.

use crate::{NoteName, PitchClass};

/// Phase 2 vocabulary. Altered and extended chords (7b9, 7#11, maj9, m9, 11, 13 ...)
/// are deferred until a concrete requirement exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChordQuality {
    Major,
    Minor,
    Diminished,
    Augmented,
    /// Dominant seventh.
    Dominant7,
    Major7,
    Minor7,
    /// Fully diminished seventh.
    Diminished7,
    /// Half-diminished seventh.
    HalfDiminished7,
    Sus2,
    Sus4,
    Sixth,
    Minor6,
    /// Major triad plus ninth, no seventh.
    Add9,
    /// Dominant seventh plus ninth.
    Dominant9,
}

/// One chord tone: scale-degree number (1, 3, 5, 7, 9 ...) and semitones above the root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChordInterval {
    pub degree: u8,
    pub semitones: u8,
    /// True for a perfect fifth that may be omitted when the configuration allows it.
    /// Fifths that define the quality (dim, aug, half-diminished) are never omissible,
    /// nor is the fifth of a sus chord (see `SUS2`).
    pub omissible_fifth: bool,
}

const fn iv(degree: u8, semitones: u8) -> ChordInterval {
    ChordInterval {
        degree,
        semitones,
        omissible_fifth: false,
    }
}

const P5: ChordInterval = ChordInterval {
    degree: 5,
    semitones: 7,
    omissible_fifth: true,
};
const REQUIRED_P5: ChordInterval = iv(5, 7);
const R: ChordInterval = iv(1, 0);
const MAJ3: ChordInterval = iv(3, 4);
const MIN3: ChordInterval = iv(3, 3);
const DIM5: ChordInterval = iv(5, 6);
const AUG5: ChordInterval = iv(5, 8);
const MIN7: ChordInterval = iv(7, 10);
const MAJ7: ChordInterval = iv(7, 11);
const DIM7: ChordInterval = iv(7, 9);
const MAJ2: ChordInterval = iv(2, 2);
const P4: ChordInterval = iv(4, 5);
const MAJ6: ChordInterval = iv(6, 9);
const MAJ9: ChordInterval = iv(9, 14);

const MAJOR: &[ChordInterval] = &[R, MAJ3, P5];
const MINOR: &[ChordInterval] = &[R, MIN3, P5];
const DIMINISHED: &[ChordInterval] = &[R, MIN3, DIM5];
const AUGMENTED: &[ChordInterval] = &[R, MAJ3, AUG5];
const DOMINANT7: &[ChordInterval] = &[R, MAJ3, P5, MIN7];
const MAJOR7: &[ChordInterval] = &[R, MAJ3, P5, MAJ7];
const MINOR7: &[ChordInterval] = &[R, MIN3, P5, MIN7];
const DIMINISHED7: &[ChordInterval] = &[R, MIN3, DIM5, DIM7];
const HALF_DIMINISHED7: &[ChordInterval] = &[R, MIN3, DIM5, MIN7];
// In sus chords the fifth is required: without it Gsus4 (G C) is the inverted C power
// chord and Csus2 (C D) a bare second, so the quality would no longer be identifiable.
const SUS2: &[ChordInterval] = &[R, MAJ2, REQUIRED_P5];
const SUS4: &[ChordInterval] = &[R, P4, REQUIRED_P5];
const SIXTH: &[ChordInterval] = &[R, MAJ3, P5, MAJ6];
const MINOR6: &[ChordInterval] = &[R, MIN3, P5, MAJ6];
const ADD9: &[ChordInterval] = &[R, MAJ3, P5, MAJ9];
const DOMINANT9: &[ChordInterval] = &[R, MAJ3, P5, MIN7, MAJ9];

impl ChordQuality {
    pub const ALL: [ChordQuality; 15] = [
        ChordQuality::Major,
        ChordQuality::Minor,
        ChordQuality::Diminished,
        ChordQuality::Augmented,
        ChordQuality::Dominant7,
        ChordQuality::Major7,
        ChordQuality::Minor7,
        ChordQuality::Diminished7,
        ChordQuality::HalfDiminished7,
        ChordQuality::Sus2,
        ChordQuality::Sus4,
        ChordQuality::Sixth,
        ChordQuality::Minor6,
        ChordQuality::Add9,
        ChordQuality::Dominant9,
    ];

    /// Chord tones from the root upward.
    pub fn intervals(self) -> &'static [ChordInterval] {
        use ChordQuality::*;
        match self {
            Major => MAJOR,
            Minor => MINOR,
            Diminished => DIMINISHED,
            Augmented => AUGMENTED,
            Dominant7 => DOMINANT7,
            Major7 => MAJOR7,
            Minor7 => MINOR7,
            Diminished7 => DIMINISHED7,
            HalfDiminished7 => HALF_DIMINISHED7,
            Sus2 => SUS2,
            Sus4 => SUS4,
            Sixth => SIXTH,
            Minor6 => MINOR6,
            Add9 => ADD9,
            Dominant9 => DOMINANT9,
        }
    }

    /// Pitch classes of the chord on `root`.
    pub fn pitch_classes(self, root: PitchClass) -> Vec<PitchClass> {
        self.intervals()
            .iter()
            .map(|i| PitchClass::from_semitone(root.value() as i32 + i.semitones as i32))
            .collect()
    }

    /// Size class used for ranking: 3 for triads and sus chords, 4 for four-note chords, 5 for the ninth.
    pub fn tone_count(self) -> usize {
        self.intervals().len()
    }
}

/// A chord: root, quality and an optional bass note different from the root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub root: NoteName,
    pub quality: ChordQuality,
    pub bass: Option<NoteName>,
}

impl Chord {
    pub const fn new(root: NoteName, quality: ChordQuality) -> Self {
        Chord {
            root,
            quality,
            bass: None,
        }
    }

    /// Chord tones spelled from the root (C7 -> C E G Bb; Cdim7 -> C Eb Gb Bbb).
    /// `None` if a tone would need more than a double accidental.
    pub fn spelled_tones(&self) -> Option<Vec<NoteName>> {
        self.quality
            .intervals()
            .iter()
            .map(|i| self.root.up((i.degree - 1) % 7, i.semitones % 12))
            .collect()
    }

    pub fn pitch_classes(&self) -> Vec<PitchClass> {
        self.quality.pitch_classes(self.root.pitch_class())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tones(root: &str, q: ChordQuality) -> String {
        let c = Chord::new(root.parse().unwrap(), q);
        c.spelled_tones()
            .unwrap()
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn interval_structures() {
        assert_eq!(tones("C", ChordQuality::Dominant7), "C E G Bb");
        assert_eq!(tones("C", ChordQuality::Diminished7), "C Eb Gb Bbb");
        assert_eq!(tones("C", ChordQuality::HalfDiminished7), "C Eb Gb Bb");
        assert_eq!(tones("C", ChordQuality::Add9), "C E G D");
        assert_eq!(tones("C", ChordQuality::Dominant9), "C E G Bb D");
        assert_eq!(tones("A", ChordQuality::Sus4), "A D E");
        assert_eq!(tones("F#", ChordQuality::Major), "F# A# C#");
    }

    #[test]
    fn only_perfect_fifths_outside_sus_chords_are_omissible() {
        for q in ChordQuality::ALL {
            let sus = matches!(q, ChordQuality::Sus2 | ChordQuality::Sus4);
            for i in q.intervals() {
                assert_eq!(
                    i.omissible_fifth,
                    i.degree == 5 && i.semitones == 7 && !sus,
                    "{q:?}"
                );
            }
        }
    }
}
