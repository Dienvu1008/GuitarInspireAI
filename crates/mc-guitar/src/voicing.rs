//! A voicing: at most one fret per string (G-01 holds by construction).
//!
//! Shape text follows chord-chart order: the first symbol is the highest-numbered
//! string (string 6, low E, on a guitar) and the last is string 1. `x32010` is open C. Frets above 9 need separators:
//! `x-3-5-5-5-3` or `8 10 10 9 8 8`.

use crate::GuitarError;
use mc_theory::fretted::{Capo, Fret, FretPosition, StringIndex, Tuning, pitch_at};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Voicing {
    /// Index 0 is string 1. `None` is a muted (not played) string.
    frets: Vec<Option<Fret>>,
}

/// One sounding note of a voicing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SoundingNote {
    pub string: StringIndex,
    pub fret: Fret,
    /// Semitone number (MIDI scale) including capo.
    pub semitone: i32,
}

impl Voicing {
    /// From per-string frets, string 1 first. Needs at least one string.
    pub fn new(frets: Vec<Option<Fret>>) -> Result<Self, GuitarError> {
        if frets.is_empty() || frets.len() > u8::MAX as usize {
            return Err(GuitarError::InvalidShape(String::new()));
        }
        Ok(Voicing { frets })
    }

    /// From positions on an instrument with `strings` strings. Rejects a second
    /// position on the same string (G-01) and strings the instrument lacks.
    pub fn from_positions(positions: &[FretPosition], strings: u8) -> Result<Self, GuitarError> {
        let mut frets = vec![None; strings as usize];
        for p in positions {
            let s = p.string.get();
            if s > strings {
                return Err(GuitarError::StringOutOfRange { string: s, strings });
            }
            let slot = &mut frets[s as usize - 1];
            if slot.is_some() {
                return Err(GuitarError::DuplicateString(s));
            }
            *slot = Some(p.fret);
        }
        Voicing::new(frets)
    }

    pub fn string_count(&self) -> u8 {
        self.frets.len() as u8
    }

    /// Fret on a string, `None` if muted or absent.
    pub fn fret(&self, string: StringIndex) -> Option<Fret> {
        self.frets.get(string.get() as usize - 1).copied().flatten()
    }

    /// Per-string frets, string 1 first.
    pub fn frets(&self) -> &[Option<Fret>] {
        &self.frets
    }

    /// Played positions, string 1 first.
    pub fn positions(&self) -> Vec<FretPosition> {
        self.frets
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                f.map(|fret| FretPosition {
                    string: StringIndex::new(i as u8 + 1).expect("1-based"),
                    fret,
                })
            })
            .collect()
    }

    pub fn sounding_count(&self) -> usize {
        self.frets.iter().filter(|f| f.is_some()).count()
    }

    pub fn has_open_strings(&self) -> bool {
        self.frets.contains(&Some(Fret(0)))
    }

    /// Lowest fretted (non-open) fret, if any.
    pub fn lowest_fretted(&self) -> Option<u8> {
        self.fretted().min()
    }

    pub fn highest_fretted(&self) -> Option<u8> {
        self.fretted().max()
    }

    /// Number of frets covered by fretted notes: highest - lowest + 1. Open strings do
    /// not count; 0 when nothing is fretted.
    pub fn fretted_span(&self) -> u8 {
        match (self.lowest_fretted(), self.highest_fretted()) {
            (Some(lo), Some(hi)) => hi - lo + 1,
            _ => 0,
        }
    }

    /// Fretting fingers needed. Each fretted note takes one finger, except that notes
    /// at the same fret `f` may share one barre finger when they lie in a run of
    /// adjacent strings that are all fretted at `f` or higher (an open, muted or
    /// lower-fretted string inside the run breaks it, because the barre would sound or
    /// override it). Open strings take no finger. At most one barre is assumed; the
    /// barre that saves the most fingers is chosen.
    pub fn fingers(&self) -> u8 {
        let fretted = self.fretted().count() as u8;
        let mut best_saving = 0u8;
        for barre in self.fretted() {
            let mut members_in_run = 0u8;
            for f in self.frets.iter().chain(std::iter::once(&None)) {
                match f {
                    Some(Fret(n)) if *n >= barre => {
                        if *n == barre {
                            members_in_run += 1;
                        }
                    }
                    _ => {
                        best_saving = best_saving.max(members_in_run.saturating_sub(1));
                        members_in_run = 0;
                    }
                }
            }
        }
        fretted - best_saving
    }

    /// Sounding notes from the lowest-numbered string upward in string number
    /// (string 1 first). The tuning must have the same number of strings.
    pub fn sounding(&self, tuning: &Tuning, capo: Capo) -> Result<Vec<SoundingNote>, GuitarError> {
        if tuning.string_count() != self.string_count() {
            return Err(GuitarError::StringCountMismatch {
                voicing: self.string_count(),
                tuning: tuning.string_count(),
            });
        }
        Ok(self
            .positions()
            .into_iter()
            .map(|p| SoundingNote {
                string: p.string,
                fret: p.fret,
                semitone: pitch_at(tuning, capo, p).expect("string count checked"),
            })
            .collect())
    }

    fn fretted(&self) -> impl Iterator<Item = u8> + '_ {
        self.frets
            .iter()
            .filter_map(|f| f.map(|f| f.0))
            .filter(|f| *f > 0)
    }
}

impl fmt::Display for Voicing {
    /// Highest-numbered string first; compact when every fret is 0..=9.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let compact = self.frets.iter().all(|x| x.is_none_or(|fr| fr.0 <= 9));
        let tokens: Vec<String> = self
            .frets
            .iter()
            .rev()
            .map(|x| x.map_or_else(|| "x".to_string(), |fr| fr.0.to_string()))
            .collect();
        f.write_str(&tokens.join(if compact { "" } else { "-" }))
    }
}

impl FromStr for Voicing {
    type Err = GuitarError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || GuitarError::InvalidShape(s.to_string());
        let text = s.trim();
        let tokens: Vec<&str> = if text.contains(['-', ' ', ',']) {
            text.split(['-', ' ', ','])
                .filter(|t| !t.is_empty())
                .collect()
        } else {
            text.char_indices()
                .map(|(i, c)| &text[i..i + c.len_utf8()])
                .collect()
        };
        let mut frets = Vec::with_capacity(tokens.len());
        for t in tokens.iter().rev() {
            frets.push(match *t {
                "x" | "X" => None,
                n => Some(Fret(n.parse::<u8>().map_err(|_| bad())?)),
            });
        }
        Voicing::new(frets).map_err(|_| bad())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Voicing {
        s.parse().unwrap()
    }

    #[test]
    fn shape_text_is_low_string_first() {
        let c = v("x32010");
        assert_eq!(c.fret(StringIndex::new(1).unwrap()), Some(Fret(0)));
        assert_eq!(c.fret(StringIndex::new(5).unwrap()), Some(Fret(3)));
        assert_eq!(c.fret(StringIndex::new(6).unwrap()), None);
        assert_eq!(c.to_string(), "x32010");
        assert_eq!(v("x-10-12-12-12-10").to_string(), "x-10-12-12-12-10");
        assert_eq!(v("8 10 10 9 8 8").to_string(), "8-10-10-9-8-8");
        for bad in ["", "x3201?", "x-3-256", "--"] {
            assert!(bad.parse::<Voicing>().is_err(), "{bad}");
        }
    }

    #[test]
    fn g01_one_position_per_string() {
        let p = |s: u8, f: u8| FretPosition {
            string: StringIndex::new(s).unwrap(),
            fret: Fret(f),
        };
        assert_eq!(
            Voicing::from_positions(&[p(1, 0), p(1, 3)], 6),
            Err(GuitarError::DuplicateString(1))
        );
        assert!(matches!(
            Voicing::from_positions(&[p(7, 0)], 6),
            Err(GuitarError::StringOutOfRange { .. })
        ));
        let ok = Voicing::from_positions(&[p(5, 3), p(4, 2), p(2, 1)], 6).unwrap();
        assert_eq!(ok.to_string(), "x32x1x");
    }

    #[test]
    fn span_and_fingers() {
        // Open C: frets 1..3, three fingers.
        assert_eq!(v("x32010").fretted_span(), 3);
        assert_eq!(v("x32010").fingers(), 3);
        // F barre: six fretted notes, the 1st-fret barre saves two fingers.
        assert_eq!(v("133211").fingers(), 4);
        // A shape with a one-finger barre at fret 2 between open strings.
        assert_eq!(v("x02220").fingers(), 1);
        // An open string inside the run breaks the barre.
        assert_eq!(v("x2023x").fingers(), 3);
        // A barre above the lowest fret: C9 with a 3rd-fret barre on strings 1-3.
        assert_eq!(v("x32333").fingers(), 3);
        // A muted string breaks the run: the two 1st-fret notes need two fingers.
        assert_eq!(v("1-2-x-3-4-1").fingers(), 5);
        // All open: no fingers, span 0.
        assert_eq!(v("000000").fingers(), 0);
        assert_eq!(v("000000").fretted_span(), 0);
    }
}
