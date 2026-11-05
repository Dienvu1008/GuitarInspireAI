//! Platform specifics of the audio adapter (FF-07 allows platform code in adapter
//! crates).
//!
//! Real-time promotion of the callback thread (ADR-007 amendment 1) is the Windows MMCSS
//! "Audio" task. The one `unsafe` call it needs lives in `mmcss.rs` under the exception
//! of ADR-027; this file stays safe code. Other platforms need no promotion in the MVP
//! (Windows is the only supported target, Q-13).

use crate::status::Promotion;

/// Promote the calling (callback) thread. Called once, on the first callback. Does not
/// allocate. Failure is not an error: the engine runs at normal priority and reports
/// [`Promotion::Unavailable`].
pub fn promote_callback_thread() -> Promotion {
    #[cfg(windows)]
    {
        if crate::mmcss::promote_current_thread() {
            Promotion::Promoted
        } else {
            Promotion::Unavailable
        }
    }
    #[cfg(not(windows))]
    {
        Promotion::NotNeeded
    }
}
