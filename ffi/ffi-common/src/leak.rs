//! Test-only heap allocation ledger for FFI memory-leak detection.
//!
//! This module is intentionally inert in production builds: it only defines a
//! [`CountingAlloc`] allocator and a [`live_bytes`] counter. A binding opts in
//! to actually *use* it by declaring, inside its own `#[cfg(test)]` block:
//!
//! ```ignore
//! #[global_allocator]
//! static TEST_ALLOC: alpha_ta_ffi_common::leak::CountingAlloc =
//!     alpha_ta_ffi_common::leak::CountingAlloc;
//! ```
//!
//! Once installed as the process global allocator for the test binary, every
//! Rust heap allocation made while running the binding's tests is counted. A
//! well-behaved FFI function that allocates a result and is later paired with
//! its `ta_free_*` function returns the live-byte count to its baseline; a
//! function that leaks (or a forgotten `ta_free_*`) makes the count grow
//! monotonically across repeated cycles, which the leak tests assert against.
//!
//! This is the portable, stable-Rust substitute for valgrind / ASan on
//! platforms where those are unavailable (e.g. Windows MSVC). It cannot catch
//! *use-after-free* or *double-free* directly, but it reliably catches the
//! "caller forgot to free" leak class that the FFI ownership contract exists
//! to prevent.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};

static LIVE_BYTES: AtomicIsize = AtomicIsize::new(0);

/// Current net live bytes allocated through [`CountingAlloc`].
///
/// Snapshot this before and after a workload loop; with a correct
/// alloc/free contract the two snapshots are (within a small tolerance)
/// equal.
#[inline]
pub fn live_bytes() -> isize {
    LIVE_BYTES.load(Ordering::SeqCst)
}

/// A [`GlobalAlloc`] wrapper around the system allocator that tallies the
/// net number of bytes currently live on the Rust heap.
#[allow(missing_debug_implementations)]
pub struct CountingAlloc;

// SAFETY: `CountingAlloc` forwards every call unchanged to the process
// `System` allocator and only maintains an atomic byte counter as a side
// effect, so it upholds the same allocation/deallocation safety invariants
// as `System` itself.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            LIVE_BYTES.fetch_add(layout.size() as isize, Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE_BYTES.fetch_sub(layout.size() as isize, Ordering::SeqCst);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            LIVE_BYTES.fetch_add(layout.size() as isize, Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            // The block moved (or resized); swap the byte accounting.
            LIVE_BYTES.fetch_sub(layout.size() as isize, Ordering::SeqCst);
            LIVE_BYTES.fetch_add(new_size as isize, Ordering::SeqCst);
        }
        p
    }
}
