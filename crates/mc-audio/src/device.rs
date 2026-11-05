//! The cpal output stream (WASAPI shared mode on Windows). The stream is created on the
//! control thread and stays there (`cpal::Stream` is not `Send` on every platform).
//! Per-callback timing is measured here, outside the `rt` directory, because cpal reports
//! no under-runs on WASAPI shared mode: a late callback is detected by the interval
//! between callbacks (ADR-007 amendment 2).

use crate::engine::{ControlEnds, build_rt};
use crate::error::AudioError;
use crate::platform::promote_callback_thread;
use crate::status::Status;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::Instant;

/// A running output stream and the control ends of its callback core.
pub struct Running {
    // Dropping the stream stops the callback.
    _stream: cpal::Stream,
    pub ends: ControlEnds,
    pub sample_rate: u32,
}

/// Open the default output device with its default configuration (f32 samples), start
/// the stream and return it. `status` is shared with the caller (it survives a device
/// rebuild); `device_failed` is raised by the stream error handler for any error other
/// than an under-run notice.
pub fn open_default(
    status: Arc<Status>,
    device_failed: Arc<AtomicBool>,
) -> Result<Running, AudioError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(AudioError::NoOutputDevice)?;
    let supported = device
        .default_output_config()
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

    let (mut rt, ends) = build_rt(sample_rate, status.clone());
    let cb_status = status.clone();
    let mut last: Option<Instant> = None;
    let mut promoted = false;
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                if !promoted {
                    promoted = true;
                    cb_status
                        .promotion
                        .store(promote_callback_thread().code(), Relaxed);
                }
                let start = Instant::now();
                if let Some(prev) = last {
                    let gap = start.duration_since(prev).as_nanos() as u64;
                    cb_status.max_interval_ns.fetch_max(gap, Relaxed);
                    let frames = (data.len() / channels.max(1)) as u64;
                    let period = frames * 1_000_000_000 / sample_rate.max(1) as u64;
                    if gap > period + period / 2 {
                        cb_status.late_callbacks.fetch_add(1, Relaxed);
                    }
                }
                last = Some(start);
                rt.process(data, channels);
                cb_status
                    .max_callback_ns
                    .fetch_max(start.elapsed().as_nanos() as u64, Relaxed);
                let ts = info.timestamp();
                let latency = ts.playback.duration_since(ts.callback).as_micros() as u64;
                cb_status.output_latency_us.store(latency, Relaxed);
                cb_status.callbacks.fetch_add(1, Relaxed);
            },
            {
                let status = status.clone();
                move |err: cpal::Error| {
                    status.device_errors.fetch_add(1, Relaxed);
                    if !matches!(err.kind(), cpal::ErrorKind::Xrun) {
                        device_failed.store(true, Relaxed);
                    }
                }
            },
            None,
        )
        .map_err(|e| AudioError::Stream(e.to_string()))?;
    stream
        .play()
        .map_err(|e| AudioError::Stream(e.to_string()))?;
    Ok(Running {
        _stream: stream,
        ends,
        sample_rate,
    })
}
