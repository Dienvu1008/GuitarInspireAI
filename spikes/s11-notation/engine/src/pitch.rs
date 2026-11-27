//! README section 4.1 and 4.2: spelling a sounding semitone in a key, the instrument
//! transposition (applied in one place), staff positions, and the accidental state of
//! a staff within a bar.

use mc_theory::{Alter, KeySignature, Letter, SpelledPitch};
use std::collections::BTreeMap;

pub const LETTERS: [Letter; 7] = [
    Letter::C,
    Letter::D,
    Letter::E,
    Letter::F,
    Letter::G,
    Letter::A,
    Letter::B,
];

pub fn letter_index(l: Letter) -> i32 {
    LETTERS.iter().position(|x| *x == l).unwrap() as i32
}

/// The alteration the key signature gives each letter (-1, 0 or +1).
pub fn key_alter(key: KeySignature, letter: Letter) -> i8 {
    const SHARPS: [Letter; 7] = [
        Letter::F,
        Letter::C,
        Letter::G,
        Letter::D,
        Letter::A,
        Letter::E,
        Letter::B,
    ];
    let f = key.fifths();
    if f > 0 {
        if SHARPS[..f as usize].contains(&letter) { 1 } else { 0 }
    } else if f < 0 {
        let flats: Vec<Letter> = SHARPS.iter().rev().copied().collect();
        if flats[..(-f) as usize].contains(&letter) { -1 } else { 0 }
    } else {
        0
    }
}

fn pitch(letter: Letter, alter: i8, semitone: i32) -> SpelledPitch {
    let base = letter.natural_semitone() + alter as i32;
    let octave = (semitone - base).div_euclid(12) - 1;
    debug_assert_eq!((octave + 1) * 12 + base, semitone);
    SpelledPitch::new(letter, Alter::new(alter).unwrap(), octave as i8)
}

/// Section 4.2 rule 1 and 2: a degree of the key's scale is spelled as the key spells it
/// (E-sharp in F-sharp major); a chromatic note gets a sharp in sharp or neutral keys
/// and a flat in flat keys.
pub fn spell(semitone: i32, key: KeySignature) -> SpelledPitch {
    let pc = semitone.rem_euclid(12);
    for &l in &LETTERS {
        let a = key_alter(key, l);
        if (l.natural_semitone() + a as i32).rem_euclid(12) == pc {
            return pitch(l, a, semitone);
        }
    }
    let chromatic = if key.fifths() < 0 {
        SpelledPitch::from_semitone_flat(semitone)
    } else {
        SpelledPitch::from_semitone_sharp(semitone)
    };
    pitch(chromatic.letter(), chromatic.alter().semitones(), semitone)
}

/// An octave transposition of a staff (section 4.1): guitar is written one octave above
/// its sounding pitch. The only place where written pitch is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Transposition {
    pub octaves: i8,
}

impl Transposition {
    pub const GUITAR: Transposition = Transposition { octaves: 1 };
}

pub fn written(sounding: SpelledPitch, t: Transposition) -> SpelledPitch {
    SpelledPitch::new(sounding.letter(), sounding.alter(), sounding.octave() + t.octaves)
}

/// Diatonic step number: 7 per octave.
pub fn step(p: SpelledPitch) -> i32 {
    p.octave() as i32 * 7 + letter_index(p.letter())
}

/// Staff position in half staff spaces above the bottom line of a treble staff (E4 is
/// 0, the top line F5 is 8). The 8 under the clef changes no position: the written
/// pitch already carries the octave.
pub fn treble_position(written: SpelledPitch) -> i32 {
    step(written) - (4 * 7 + 2)
}

/// Accidental state of one staff in one bar (section 4.2).
#[derive(Clone, Debug)]
pub struct AccidentalState {
    key: KeySignature,
    /// Alteration in force per written position (letter and octave), once a note set it.
    set: BTreeMap<i32, i8>,
}

impl AccidentalState {
    /// A new bar: the state is the key signature again.
    pub fn new_bar(key: KeySignature) -> Self {
        AccidentalState {
            key,
            set: BTreeMap::new(),
        }
    }

    fn in_force(&self, p: SpelledPitch) -> i8 {
        self.set
            .get(&step(p))
            .copied()
            .unwrap_or_else(|| key_alter(self.key, p.letter()))
    }

    /// The accidental to show for a note (`None` for none), updating the state. A note
    /// tied from the previous beat shows none; across a bar line it does not set the
    /// new bar's state (`tied_across_bar`).
    pub fn note(&mut self, p: SpelledPitch, tied: bool, tied_across_bar: bool) -> Option<i8> {
        let alter = p.alter().semitones();
        if tied {
            if !tied_across_bar {
                self.set.insert(step(p), alter);
            }
            return None;
        }
        let shown = (alter != self.in_force(p)).then_some(alter);
        self.set.insert(step(p), alter);
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_theory::Mode;

    fn key(f: i8) -> KeySignature {
        KeySignature::new(f, Mode::Major).unwrap()
    }

    fn name(p: SpelledPitch) -> String {
        let a = match p.alter().semitones() {
            1 => "#",
            -1 => "b",
            2 => "x",
            -2 => "bb",
            _ => "",
        };
        format!("{:?}{a}{}", p.letter(), p.octave())
    }

    #[test]
    fn spelling_follows_the_key() {
        // F-sharp major spells semitone 65 (F4 sounding) as E-sharp 4.
        assert_eq!(name(spell(65, key(6))), "E#4");
        // G-flat major spells semitone 59 as C-flat 4 (B3 sounding).
        assert_eq!(name(spell(59, key(-6))), "Cb4");
        // Chromatic notes: sharp in G major, flat in F major.
        assert_eq!(name(spell(63, key(1))), "D#4");
        assert_eq!(name(spell(63, key(-1))), "Eb4");
        assert_eq!(name(spell(70, key(-1))), "Bb4");
    }

    #[test]
    fn guitar_is_written_an_octave_up() {
        let low_e = spell(40, key(0)); // E2 sounding
        let w = written(low_e, Transposition::GUITAR);
        assert_eq!(name(w), "E3");
        // E3 is three ledger lines below the treble staff: position -7.
        assert_eq!(treble_position(w), -7);
        // Open high E (E4 sounding) is written E5, the top space: position 7.
        assert_eq!(treble_position(written(spell(64, key(0)), Transposition::GUITAR)), 7);
    }

    #[test]
    fn accidentals_follow_the_bar() {
        let mut s = AccidentalState::new_bar(key(0));
        let f = |a: i8| SpelledPitch::new(Letter::F, Alter::new(a).unwrap(), 4);
        assert_eq!(s.note(f(1), false, false), Some(1)); // F#: shown
        assert_eq!(s.note(f(1), false, false), None); // again: in force
        assert_eq!(s.note(f(0), false, false), Some(0)); // F natural: shown
        // Another octave is another position.
        let f5 = SpelledPitch::new(Letter::F, Alter::new(1).unwrap(), 5);
        assert_eq!(s.note(f5, false, false), Some(1));
        // New bar: reset to the key.
        let mut s = AccidentalState::new_bar(key(1)); // G major: F#
        assert_eq!(s.note(f(1), false, false), None);
        assert_eq!(s.note(f(0), false, false), Some(0));
    }

    #[test]
    fn ties_show_nothing_and_do_not_carry_over_the_bar_line() {
        let f = |a: i8| SpelledPitch::new(Letter::F, Alter::new(a).unwrap(), 4);
        // F# tied into the next bar: no accidental on the tied note, and a later F# in
        // that bar shows its sharp again.
        let mut s = AccidentalState::new_bar(key(0));
        assert_eq!(s.note(f(1), true, true), None);
        assert_eq!(s.note(f(1), false, false), Some(1));
    }
}
