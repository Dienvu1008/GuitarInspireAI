//! FF-16: the callback body must not allocate (GR-10, A1, A3, A6). `assert_no_alloc`
//! installs a global allocator that aborts the process if memory is allocated, freed or
//! reallocated inside the checked closure. Everything that allocates (building the
//! engine, the timelines, the commands, the output buffer) happens outside the closure,
//! exactly as in the product, where the control thread does it.

use assert_no_alloc::{AllocDisabler, assert_no_alloc};
use mc_app::fixtures::{Shape, reference};
use mc_audio::OfflineEngine;
use mc_playback::{CompileOptions, PerformanceTimeline, TempoFactor, Tick, compile};

#[global_allocator]
static ALLOCATOR: AllocDisabler = AllocDisabler;

const SF2: &[u8] = include_bytes!("../../../assets/soundfonts/GuitarSteel.sf2");
const RATE: u32 = 48_000;
const BLOCK: usize = 480; // the 10 ms WASAPI shared period measured in S2

fn timeline(bars: usize, seed: usize) -> PerformanceTimeline {
    let score = reference(Shape {
        tracks: 3,
        bars: bars + seed % 2,
        beats: 8,
        notes_per_beat: 2,
    });
    compile(&score, &CompileOptions::new()).unwrap().timeline
}

#[test]
fn the_callback_does_not_allocate_over_twenty_seconds_of_transport_activity() {
    let mut engine = OfflineEngine::new(RATE);
    engine.install_soundfont(SF2).unwrap();
    engine.load_timeline(timeline(16, 0));
    engine.play();
    let mut out = vec![0.0f32; BLOCK * 2];
    let mut sounded = 0usize;

    // 2000 blocks of 10 ms = 20 s. Every 100 blocks a command arrives.
    for block in 0..2000usize {
        match block % 100 {
            10 => engine.load_timeline(timeline(16, block)), // A6: swap while playing
            30 => engine.seek(Tick(0)),
            50 => engine.set_tempo_factor(TempoFactor::new(25 + 5 * (block / 100) as u16).unwrap()),
            70 => engine.stop(),
            72 => engine.play(),
            90 => engine.set_tempo_factor(TempoFactor::NORMAL),
            _ => {}
        }
        assert_no_alloc(|| engine.process(&mut out, 2));
        if out.iter().any(|&x| x.abs() > 1e-3) {
            sounded += 1;
        }
        // Free what the callback handed back, outside the checked region (as the
        // control thread does).
        engine.drain_retired();
    }
    assert!(
        sounded > 500,
        "the test must actually render sound ({sounded})"
    );
    let status = engine.status();
    assert!(!status.panicked);
    assert_eq!(status.return_ring_full, 0);
    assert!(status.swaps >= 20, "swaps {}", status.swaps);
}

/// Negative control, part 1: allocates inside the checked region on purpose. It is
/// ignored in normal runs because it aborts the process; part 2 runs it in a child.
#[test]
#[ignore = "aborts the process on purpose; run by the negative control below"]
fn negative_control_allocation_is_detected() {
    assert_no_alloc(|| {
        let v = vec![1u8; 64];
        std::hint::black_box(&v);
    });
}

/// Negative control, part 2: the checker really fires. If `assert_no_alloc` stopped
/// working (wrong allocator, disabled feature), the child would exit successfully and
/// this test would fail, so a green FF-16 can be trusted.
#[test]
fn negative_control_the_checker_aborts_on_allocation() {
    let exe = std::env::current_exe().unwrap();
    let status = std::process::Command::new(exe)
        .args([
            "--ignored",
            "--exact",
            "negative_control_allocation_is_detected",
            "--test-threads=1",
        ])
        .status()
        .unwrap();
    assert!(
        !status.success(),
        "an allocation inside assert_no_alloc must abort the child process"
    );
}

#[test]
fn playing_to_the_end_and_past_it_does_not_allocate() {
    let mut engine = OfflineEngine::new(RATE);
    engine.install_soundfont(SF2).unwrap();
    engine.load_timeline(timeline(1, 0)); // 2 s
    engine.play();
    let mut out = vec![0.0f32; BLOCK * 2];
    for _ in 0..600 {
        assert_no_alloc(|| engine.process(&mut out, 2));
    }
    assert!(engine.status().finished);
}

#[test]
fn a_block_larger_than_the_internal_buffers_does_not_allocate() {
    let mut engine = OfflineEngine::new(RATE);
    engine.install_soundfont(SF2).unwrap();
    engine.load_timeline(timeline(4, 0));
    engine.play();
    // 20 000 frames > MAX_BLOCK (8192): rendered in several spans.
    let mut out = vec![0.0f32; 20_000 * 2];
    assert_no_alloc(|| engine.process(&mut out, 2));
    assert!(out.iter().any(|&x| x.abs() > 1e-3));
}
