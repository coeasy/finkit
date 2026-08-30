//! Numeric / array conversion helpers shared by bindings.

use crate::error::FfiError;

/// Copy a borrowed `f64` slice into an owned `Vec<f64>`.
///
/// This is the common return shape for bindings that cannot return borrowed
/// slices across the FFI wall (C, Go, .NET). Keeping the copy in one place
/// avoids each binding re-implementing the same allocation.
pub fn to_vec(slice: &[f64]) -> Vec<f64> {
    slice.to_vec()
}

/// Validate a `period` argument, returning [`FfiError::InvalidArgument`] when
/// the caller passed `0` — the most common invalid-argument case across
/// indicators. Bindings should call this before invoking the core function.
pub fn validate_period(period: usize) -> Result<usize, FfiError> {
    if period == 0 {
        Err(FfiError::InvalidArgument {
            param: "period".to_string(),
            reason: "must be >= 1".to_string(),
        })
    } else {
        Ok(period)
    }
}

/// Validate two input slices have equal length (e.g. `high`/`low`/`close`).
pub fn validate_equal_len(a: &[f64], b: &[f64], a_name: &str, b_name: &str) -> Result<(), FfiError> {
    if a.len() != b.len() {
        Err(FfiError::InvalidInput {
            reason: format!("`{a_name}` (len {}) and `{b_name}` (len {}) must match", a.len(), b.len()),
        })
    } else {
        Ok(())
    }
}
