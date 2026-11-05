//! Microphone tuner (ADR-029 section 7, audio-dsp-architecture section 4). The input
//! callback copies the mono mix into a wait-free ring and does nothing else (GR-10,
//! FF-17 apply to input real-time code too). A separate non-real-time tuner thread reads
//! windows from the ring and runs the allocation-free `mc-dsp` detector, publishing the
//! latest reading to an atomic snapshot the caller polls.
//!
//! Reference pitches (which string, how many cents off) are NOT computed here: those
//! depend on the active track's tuning and capo, a music rule the application owns
//! (GR-05, ADR-029 T8). The tuner reports only a frequency and a clarity.
//!
//! The tuner thread is promoted with the existing MMCSS call ([`crate::mmcss`], ADR-027);
//! no new `unsafe` is added. The input stream is opened by [`Tuner::start`] and closed
//! when the [`Tuner`] is dropped (the microphone is live only while the tuner exists, a
//! privacy requirement).

use crate::error::AudioError;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use mc_dsp::{DetectorConfig, PitchDetector};
use rtrb::RingBuffer;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};
use std::thread::JoinHandle;

/// Latest tuner reading, polled by the caller (~25 Hz coalesced). `hz` is `NaN` when no
/// confident pitch is present; `clarity` is always current so the UI can tell "no
/// signal" from "listening". Updated by the tuner thread, read by any thread.
#[derive(Debug, Default)]
struct Shared {
    hz_bits: AtomicU32,
    clarity_bits: AtomicU32,
    updates: AtomicU64,
    /// Set if the input stream failed after it started (device lost); the caller reads
    /// the message via `last_error`.
    failed: AtomicBool,
}

/// A plain copy of the latest reading for callers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TunerReading {
    /// Detected frequency in Hz, or `None` when there is no confident pitch right now.
    pub hz: Option<f32>,
    /// Detector confidence 0.0..=1.0.
    pub clarity: f32,
    /// Monotonic counter of readings produced, so the caller can tell the tuner is live.
    pub updates: u64,
}

/// A running microphone tuner. Dropping it stops the input stream and joins the thread,
/// releasing the microphone.
pub struct Tuner {
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    // The stream lives on this struct; dropping it stops capture. `cpal::Stream` is not
    // `Send` on every platform, so the `Tuner` is created and dropped on one thread.
    _stream: cpal::Stream,
    sample_rate: u32,
}

impl std::fmt::Debug for Tuner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tuner")
            .field("sample_rate", &self.sample_rate)
            .field("failed", &self.failed())
            .finish_non_exhaustive()
    }
}

impl Tuner {
    /// Open the default input device and start detecting. Returns a typed error (A10)
    /// when there is no microphone, when the OS denies access, or when the device is
    /// busy or offers no usable format; the caller disables the tuner and keeps the rest
    /// of the app working.
    pub fn start() -> Result<Tuner, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(AudioError::NoInputDevice)?;
        let supported = device
            .default_input_config()
            .map_err(|e| AudioError::Stream(e.to_string()))?;
        if supported.sample_format() != cpal::SampleFormat::F32 {
            return Err(AudioError::UnsupportedFormat(format!(
                "{:?}",
                supported.sample_format()
            )));
        }
        let config: cpal::StreamConfig = supported.into();
        let sample_rate = config.sample_rate;
        let channels = config.channels as usize;

        // Ring sized for a few analysis windows so the tuner thread never blocks the
        // input callback. The callback only pushes; it never allocates or waits.
        let capacity = (sample_rate as usize).max(8192);
        let (mut producer, mut consumer) = RingBuffer::<f32>::new(capacity);

        let shared = Arc::new(Shared::default());
        let stop = Arc::new(AtomicBool::new(false));
        let failed = shared.clone();

        let stream = device
            .build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Copy only: mono mix into the ring. Drop samples if the consumer is
                    // behind rather than block (the tuner tolerates gaps).
                    let ch = channels.max(1);
                    for frame in data.chunks(ch) {
                        let mono = frame.iter().copied().sum::<f32>() / ch as f32;
                        let _ = producer.push(mono);
                    }
                },
                move |err: cpal::Error| {
                    failed.failed.store(true, Relaxed);
                    let _ = err;
                },
                None,
            )
            .map_err(|e| classify_build_error(&e.to_string()))?;
        stream
            .play()
            .map_err(|e| AudioError::Stream(e.to_string()))?;

        let worker_shared = shared.clone();
        let worker_stop = stop.clone();
        let thread = std::thread::Builder::new()
            .name("mc-audio-tuner".into())
            .spawn(move || {
                tuner_loop(&mut consumer, sample_rate, &worker_shared, &worker_stop);
            })
            .map_err(|e| AudioError::Stream(e.to_string()))?;

        Ok(Tuner {
            shared,
            stop,
            thread: Some(thread),
            _stream: stream,
            sample_rate,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The latest reading. Cheap; call at the UI rate.
    pub fn reading(&self) -> TunerReading {
        let hz = f32::from_bits(self.shared.hz_bits.load(Relaxed));
        TunerReading {
            hz: hz.is_finite().then_some(hz),
            clarity: f32::from_bits(self.shared.clarity_bits.load(Relaxed)),
            updates: self.shared.updates.load(Relaxed),
        }
    }

    /// `true` if the input stream failed after starting (device lost). The caller then
    /// stops the tuner and shows a message.
    pub fn failed(&self) -> bool {
        self.shared.failed.load(Relaxed)
    }
}

impl Drop for Tuner {
    fn drop(&mut self) {
        self.stop.store(true, Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        // `_stream` drops here, stopping capture and releasing the microphone.
    }
}

/// The tuner thread: promote once, then read windows from the ring and detect. Not
/// real-time (it may allocate at construction), but it never blocks the input callback.
fn tuner_loop(
    consumer: &mut rtrb::Consumer<f32>,
    sample_rate: u32,
    shared: &Arc<Shared>,
    stop: &Arc<AtomicBool>,
) {
    // Reuse the existing MMCSS promotion (ADR-027); no new unsafe. Not fatal if refused.
    #[cfg(windows)]
    let _ = crate::mmcss::promote_current_thread();

    let config = DetectorConfig::guitar(sample_rate as f32);
    let window = config.window;
    let hop = window / 4; // ~25 Hz updates at 2048/48k
    let mut detector = PitchDetector::new(config);
    let mut buf: Vec<f32> = Vec::with_capacity(window * 2);
    let mut frame: Vec<f32> = vec![0.0; window];

    while !stop.load(Relaxed) {
        // Drain whatever the callback has pushed.
        while let Ok(s) = consumer.pop() {
            buf.push(s);
            if buf.len() > window * 4 {
                // Keep the buffer bounded if we ever fall behind.
                let drop = buf.len() - window * 2;
                buf.drain(..drop);
            }
        }
        while buf.len() >= window {
            frame.copy_from_slice(&buf[..window]);
            let est = detector.process(&frame);
            shared
                .hz_bits
                .store(est.hz.unwrap_or(f32::NAN).to_bits(), Relaxed);
            shared.clarity_bits.store(est.clarity.to_bits(), Relaxed);
            shared.updates.fetch_add(1, Relaxed);
            buf.drain(..hop);
        }
        // Sleep roughly one hop so we poll at about the analysis rate without spinning.
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// Map a cpal input-stream build error message to a typed tuner error (A10). WASAPI
/// reports a denied microphone (Windows privacy setting) through the build error text.
fn classify_build_error(msg: &str) -> AudioError {
    let lower = msg.to_lowercase();
    if lower.contains("denied") || lower.contains("access") {
        AudioError::MicPermissionDenied(msg.to_string())
    } else {
        AudioError::Stream(msg.to_string())
    }
}
