//! Counting global allocator. Any allocation, reallocation or deallocation on a
//! thread while `forbid` is active is counted as a real-time violation (A1, A3).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

pub struct CountingAlloc;

thread_local! {
    static FORBIDDEN: Cell<bool> = const { Cell::new(false) };
}

pub static VIOLATIONS: AtomicU64 = AtomicU64::new(0);
pub static LAST_VIOLATION_SIZE: AtomicU64 = AtomicU64::new(0);

#[inline]
fn note(size: usize) {
    if FORBIDDEN.try_with(Cell::get).unwrap_or(false) {
        VIOLATIONS.fetch_add(1, Relaxed);
        LAST_VIOLATION_SIZE.store(size as u64, Relaxed);
    }
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        note(layout.size());
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Runs `f` with allocation forbidden on the current thread.
#[inline]
pub fn forbid<R>(f: impl FnOnce() -> R) -> R {
    FORBIDDEN.with(|c| c.set(true));
    let r = f();
    FORBIDDEN.with(|c| c.set(false));
    r
}

pub fn violations() -> u64 {
    VIOLATIONS.load(Relaxed)
}
