//! Verifies the bundled trimmed SoundFont (ADR-026, slice 4c) renders the full guitar
//! range E2..=E6 through General MIDI program 25 with the correct fundamental pitch.
//!
//! The trim keeps 10 of FluidR3's 1418 samples (0.7%); the lowest sample is "Steel E3",
//! so E2 (MIDI 40) is produced by pitch-shifting a sample down an octave. This test
//! confirms every note in the range is audible and sounds at its expected frequency
//! (within a quarter tone), so the trim did not drop a region or leave a dead key.
//!
//! No audio device is needed (pure synthesis), but it reads the committed .sf2, so it
//! runs with:
//!   cargo test -p mc-audio --test soundfont_pitch -- --ignored --nocapture
//! It is #[ignore] so the normal `cargo test` run does not depend on the asset path; it
//! is meant to be run on the developer machine (ADR-025 single-machine evidence).

use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::path::PathBuf;
use std::sync::Arc;

const SAMPLE_RATE: i32 = 48_000;
const STEEL_PROGRAM: i32 = 25;

fn soundfont_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/soundfonts/GuitarSteel.sf2"
    ))
}

/// Equal-tempered frequency of a MIDI key (A4 = 69 = 440 Hz).
fn midi_to_hz(key: i32) -> f64 {
    440.0 * 2f64.powf((key as f64 - 69.0) / 12.0)
}

/// Dominant frequency of a mono signal by autocorrelation over a plausible guitar range
/// (down to ~70 Hz, up to ~1400 Hz). Returns `None` if the signal is too quiet.
fn dominant_hz(signal: &[f32], sample_rate: i32) -> Option<f64> {
    let rms = (signal.iter().map(|s| s * s).sum::<f32>() / signal.len() as f32).sqrt();
    if rms < 1e-3 {
        return None;
    }
    let min_lag = (sample_rate as f64 / 1400.0) as usize;
    let max_lag = (sample_rate as f64 / 70.0) as usize;
    let n = signal.len();
    // Normalized autocorrelation at every candidate lag.
    let mut corr = vec![0.0f64; max_lag.min(n - 1) + 1];
    for (lag, c) in corr.iter_mut().enumerate().skip(min_lag) {
        let mut sum = 0.0f64;
        for i in 0..n - lag {
            sum += signal[i] as f64 * signal[i + lag] as f64;
        }
        *c = sum;
    }
    let peak = corr.iter().cloned().fold(f64::MIN, f64::max);
    if peak <= 0.0 {
        return None;
    }
    // The true period is the *first local maximum* of the autocorrelation that is strong
    // (within 85% of the global peak). Using "first lag over a threshold" biases sharp,
    // because it lands on the rising edge before the peak; using the global argmax biases
    // toward subharmonics for bright notes. The first strong local max avoids both.
    let threshold = 0.85 * peak;
    let mut best_lag = 0usize;
    for lag in (min_lag + 1)..corr.len() - 1 {
        if corr[lag] >= threshold && corr[lag] >= corr[lag - 1] && corr[lag] >= corr[lag + 1] {
            best_lag = lag;
            break;
        }
    }
    if best_lag == 0 {
        return None;
    }
    // Parabolic interpolation around the peak for a sub-sample lag. Integer lags are
    // coarse at short periods (at ~1300 Hz one lag step is ~36 cents), so without this
    // the top octave looks sharp purely from quantization, not from the SoundFont.
    let refined = if best_lag > min_lag && best_lag + 1 < corr.len() {
        let (a, b, c) = (corr[best_lag - 1], corr[best_lag], corr[best_lag + 1]);
        let denom = a - 2.0 * b + c;
        let delta = if denom.abs() > 1e-12 {
            0.5 * (a - c) / denom
        } else {
            0.0
        };
        best_lag as f64 + delta.clamp(-1.0, 1.0)
    } else {
        best_lag as f64
    };
    Some(sample_rate as f64 / refined)
}

/// Render one key to a mono buffer of `seconds` using the given SoundFont.
fn render_note(font: &Arc<SoundFont>, key: i32, seconds: f64) -> Vec<f32> {
    let settings = SynthesizerSettings::new(SAMPLE_RATE);
    let mut synth = Synthesizer::new(font, &settings).expect("synth");
    // Select program 25 on channel 0 (bank 0).
    synth.process_midi_message(0, 0xC0, STEEL_PROGRAM, 0);
    synth.note_on(0, key, 100);
    let frames = (SAMPLE_RATE as f64 * seconds) as usize;
    let mut left = vec![0.0f32; frames];
    let mut right = vec![0.0f32; frames];
    synth.render(&mut left, &mut right);
    // Mono mix.
    left.iter()
        .zip(&right)
        .map(|(l, r)| (l + r) * 0.5)
        .collect()
}

#[test]
#[ignore = "reads the bundled SoundFont; run manually on the developer machine"]
fn program_25_renders_the_whole_guitar_range_e2_to_e6() {
    let bytes = std::fs::read(soundfont_path()).expect("the bundled SoundFont");
    let font = Arc::new(SoundFont::new(&mut bytes.as_slice()).expect("valid SF2"));

    // E2 = 40 (guitar low E) up to E6 = 88, every semitone.
    let mut failures = Vec::new();
    for key in 40..=88 {
        let signal = render_note(&font, key, 1.0);
        // Skip the attack; measure the sustained middle of the note.
        let start = signal.len() / 4;
        let window = &signal[start..start + SAMPLE_RATE as usize / 2];
        let expected = midi_to_hz(key);
        match dominant_hz(window, SAMPLE_RATE) {
            None => failures.push(format!("key {key}: silent")),
            Some(hz) => {
                let cents = 1200.0 * (hz / expected).log2();
                println!(
                    "key {key:3} expected {expected:7.1} Hz  measured {hz:7.1} Hz  ({cents:+.0} cents)"
                );
                // Allow a quarter tone (50 cents) plus octave ambiguity of the estimator.
                let octave_folded = {
                    let mut c = cents;
                    while c > 600.0 {
                        c -= 1200.0;
                    }
                    while c < -600.0 {
                        c += 1200.0;
                    }
                    c
                };
                if octave_folded.abs() > 50.0 {
                    failures.push(format!(
                        "key {key}: {hz:.1} Hz vs {expected:.1} Hz ({cents:+.0} cents)"
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "pitch problems:\n{}",
        failures.join("\n")
    );
}
