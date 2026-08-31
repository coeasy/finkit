//! Domain error types for the technical analysis library.
//!
//! The error model follows a three-tier classification designed for industrial use:
//!
//! 1. [`IndicatorError`] — domain errors raised by indicator computations
//!    (numeric / data-shape problems).
//! 2. [`FormulaError`]   — domain errors raised by the formula engine
//!    (parsing, evaluation, resource limits).
//! 3. [`FfiError`][finkit::FfiError] — boundary errors raised when crossing the FFI
//!    edge into other languages (null pointers, undersized buffers, and
//!    conversions of the two domain errors above).
//!
//! The legacy [`TaError`] enum is retained as a top-level compatibility
//! shim. Old call sites that still construct [`TaError::EmptyInput`],
//! [`TaError::InsufficientData`], etc. continue to compile, and
//! the `?` operator transparently converts any of the three new
//! domain errors into [`TaError`] via the `From` impls below.

use thiserror::Error;

/// Indicator computation error.
///
/// Raised when an indicator function cannot produce a valid output
/// because of input data shape, invalid parameters, or numerical issues.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum IndicatorError {
    /// Not enough data points are available for the requested computation.
    #[error("insufficient data: need {required}, got {actual}")]
    InsufficientData {
        /// Minimum number of data points required.
        required: usize,
        /// Actual number of data points supplied.
        actual: usize,
    },

    /// A parameter violates its constraint.
    #[error("invalid parameter `{param}`: {reason}")]
    InvalidParameter {
        /// Name of the offending parameter.
        param: String,
        /// Human-readable reason the parameter is invalid.
        reason: String,
    },

    /// A numeric overflow was detected inside an indicator.
    #[error("numeric overflow in {indicator} at index {index}")]
    NumericOverflow {
        /// Name of the indicator that overflowed.
        indicator: String,
        /// Index in the input series at which the overflow occurred.
        index: usize,
    },

    /// NaN propagated through an indicator that does not tolerate NaN.
    #[error("NaN propagation in {indicator}")]
    NanPropagation {
        /// Name of the indicator that encountered NaN.
        indicator: String,
    },
}

/// Formula engine error.
///
/// Raised by the parser, type checker, and runtime evaluator of the
/// formula engine.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum FormulaError {
    /// A parse failure with a specific source location.
    #[error("parse error at line {line}, col {col}: {message}")]
    Parse {
        /// Line number (1-based) at which the parse failure occurred.
        line: usize,
        /// Column number (1-based) at which the parse failure occurred.
        col: usize,
        /// Description of the parse failure.
        message: String,
    },

    /// A function name was used that is not defined in the engine.
    #[error("undefined function: {name}")]
    UndefinedFunction {
        /// Name of the missing function.
        name: String,
    },

    /// A type mismatch was detected between operands.
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Name of the expected type.
        expected: String,
        /// Name of the actual type.
        actual: String,
    },

    /// Formula execution exceeded the allowed time budget.
    #[error("execution timeout after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds before the timeout fired.
        elapsed_ms: u64,
    },

    /// Formula execution exceeded the allowed memory budget.
    #[error("memory limit exceeded: used {used} bytes, limit {limit} bytes")]
    MemoryLimit {
        /// Number of bytes actually used.
        used: usize,
        /// Maximum number of bytes allowed.
        limit: usize,
    },

    /// Insufficient data available to evaluate the formula.
    ///
    /// Kept for backward compatibility — the canonical variant is
    /// [`IndicatorError::InsufficientData`].
    #[error("insufficient data: {0}")]
    InsufficientData(String),

    /// A parameter is invalid (e.g. out-of-range, unparseable, or missing).
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// A runtime error occurred during formula evaluation.
    #[error("runtime error: {0}")]
    RuntimeError(String),

    /// An invalid operation was attempted (e.g. type mismatch at runtime).
    #[error("invalid operation: {0}")]
    InvalidOperation(String),

    /// A parse error during formula compilation.
    #[error("parse error: {0}")]
    ParseError(String),

    /// A function is not supported in the current context.
    #[error("unsupported function: {0}")]
    UnsupportedFunction(String),
}

/// FFI boundary error.
///
/// Raised when crossing the FFI edge into other languages. Wraps
/// both [`IndicatorError`] and [`FormulaError`] so that they can be
/// reported uniformly to foreign callers.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum FfiError {
    /// A null pointer was supplied for a required input or output.
    #[error("null pointer")]
    NullPointer,

    /// A caller-supplied buffer is too small for the produced result.
    #[error("buffer too small: need {required}, got {actual}")]
    BufferTooSmall {
        /// Number of elements required.
        required: usize,
        /// Number of elements actually supplied.
        actual: usize,
    },

    /// An underlying indicator computation failed.
    #[error("{0}")]
    Indicator(#[from] IndicatorError),

    /// An underlying formula evaluation failed.
    #[error("{0}")]
    Formula(#[from] FormulaError),
}

/// Top-level error type for the technical analysis library.
///
/// `TaError` is preserved for backward compatibility with v0.x call sites.
/// New code should construct one of [`IndicatorError`], [`FormulaError`],
/// or [`FfiError`] directly; the `?` operator converts them into
/// `TaError` automatically via the `From` impls below.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum TaError {
    /// Input data is empty.
    ///
    /// Exists for backward compatibility; canonical variant is
    /// [`IndicatorError::InsufficientData`] / an empty `actual` value.
    #[deprecated(
        since = "0.4.0",
        note = "Use `TaError::Indicator(IndicatorError::InsufficientData { .. })` instead"
    )]
    #[error("Input data is empty")]
    EmptyInput,

    /// Input data is shorter than the minimum required length.
    ///
    /// Exists for backward compatibility; canonical variant is
    /// [`IndicatorError::InsufficientData`].
    #[deprecated(
        since = "0.4.0",
        note = "Use `TaError::Indicator(IndicatorError::InsufficientData { .. })` instead"
    )]
    #[error("Input data length ({length}) is less than required minimum ({required})")]
    InsufficientData {
        /// Actual length of the input data.
        length: usize,
        /// Minimum length required for the computation.
        required: usize,
    },

    /// A parameter value violates its constraint.
    ///
    /// Exists for backward compatibility; canonical variant is
    /// [`IndicatorError::InvalidParameter`].
    #[deprecated(
        since = "0.4.0",
        note = "Use `TaError::Indicator(IndicatorError::InvalidParameter { .. })` instead"
    )]
    #[error("Invalid parameter: {name} must be {constraint}")]
    InvalidParameter {
        /// Name of the invalid parameter.
        name: String,
        /// Constraint that the parameter must satisfy.
        constraint: String,
    },

    /// All values in the input data are NaN.
    ///
    /// Exists for backward compatibility; canonical variant is
    /// [`IndicatorError::NanPropagation`].
    #[deprecated(
        since = "0.4.0",
        note = "Use `TaError::Indicator(IndicatorError::NanPropagation { .. })` instead"
    )]
    #[error("All input values are NaN")]
    AllNaN,

    /// Division by zero was encountered during computation.
    ///
    /// Exists for backward compatibility; the closest canonical variant is
    /// [`IndicatorError::NanPropagation`].
    #[deprecated(
        since = "0.4.0",
        note = "Use `TaError::Indicator(IndicatorError::NanPropagation { .. })` instead"
    )]
    #[error("Division by zero encountered")]
    DivisionByZero,

    /// Price data is invalid (e.g. negative or zero where not allowed).
    ///
    /// Exists for backward compatibility; canonical variant is
    /// [`IndicatorError::InvalidParameter`] with a `param: "price"` field.
    #[deprecated(
        since = "0.4.0",
        note = "Use `TaError::Indicator(IndicatorError::InvalidParameter { .. })` instead"
    )]
    #[error("Invalid price data: {message}")]
    InvalidPrice {
        /// Description of why the price data is invalid.
        message: String,
    },

    /// A generic computation error occurred.
    ///
    /// Exists for backward compatibility; canonical variant is
    /// [`IndicatorError::NumericOverflow`].
    #[deprecated(
        since = "0.4.0",
        note = "Use `TaError::Indicator(IndicatorError::NumericOverflow { .. })` instead"
    )]
    #[error("Computation error: {message}")]
    ComputationError {
        /// Description of the computation failure.
        message: String,
    },

    /// An FFI boundary error.
    #[error(transparent)]
    Ffi(FfiError),

    /// A domain error from an indicator computation.
    #[error("{0}")]
    Indicator(IndicatorError),

    /// A domain error from the formula engine.
    #[error("{0}")]
    Formula(FormulaError),
}

impl From<IndicatorError> for TaError {
    fn from(err: IndicatorError) -> Self {
        TaError::Indicator(err)
    }
}

impl From<FormulaError> for TaError {
    fn from(err: FormulaError) -> Self {
        TaError::Formula(err)
    }
}

impl From<FfiError> for TaError {
    fn from(err: FfiError) -> Self {
        TaError::Ffi(err)
    }
}

impl TaError {
    /// Returns `true` if this error (or its inner variant) corresponds to
    /// an "empty input" condition.
    pub fn is_empty_input(&self) -> bool {
        matches!(self, TaError::EmptyInput)
    }

    /// Returns `true` if this error (or its inner variant) corresponds to
    /// an "insufficient data" condition.
    #[allow(deprecated)]
    pub fn is_insufficient_data(&self) -> bool {
        matches!(
            self,
            TaError::InsufficientData { .. }
                | TaError::Indicator(IndicatorError::InsufficientData { .. })
        )
    }

    /// Returns `true` if this error (or its inner variant) corresponds to
    /// an "invalid parameter" condition.
    #[allow(deprecated)]
    pub fn is_invalid_parameter(&self) -> bool {
        matches!(
            self,
            TaError::InvalidParameter { .. }
                | TaError::Indicator(IndicatorError::InvalidParameter { .. })
        )
    }

    /// Returns `true` if this error (or its inner variant) corresponds to
    /// a NaN propagation condition.
    #[allow(deprecated)]
    pub fn is_all_nan(&self) -> bool {
        matches!(self, TaError::AllNaN)
    }

    /// Returns `true` if this error (or its inner variant) corresponds to
    /// a division-by-zero condition.
    #[allow(deprecated)]
    pub fn is_division_by_zero(&self) -> bool {
        matches!(self, TaError::DivisionByZero)
    }

    /// Returns `true` if this error (or its inner variant) corresponds to
    /// an invalid price condition.
    #[allow(deprecated)]
    pub fn is_invalid_price(&self) -> bool {
        matches!(self, TaError::InvalidPrice { .. })
    }

    /// Returns `true` if this error (or its inner variant) corresponds to
    /// a generic computation failure.
    #[allow(deprecated)]
    pub fn is_computation_error(&self) -> bool {
        matches!(self, TaError::ComputationError { .. })
    }

    /// If this `TaError` wraps an [`IndicatorError`], returns a reference
    /// to it; otherwise returns `None`.
    pub fn as_indicator_error(&self) -> Option<&IndicatorError> {
        match self {
            TaError::Indicator(e) => Some(e),
            _ => None,
        }
    }

    /// If this `TaError` wraps a [`FormulaError`], returns a reference
    /// to it; otherwise returns `None`.
    pub fn as_formula_error(&self) -> Option<&FormulaError> {
        match self {
            TaError::Formula(e) => Some(e),
            _ => None,
        }
    }

    /// If this `TaError` wraps an [`FfiError`], returns a reference to it;
    /// otherwise returns `None`.
    pub fn as_ffi_error(&self) -> Option<&FfiError> {
        match self {
            TaError::Ffi(e) => Some(e),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, TaError>;

#[cfg(test)]
mod tests {
    use super::*;

    // ---- New domain error tests (Phase 6 acceptance) ---------------------

    #[test]
    fn test_indicator_error_insufficient_data_display() {
        let err = IndicatorError::InsufficientData {
            required: 10,
            actual: 5,
        };
        let s = err.to_string();
        assert!(
            s.contains("10"),
            "expected `10` in display string, got: {s}"
        );
        assert!(s.contains("5"), "expected `5` in display string, got: {s}");
    }

    #[test]
    fn test_indicator_error_invalid_parameter_display() {
        let err = IndicatorError::InvalidParameter {
            param: "period".to_string(),
            reason: "must be > 0".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("period"));
        assert!(s.contains("must be > 0"));
    }

    #[test]
    fn test_indicator_error_numeric_overflow_display() {
        let err = IndicatorError::NumericOverflow {
            indicator: "RSI".to_string(),
            index: 42,
        };
        let s = err.to_string();
        assert!(s.contains("RSI"));
        assert!(s.contains("42"));
    }

    #[test]
    fn test_indicator_error_nan_propagation_display() {
        let err = IndicatorError::NanPropagation {
            indicator: "EMA".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("EMA"));
    }

    #[test]
    fn test_indicator_error_into_ta_error() {
        let ind_err = IndicatorError::NumericOverflow {
            indicator: "ATR".to_string(),
            index: 7,
        };
        let ta_err: TaError = ind_err.into();
        assert!(matches!(ta_err, TaError::Indicator(_)));
        match ta_err {
            TaError::Indicator(IndicatorError::NumericOverflow { indicator, index }) => {
                assert_eq!(indicator, "ATR");
                assert_eq!(index, 7);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_formula_error_parse_display() {
        let err = FormulaError::Parse {
            line: 3,
            col: 12,
            message: "unexpected token".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("3"));
        assert!(s.contains("12"));
        assert!(s.contains("unexpected token"));
    }

    #[test]
    fn test_formula_error_undefined_function_display() {
        let err = FormulaError::UndefinedFunction {
            name: "FOO".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("FOO"));
    }

    #[test]
    fn test_formula_error_type_mismatch_display() {
        let err = FormulaError::TypeMismatch {
            expected: "f64".to_string(),
            actual: "i32".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("f64"));
        assert!(s.contains("i32"));
    }

    #[test]
    fn test_formula_error_timeout_display() {
        let err = FormulaError::Timeout { elapsed_ms: 1500 };
        assert_eq!(err.to_string(), "execution timeout after 1500ms");
    }

    #[test]
    fn test_formula_error_memory_limit_display() {
        let err = FormulaError::MemoryLimit {
            used: 2048,
            limit: 1024,
        };
        let s = err.to_string();
        assert!(s.contains("2048"));
        assert!(s.contains("1024"));
    }

    #[test]
    fn test_formula_error_into_ta_error() {
        let f_err = FormulaError::UndefinedFunction {
            name: "BAR".to_string(),
        };
        let ta_err: TaError = f_err.into();
        assert!(matches!(ta_err, TaError::Formula(_)));
    }

    #[test]
    fn test_ffi_error_null_pointer_display() {
        let err = FfiError::NullPointer;
        assert_eq!(err.to_string(), "null pointer");
    }

    #[test]
    fn test_ffi_error_buffer_too_small_display() {
        let err = FfiError::BufferTooSmall {
            required: 100,
            actual: 50,
        };
        let s = err.to_string();
        assert!(s.contains("100"));
        assert!(s.contains("50"));
    }

    #[test]
    fn test_ffi_error_from_indicator_error() {
        let ind_err = IndicatorError::InsufficientData {
            required: 30,
            actual: 10,
        };
        let ffi_err: FfiError = ind_err.into();
        assert!(matches!(ffi_err, FfiError::Indicator(_)));
    }

    #[test]
    fn test_ffi_error_from_formula_error() {
        let f_err = FormulaError::Timeout { elapsed_ms: 100 };
        let ffi_err: FfiError = f_err.into();
        assert!(matches!(ffi_err, FfiError::Formula(_)));
    }

    #[test]
    fn test_ffi_error_into_ta_error() {
        let ffi_err = FfiError::NullPointer;
        let ta_err: TaError = ffi_err.into();
        assert!(matches!(ta_err, TaError::Ffi(_)));
    }

    // ---- Backward-compatibility tests -----------------------------------

    #[test]
    fn test_ta_error_invalid_parameter_backward_compat() {
        // The legacy direct construction path must still work.
        #[allow(deprecated)]
        {
            let _ = TaError::InvalidParameter {
                name: "p".to_string(),
                constraint: "> 0".to_string(),
            };
        }
    }

    #[test]
    #[allow(deprecated)]
    fn test_backward_compat_all_ta_error_variants() {
        let _ = TaError::EmptyInput;
        let _ = TaError::InsufficientData {
            length: 0,
            required: 1,
        };
        let _ = TaError::InvalidParameter {
            name: "p".into(),
            constraint: "> 0".into(),
        };
        let _ = TaError::AllNaN;
        let _ = TaError::DivisionByZero;
        let _ = TaError::InvalidPrice {
            message: "neg".into(),
        };
        let _ = TaError::ComputationError {
            message: "overflow".into(),
        };
    }

    #[test]
    #[allow(deprecated)]
    fn test_legacy_display_messages() {
        let err = TaError::EmptyInput;
        assert_eq!(format!("{err}"), "Input data is empty");

        let err = TaError::InsufficientData {
            length: 5,
            required: 10,
        };
        assert_eq!(
            format!("{err}"),
            "Input data length (5) is less than required minimum (10)"
        );

        let err = TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "positive".to_string(),
        };
        assert_eq!(
            format!("{err}"),
            "Invalid parameter: period must be positive"
        );
    }

    #[test]
    fn test_result_type_alias() {
        let ok: Result<f64> = Ok(1.0);
        assert!(ok.is_ok());

        let err: Result<f64> = Err(TaError::Indicator(IndicatorError::InsufficientData {
            required: 10,
            actual: 3,
        }));
        assert!(err.is_err());
    }

    #[test]
    fn test_errors_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<IndicatorError>();
        assert_send_sync::<FormulaError>();
        assert_send_sync::<FfiError>();
        assert_send_sync::<TaError>();
    }

    #[test]
    fn test_errors_implement_std_error() {
        fn assert_std_error<T: std::error::Error>() {}
        assert_std_error::<IndicatorError>();
        assert_std_error::<FormulaError>();
        assert_std_error::<FfiError>();
        assert_std_error::<TaError>();
    }

    #[test]
    fn test_as_indicator_error_returns_some() {
        let err = TaError::Indicator(IndicatorError::InsufficientData {
            required: 10,
            actual: 5,
        });
        assert!(err.as_indicator_error().is_some());
        assert!(err.as_formula_error().is_none());
        assert!(err.as_ffi_error().is_none());
    }

    #[test]
    fn test_as_formula_error_returns_some() {
        let err = TaError::Formula(FormulaError::Timeout { elapsed_ms: 1 });
        assert!(err.as_formula_error().is_some());
        assert!(err.as_indicator_error().is_none());
        assert!(err.as_ffi_error().is_none());
    }

    #[test]
    fn test_as_ffi_error_returns_some() {
        let err = TaError::Ffi(FfiError::NullPointer);
        assert!(err.as_ffi_error().is_some());
        assert!(err.as_indicator_error().is_none());
        assert!(err.as_formula_error().is_none());
    }

    #[test]
    fn test_is_methods_indirect_variants() {
        let err = TaError::Indicator(IndicatorError::InsufficientData {
            required: 10,
            actual: 5,
        });
        assert!(err.is_insufficient_data());
        assert!(!err.is_invalid_parameter());

        let err = TaError::Indicator(IndicatorError::InvalidParameter {
            param: "p".to_string(),
            reason: "> 0".to_string(),
        });
        assert!(err.is_invalid_parameter());
    }
}
