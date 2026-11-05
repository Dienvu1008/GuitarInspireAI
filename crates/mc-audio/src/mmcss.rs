// The only file of `mc-audio` that may contain `unsafe` (ADR-027, checked by FF-34:
// exactly one occurrence, with a SAFETY comment directly above it). Compiled on
// Windows only. It exposes one safe function; the real-time core never sees the call.
//
// What it does: registers the calling thread with the Multimedia Class Scheduler Service
// under the task name "Audio" (ADR-007 amendment 1). Spike S2 measured late callbacks of
// up to 37 ms without this and none in 600 s with it.

use windows_sys::Win32::System::Threading::AvSetMmThreadCharacteristicsW;

/// `"Audio"` as a NUL-terminated UTF-16 string. A `static`, so it is valid for the whole
/// program and nothing is allocated when the callback thread uses it (GR-10).
static TASK_NAME: [u16; 6] = [
    b'A' as u16,
    b'u' as u16,
    b'd' as u16,
    b'i' as u16,
    b'o' as u16,
    0,
];

/// Promote the calling thread. Returns whether Windows accepted the registration. The
/// registration lasts as long as the thread; the handle is not released because the
/// callback thread lives as long as the stream.
pub(crate) fn promote_current_thread() -> bool {
    let mut task_index: u32 = 0;
    // SAFETY: `TASK_NAME` is a valid, NUL-terminated UTF-16 string that lives for the
    // whole program. `task_index` is a local `u32` that outlives the call and is the
    // out-parameter the API expects. The returned handle is only compared with null and
    // never dereferenced or passed on.
    let handle = unsafe { AvSetMmThreadCharacteristicsW(TASK_NAME.as_ptr(), &mut task_index) };
    !handle.is_null()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_task_name_is_audio_and_nul_terminated() {
        let text = String::from_utf16(&TASK_NAME[..5]).unwrap();
        assert_eq!(text, "Audio");
        assert_eq!(TASK_NAME[5], 0);
    }

    #[test]
    fn promoting_a_thread_twice_does_not_crash() {
        // Whether MMCSS accepts a thread depends on the machine's services, so the first
        // result is not asserted here; the device test checks it on the reference
        // machine. Measured there: a second registration of the same thread is refused
        // (returns false), which is why the engine calls this once per callback thread.
        std::thread::spawn(|| {
            let _first = promote_current_thread();
            let _second = promote_current_thread();
        })
        .join()
        .unwrap();
    }
}
