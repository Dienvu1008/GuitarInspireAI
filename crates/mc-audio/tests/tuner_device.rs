//! Tuner on the real default input device (ADR-029 §7, A10). Needs a microphone, so it
//! is ignored in CI; run on the reference machine:
//!   cargo test -p mc-audio --test tuner_device -- --ignored --nocapture
//! The A10 cases that need OS state (privacy denied, device busy) are in the MVP manual
//! script; this checks the start outcome and that readings advance when a device exists.

use mc_audio::{AudioError, TunerHandle};
use std::time::{Duration, Instant};

#[test]
#[ignore = "needs a microphone; run on the reference machine"]
fn tuner_starts_and_produces_readings_or_a_typed_error() {
    match TunerHandle::start() {
        Ok(tuner) => {
            println!("tuner started on a {} Hz input device", tuner.sample_rate());
            // Let the thread run; readings should advance even in a quiet room (clarity
            // low, hz None), proving the capture -> ring -> detector path is live.
            let start = Instant::now();
            let first = tuner.reading().updates;
            let mut advanced = false;
            while start.elapsed() < Duration::from_secs(3) {
                std::thread::sleep(Duration::from_millis(100));
                if tuner.reading().updates > first {
                    advanced = true;
                    break;
                }
            }
            let r = tuner.reading();
            println!(
                "readings: updates {} hz {:?} clarity {:.2} failed {}",
                r.updates,
                r.hz,
                r.clarity,
                tuner.failed()
            );
            assert!(advanced, "the tuner thread produced no readings");
            // Dropping the tuner must release the microphone without hanging.
            drop(tuner);
        }
        Err(AudioError::NoInputDevice) => {
            println!("no microphone: typed NoInputDevice (A10) — acceptable on this machine");
        }
        Err(AudioError::MicPermissionDenied(m)) => {
            println!("permission denied: typed error (A10), no crash: {m}");
        }
        Err(e) => {
            println!("typed start error (A10), no crash: {e}");
        }
    }
}
