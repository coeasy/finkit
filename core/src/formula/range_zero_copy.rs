//! Borrowed-input range evaluation for FormulaEngine.
//!
//! Kept as a small extension module so the range contract can be reused by FFI
//! callers without duplicating the core engine's dependency-window semantics.

use crate::formula::{CompiledFormula, FormulaContext, FormulaEngine, FormulaError};
use ndarray::Array1;

impl FormulaEngine {
    /// Evaluate a half-open range while borrowing caller-owned OHLCV slices.
    ///
    /// The public inputs are not copied at the FFI boundary.  `eval_range` still
    /// decides the minimum safe dependency window and preserves effectful-formula
    /// semantics.  `amount` remains optional for compatibility with the existing
    /// borrowed context constructor.
    #[allow(clippy::too_many_arguments)]
    pub fn eval_range_zero_copy_inputs(
        &self,
        formula: &CompiledFormula,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
        start: usize,
        end: usize,
        amount: Option<&[f64]>,
    ) -> Result<Array1<f64>, FormulaError> {
        if close.is_empty()
            || [open, high, low, close, volume]
                .iter()
                .any(|values| values.len() != close.len())
            || amount.is_some_and(|values| values.len() != close.len())
            || start > end
            || end > close.len()
        {
            return Err(FormulaError::InvalidParameter(
                "zero-copy range inputs must be non-empty/equal-length and satisfy 0 <= start <= end <= len"
                    .to_string(),
            ));
        }

        let context = FormulaContext::from_borrowed_ohlcv(
            open,
            high,
            low,
            close,
            volume,
            amount.map(|values| Array1::from_vec(values.to_vec())),
        );
        self.eval_range(formula, &context, start, end)
    }
}
