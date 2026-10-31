//! Voicing search constraints (G-02). The defaults are the Phase 2 decisions; they are
//! configurable, not invariants.

use crate::Voicing;
use std::fmt;

/// Which note must sound lowest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum BassRule {
    /// The chord's slash bass if it has one, otherwise the root (default).
    #[default]
    ChordBass,
    /// Any chord tone may be lowest (inversions are allowed).
    AnyChordTone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VoicingConstraints {
    /// Highest minus lowest fretted fret plus one. Open strings do not count.
    pub max_span: u8,
    /// Fretting fingers; a barre counts as one and an open string as zero.
    pub max_fingers: u8,
    pub allow_open_strings: bool,
    /// Lowest allowed fret for fretted notes (open strings are governed by
    /// `allow_open_strings` only).
    pub min_fret: u8,
    /// Highest allowed fret, inclusive.
    pub max_fret: u8,
    /// A perfect fifth may be left out (never a fifth that defines the quality).
    pub allow_omitted_fifth: bool,
    /// Minimum number of sounding strings.
    pub min_sounding: u8,
    pub bass: BassRule,
}

impl Default for VoicingConstraints {
    fn default() -> Self {
        VoicingConstraints {
            max_span: 4,
            max_fingers: 4,
            allow_open_strings: true,
            min_fret: 0,
            max_fret: 12,
            allow_omitted_fifth: true,
            min_sounding: 3,
            bass: BassRule::ChordBass,
        }
    }
}

/// Why a voicing is not playable under the constraints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintViolation {
    Span { span: u8, max: u8 },
    Fingers { fingers: u8, max: u8 },
    OpenString,
    FretBelowMin { fret: u8, min: u8 },
    FretAboveMax { fret: u8, max: u8 },
    TooFewStrings { sounding: u8, min: u8 },
}

impl fmt::Display for ConstraintViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Span { span, max } => write!(f, "fret span {span} exceeds {max}"),
            Self::Fingers { fingers, max } => write!(f, "needs {fingers} fingers, max {max}"),
            Self::OpenString => write!(f, "open strings are not allowed"),
            Self::FretBelowMin { fret, min } => write!(f, "fret {fret} below minimum {min}"),
            Self::FretAboveMax { fret, max } => write!(f, "fret {fret} above maximum {max}"),
            Self::TooFewStrings { sounding, min } => {
                write!(f, "{sounding} sounding strings, at least {min} needed")
            }
        }
    }
}

impl VoicingConstraints {
    /// Physical playability checks of G-02 (span, fingers, open strings, fret range,
    /// sounding strings). Chord content is checked by the search itself.
    pub fn check(&self, v: &Voicing) -> Result<(), ConstraintViolation> {
        if !self.allow_open_strings && v.has_open_strings() {
            return Err(ConstraintViolation::OpenString);
        }
        if let Some(lo) = v.lowest_fretted()
            && lo < self.min_fret
        {
            return Err(ConstraintViolation::FretBelowMin {
                fret: lo,
                min: self.min_fret,
            });
        }
        if let Some(hi) = v.highest_fretted()
            && hi > self.max_fret
        {
            return Err(ConstraintViolation::FretAboveMax {
                fret: hi,
                max: self.max_fret,
            });
        }
        let span = v.fretted_span();
        if span > self.max_span {
            return Err(ConstraintViolation::Span {
                span,
                max: self.max_span,
            });
        }
        let fingers = v.fingers();
        if fingers > self.max_fingers {
            return Err(ConstraintViolation::Fingers {
                fingers,
                max: self.max_fingers,
            });
        }
        let sounding = v.sounding_count() as u8;
        if sounding < self.min_sounding {
            return Err(ConstraintViolation::TooFewStrings {
                sounding,
                min: self.min_sounding,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(shape: &str, c: &VoicingConstraints) -> Result<(), ConstraintViolation> {
        c.check(&shape.parse().unwrap())
    }

    #[test]
    fn defaults_match_phase_2_decisions() {
        let c = VoicingConstraints::default();
        assert_eq!(
            (c.max_span, c.max_fingers, c.min_fret, c.max_fret),
            (4, 4, 0, 12)
        );
        assert!(c.allow_open_strings && c.allow_omitted_fifth);
    }

    #[test]
    fn g02_boundaries() {
        let c = VoicingConstraints::default();
        // Span 4 (frets 1..4) passes, span 5 fails; open strings do not widen the span.
        assert_eq!(check("x-1-x-x-4-0", &c), Ok(()));
        assert_eq!(
            check("x-1-x-x-5-0", &c),
            Err(ConstraintViolation::Span { span: 5, max: 4 })
        );
        // Fret 12 is included, 13 is not.
        assert_eq!(check("x-x-10-12-12-12", &c), Ok(()));
        assert_eq!(
            check("x-x-10-12-13-12", &c),
            Err(ConstraintViolation::FretAboveMax { fret: 13, max: 12 })
        );
        // Five fingers: the muted 4th string breaks any barre.
        assert_eq!(
            check("1-2-x-3-4-1", &c),
            Err(ConstraintViolation::Fingers { fingers: 5, max: 4 })
        );
        let no_open = VoicingConstraints {
            allow_open_strings: false,
            ..c
        };
        assert_eq!(
            check("x32010", &no_open),
            Err(ConstraintViolation::OpenString)
        );
        assert_eq!(
            check("xx0x0x", &c),
            Err(ConstraintViolation::TooFewStrings {
                sounding: 2,
                min: 3
            })
        );
    }
}
