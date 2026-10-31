//! Chord identification from sounding notes (G-03).
//!
//! A candidate is a (root, quality) pair such that:
//! - every sounding pitch class is a chord tone,
//! - the root sounds (rootless voicings are not named in Phase 2),
//! - every chord tone sounds, except a perfect fifth when omission is allowed.
//!
//! Several candidates are usually valid (C6 and Am7/C share their notes). They are all
//! returned, ranked; the first is the preferred name, not the only interpretation.
//! Ranking: fewer missing tones, then root in the bass, then fewer chord tones, then
//! the quality order of `ChordQuality::ALL`, then the root's distance above the bass
//! (the last two only make the order deterministic and transposition-invariant).

use crate::{GuitarError, SoundingNote, Voicing};
use mc_theory::chord_symbol;
use mc_theory::fretted::{Capo, Tuning};
use mc_theory::{Chord, ChordInterval, ChordQuality, NoteName, PitchClass};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IdentifyOptions {
    /// A missing perfect fifth is accepted (default true).
    pub allow_omitted_fifth: bool,
}

impl Default for IdentifyOptions {
    fn default() -> Self {
        IdentifyOptions {
            allow_omitted_fifth: true,
        }
    }
}

/// One chord tone of a candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CandidateTone {
    pub interval: ChordInterval,
    /// Spelled from the candidate's root.
    pub name: NoteName,
    pub sounding: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordCandidate {
    /// Root, quality and slash bass (when the bass is not the root).
    pub chord: Chord,
    /// Canonical symbol, for example `Am7/C`.
    pub symbol: String,
    /// All chord tones in formula order, with whether each sounds.
    pub tones: Vec<CandidateTone>,
    /// The chord tone that sounds lowest.
    pub bass: CandidateTone,
    /// Human-readable reasons for the candidate and its rank.
    pub explanation: Vec<String>,
}

impl ChordCandidate {
    pub fn missing(&self) -> impl Iterator<Item = &CandidateTone> {
        self.tones.iter().filter(|t| !t.sounding)
    }

    pub fn root_in_bass(&self) -> bool {
        self.bass.interval.degree == 1
    }

    fn rank_key(&self) -> (usize, bool, usize, usize, u8) {
        let quality_index = ChordQuality::ALL
            .iter()
            .position(|q| *q == self.chord.quality)
            .expect("listed");
        (
            self.missing().count(),
            !self.root_in_bass(),
            self.tones.len(),
            quality_index,
            (12 - self.bass.interval.semitones % 12) % 12,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identification {
    /// Sounding notes, lowest pitch first.
    pub notes: Vec<SoundingNote>,
    /// Distinct pitch classes in order of first appearance from the bass upward.
    pub pitch_classes: Vec<PitchClass>,
    /// Lowest sounding note.
    pub bass: Option<SoundingNote>,
    /// Ranked candidates; the first is preferred. Empty when nothing in the Phase 2
    /// vocabulary fits.
    pub candidates: Vec<ChordCandidate>,
}

impl Identification {
    pub fn preferred(&self) -> Option<&ChordCandidate> {
        self.candidates.first()
    }
}

/// Identifies a voicing on an instrument.
pub fn identify(
    voicing: &Voicing,
    tuning: &Tuning,
    capo: Capo,
    options: IdentifyOptions,
) -> Result<Identification, GuitarError> {
    let mut notes = voicing.sounding(tuning, capo)?;
    // Lowest pitch first; equal pitches keep the higher-numbered (thicker) string first.
    notes.sort_by_key(|n| (n.semitone, std::cmp::Reverse(n.string)));
    let semitones: Vec<i32> = notes.iter().map(|n| n.semitone).collect();
    let mut pitch_classes = Vec::new();
    for s in &semitones {
        let pc = PitchClass::from_semitone(*s);
        if !pitch_classes.contains(&pc) {
            pitch_classes.push(pc);
        }
    }
    Ok(Identification {
        bass: notes.first().copied(),
        candidates: candidates_for(&semitones, options),
        pitch_classes,
        notes,
    })
}

/// Ranked candidates for a set of sounding semitones (any order; the lowest is the bass).
pub fn candidates_for(semitones: &[i32], options: IdentifyOptions) -> Vec<ChordCandidate> {
    let Some(&bass_semitone) = semitones.iter().min() else {
        return Vec::new();
    };
    let bass_pc = PitchClass::from_semitone(bass_semitone);
    let mut sounding = [false; 12];
    for s in semitones {
        sounding[PitchClass::from_semitone(*s).value() as usize] = true;
    }
    let mut out = Vec::new();
    for root_value in 0..12u8 {
        if !sounding[root_value as usize] {
            continue;
        }
        let root_pc = PitchClass::from_semitone(root_value as i32);
        for quality in ChordQuality::ALL {
            if let Some(c) = candidate(root_pc, quality, &sounding, bass_pc, options) {
                out.push(c);
            }
        }
    }
    out.sort_by_key(ChordCandidate::rank_key);
    explain_ranks(&mut out);
    out
}

fn candidate(
    root_pc: PitchClass,
    quality: ChordQuality,
    sounding: &[bool; 12],
    bass_pc: PitchClass,
    options: IdentifyOptions,
) -> Option<ChordCandidate> {
    let intervals = quality.intervals();
    let pc_of =
        |i: &ChordInterval| PitchClass::from_semitone((root_pc.value() + i.semitones) as i32);
    // Every sounding pitch class must be a chord tone.
    let covered = (0..12u8).filter(|v| sounding[*v as usize]).all(|v| {
        intervals
            .iter()
            .any(|i| pc_of(i) == PitchClass::from_semitone(v as i32))
    });
    if !covered {
        return None;
    }
    let root = NoteName::default_for(root_pc);
    let mut tones = Vec::with_capacity(intervals.len());
    for i in intervals {
        let is_sounding = sounding[pc_of(i).value() as usize];
        if !is_sounding && !(i.omissible_fifth && options.allow_omitted_fifth) {
            return None;
        }
        tones.push(CandidateTone {
            interval: *i,
            name: root
                .up((i.degree - 1) % 7, i.semitones % 12)
                .unwrap_or_else(|| NoteName::default_for(pc_of(i))),
            sounding: is_sounding,
        });
    }
    let bass = *tones
        .iter()
        .find(|t| pc_of(&t.interval) == bass_pc)
        .expect("the bass sounds and every sounding note is a chord tone");
    let chord = Chord {
        root,
        quality,
        bass: (bass.interval.degree != 1).then_some(bass.name),
    };
    let mut explanation = Vec::new();
    let missing: Vec<&CandidateTone> = tones.iter().filter(|t| !t.sounding).collect();
    if missing.is_empty() {
        explanation.push(format!("complete: all {} chord tones sound", tones.len()));
    } else {
        for m in &missing {
            explanation.push(format!(
                "omits the {} ({}); allowed by configuration",
                interval_name(&m.interval),
                m.name
            ));
        }
    }
    explanation.push(match bass.interval.degree {
        1 => format!("root position: root {root} is the lowest note"),
        3 => format!(
            "first inversion: bass {} is the {}",
            bass.name,
            interval_name(&bass.interval)
        ),
        5 => format!(
            "second inversion: bass {} is the {}",
            bass.name,
            interval_name(&bass.interval)
        ),
        7 => format!(
            "third inversion: bass {} is the {}",
            bass.name,
            interval_name(&bass.interval)
        ),
        _ => format!(
            "bass {} is the {}",
            bass.name,
            interval_name(&bass.interval)
        ),
    });
    Some(ChordCandidate {
        symbol: chord_symbol::format(&chord),
        chord,
        tones,
        bass,
        explanation,
    })
}

/// Adds a line to each candidate saying why it ranks where it does.
fn explain_ranks(candidates: &mut [ChordCandidate]) {
    let total = candidates.len();
    if total == 0 {
        return;
    }
    let first_key = candidates[0].rank_key();
    let first_symbol = candidates[0].symbol.clone();
    for (i, c) in candidates.iter_mut().enumerate() {
        let line = if i == 0 {
            if total == 1 {
                "only candidate in the Phase 2 vocabulary".to_string()
            } else {
                format!("preferred of {total} candidates with these notes")
            }
        } else {
            let k = c.rank_key();
            let reason = if k.0 != first_key.0 {
                "it leaves out more chord tones"
            } else if k.1 != first_key.1 {
                "its root is not in the bass"
            } else if k.2 != first_key.2 {
                "it has more chord tones"
            } else {
                "tie broken by quality order"
            };
            format!(
                "alternative {} of {total}: ranked below {first_symbol} because {reason}",
                i + 1
            )
        };
        c.explanation.push(line);
    }
}

/// Interval label for explanations.
pub fn interval_name(i: &ChordInterval) -> &'static str {
    match (i.degree, i.semitones) {
        (1, 0) => "root",
        (2, 2) => "major 2nd",
        (3, 3) => "minor 3rd",
        (3, 4) => "major 3rd",
        (4, 5) => "perfect 4th",
        (5, 6) => "diminished 5th",
        (5, 7) => "perfect 5th",
        (5, 8) => "augmented 5th",
        (6, 9) => "major 6th",
        (7, 9) => "diminished 7th",
        (7, 10) => "minor 7th",
        (7, 11) => "major 7th",
        (9, 14) => "major 9th",
        _ => "chord tone",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbols(shape: &str) -> Vec<String> {
        let v: Voicing = shape.parse().unwrap();
        identify(
            &v,
            &Tuning::standard_guitar(),
            Capo(0),
            IdentifyOptions::default(),
        )
        .unwrap()
        .candidates
        .into_iter()
        .map(|c| c.symbol)
        .collect()
    }

    #[test]
    fn g03_open_c_and_c6_vs_am7() {
        assert_eq!(symbols("x32010"), ["C"]);
        // C E A C E: Am/C is complete; C6 lacks its (omissible) fifth, so it ranks second.
        assert_eq!(symbols("x32210"), ["Am/C", "C6"]);
        // A E A C E: Am; C6/A (no 5th) is also a valid reading.
        assert_eq!(symbols("x02210"), ["Am", "C6/A"]);
        // C E A C G: both complete; C6 has its root in the bass.
        assert_eq!(symbols("x32253"), ["C6", "Am7/C"]);
    }

    #[test]
    fn g03_fifth_omission_is_configurable() {
        let v: Voicing = "x32x1x".parse().unwrap(); // C E C: C without fifth
        let t = Tuning::standard_guitar();
        let on = identify(&v, &t, Capo(0), IdentifyOptions::default()).unwrap();
        assert_eq!(on.candidates[0].symbol, "C");
        assert_eq!(on.candidates[0].missing().count(), 1);
        let off = identify(
            &v,
            &t,
            Capo(0),
            IdentifyOptions {
                allow_omitted_fifth: false,
            },
        )
        .unwrap();
        assert!(off.candidates.is_empty());
    }

    #[test]
    fn g03_thirds_are_never_omitted() {
        // C and G only (power chord): out of vocabulary.
        assert!(symbols("x355xx").is_empty());
    }
}
