//! Process-local runtime reuse for stateless FFI entry points.
//!
//! Language bindings expose JSON functions rather than Rust object handles.
//! Rebuilding the unified engine for every call would discard its compiled
//! plan and revision-scoped result caches. A thread-local engine keeps the hot
//! path lock-free while preserving isolation between concurrent callers.

use finkit::factors::builtin_factor_registry;
use finkit::operation::UnifiedOperationEngine;
use std::cell::RefCell;

const DEFAULT_OPERATION_CACHE_CAPACITY: usize = 256;

thread_local! {
    static ENGINE: RefCell<Option<UnifiedOperationEngine>> = const { RefCell::new(None) };
}

/// Borrow the canonical built-in runtime for one FFI operation.
pub(crate) fn with_unified_engine<T>(
    operation: impl FnOnce(&mut UnifiedOperationEngine) -> T,
) -> T {
    ENGINE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let engine = slot.get_or_insert_with(|| {
            UnifiedOperationEngine::with_cache_capacity(
                builtin_factor_registry(),
                DEFAULT_OPERATION_CACHE_CAPACITY,
            )
        });
        operation(engine)
    })
}
