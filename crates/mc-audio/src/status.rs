//! State shared between the callback, the control thread and callers. The callback only
//! touches atomics; the one text field (`last_error`) is used by non-real-time threads.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU8, AtomicU64, Ordering::Relaxed};

/// Result of trying to promote the callback thread to real-time scheduling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Promotion {
    /// Not attempted yet (the callback has not run).
    Pending,
    /// The platform needs no promotion.
    NotNeeded,
    /// Promoted (Windows MMCSS "Audio" task).
    Promoted,
    /// Promotion is not available or failed; audio still works at normal priority and
    /// late callbacks are possible (S2 saw up to 37 ms without promotion).
    Unavailable,
}

impl Promotion {
    pub(crate) fn code(self) -> u8 {
        match self {
            Promotion::Pending => 0,
            Promotion::NotNeeded => 1,
            Promotion::Promoted => 2,
            Promotion::Unavailable => 3,
        }
    }

    pub(crate) fn from_code(c: u8) -> Self {
        match c {
            1 => Promotion::NotNeeded,
            2 => Promotion::Promoted,
            3 => Promotion::Unavailable,
            _ => Promotion::Pending,
        }
    }
}

/// Atomics written by the callback and the device error handler.
#[derive(Debug, Default)]
pub struct Status {
    /// Current position in ticks of the loaded timeline.
    pub tick: AtomicI64,
    pub playing: AtomicBool,
    /// The timeline played to its end and stopped.
    pub finished: AtomicBool,
    /// The callback panicked and now outputs silence (ADR-024 item 7).
    pub panicked: AtomicBool,
    pub synth_ready: AtomicBool,
    pub callbacks: AtomicU64,
    /// Callbacks that arrived more than 1.5 periods after the previous one (A4 proxy;
    /// cpal reports no xruns on WASAPI shared mode).
    pub late_callbacks: AtomicU64,
    pub max_interval_ns: AtomicU64,
    pub max_callback_ns: AtomicU64,
    pub swaps: AtomicU64,
    pub return_ring_full: AtomicU64,
    pub device_errors: AtomicU64,
    pub device_rebuilds: AtomicU64,
    pub promotion: AtomicU8,
    pub output_latency_us: AtomicU64,
    pub sample_rate: AtomicU64,
}

/// Plain copy of [`Status`] for callers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineStatus {
    pub tick: i64,
    pub playing: bool,
    pub finished: bool,
    pub panicked: bool,
    pub synth_ready: bool,
    pub callbacks: u64,
    pub late_callbacks: u64,
    pub max_interval_ns: u64,
    pub max_callback_ns: u64,
    pub swaps: u64,
    pub return_ring_full: u64,
    pub device_errors: u64,
    pub device_rebuilds: u64,
    pub promotion: Promotion,
    pub output_latency_us: u64,
    pub sample_rate: u32,
}

impl Status {
    pub fn snapshot(&self) -> EngineStatus {
        EngineStatus {
            tick: self.tick.load(Relaxed),
            playing: self.playing.load(Relaxed),
            finished: self.finished.load(Relaxed),
            panicked: self.panicked.load(Relaxed),
            synth_ready: self.synth_ready.load(Relaxed),
            callbacks: self.callbacks.load(Relaxed),
            late_callbacks: self.late_callbacks.load(Relaxed),
            max_interval_ns: self.max_interval_ns.load(Relaxed),
            max_callback_ns: self.max_callback_ns.load(Relaxed),
            swaps: self.swaps.load(Relaxed),
            return_ring_full: self.return_ring_full.load(Relaxed),
            device_errors: self.device_errors.load(Relaxed),
            device_rebuilds: self.device_rebuilds.load(Relaxed),
            promotion: Promotion::from_code(self.promotion.load(Relaxed)),
            output_latency_us: self.output_latency_us.load(Relaxed),
            sample_rate: self.sample_rate.load(Relaxed) as u32,
        }
    }
}

/// Text shown to the user when playback cannot work (missing SoundFont, no device).
#[derive(Debug, Default)]
pub struct LastError(Mutex<Option<String>>);

impl LastError {
    pub fn set(&self, message: impl Into<String>) {
        if let Ok(mut g) = self.0.lock() {
            *g = Some(message.into());
        }
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.0.lock() {
            *g = None;
        }
    }

    pub fn get(&self) -> Option<String> {
        self.0.lock().ok().and_then(|g| g.clone())
    }
}
