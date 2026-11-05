//! FF-18: the playback glitch procedure (GR-10, A4; performance-requirements section 3,
//! "Audio underruns: zero in 10 min reference playback"). Plays the reference score on
//! the real default output device and watches callback timing. cpal reports no xruns on
//! WASAPI shared render (S2), so a gap is detected from callback intervals: a late
//! callback is one that arrives more than 1.5x the device period after the previous one.
//!
//! This is a manual procedure on the reference machine (hosted CI has no audio device,
//! ADR-025). Run it and record the printed summary in `benchmarks/ff18-glitch.md`:
//!   $env:FF18_MINUTES=10; cargo test -p mc-audio --test glitch -- --ignored --nocapture
//! Without FF18_MINUTES it runs a short smoke check (default 1 minute) so the harness
//! itself stays exercised. The result is evidence for this machine only (ADR-025).

use mc_app::fixtures::{Shape, reference};
use mc_audio::{AudioEngine, EngineConfig};
use mc_playback::{CompileOptions, PerformanceTimeline, compile};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn soundfont() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/soundfonts/GuitarSteel.sf2"
    ))
}

fn reference_timeline() -> PerformanceTimeline {
    // A long, dense reference performance so playback keeps the synth busy for the run.
    let score = reference(Shape {
        tracks: 4,
        bars: 64,
        beats: 8,
        notes_per_beat: 2,
    });
    compile(&score, &CompileOptions::new()).unwrap().timeline
}

#[test]
#[ignore = "needs an audio output device; run as the FF-18 procedure on the reference machine"]
fn ten_minute_playback_has_no_underruns() {
    let minutes: u64 = std::env::var("FF18_MINUTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let duration = Duration::from_secs(minutes * 60);

    let engine = AudioEngine::start(EngineConfig {
        soundfont_path: Some(soundfont()),
    })
    .expect("a default output device");
    let timeline = reference_timeline();
    let end_tick = timeline.end().0;
    engine.load_timeline(timeline).unwrap();
    engine.play().unwrap();

    let start = Instant::now();
    let mut last_report = Instant::now();
    while start.elapsed() < duration {
        std::thread::sleep(Duration::from_millis(200));
        let s = engine.status();
        // Loop back to the start when the performance ends, to keep playing the whole run.
        if s.finished || s.tick >= end_tick {
            engine.seek(mc_playback::Tick(0)).unwrap();
            engine.play().unwrap();
        }
        if last_report.elapsed() >= Duration::from_secs(30) {
            last_report = Instant::now();
            println!(
                "t={:4}s callbacks {} late {} max_interval {:.1} ms max_callback {:.2} ms",
                start.elapsed().as_secs(),
                s.callbacks,
                s.late_callbacks,
                s.max_interval_ns as f64 / 1e6,
                s.max_callback_ns as f64 / 1e6,
            );
        }
        assert!(!s.panicked, "the callback panicked during the run");
    }
    engine.stop().unwrap();
    let s = engine.status();
    println!(
        "FF-18 result: {minutes} min, device {} Hz, promotion {:?}\n\
         callbacks {} late {} device_errors {} device_rebuilds {}\n\
         max_interval {:.1} ms max_callback {:.2} ms output_latency {} us",
        s.sample_rate,
        s.promotion,
        s.callbacks,
        s.late_callbacks,
        s.device_errors,
        s.device_rebuilds,
        s.max_interval_ns as f64 / 1e6,
        s.max_callback_ns as f64 / 1e6,
        s.output_latency_us,
    );
    assert!(s.callbacks > 0, "no callbacks ran");
    assert_eq!(
        s.late_callbacks, 0,
        "FF-18: {} late callbacks in {minutes} min (A4 requires zero underruns)",
        s.late_callbacks
    );
}
