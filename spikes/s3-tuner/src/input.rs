//! S3 A10 + live mic readings. Demonstrates the production threading model (ADR-029 s.7,
//! audio-dsp section 4): the input callback only copies the mono mix into a ring; a
//! separate tuner thread runs the detector. Device and permission failures become typed
//! errors here (printed), not panics. Throwaway spike code.

use crate::detect::{Algo, cents, detect, rms};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

/// A typed outcome for the A10 cases.
#[derive(Debug)]
enum TunerStart {
    Ok,
    NoInputDevice,
    PermissionDenied(String),
    DeviceBusyOrConfig(String),
}

const OPEN: [(&str, f32); 6] = crate::signal::OPEN_STRINGS;

fn nearest(hz: f32) -> (&'static str, f32) {
    // Nearest open string by cents (the spike tunes against open strings; the product
    // tuner uses the active track's tuning + capo, ADR-029 T8).
    let mut best = OPEN[0];
    let mut best_c = f32::MAX;
    for s in OPEN {
        let c = cents(hz, s.1).abs();
        if c < best_c {
            best_c = c;
            best = s;
        }
    }
    best
}

pub fn run(seconds: u64, sr_target: f32, win: usize, hop: usize, gate: f32, min_hz: f32, max_hz: f32) {
    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            report(TunerStart::NoInputDevice);
            return;
        }
    };
    let supported = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
            report(TunerStart::DeviceBusyOrConfig(e.to_string()));
            return;
        }
    };
    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let sr = config.sample_rate as f32;
    let channels = config.channels as usize;
    println!("default input device: {sr} Hz, {channels} ch (target analysis {sr_target} Hz)");

    // Ring between the input callback (producer) and the tuner thread (consumer).
    let (tx, rx) = mpsc::channel::<Vec<f32>>();
    let dropped = Arc::new(AtomicUsize::new(0));
    let dropped_cb = dropped.clone();

    let err_fn = |e: cpal::Error| eprintln!("stream error: {e}");
    if sample_format != cpal::SampleFormat::F32 {
        report(TunerStart::DeviceBusyOrConfig(format!(
            "unsupported sample format {sample_format:?}"
        )));
        return;
    }
    let stream = match device.build_input_stream(
        config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Copy only: mono mix into a fresh buffer handed to the tuner thread.
            let mut mono = Vec::with_capacity(data.len() / channels.max(1));
            for frame in data.chunks(channels.max(1)) {
                mono.push(frame.iter().copied().sum::<f32>() / channels as f32);
            }
            if tx.send(mono).is_err() {
                dropped_cb.fetch_add(1, Ordering::Relaxed);
            }
        },
        err_fn,
        None,
    ) {
        Ok(s) => s,
        Err(e) => {
            // WASAPI surfaces a denied microphone (Windows privacy setting) here.
            let msg = e.to_string();
            if msg.to_lowercase().contains("denied") || msg.to_lowercase().contains("access") {
                report(TunerStart::PermissionDenied(msg));
            } else {
                report(TunerStart::DeviceBusyOrConfig(msg));
            }
            return;
        }
    };
    if let Err(e) = stream.play() {
        report(TunerStart::DeviceBusyOrConfig(e.to_string()));
        return;
    }
    report(TunerStart::Ok);

    // Tuner thread: accumulate a sliding window, run the detector every `hop`.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    let mut buf: Vec<f32> = Vec::new();
    println!("listening {seconds}s; pluck an open string...\n");
    while std::time::Instant::now() < deadline {
        let Ok(chunk) = rx.recv_timeout(std::time::Duration::from_millis(200)) else {
            continue;
        };
        buf.extend(chunk);
        while buf.len() >= win {
            let frame = &buf[..win];
            if rms(frame) > 1e-3 {
                if let Some(hz) = detect(Algo::Mpm, frame, sr, gate, min_hz, max_hz).hz {
                    let (nm, rf) = nearest(hz);
                    println!("{hz:7.2} Hz  nearest {nm} ({:+.0} cents)", cents(hz, rf));
                }
            }
            buf.drain(..hop);
        }
    }
    drop(stream);
    let d = dropped.load(Ordering::Relaxed);
    if d > 0 {
        println!("\n(dropped {d} input buffers; the tuner thread fell behind)");
    }
}

fn report(outcome: TunerStart) {
    match outcome {
        TunerStart::Ok => println!("A10: input stream started (Ok)"),
        TunerStart::NoInputDevice => {
            println!("A10: no default input device -> typed error NoInputDevice (no crash)");
        }
        TunerStart::PermissionDenied(m) => {
            println!("A10: microphone permission denied -> typed error (no crash): {m}");
        }
        TunerStart::DeviceBusyOrConfig(m) => {
            println!("A10: device busy or config error -> typed error (no crash): {m}");
        }
    }
}
