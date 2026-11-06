//! S3 spike: MPM and YIN monophonic pitch detectors. Throwaway (ADR-029 s.6); not
//! imported by mc-dsp. Written close to the production shape (fixed window, parabolic
//! interpolation, a clarity value) so the measured behaviour transfers, but it allocates
//! freely here; 5b reimplements allocation-free.

use realfft::RealFftPlanner;
use realfft::num_complex::Complex32;

/// A pitch estimate: frequency in Hz and a clarity in 0..=1 (confidence). `None` hz
/// means no confident pitch at this window.
#[derive(Clone, Copy, Debug)]
pub struct Estimate {
    pub hz: Option<f32>,
    // Kept in the data model (the product tuner gates on it); not all spike modes read it.
    #[allow(dead_code)]
    pub clarity: f32,
}

/// Which algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algo {
    Mpm,
    Yin,
}

/// Detect pitch in `frame` at `sample_rate`. `clarity_gate` is the minimum clarity for a
/// confident reading; below it, `hz` is `None`. `min_hz`/`max_hz` bound the lag search.
pub fn detect(
    algo: Algo,
    frame: &[f32],
    sample_rate: f32,
    clarity_gate: f32,
    min_hz: f32,
    max_hz: f32,
) -> Estimate {
    match algo {
        Algo::Mpm => mpm(frame, sample_rate, clarity_gate, min_hz, max_hz),
        Algo::Yin => yin(frame, sample_rate, clarity_gate, min_hz, max_hz),
    }
}

/// RMS of the frame, to reject silence before running a detector.
pub fn rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt()
}

// ---------------------------------------------------------------- MPM

/// McLeod Pitch Method: the normalized square difference function (NSDF) computed from
/// the autocorrelation (via FFT) and the signal's squared running sum, then the first
/// key maximum above a fraction of the global max, with parabolic interpolation.
fn mpm(frame: &[f32], sr: f32, gate: f32, min_hz: f32, max_hz: f32) -> Estimate {
    let n = frame.len();
    let acf = autocorrelation(frame);
    // m(tau) = sum_{j} x[j]^2 + x[j+tau]^2, computed incrementally from both ends.
    let mut nsdf = vec![0.0f32; n];
    // Total energy for tau = 0.
    let mut energy: f32 = frame.iter().map(|s| s * s).sum();
    let mut m = 2.0 * energy;
    nsdf[0] = if m > 0.0 { 2.0 * acf[0] / m } else { 0.0 };
    for tau in 1..n {
        // Remove the two samples that leave the overlap as tau grows.
        energy -= frame[n - tau] * frame[n - tau];
        // m(tau) = sum_{j=0}^{n-1-tau} x[j]^2 + x[j+tau]^2.
        // Recompute m incrementally: subtract the tail squares.
        m = running_m(frame, tau);
        nsdf[tau] = if m > 0.0 { 2.0 * acf[tau] / m } else { 0.0 };
    }
    let _ = energy;

    let min_lag = (sr / max_hz).floor().max(1.0) as usize;
    let max_lag = (sr / min_hz).ceil().min((n - 1) as f32) as usize;

    // MPM: skip the initial lobe at lag 0 (the signal always correlates with itself at
    // small shifts). Start collecting key maxima only AFTER the NSDF first goes negative;
    // otherwise a low-pitched signal's broad first plateau yields a spurious short-lag
    // peak. Then within each positive lobe, take its peak.
    let mut pos = min_lag;
    while pos < max_lag && nsdf[pos] > 0.0 {
        pos += 1; // walk past the initial positive region
    }
    let mut peaks: Vec<(usize, f32)> = Vec::new();
    while pos < max_lag {
        while pos < max_lag && nsdf[pos] <= 0.0 {
            pos += 1; // to the next positively sloped zero crossing
        }
        let mut best_lag = pos;
        let mut best_val = f32::MIN;
        while pos < max_lag && nsdf[pos] > 0.0 {
            if nsdf[pos] > best_val {
                best_val = nsdf[pos];
                best_lag = pos;
            }
            pos += 1;
        }
        if best_val > f32::MIN && best_lag >= min_lag {
            peaks.push((best_lag, best_val));
        }
    }
    if peaks.is_empty() {
        return Estimate { hz: None, clarity: 0.0 };
    }
    let global = peaks.iter().map(|p| p.1).fold(f32::MIN, f32::max);
    // The first key maximum at least k * global (k = 0.9 is the common MPM cutoff).
    let cutoff = 0.9 * global;
    let (lag, clarity) = peaks
        .iter()
        .copied()
        .find(|&(_, v)| v >= cutoff)
        .unwrap_or(peaks[0]);
    let refined = parabolic(&nsdf, lag);
    let hz = sr / refined;
    Estimate {
        hz: (clarity >= gate && hz.is_finite()).then_some(hz),
        clarity: clarity.clamp(0.0, 1.0),
    }
}

/// m(tau) for the NSDF denominator: sum over the overlap of x[j]^2 + x[j+tau]^2.
fn running_m(x: &[f32], tau: usize) -> f32 {
    let n = x.len();
    let mut s = 0.0f32;
    for j in 0..n - tau {
        s += x[j] * x[j] + x[j + tau] * x[j + tau];
    }
    s
}

/// Autocorrelation r(tau) for tau in 0..n via FFT (zero-padded to 2n).
fn autocorrelation(x: &[f32]) -> Vec<f32> {
    let n = x.len();
    let fft_len = (2 * n).next_power_of_two();
    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(fft_len);
    let c2r = planner.plan_fft_inverse(fft_len);
    let mut input = r2c.make_input_vec();
    for (i, v) in input.iter_mut().enumerate() {
        *v = if i < n { x[i] } else { 0.0 };
    }
    let mut spectrum = r2c.make_output_vec();
    r2c.process(&mut input, &mut spectrum).unwrap();
    // Power spectrum -> autocorrelation.
    for c in spectrum.iter_mut() {
        *c = Complex32::new(c.norm_sqr(), 0.0);
    }
    let mut out = c2r.make_output_vec();
    c2r.process(&mut spectrum, &mut out).unwrap();
    let scale = 1.0 / fft_len as f32;
    out.iter().take(n).map(|v| v * scale).collect()
}

// ---------------------------------------------------------------- YIN

/// YIN: cumulative mean normalized difference function, absolute threshold 0.1, first dip
/// below it, parabolic interpolation. Clarity reported as 1 - d'(tau).
fn yin(frame: &[f32], sr: f32, gate: f32, min_hz: f32, max_hz: f32) -> Estimate {
    let n = frame.len();
    let max_lag = (sr / min_hz).ceil().min((n / 2) as f32) as usize;
    let min_lag = (sr / max_hz).floor().max(1.0) as usize;
    let mut d = vec![0.0f32; max_lag + 1];
    for tau in 1..=max_lag {
        let mut sum = 0.0f32;
        for j in 0..n - tau {
            let diff = frame[j] - frame[j + tau];
            sum += diff * diff;
        }
        d[tau] = sum;
    }
    // Cumulative mean normalized difference.
    let mut dp = vec![0.0f32; max_lag + 1];
    dp[0] = 1.0;
    let mut running = 0.0f32;
    for tau in 1..=max_lag {
        running += d[tau];
        dp[tau] = if running > 0.0 {
            d[tau] * tau as f32 / running
        } else {
            1.0
        };
    }
    const THRESHOLD: f32 = 0.1;
    let mut tau = min_lag.max(1);
    let mut chosen = None;
    while tau <= max_lag {
        if dp[tau] < THRESHOLD {
            while tau + 1 <= max_lag && dp[tau + 1] < dp[tau] {
                tau += 1;
            }
            chosen = Some(tau);
            break;
        }
        tau += 1;
    }
    match chosen {
        Some(lag) => {
            let refined = parabolic_min(&dp, lag);
            let hz = sr / refined;
            let clarity = (1.0 - dp[lag]).clamp(0.0, 1.0);
            Estimate {
                hz: (clarity >= gate && hz.is_finite()).then_some(hz),
                clarity,
            }
        }
        None => Estimate { hz: None, clarity: 0.0 },
    }
}

// ---------------------------------------------------------------- shared

/// Parabolic interpolation around a maximum at `lag` (sub-sample lag).
fn parabolic(y: &[f32], lag: usize) -> f32 {
    if lag == 0 || lag + 1 >= y.len() {
        return lag as f32;
    }
    let (a, b, c) = (y[lag - 1], y[lag], y[lag + 1]);
    let denom = a - 2.0 * b + c;
    if denom.abs() < 1e-9 {
        lag as f32
    } else {
        lag as f32 + 0.5 * (a - c) / denom
    }
}

/// Parabolic interpolation around a minimum at `lag`.
fn parabolic_min(y: &[f32], lag: usize) -> f32 {
    if lag == 0 || lag + 1 >= y.len() {
        return lag as f32;
    }
    let (a, b, c) = (y[lag - 1], y[lag], y[lag + 1]);
    let denom = a - 2.0 * b + c;
    if denom.abs() < 1e-9 {
        lag as f32
    } else {
        lag as f32 - 0.5 * (a - c) / denom
    }
}

/// Cents between a measured and a reference frequency.
pub fn cents(measured: f32, reference: f32) -> f32 {
    1200.0 * (measured / reference).log2()
}
