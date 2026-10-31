//! Named tuning presets. Strings are listed string 1 (highest) first (Q-07).

use crate::GuitarError;
use mc_theory::SpelledPitch;
use mc_theory::fretted::Tuning;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TuningPreset {
    Standard,
    DropD,
    Dadgad,
    OpenG,
    OpenD,
    HalfStepDown,
}

impl TuningPreset {
    pub const ALL: [TuningPreset; 6] = [
        TuningPreset::Standard,
        TuningPreset::DropD,
        TuningPreset::Dadgad,
        TuningPreset::OpenG,
        TuningPreset::OpenD,
        TuningPreset::HalfStepDown,
    ];

    /// Stable identifier used by the CLI and scripts.
    pub fn name(self) -> &'static str {
        match self {
            TuningPreset::Standard => "standard",
            TuningPreset::DropD => "drop-d",
            TuningPreset::Dadgad => "dadgad",
            TuningPreset::OpenG => "open-g",
            TuningPreset::OpenD => "open-d",
            TuningPreset::HalfStepDown => "half-step-down",
        }
    }

    pub fn from_name(name: &str) -> Result<Self, GuitarError> {
        Self::ALL
            .into_iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| GuitarError::UnknownPreset(name.to_string()))
    }

    /// Open-string pitches, string 1 first.
    pub fn pitches(self) -> [&'static str; 6] {
        match self {
            TuningPreset::Standard => ["E4", "B3", "G3", "D3", "A2", "E2"],
            TuningPreset::DropD => ["E4", "B3", "G3", "D3", "A2", "D2"],
            TuningPreset::Dadgad => ["D4", "A3", "G3", "D3", "A2", "D2"],
            TuningPreset::OpenG => ["D4", "B3", "G3", "D3", "G2", "D2"],
            TuningPreset::OpenD => ["D4", "A3", "F#3", "D3", "A2", "D2"],
            TuningPreset::HalfStepDown => ["Eb4", "Bb3", "Gb3", "Db3", "Ab2", "Eb2"],
        }
    }

    pub fn tuning(self) -> Tuning {
        let strings: Vec<SpelledPitch> = self
            .pitches()
            .iter()
            .map(|p| p.parse().expect("preset literals are valid pitches"))
            .collect();
        Tuning::new(strings).expect("presets have six strings")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_parse_and_roundtrip_names() {
        for p in TuningPreset::ALL {
            assert_eq!(p.tuning().string_count(), 6);
            assert_eq!(TuningPreset::from_name(p.name()), Ok(p));
        }
        assert_eq!(TuningPreset::Standard.tuning(), Tuning::standard_guitar());
        assert!(TuningPreset::from_name("nashville").is_err());
    }
}
