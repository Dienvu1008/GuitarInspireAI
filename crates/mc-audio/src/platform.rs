//! Platform specifics of the audio adapter (FF-07 allows platform code in adapter
//! crates).
//!
//! Real-time promotion of the callback thread (ADR-007 amendment 1) is the Windows MMCSS
//! "Audio" task. Calling `AvSetMmThreadCharacteristicsW` needs one `unsafe` call, and the
//! workspace forbids `unsafe` everywhere except the generated bridge module (ADR-022,
//! FF-34). Until that exception is decided, promotion reports
//! [`Promotion::Unavailable`] on Windows and the engine runs at normal priority.

use crate::status::Promotion;

/// Promote the calling (callback) thread. Called once, on the first callback. Must not
/// allocate.
pub fn promote_callback_thread() -> Promotion {
    if cfg!(windows) {
        Promotion::Unavailable
    } else {
        Promotion::NotNeeded
    }
}
