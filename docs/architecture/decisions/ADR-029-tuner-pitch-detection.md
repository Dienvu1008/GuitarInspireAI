# ADR-029 Tuner pitch detection: owned MPM/YIN, thresholds and S3 results

Status: Draft (thresholds fixed before S3 runs, 2026-10-31). Becomes Accepted when the S3 results and the algorithm decision are filled in (section 5).
Date: 2026-10-31
Related documents: [audio-dsp-architecture.md](../audio-dsp-architecture.md), [technology-evaluation.md](../../research/technology-evaluation.md), [technical-spikes.md](../../research/technical-spikes.md), [performance-requirements.md](../performance-requirements.md), [ADR-007](ADR-007-audio-thread-isolation.md), [ADR-025](ADR-025-single-machine-evidence-accepted.md), [risk-register.md](../../roadmap/risk-register.md)

## Context

Phase 5 adds a guitar tuner. `mc-dsp` will own an allocation-free pitch detector; the tech evaluation already decided to implement a small owned YIN/MPM rather than use the unmaintained `pitch-detection` crate, with `realfft`/`rustfft` as the only external DSP dependencies. Spike S3 (budgeted inside Phase 5) answers the open feasibility assumptions A8 to A10 from [audio-dsp-architecture.md](../audio-dsp-architecture.md). The tuner's hard risk is stability on the low strings and with a real, noisy signal (R-11).

This ADR is written as a **draft before S3 runs** so the pass/fail thresholds and the MPM-vs-YIN decision rule are fixed in advance and the decision cannot become subjective after seeing numbers (the spike discipline in [technical-spikes.md](../../research/technical-spikes.md): a failed criterion is recorded as Fail with its measured value, not tuned away).

## Decision

### 1. Algorithm and the MPM-vs-YIN decision rule

Implement **MPM** (McLeod Pitch Method: normalized square difference via autocorrelation, with `realfft`) as the primary detector. Also implement **YIN** (difference function with the cumulative-mean-normalized threshold) as a fallback. S3 measures both on the same inputs and the production choice (5b) follows this rule, decided only after the numbers exist:

> Prefer MPM. Choose YIN instead only if, on the low strings **E2 (82.4 Hz)** and **A2 (110.0 Hz)** of the real-guitar set, MPM's **mean absolute error exceeds 10 cents OR its p95 absolute error exceeds 25 cents OR its detection rate is below 90% of voiced frames**, while YIN meets all three. If both fail the low strings, neither is shipped for those strings and the MVP scope is reduced (section 3), not the thresholds.

"Tệ hơn" is thus defined numerically per the thresholds in section 2, not by impression.

### 2. Pass/fail thresholds for S3 (A8 to A10), fixed before the run

All measured on the reference machine; single-machine evidence (ADR-025). Per open string (E2, A2, D3, G3, B3, E4) and summarized:

| Id | Metric | Pass threshold |
|---|---|---|
| A9-cents-mean | Mean absolute pitch error, steady state, per string | <= 2 cents on D3..E4; <= 5 cents on E2, A2 (low strings allowed a wider band) |
| A9-cents-p95 | p95 absolute pitch error, per string | <= 10 cents on D3..E4; <= 25 cents on E2, A2 |
| A9-octave | Octave-error rate (estimate off by ~1200 cents), per string | <= 1% of voiced frames; **0% on E2, A2** after stabilization |
| A9-detect | Detection rate: voiced frames with a confident estimate (clarity above gate) on a real plucked note held ~2 s | >= 90% of the steady-state window (attack and release excluded) |
| A8-latency | Time from a step in the input to the first stable reading | <= ~100 ms end to end (window 2048 @ 48 kHz is ~43 ms; plus hop and coalescing) |
| A10-permission | Microphone permission denied | Typed error, no crash; UI shows a clear message; chord finder and fretboard still work |
| A10-busy/none | Device busy or no input device | Typed error, no crash; same isolation |

Reading cadence and smoothing: windows of 2048 samples at 48 kHz, hop 512; UI readings coalesced to ~25 Hz; below the clarity gate the UI shows no confident pitch ("—"). The clarity gate value is chosen from S3 (the lowest gate that keeps the octave-error and detection thresholds on E2/A2).

If a threshold is not met, the result is recorded as Fail with its number and the MVP scope is adjusted (section 3). The thresholds are **not** lowered to match a result.

### 3. Fallback scope if the low strings fail

If neither MPM nor YIN meets the E2/A2 thresholds on real guitar, the MVP tuner ships for the strings that do pass (typically D3 and up) and the known-limitations list says the low E and A are unreliable on this input path, with a follow-up (window size, a low-string-specific analysis, or a cleaner-signal requirement). This is a scope reduction, decided with the user after the numbers, not a threshold change.

### 4. Test data for S3 (honesty about sources)

S3 must be measured on **both**:

- **Synthetic signals** (sine, and a harmonic-rich saw/guitar-like tone) at the exact open-string frequencies across E2..E6, with added white noise at a few SNRs. These are generated in the spike and give a reproducible lower bound; they are not sufficient on their own.
- **Real guitar recordings** of all six open strings, at several dynamics (soft to hard pick), with and without a capo, recorded on the developer's instrument and audio interface. Each take's source (instrument, interface, room, sample rate) and the number of samples per string are recorded in section 5 and under `spikes/s3-tuner/results/`.

Constraint recorded honestly: the coding agent cannot record from a physical instrument. The spike provides the capture harness and the analysis script; the **developer records the real-guitar set and runs the analysis on the reference machine**, then the measured numbers are filled into section 5. Until the real-guitar numbers exist, section 5 states "synthetic only, real-guitar pending" and the ADR stays Draft. No real-guitar number is invented.

### 5. S3 results

Status: **synthetic and latency measured; real-guitar pending**; this ADR stays Draft until the real-guitar numbers (§4) are filled in. Reference machine (ADR-025). Window 2048 @ 48 kHz, hop 512, clarity gate 0.5 (provisional). Raw output in `spikes/s3-tuner/results/`.

A spike bug was found and fixed before these numbers: MPM's key-maximum search did not skip the initial lobe at lag 0, so a low-pitched signal returned a spurious short lag (E2 read as ~427 kHz). After skipping the initial positive region before collecting key maxima, MPM is correct across the range. This is a spike-implementation detail; 5b includes the same initial-lobe skip. A second refinement was needed in 5b (not visible in the spike's open-string run, caught by the production crate's chromatic sine-sweep property test): the initial-lobe skip must start from lag 1, not from `min_lag`, because for a short true period (high notes, ~1.1 kHz and up) `min_lag` can sit inside the first true lobe and skipping from there lands on the 2x sub-harmonic (an octave low). `min_lag` is instead applied as a floor on candidate peak lags. With both fixes the `mc-dsp` detector is within ~0.02 cents across E2..E6 on synthetic signals and allocation-free per window.

**A9 — synthetic accuracy (mean / p95 absolute cents; octave-error %; detection %):**

| String | MPM clean (sine/pluck) | MPM pluck +20 dB | YIN clean | YIN pluck +20 dB |
|---|---|---|---|---|
| E2 | 0.02–0.12 / <0.5; 0% oct; 100% | 8.5 / 39.7; 0% oct; 77% | 2.8 / ~3; 100% | 28.7 / 87.8; 0% oct; 51% |
| A2 | 0.01–0.18 / <0.4; 0% oct; 100% | 8.7 / 41.4; 0% oct; 78% | 2.9 / ~3; 100% | 21.7 / 71.5; 51% |
| D3 | 0.01–0.10; 100% | 7.3 / 27.7; 78% | 1.0; 100% | 13.8 / 49.6; 52% |
| G3 | 0.00–0.04; 100% | 8.5 / 29.6; 77% | 1.4; 100% | 8.3 / 45.3; 51% |
| B3 | 0.01; 100% | 6.3 / 24.5; 78% | 6.7; 100% | 9.4 / 23.4; 52% |
| E4 | 0.00–0.01; 100% | 6.2 / 39.7; 77% | 9.1; 100% | 11.0 / 23.4; 53% |
| sweep E2..E6 worst mean | **0.02 cents** | — | 38.57 cents | — |

**A8 — latency to first stable reading (ms):** MPM 24–35, YIN 35–56. Both pass (≤ ~100 ms).

**A10 — input path:** a default input device opened (48 kHz, 2 ch); the typed outcome is `Ok` / `NoInputDevice` / `PermissionDenied` / `DeviceBusyOrConfig`, and a stream error surfaced as a typed error, not a crash. The permission-denied and device-busy cases are in the MVP manual script (toggle Windows mic privacy; hold the device in another app).

**Provisional recommendation: MPM.** On synthetic signals MPM meets every §2 threshold (E2/A2 mean ≤ 5 cents, p95 ≤ 25, 0% octave, 100% clean detection) and is far more accurate than YIN across the range (sweep worst 0.02 vs 38.57 cents), with lower latency. YIN is steadier only in that its clean error is uniform, but it is worse everywhere and shows harmonic/octave errors high up and poor detection under noise. Per the §1 rule, MPM is not "worse" on E2/A2, so MPM is chosen — **provisionally**, because the §1 rule requires the real-guitar set, which is still pending. Noise robustness (detection ~77% at 20 dB SNR, octave-safe) is acceptable for a tuner where the user holds a note; the clarity gate and hop can be tuned in 5b against the real takes.

**Data sources:** synthetic signals generated in the spike (sine and a 6-harmonic decaying pluck, ± white noise at 20 dB SNR, deterministic). **Real guitar: not yet recorded — the agent cannot capture a physical instrument (§4).** The capture harness (`wav` mode) and instructions are in `spikes/s3-tuner/results/README.md`; the developer records the set on the reference instrument and fills the real-guitar table here before this ADR is Accepted. No real-guitar number is invented.

### 6. Spike is throwaway

S3 code lives in `spikes/s3-tuner/` with its own workspace and lockfile, excluded from the production workspace, exactly as S2 (`spikes/s2-audio/`). It is **not** imported into `mc-dsp` or any workspace crate. 5b reimplements the chosen algorithm against the architecture (FF-03 purity, FF-04 wasm, allocation-free with preallocated buffers), guided by the S3 findings but not sharing its code. Raw S3 output stays under `spikes/s3-tuner/results/`.

### 7. RT and threading for the production tuner (5c, confirms audio-dsp section 4)

The tuner input path reuses the existing engine ownership model (ADR-007): the cpal **input** callback only copies the mono mix into a preallocated SPSC ring (rtrb) and does nothing else (GR-10, FF-17 rules apply to input RT code too). A separate non-real-time **tuner thread** reads windows from the ring and runs `mc-dsp` with preallocated buffers; it may allocate at construction but not per window. The tuner thread is promoted with the **existing** `mmcss.rs` call (ADR-027) — **no new `unsafe`** is added; FF-34 still sees exactly the two reviewed exceptions. The input stream is opened only while the tuner is visible and closed when it is hidden (microphone privacy, security-and-privacy). Device/permission failures surface as typed errors (A10).

## Alternatives

- The `pitch-detection` crate: unmaintained (0.3.0, 2022) and not allocation-free with our preallocated-buffer requirement; used as a reference only.
- A single algorithm (MPM only): rejected for the spike because the low-string risk (R-11) is exactly where MPM and YIN differ; measuring both is cheap in a spike and makes the choice evidence-based.
- FFT-only (peak picking): weaker on the low strings at a 43 ms window; autocorrelation-family methods (MPM/YIN) are the standard choice for monophonic pitch.

## Trade-offs

Implementing two detectors in the spike costs a little more spike time but removes guesswork from the production choice. A wider cent band on E2/A2 than the ideal ±2 cents reflects physical reality (low-string pitch is harder to track at a short window) while still being well within a usable tuner tolerance; the band is fixed in advance, not after the fact.

## Consequences

- `mc-dsp` (Phase 5) implements the chosen detector, allocation-free, with a sine-sweep accuracy property test and an `assert_no_alloc` test over the frame-processing function (the precondition for 5c to run it off the input callback).
- `mc-audio` gains the input capture stream, the tuner thread and typed tuner errors; `audio-dsp-architecture.md` section 4 is updated from Proposed to the implemented design when 5c lands.
- The risk register R-11 entry is updated with the S3 outcome; R-08 (device/permission) is exercised by A10.

## Related fitness functions

FF-03, FF-04 (mc-dsp purity and wasm), FF-07 (platform/input code in the adapter), FF-16/FF-17 (RT rules, including the input callback), FF-34 (no new unsafe), FF-22 (mc-dsp public API snapshot added in 5b).

## Reconsideration conditions

A different input path (ASIO, exclusive mode, a dedicated audio interface) changes the latency or noise floor enough to revisit the window size or thresholds; or a later ML pitch tracker (CREPE class, Phase 8+) supersedes the deterministic tuner for hard cases.
