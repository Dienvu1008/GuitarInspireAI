//! Deterministic DSP (GR-03): a monophonic pitch detector for the tuner (ADR-029).
//!
//! [`PitchDetector`] implements the McLeod Pitch Method (MPM): the normalized square
//! difference function (NSDF) from the autocorrelation (via a real FFT) and the signal's
//! running square sum, then the first key maximum above a fraction of the global peak,
//! with parabolic interpolation for a sub-sample lag. S3 chose MPM over YIN on the
//! synthetic set (ADR-029 §5); the real-guitar confirmation is recorded in that ADR.
//!
//! Everything the hot path needs is preallocated in [`PitchDetector::new`]; [`process`]
//! does no heap allocation (checked by the `no_alloc` test, the precondition for the
//! tuner thread to run this off the input ring, ADR-029 §7). Pure: no clocks, no I/O, no
//! randomness, no hash-map iteration order; compiles for wasm32 (FF-04).
//!
//! [`process`]: PitchDetector::process

use realfft::num_complex::Complex32;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::sync::Arc;

/// A pitch estimate for one analysis window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Estimate {
    /// Detected frequency in Hz, or `None` when no confident pitch is present (clarity
    /// below the gate, or the frame is below the silence floor).
    pub hz: Option<f32>,
    /// Confidence in `0.0..=1.0` (the NSDF peak height). Reported even when `hz` is
    /// `None`, so the UI can show "listening" versus "no signal".
    pub clarity: f32,
}

/// How the detector is configured. Built with sensible guitar defaults; the tuner can
/// widen the range or change the clarity gate from the S3 findings.
#[derive(Clone, Copy, Debug)]
pub struct DetectorConfig {
    /// Analysis window length in samples (power of two recommended; 2048 @ 48 kHz).
    pub window: usize,
    pub sample_rate: f32,
    /// Lowest detectable frequency; bounds the autocorrelation lag search.
    pub min_hz: f32,
    /// Highest detectable frequency.
    pub max_hz: f32,
    /// Minimum clarity for a confident reading (0..=1). Below it, `hz` is `None`.
    pub clarity_gate: f32,
    /// RMS below this is treated as silence (no estimate).
    pub silence_floor: f32,
}

impl DetectorConfig {
    /// Guitar tuner defaults: 2048 @ 48 kHz, 70..1400 Hz (E2 well covered), gate 0.6.
    pub fn guitar(sample_rate: f32) -> Self {
        DetectorConfig {
            window: 2048,
            sample_rate,
            min_hz: 70.0,
            max_hz: 1400.0,
            clarity_gate: 0.6,
            silence_floor: 1e-3,
        }
    }
}

/// Allocation-free MPM pitch detector over a fixed window. Construct once, reuse for
/// every window; [`process`](Self::process) allocates nothing.
pub struct PitchDetector {
    config: DetectorConfig,
    fft_len: usize,
    forward: Arc<dyn RealToComplex<f32>>,
    inverse: Arc<dyn ComplexToReal<f32>>,
    // Preallocated buffers, all sized in `new`.
    real_in: Vec<f32>,
    spectrum: Vec<Complex32>,
    fwd_scratch: Vec<Complex32>,
    inv_scratch: Vec<Complex32>,
    acf: Vec<f32>,
    nsdf: Vec<f32>,
    min_lag: usize,
    max_lag: usize,
}

impl std::fmt::Debug for PitchDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PitchDetector")
            .field("window", &self.config.window)
            .field("sample_rate", &self.config.sample_rate)
            .finish_non_exhaustive()
    }
}

impl PitchDetector {
    /// Build a detector for `config`, preallocating the FFT plans and every scratch and
    /// output buffer. This allocates; [`process`](Self::process) does not.
    pub fn new(config: DetectorConfig) -> Self {
        let n = config.window.max(2);
        let fft_len = (2 * n).next_power_of_two();
        let mut planner = RealFftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(fft_len);
        let inverse = planner.plan_fft_inverse(fft_len);
        let real_in = forward.make_input_vec();
        let spectrum = forward.make_output_vec();
        let fwd_scratch = forward.make_scratch_vec();
        let inv_scratch = inverse.make_scratch_vec();
        let min_lag = (config.sample_rate / config.max_hz).floor().max(1.0) as usize;
        let max_lag = (config.sample_rate / config.min_hz)
            .ceil()
            .min((n - 1) as f32) as usize;
        PitchDetector {
            config,
            fft_len,
            forward,
            inverse,
            real_in,
            spectrum,
            fwd_scratch,
            inv_scratch,
            acf: vec![0.0; n],
            nsdf: vec![0.0; n],
            min_lag,
            max_lag,
        }
    }

    pub fn config(&self) -> &DetectorConfig {
        &self.config
    }

    /// RMS of a frame (public so callers can gate on signal level before calling).
    pub fn rms(frame: &[f32]) -> f32 {
        if frame.is_empty() {
            return 0.0;
        }
        (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt()
    }

    /// Estimate the pitch of one window. `frame` must be exactly `config.window` samples.
    /// Allocation-free: all buffers were sized in [`new`](Self::new).
    pub fn process(&mut self, frame: &[f32]) -> Estimate {
        debug_assert_eq!(
            frame.len(),
            self.config.window,
            "frame length must equal window"
        );
        if Self::rms(frame) < self.config.silence_floor {
            return Estimate {
                hz: None,
                clarity: 0.0,
            };
        }
        self.autocorrelation(frame);
        self.build_nsdf(frame);
        self.pick_pitch()
    }

    /// Autocorrelation `acf[tau]` for `tau in 0..n` via FFT (zero-padded). Reuses the
    /// preallocated plans, spectrum and scratch; no allocation.
    fn autocorrelation(&mut self, frame: &[f32]) {
        let n = self.config.window;
        for (i, v) in self.real_in.iter_mut().enumerate() {
            *v = if i < n { frame[i] } else { 0.0 };
        }
        self.forward
            .process_with_scratch(&mut self.real_in, &mut self.spectrum, &mut self.fwd_scratch)
            .expect("fft forward");
        // Power spectrum -> autocorrelation via the inverse transform.
        for c in self.spectrum.iter_mut() {
            *c = Complex32::new(c.norm_sqr(), 0.0);
        }
        self.inverse
            .process_with_scratch(&mut self.spectrum, &mut self.real_in, &mut self.inv_scratch)
            .expect("fft inverse");
        let scale = 1.0 / self.fft_len as f32;
        for tau in 0..n {
            self.acf[tau] = self.real_in[tau] * scale;
        }
    }

    /// NSDF from the autocorrelation and the running square-sum `m(tau)`, computed
    /// incrementally in O(n): `m(0) = 2 Σ x²`, `m(tau) = m(tau-1) - x[tau-1]² - x[n-tau]²`.
    fn build_nsdf(&mut self, frame: &[f32]) {
        let n = self.config.window;
        let mut m: f32 = 2.0 * frame.iter().map(|s| s * s).sum::<f32>();
        self.nsdf[0] = if m > 0.0 { 2.0 * self.acf[0] / m } else { 0.0 };
        for tau in 1..n {
            m -= frame[tau - 1] * frame[tau - 1] + frame[n - tau] * frame[n - tau];
            if m <= 0.0 {
                self.nsdf[tau] = 0.0;
            } else {
                self.nsdf[tau] = 2.0 * self.acf[tau] / m;
            }
        }
    }

    /// Key-maximum pick (MPM): skip the initial positive lobe at lag 0, then take the
    /// peak of each positive lobe; choose the first lobe peak at least 0.9 of the global
    /// peak; refine with parabolic interpolation. No allocation (single pass, no Vec).
    fn pick_pitch(&self) -> Estimate {
        let nsdf = &self.nsdf;
        let (min_lag, max_lag) = (self.min_lag, self.max_lag);
        if max_lag <= min_lag {
            return Estimate {
                hz: None,
                clarity: 0.0,
            };
        }
        // Walk past the initial positive region at lag 0 (the signal always correlates
        // with itself at small shifts). Start from lag 1, NOT from min_lag: for a short
        // true period, min_lag can sit inside the first true lobe, and skipping from
        // there would consume the correct peak and land on the 2x sub-harmonic (an octave
        // low). `min_lag` is applied below as a floor on candidate peak lags instead.
        let mut pos = 1usize;
        while pos < max_lag && nsdf[pos] > 0.0 {
            pos += 1;
        }
        // First pass: the global maximum among lobe peaks (for the 0.9 cutoff).
        let mut global = f32::MIN;
        let mut p = pos;
        while p < max_lag {
            while p < max_lag && nsdf[p] <= 0.0 {
                p += 1;
            }
            let mut lobe_max = f32::MIN;
            while p < max_lag && nsdf[p] > 0.0 {
                if nsdf[p] > lobe_max {
                    lobe_max = nsdf[p];
                }
                p += 1;
            }
            if lobe_max > global {
                global = lobe_max;
            }
        }
        if global <= f32::MIN {
            return Estimate {
                hz: None,
                clarity: 0.0,
            };
        }
        let cutoff = 0.9 * global;
        // Second pass: the first lobe whose peak reaches the cutoff; its peak lag.
        let mut p = pos;
        while p < max_lag {
            while p < max_lag && nsdf[p] <= 0.0 {
                p += 1;
            }
            let mut lobe_lag = p;
            let mut lobe_max = f32::MIN;
            while p < max_lag && nsdf[p] > 0.0 {
                if nsdf[p] > lobe_max {
                    lobe_max = nsdf[p];
                    lobe_lag = p;
                }
                p += 1;
            }
            if lobe_max >= cutoff && lobe_lag >= min_lag {
                let refined = self.parabolic(lobe_lag);
                let hz = self.config.sample_rate / refined;
                let clarity = lobe_max.clamp(0.0, 1.0);
                return Estimate {
                    hz: (clarity >= self.config.clarity_gate && hz.is_finite() && hz > 0.0)
                        .then_some(hz),
                    clarity,
                };
            }
        }
        Estimate {
            hz: None,
            clarity: 0.0,
        }
    }

    /// Parabolic interpolation around the maximum at `lag` for a sub-sample lag.
    fn parabolic(&self, lag: usize) -> f32 {
        if lag == 0 || lag + 1 >= self.nsdf.len() {
            return lag as f32;
        }
        let (a, b, c) = (self.nsdf[lag - 1], self.nsdf[lag], self.nsdf[lag + 1]);
        let denom = a - 2.0 * b + c;
        if denom.abs() < 1e-9 {
            lag as f32
        } else {
            lag as f32 + 0.5 * (a - c) / denom
        }
    }
}

/// Cents between a measured and a reference frequency (both > 0).
pub fn cents(measured: f32, reference: f32) -> f32 {
    1200.0 * (measured / reference).log2()
}
