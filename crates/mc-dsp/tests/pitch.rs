//! Accuracy of the MPM detector (ADR-029). On synthetic signals of a known frequency
//! across E2..E6 the error must stay within a tight band; a property test covers the
//! whole chromatic range. Real-guitar accuracy is recorded separately in ADR-029 §5
//! (the agent cannot record an instrument); these tests are the reproducible lower bound.

use mc_dsp::{DetectorConfig, PitchDetector, cents};
use std::f32::consts::PI;

const SR: f32 = 48_000.0;

fn sine(hz: f32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| (2.0 * PI * hz * i as f32 / SR).sin() * 0.5)
        .collect()
}

/// A harmonic-rich decaying tone, closer to a plucked string than a sine.
fn plucked(hz: f32, n: usize) -> Vec<f32> {
    let harmonics = [1.0f32, 0.6, 0.4, 0.25, 0.15, 0.1];
    let mut out = vec![0.0f32; n];
    for (h, amp) in harmonics.iter().enumerate() {
        let f = hz * (h as f32 + 1.0);
        if f >= SR / 2.0 {
            break;
        }
        for (i, s) in out.iter_mut().enumerate() {
            let t = i as f32 / SR;
            *s += amp * (-3.0 * t * (h as f32 + 1.0)).exp() * (2.0 * PI * f * t).sin();
        }
    }
    let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-6);
    for s in out.iter_mut() {
        *s = *s / peak * 0.5;
    }
    out
}

fn midi_hz(key: i32) -> f32 {
    440.0 * 2f32.powf((key as f32 - 69.0) / 12.0)
}

/// Median detected frequency over the steady middle of a signal.
fn detect_median(det: &mut PitchDetector, sig: &[f32]) -> Option<f32> {
    let win = det.config().window;
    let hop = win / 4;
    let mut hits = Vec::new();
    let mut pos = win; // skip the first window (attack)
    while pos + win <= sig.len() {
        if let Some(hz) = det.process(&sig[pos..pos + win]).hz {
            hits.push(hz);
        }
        pos += hop;
    }
    if hits.is_empty() {
        return None;
    }
    hits.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(hits[hits.len() / 2])
}

const OPEN_STRINGS: [(&str, f32); 6] = [
    ("E2", 82.4069),
    ("A2", 110.0000),
    ("D3", 146.8324),
    ("G3", 195.9977),
    ("B3", 246.9417),
    ("E4", 329.6276),
];

#[test]
fn open_strings_are_detected_within_tolerance() {
    let mut det = PitchDetector::new(DetectorConfig::guitar(SR));
    let n = (1.5 * SR) as usize;
    for (name, hz) in OPEN_STRINGS {
        for (kind, sig) in [("sine", sine(hz, n)), ("pluck", plucked(hz, n))] {
            let measured = detect_median(&mut det, &sig)
                .unwrap_or_else(|| panic!("{name} {kind}: no detection"));
            let err = cents(measured, hz).abs();
            // Low strings are allowed a wider band (ADR-029 §2); mids/highs tight.
            let tol = if hz < 120.0 { 5.0 } else { 2.0 };
            assert!(
                err <= tol,
                "{name} {kind}: {measured:.2} Hz vs {hz:.2} Hz ({err:.2} cents > {tol})"
            );
        }
    }
}

#[test]
fn no_octave_errors_on_the_low_strings() {
    let mut det = PitchDetector::new(DetectorConfig::guitar(SR));
    let n = (1.5 * SR) as usize;
    for (name, hz) in OPEN_STRINGS.iter().take(2).copied() {
        let measured = detect_median(&mut det, &plucked(hz, n)).unwrap();
        let err = cents(measured, hz).abs();
        assert!(err < 50.0, "{name}: octave/large error {err:.0} cents");
    }
}

#[test]
fn silence_and_noise_floor_give_no_pitch() {
    let mut det = PitchDetector::new(DetectorConfig::guitar(SR));
    let win = det.config().window;
    let silence = vec![0.0f32; win];
    assert_eq!(det.process(&silence).hz, None);
    let tiny: Vec<f32> = (0..win).map(|i| (i as f32 * 0.01).sin() * 1e-5).collect();
    assert_eq!(det.process(&tiny).hz, None);
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(64))]

    /// For a pure sine at any frequency in the guitar range E2..E6, the detector is
    /// within 5 cents. Covers the whole range, not just the open strings.
    #[test]
    fn sine_sweep_is_accurate(key in 40i32..=88) {
        let mut det = PitchDetector::new(DetectorConfig::guitar(SR));
        let hz = midi_hz(key);
        let sig = sine(hz, (1.0 * SR) as usize);
        let measured = detect_median(&mut det, &sig);
        proptest::prop_assert!(measured.is_some(), "no detection for key {}", key);
        let err = cents(measured.unwrap(), hz).abs();
        proptest::prop_assert!(err <= 5.0, "key {}: {:.2} cents", key, err);
    }
}
