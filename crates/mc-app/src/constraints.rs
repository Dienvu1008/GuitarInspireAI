//! Configurable constraints (category C in domain-invariants.md). They limit user input,
//! not the domain model: a file from another tool may contain values outside them.

use crate::CommandError;
use mc_theory::Rational;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Constraints {
    /// T-06: user tempo input range in quarter-note BPM (default 20 to 400).
    pub tempo_min: u32,
    pub tempo_max: u32,
    /// T-07: strings per tuning (default 1 to 12).
    pub max_strings: u8,
    /// C-01: fret count per track (default 1 to 36).
    pub max_fret_count: u8,
    /// Longest track name in characters.
    pub max_track_name: usize,
}

impl Default for Constraints {
    fn default() -> Self {
        Constraints {
            tempo_min: 20,
            tempo_max: 400,
            max_strings: 12,
            max_fret_count: 36,
            max_track_name: 100,
        }
    }
}

impl Constraints {
    pub fn tempo(&self, bpm: Rational) -> Result<(), CommandError> {
        if bpm < Rational::from_int(self.tempo_min as i64)
            || bpm > Rational::from_int(self.tempo_max as i64)
        {
            return Err(CommandError::Constraint {
                rule: "T-06",
                message: format!(
                    "tempo {bpm} outside {} to {} BPM",
                    self.tempo_min, self.tempo_max
                ),
            });
        }
        Ok(())
    }

    pub fn strings(&self, count: usize) -> Result<(), CommandError> {
        if count == 0 || count > self.max_strings as usize {
            return Err(CommandError::Constraint {
                rule: "T-07",
                message: format!("{count} strings; allowed 1 to {}", self.max_strings),
            });
        }
        Ok(())
    }

    pub fn fret_count(&self, frets: u8) -> Result<(), CommandError> {
        if frets == 0 || frets > self.max_fret_count {
            return Err(CommandError::Constraint {
                rule: "C-01",
                message: format!("{frets} frets; allowed 1 to {}", self.max_fret_count),
            });
        }
        Ok(())
    }

    pub fn track_name(&self, name: &str) -> Result<(), CommandError> {
        if name.trim().is_empty() || name.chars().count() > self.max_track_name {
            return Err(CommandError::Constraint {
                rule: "C-02",
                message: format!("track name must be 1 to {} characters", self.max_track_name),
            });
        }
        Ok(())
    }
}
