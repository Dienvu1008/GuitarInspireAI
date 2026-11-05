//! Tuner reference pitches for a fretted instrument (ADR-029 T8). A Guitar-context rule
//! (GR-05): mapping a measured frequency to "which string, how many cents off" needs the
//! track's tuning and capo, so it lives here, not in the audio adapter or in Dart. Pure:
//! no I/O, no clocks; equal temperament with A4 = 440 Hz.

use mc_theory::fretted::{Capo, StringIndex, Tuning};

/// Concert A for equal temperament. Fixed in the MVP (a configurable reference pitch is
/// a later option).
pub const A4_HZ: f32 = 440.0;

/// Frequency of a MIDI-numbered semitone (A4 = 69). Pure equal temperament.
pub fn semitone_hz(semitone: i32) -> f32 {
    A4_HZ * 2f32.powf((semitone as f32 - 69.0) / 12.0)
}

/// One open string the tuner can match against: its 1-based index, its sounding semitone
/// (open pitch raised by the capo) and that pitch in Hz.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TunerTarget {
    pub string: StringIndex,
    pub semitone: i32,
    pub hz: f32,
}

/// The open-string targets of `tuning` with `capo` applied, string 1 first. These are
/// what the tuner compares a measured frequency against (ADR-029 T8). When the track
/// changes tuning or capo, the caller rebuilds this list; no engine restart is needed,
/// because the audio tuner only produces a frequency.
pub fn tuner_targets(tuning: &Tuning, capo: Capo) -> Vec<TunerTarget> {
    let mut out = Vec::with_capacity(tuning.string_count() as usize);
    for (i, pitch) in tuning.strings().iter().enumerate() {
        let string = StringIndex::new((i + 1) as u8).expect("1-based, non-zero");
        let semitone = pitch.semitone() + capo.0 as i32;
        out.push(TunerTarget {
            string,
            semitone,
            hz: semitone_hz(semitone),
        });
    }
    out
}

/// A tuner match: the nearest target string to a measured frequency and how far off it
/// is, in cents (positive means the played note is sharp of the target).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TunerMatch {
    pub string: StringIndex,
    pub target_hz: f32,
    pub cents: f32,
}

/// Cents from `target` to `measured` (both > 0); positive is sharp.
pub fn cents(measured: f32, target: f32) -> f32 {
    1200.0 * (measured / target).log2()
}

/// Nearest target to `measured_hz` by absolute cents. `None` if `targets` is empty or
/// the frequency is not positive. The nearest-by-cents choice means an out-of-tune
/// string still matches its own target (not an octave neighbour) as long as it is within
/// about 600 cents, which open-string tuning always is.
pub fn nearest(targets: &[TunerTarget], measured_hz: f32) -> Option<TunerMatch> {
    if measured_hz <= 0.0 {
        return None;
    }
    targets
        .iter()
        .map(|t| TunerMatch {
            string: t.string,
            target_hz: t.hz,
            cents: cents(measured_hz, t.hz),
        })
        .min_by(|a, b| {
            a.cents
                .abs()
                .partial_cmp(&b.cents.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_theory::fretted::Tuning;

    #[test]
    fn standard_targets_are_the_open_string_frequencies() {
        let t = tuner_targets(&Tuning::standard_guitar(), Capo(0));
        assert_eq!(t.len(), 6);
        // string 1 = E4 (~329.6), string 6 = E2 (~82.4).
        assert!((t[0].hz - 329.6276).abs() < 0.1, "{}", t[0].hz);
        assert!((t[5].hz - 82.4069).abs() < 0.1, "{}", t[5].hz);
    }

    #[test]
    fn capo_raises_every_target() {
        let open = tuner_targets(&Tuning::standard_guitar(), Capo(0));
        let capo2 = tuner_targets(&Tuning::standard_guitar(), Capo(2));
        for (a, b) in open.iter().zip(&capo2) {
            assert_eq!(b.semitone - a.semitone, 2);
            // Two semitones up is a frequency ratio of 2^(2/12).
            assert!((b.hz / a.hz - 2f32.powf(2.0 / 12.0)).abs() < 1e-4);
        }
    }

    #[test]
    fn nearest_matches_the_right_string_with_cents() {
        let targets = tuner_targets(&Tuning::standard_guitar(), Capo(0));
        // 10 cents sharp of A2 (110 Hz).
        let sharp = 110.0 * 2f32.powf(10.0 / 1200.0);
        let m = nearest(&targets, sharp).unwrap();
        assert_eq!(m.string.get(), 5, "A2 is string 5");
        assert!((m.cents - 10.0).abs() < 0.5, "{}", m.cents);
    }

    #[test]
    fn a_flat_low_e_still_matches_low_e() {
        let targets = tuner_targets(&Tuning::standard_guitar(), Capo(0));
        // 30 cents flat of E2.
        let flat = 82.4069 * 2f32.powf(-30.0 / 1200.0);
        let m = nearest(&targets, flat).unwrap();
        assert_eq!(m.string.get(), 6, "E2 is string 6");
        assert!(m.cents < 0.0 && m.cents > -40.0, "{}", m.cents);
    }

    #[test]
    fn no_match_for_nonpositive_frequency() {
        let targets = tuner_targets(&Tuning::standard_guitar(), Capo(0));
        assert!(nearest(&targets, 0.0).is_none());
    }
}
