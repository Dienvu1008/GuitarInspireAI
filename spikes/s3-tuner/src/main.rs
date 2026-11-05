//! S3 tuner spike harness (ADR-029). Throwaway; measures A8 to A10 for MPM and YIN.
//!
//!   cargo run --release -- synthetic          # A9 on sine + plucked +/- noise, E2..E6
//!   cargo run --release -- latency            # A8: frames to first stable reading
//!   cargo run --release -- wav <dir>          # A9 on real-guitar WAVs in <dir>
//!   cargo run --release -- input [seconds]    # A10 + live readings from the mic
//!
//! Window 2048 @ 48 kHz, hop 512 (audio-dsp section 4). Results go under results/.

mod detect;
mod input;
mod signal;

use detect::{Algo, Estimate, cents, detect, rms};
use signal::{OPEN_STRINGS, add_noise, midi_hz, plucked, read_wav, sine};

const SR: f32 = 48_000.0;
const WIN: usize = 2048;
const HOP: usize = 512;
const GATE: f32 = 0.5; // provisional clarity gate; S3 tunes it
const MIN_HZ: f32 = 70.0;
const MAX_HZ: f32 = 1400.0;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("synthetic") => synthetic(),
        Some("latency") => latency(),
        Some("wav") => wav(args.get(1).map(String::as_str).unwrap_or("results")),
        Some("input") => {
            let secs = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
            input::run(secs, SR, WIN, HOP, GATE, MIN_HZ, MAX_HZ);
        }
        _ => {
            eprintln!("usage: synthetic | latency | wav <dir> | input [seconds]");
            std::process::exit(2);
        }
    }
}

/// Summary of per-string errors over the steady-state windows.
struct Stats {
    mean_cents: f32,
    p95_cents: f32,
    octave_rate: f32,
    detect_rate: f32,
}

/// Run a detector over a signal in hops and collect errors vs the reference.
fn measure(algo: Algo, sig: &[f32], reference: f32) -> Stats {
    let mut errs: Vec<f32> = Vec::new();
    let mut octaves = 0usize;
    let mut detected = 0usize;
    let mut total = 0usize;
    let mut pos = 0usize;
    // Skip the first ~100 ms (attack) and the last ~100 ms (release).
    let skip = (0.1 * SR) as usize;
    while pos + WIN <= sig.len() {
        let center = pos + WIN / 2;
        let steady = center >= skip && center + skip < sig.len();
        if steady {
            total += 1;
            let frame = &sig[pos..pos + WIN];
            if rms(frame) > 1e-3 {
                let Estimate { hz, .. } = detect(algo, frame, SR, GATE, MIN_HZ, MAX_HZ);
                if let Some(hz) = hz
                    && hz.is_finite()
                    && hz > 0.0
                {
                    detected += 1;
                    let c = cents(hz, reference);
                    if !c.is_finite() {
                        // Degenerate estimate; count as a non-detection, not an error.
                    } else if c.abs() > 600.0 {
                        octaves += 1; // off by about an octave or more
                    } else {
                        errs.push(c.abs());
                    }
                }
            }
        }
        pos += HOP;
    }
    errs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mean = if errs.is_empty() {
        f32::NAN
    } else {
        errs.iter().sum::<f32>() / errs.len() as f32
    };
    let p95 = if errs.is_empty() {
        f32::NAN
    } else {
        errs[((errs.len() as f32 * 0.95) as usize).min(errs.len() - 1)]
    };
    Stats {
        mean_cents: mean,
        p95_cents: p95,
        octave_rate: if total > 0 { octaves as f32 / total as f32 } else { 0.0 },
        detect_rate: if total > 0 { detected as f32 / total as f32 } else { 0.0 },
    }
}

fn print_row(label: &str, algo: Algo, sig: &[f32], reference: f32) {
    let s = measure(algo, sig, reference);
    println!(
        "{label:<20} {:>8.2} {:>8.2} {:>7.1}% {:>7.1}%",
        s.mean_cents,
        s.p95_cents,
        s.octave_rate * 100.0,
        s.detect_rate * 100.0,
    );
}

fn synthetic() {
    let dur = (1.5 * SR) as usize;
    println!("S3 synthetic accuracy (window {WIN} @ {} kHz, gate {GATE})", SR / 1000.0);
    println!("reference signals are synthetic; real-guitar numbers come from `wav` (ADR-029 s.4)\n");
    for algo in [Algo::Mpm, Algo::Yin] {
        println!("=== {algo:?} ===");
        println!("{:<20} {:>8} {:>8} {:>8} {:>8}", "signal", "mean c", "p95 c", "oct%", "det%");
        // Open strings, sine and plucked, clean and at 20 dB SNR.
        for (name, hz) in OPEN_STRINGS {
            print_row(&format!("{name} sine"), algo, &sine(hz, dur, SR), hz);
            print_row(&format!("{name} pluck"), algo, &plucked(hz, dur, SR), hz);
            let noisy = add_noise(&plucked(hz, dur, SR), 20.0, 0x53_u64 ^ hz as u64);
            print_row(&format!("{name} pluck+20dB"), algo, &noisy, hz);
        }
        // Full chromatic sine sweep E2..=E6 for the property-style accuracy bound.
        let mut worst = 0.0f32;
        for key in 40..=88 {
            let hz = midi_hz(key);
            let s = measure(algo, &sine(hz, dur, SR), hz);
            if s.mean_cents.is_finite() {
                worst = worst.max(s.mean_cents);
            }
        }
        println!("sine sweep E2..E6 worst mean |cents|: {worst:.2}\n");
    }
}

fn latency() {
    // A8: a signal that is silent then turns on; count hops until the first confident,
    // stable (within 10 cents of the target for 2 consecutive hops) reading.
    println!("S3 A8 latency (hops to first stable reading)\n");
    println!("{:<8} {:>12} {:>12}", "string", "MPM ms", "YIN ms");
    let silent = (0.2 * SR) as usize;
    let dur = (1.0 * SR) as usize;
    for (name, hz) in OPEN_STRINGS {
        let mut sig = vec![0.0f32; silent];
        sig.extend(plucked(hz, dur, SR));
        let onset = silent;
        let mut row = [f32::NAN; 2];
        for (ai, algo) in [Algo::Mpm, Algo::Yin].into_iter().enumerate() {
            let mut pos = 0usize;
            let mut stable_since: Option<usize> = None;
            let mut prev_ok = false;
            while pos + WIN <= sig.len() {
                let frame = &sig[pos..pos + WIN];
                let ok = detect(algo, frame, SR, GATE, MIN_HZ, MAX_HZ)
                    .hz
                    .map(|h| cents(h, hz).abs() <= 10.0)
                    .unwrap_or(false);
                if ok && prev_ok {
                    stable_since = Some(pos);
                    break;
                }
                prev_ok = ok;
                pos += HOP;
            }
            if let Some(end) = stable_since {
                // The reading is confirmed when the window's trailing edge is past onset.
                let ms = ((end + WIN).saturating_sub(onset)) as f32 / SR * 1000.0;
                row[ai] = ms;
            }
        }
        println!("{name:<8} {:>12.1} {:>12.1}", row[0], row[1]);
    }
}

fn wav(dir: &str) {
    // A9 on real-guitar takes. File name convention: <STRING>[_...].wav, e.g. E2_soft.wav,
    // A2_capo2.wav. The leading token before '_' or '.' must be an open-string name.
    let path = std::path::Path::new(dir);
    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => {
            println!("no directory '{dir}'. Drop 16-bit PCM WAVs named <STRING>_<tag>.wav there.");
            println!("real-guitar numbers are REQUIRED before ADR-029 leaves Draft (s.4).");
            return;
        }
    };
    let refs: std::collections::HashMap<&str, f32> = OPEN_STRINGS.iter().copied().collect();
    let mut found = 0;
    println!("S3 A9 on real guitar ({dir})\n");
    println!("{:<24} {:<5} {:>8} {:>8} {:>7} {:>7}", "file", "algo", "mean c", "p95 c", "oct%", "det%");
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.to_lowercase().ends_with(".wav") {
            continue;
        }
        let token = name.split(['_', '.']).next().unwrap_or("");
        let Some(&reference) = refs.get(token) else {
            println!("{name}: skipped (name must start with an open-string token E2/A2/D3/G3/B3/E4)");
            continue;
        };
        let bytes = match std::fs::read(e.path()) {
            Ok(b) => b,
            Err(err) => {
                println!("{name}: read error {err}");
                continue;
            }
        };
        let (samples, sr) = match read_wav(&bytes) {
            Ok(x) => x,
            Err(err) => {
                println!("{name}: {err}");
                continue;
            }
        };
        if (sr as f32 - SR).abs() > 1.0 {
            println!("{name}: {sr} Hz (resample to 48000 for a clean comparison; measuring anyway)");
        }
        found += 1;
        for algo in [Algo::Mpm, Algo::Yin] {
            // Reuse `measure` with the file's own sample rate by overriding SR via closure
            // would need refactor; the fixtures are expected at 48 kHz (noted above).
            let s = measure(algo, &samples, reference);
            println!(
                "{name:<24} {:<5?} {:>8.2} {:>8.2} {:>6.1}% {:>6.1}%",
                algo, s.mean_cents, s.p95_cents, s.octave_rate * 100.0, s.detect_rate * 100.0
            );
        }
    }
    if found == 0 {
        println!("\nNo usable WAVs found. Real-guitar evidence is pending (ADR-029 s.5).");
    }
}
