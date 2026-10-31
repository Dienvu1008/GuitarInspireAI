//! Retune planning: how frets change when the tuning or capo changes. The score
//! applies the plan atomically and validates every note (H-03, H-04).

use crate::GuitarError;
use mc_theory::fretted::{Capo, Tuning};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RetuneMode {
    /// Every note keeps its sounding pitch; frets move by the per-string difference.
    KeepPitches,
    /// Every note keeps its string and fret; pitches change.
    KeepFrets,
}

impl RetuneMode {
    pub fn name(self) -> &'static str {
        match self {
            RetuneMode::KeepPitches => "keep-pitches",
            RetuneMode::KeepFrets => "keep-frets",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [RetuneMode::KeepPitches, RetuneMode::KeepFrets]
            .into_iter()
            .find(|m| m.name() == name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RetunePlan {
    /// Per string, string 1 first: new fret = old fret + delta. `None` keeps frets.
    pub fret_deltas: Option<Vec<i16>>,
}

/// Plans a change from (`old`, `old_capo`) to (`new`, `new_capo`).
pub fn plan_retune(
    old: &Tuning,
    old_capo: Capo,
    new: &Tuning,
    new_capo: Capo,
    mode: RetuneMode,
) -> Result<RetunePlan, GuitarError> {
    match mode {
        RetuneMode::KeepFrets => Ok(RetunePlan { fret_deltas: None }),
        RetuneMode::KeepPitches => {
            if old.string_count() != new.string_count() {
                return Err(GuitarError::RetuneStringCount {
                    old: old.string_count(),
                    new: new.string_count(),
                });
            }
            let deltas = old
                .strings()
                .iter()
                .zip(new.strings())
                .map(|(o, n)| {
                    let before = o.semitone() + old_capo.0 as i32;
                    let after = n.semitone() + new_capo.0 as i32;
                    (before - after) as i16
                })
                .collect();
            Ok(RetunePlan {
                fret_deltas: Some(deltas),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TuningPreset;

    #[test]
    fn drop_d_keeps_pitches_by_moving_string_6_up_two_frets() {
        let plan = plan_retune(
            &TuningPreset::Standard.tuning(),
            Capo(0),
            &TuningPreset::DropD.tuning(),
            Capo(0),
            RetuneMode::KeepPitches,
        )
        .unwrap();
        assert_eq!(plan.fret_deltas, Some(vec![0, 0, 0, 0, 0, 2]));
    }

    #[test]
    fn capo_and_half_step_down() {
        // Half-step-down open strings are one semitone lower, so frets go up by one.
        let plan = plan_retune(
            &TuningPreset::Standard.tuning(),
            Capo(0),
            &TuningPreset::HalfStepDown.tuning(),
            Capo(0),
            RetuneMode::KeepPitches,
        )
        .unwrap();
        assert_eq!(plan.fret_deltas, Some(vec![1; 6]));
        let with_capo = plan_retune(
            &TuningPreset::Standard.tuning(),
            Capo(0),
            &TuningPreset::Standard.tuning(),
            Capo(2),
            RetuneMode::KeepPitches,
        )
        .unwrap();
        assert_eq!(with_capo.fret_deltas, Some(vec![-2; 6]));
    }

    #[test]
    fn keep_frets_and_string_count_change() {
        let seven = Tuning::new(
            ["E4", "B3", "G3", "D3", "A2", "E2", "B1"]
                .iter()
                .map(|p| p.parse().unwrap())
                .collect(),
        )
        .unwrap();
        let std = TuningPreset::Standard.tuning();
        assert_eq!(
            plan_retune(&std, Capo(0), &seven, Capo(0), RetuneMode::KeepPitches),
            Err(GuitarError::RetuneStringCount { old: 6, new: 7 })
        );
        assert_eq!(
            plan_retune(&std, Capo(0), &seven, Capo(0), RetuneMode::KeepFrets)
                .unwrap()
                .fret_deltas,
            None
        );
    }
}
