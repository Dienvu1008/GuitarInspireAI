//! Synthetic test signals and a minimal 16-bit PCM WAV reader for real-guitar takes.
//! Throwaway spike code (ADR-029 s.6).

use std::f32::consts::PI;

/// The six open-string frequencies of standard tuning (E2..E4), plus the full chromatic
/// range E2..=E6 for the sine-sweep accuracy check.
pub const OPEN_STRINGS: [(&str, f32); 6] = [
    ("E2", 82.4069),
    ("A2", 110.0000),
    ("D3", 146.8324),
    ("G3", 195.9977),
    ("B3", 246.9417),
    ("E4", 329.6276),
];

/// MIDI key to frequency (A4 = 69 = 440 Hz).
pub fn midi_hz(key: i32) -> f32 {
    440.0 * 2f32.powf((key as f32 - 69.0) / 12.0)
}

/// A pure sine of `hz` for `n` samples at `sr`.
pub fn sine(hz: f32, n: usize, sr: f32) -> Vec<f32> {
    (0..n).map(|i| (2.0 * PI * hz * i as f32 / sr).sin() * 0.5).collect()
}

/// A harmonic-rich tone approximating a plucked string: a few decaying harmonics. Not a
/// real guitar, but closer than a sine for the detector's behaviour on overtones.
pub fn plucked(hz: f32, n: usize, sr: f32) -> Vec<f32> {
    let harmonics = [1.0f32, 0.6, 0.4, 0.25, 0.15, 0.1];
    let mut out = vec![0.0f32; n];
    for (h, amp) in harmonics.iter().enumerate() {
        let f = hz * (h as f32 + 1.0);
        if f >= sr / 2.0 {
            break;
        }
        for (i, s) in out.iter_mut().enumerate() {
            let t = i as f32 / sr;
            let decay = (-3.0 * t * (h as f32 + 1.0)).exp(); // higher harmonics decay faster
            *s += amp * decay * (2.0 * PI * f * t).sin();
        }
    }
    let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-6);
    for s in out.iter_mut() {
        *s = *s / peak * 0.5;
    }
    out
}

/// Add white noise at a given signal-to-noise ratio in dB (deterministic LCG, so runs
/// are reproducible).
pub fn add_noise(signal: &[f32], snr_db: f32, seed: u64) -> Vec<f32> {
    let sig_power = signal.iter().map(|s| s * s).sum::<f32>() / signal.len().max(1) as f32;
    let noise_power = sig_power / 10f32.powf(snr_db / 10.0);
    let noise_amp = noise_power.sqrt();
    let mut state = seed | 1;
    let mut next = || {
        // xorshift64
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        // to -1..1
        (state as f64 / u64::MAX as f64 * 2.0 - 1.0) as f32
    };
    signal.iter().map(|s| s + next() * noise_amp * 1.732).collect()
}

/// Minimal 16-bit PCM WAV reader (mono or stereo -> mono mix). Returns (samples, sr).
/// Enough for real-guitar takes the developer drops under results/.
pub fn read_wav(bytes: &[u8]) -> Result<(Vec<f32>, u32), String> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let mut i = 12;
    let mut channels = 1u16;
    let mut sample_rate = 44100u32;
    let mut bits = 16u16;
    let mut data: Option<&[u8]> = None;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let size = u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]])
            as usize;
        let body = i + 8;
        if id == b"fmt " && body + 16 <= bytes.len() {
            channels = u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]);
            sample_rate = u32::from_le_bytes([
                bytes[body + 4],
                bytes[body + 5],
                bytes[body + 6],
                bytes[body + 7],
            ]);
            bits = u16::from_le_bytes([bytes[body + 14], bytes[body + 15]]);
        } else if id == b"data" {
            data = Some(&bytes[body..(body + size).min(bytes.len())]);
        }
        i = body + size + (size & 1);
    }
    let data = data.ok_or("no data chunk")?;
    if bits != 16 {
        return Err(format!("only 16-bit PCM supported, got {bits}-bit"));
    }
    let ch = channels.max(1) as usize;
    let frames = data.len() / 2 / ch;
    let mut out = Vec::with_capacity(frames);
    for f in 0..frames {
        let mut acc = 0.0f32;
        for c in 0..ch {
            let o = (f * ch + c) * 2;
            let s = i16::from_le_bytes([data[o], data[o + 1]]);
            acc += s as f32 / 32768.0;
        }
        out.push(acc / ch as f32);
    }
    Ok((out, sample_rate))
}
