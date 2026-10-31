//! S1 interop shape: opaque document handle, typed commands mirrored from a
//! crate without frb, async execute, event stream, fixed-rate position stream.

use crate::frb_generated::StreamSink;
use flutter_rust_bridge::frb;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub use fake_app::{CommandKind, Outcome};

#[frb(mirror(Outcome))]
pub struct _Outcome {
    pub revision: u64,
    pub event_seq: u64,
}

#[frb(mirror(CommandKind))]
pub enum _CommandKind {
    Noop,
    InsertNote { string_index: u8, fret: u8 },
    SetTempo { bpm_num: u32, bpm_den: u32 },
}

pub struct EventDto {
    pub seq: u64,
    pub revision: u64,
}

#[frb(opaque)]
pub struct DocumentHandle {
    session: Mutex<fake_app::Session>,
    sink: Mutex<Option<StreamSink<EventDto>>>,
}

impl DocumentHandle {
    #[frb(sync)]
    pub fn new() -> DocumentHandle {
        DocumentHandle { session: Mutex::new(fake_app::Session::default()), sink: Mutex::new(None) }
    }

    /// Plain Rust function: frb runs it on its worker pool, Dart gets a Future.
    pub fn execute(&self, cmd: CommandKind) -> Outcome {
        let outcome = self.session.lock().unwrap().execute(&cmd);
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            let _ = sink.add(EventDto { seq: outcome.event_seq, revision: outcome.revision });
        }
        outcome
    }

    /// Async Rust function (frb `rust-async` feature).
    pub async fn execute_async(&self, cmd: CommandKind) -> Outcome {
        self.execute(cmd)
    }

    #[frb(sync)]
    pub fn revision_sync(&self) -> u64 {
        self.session.lock().unwrap().revision()
    }

    /// Per-document event stream. Replaces an earlier subscriber.
    pub fn events(&self, sink: StreamSink<EventDto>) {
        *self.sink.lock().unwrap() = Some(sink);
    }
}

pub struct PositionDto {
    pub tick: u64,
    pub sent_us: u64,
}

/// Emits `hz` updates per second for `seconds` from a dedicated thread with
/// absolute deadlines (no drift accumulation). Stops when Dart cancels.
pub fn position_stream(sink: StreamSink<PositionDto>, hz: u32, seconds: u32) {
    std::thread::spawn(move || {
        let start = Instant::now();
        let period = Duration::from_secs_f64(1.0 / hz as f64);
        let mut deadline = start;
        for i in 0..(hz as u64 * seconds as u64) {
            deadline += period;
            let now = Instant::now();
            if deadline > now {
                std::thread::sleep(deadline - now);
            }
            if sink.add(PositionDto { tick: i, sent_us: start.elapsed().as_micros() as u64 }).is_err() {
                break;
            }
        }
    });
}
