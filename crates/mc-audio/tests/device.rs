//! Tests on the real default output device. They need audio hardware, so they are
//! ignored in CI. Run on the reference machine with:
//!   cargo test -p mc-audio --test device -- --ignored --nocapture
//! The numbers they print are evidence for that machine only (ADR-025).

use mc_app::fixtures::{Shape, reference};
use mc_audio::{AudioEngine, EngineConfig, Promotion};
use mc_playback::{CompileOptions, PerformanceTimeline, TempoFactor, Tick, compile};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn soundfont() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/soundfonts/GuitarSteel.sf2"
    ))
}

fn timeline(bars: usize) -> PerformanceTimeline {
    let score = reference(Shape {
        tracks: 2,
        bars,
        beats: 8,
        notes_per_beat: 2,
    });
    compile(&score, &CompileOptions::new()).unwrap().timeline
}

fn wait_until(deadline: Duration, mut done: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
#[ignore = "needs an audio output device"]
fn plays_on_the_default_device() {
    let engine = AudioEngine::start(EngineConfig {
        soundfont_path: Some(soundfont()),
    })
    .expect("a default output device");
    let before = engine.status();
    println!(
        "device: {} Hz, promotion {:?}",
        before.sample_rate, before.promotion
    );
    engine.load_timeline(timeline(8)).unwrap();
    let t0 = Instant::now();
    engine.play().unwrap();
    // Play to first callback with the transport running (A5, start latency proxy).
    assert!(wait_until(Duration::from_secs(5), || engine
        .status()
        .playing));
    println!("play command to playing flag: {:?}", t0.elapsed());
    assert!(engine.last_error().is_none(), "{:?}", engine.last_error());

    std::thread::sleep(Duration::from_millis(2000));
    let s = engine.status();
    println!(
        "after 2 s: tick {} callbacks {} late {} max_interval {:.1} ms max_callback {:.2} ms latency {} us",
        s.tick,
        s.callbacks,
        s.late_callbacks,
        s.max_interval_ns as f64 / 1e6,
        s.max_callback_ns as f64 / 1e6,
        s.output_latency_us
    );
    assert!(s.tick > 0, "the position advances while playing");
    assert!(s.callbacks > 100, "callbacks are running");
    assert!(!s.panicked);

    // Tempo factor, seek and stop work on the device too.
    engine
        .set_tempo_factor(TempoFactor::new(200).unwrap())
        .unwrap();
    engine.seek(Tick(0)).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    assert!(engine.status().tick > 0);
    engine.stop().unwrap();
    assert!(wait_until(Duration::from_secs(2), || !engine
        .status()
        .playing));
    let promotion = engine.status().promotion;
    println!("promotion: {promotion:?}");
    if cfg!(windows) {
        assert_eq!(promotion, Promotion::Promoted, "MMCSS promotion on Windows");
    } else {
        assert_eq!(promotion, Promotion::NotNeeded);
    }
}

#[test]
#[ignore = "needs an audio output device"]
fn a_missing_soundfont_disables_playback_with_a_message() {
    let engine = AudioEngine::start(EngineConfig {
        soundfont_path: Some(PathBuf::from("does-not-exist.sf2")),
    })
    .expect("a default output device");
    engine.load_timeline(timeline(2)).unwrap();
    engine.play().unwrap();
    assert!(wait_until(Duration::from_secs(3), || engine
        .last_error()
        .is_some()));
    let message = engine.last_error().unwrap();
    println!("message: {message}");
    assert!(message.contains("does-not-exist.sf2"));
    assert!(!engine.status().playing, "no sine fallback");
}
