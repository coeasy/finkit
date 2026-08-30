//! Common error type shared across FFI boundaries.

use std::fmt;

/// Error type shared across language bindings.
///
/// Each binding maps its native error representation into this enum so the
/// *meaning* of a failure is consistent across Python / Node / Java / .NET /
/// Go / C. When extending a binding, prefer mapping into one of these
/// variants rather than inventing a new stringly-typed message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FfiError {
    /// Invalid or out-of-range input argument (e.g. `period == 0`).
    InvalidArgument { param: String, reason: String },
    /// Input arrays have mismatched lengths or insufficient data for warm-up.
    InvalidInput { reason: String },
    /// Requested indicator / category is not present in the registry.
    UnknownIndicator(String),
    /// Numeric failure (NaN / Inf, overflow) from the underlying computation.
    Computation(String),
    /// Catch-all for bindings that surface an opaque message.
    Other(String),
}

impl std::error::Error for FfiError {}

impl fmt::Display for FfiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FfiError::InvalidArgument { param, reason } => {
                write!(f, "invalid argument `{param}`: {reason}")
            }
            FfiError::InvalidInput { reason } => write!(f, "invalid input: {reason}"),
            FfiError::UnknownIndicator(name) => write!(f, "unknown indicator: {name}"),
            FfiError::Computation(msg) => write!(f, "computation error: {msg}"),
            FfiError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

/// Split a `Result<T, FfiError>` into an optional value and an optional owned
/// error string — the idiomatic pattern for C/FFI callers that return
/// `null` plus a message.
pub fn into_parts<T>(r: Result<T, FfiError>) -> (Option<T>, Option<String>) {
    match r {
        Ok(v) => (Some(v), None),
        Err(e) => (None, Some(e.to_string())),
    }
}
