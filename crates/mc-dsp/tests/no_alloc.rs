//! `PitchDetector::process` must not allocate (ADR-029 §7): the tuner thread runs it on
//! every window off the input ring, and an allocation there would risk a stall. The
//! detector preallocates its FFT plans and buffers in `new`; the per-window call is
//! checked here with `assert_no_alloc`, the same mechanism as the audio callback (FF-16).

use assert_no_alloc::{AllocDisabler, assert_no_alloc};
use mc_dsp::{DetectorConfig, PitchDetector};
use std::f32::consts::PI;

#[global_allocator]
static ALLOCATOR: AllocDisabler = AllocDisabler;

const SR: f32 = 48_000.0;

fn frame(hz: f32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| (2.0 * PI * hz * i as f32 / SR).sin() * 0.5)
        .collect()
}

#[test]
fn process_does_not_allocate() {
    let config = DetectorConfig::guitar(SR);
    let mut det = PitchDetector::new(config);
    // Build every input outside the checked region, as the tuner thread would.
    let low = frame(82.41, config.window); // E2
    let high = frame(329.63, config.window); // E4
    let silence = vec![0.0f32; config.window];

    // Many windows, including silence and both ends of the range, inside the check.
    for i in 0..256usize {
        let f: &[f32] = match i % 3 {
            0 => &low,
            1 => &high,
            _ => &silence,
        };
        let est = assert_no_alloc(|| det.process(f));
        std::hint::black_box(est);
    }
}

/// Negative control, part 1: allocates inside the checked region on purpose (aborts).
#[test]
#[ignore = "aborts the process on purpose; run by the negative control below"]
fn negative_control_allocation_is_detected() {
    assert_no_alloc(|| {
        let v = vec![1u8; 64];
        std::hint::black_box(&v);
    });
}

/// Negative control, part 2: confirms the checker actually fires, so a green no-alloc
/// test can be trusted.
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
