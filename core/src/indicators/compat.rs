//! Compatibility wrappers for public indicator names whose historical internal
//! implementation intentionally differs from the release-gate reference.
//!
//! Keep these wrappers small: the canonical math kernels remain reusable from
//! Python, formulas, batch plans, and future language bindings.

use crate::error::Result;
use ndarray::Array1;

/// Commodity Channel Index using the TA-Lib 0.7.1 circular-buffer operation
/// order. This avoids the float-reordering drift of the sorted-window research
/// implementation while also reducing overhead for the common small periods.
pub fn cci(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    crate::math::cci::cci(high, low, close, period).map(Array1::from_vec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_compat_cci_has_expected_warmup() {
        let high: Vec<f64> = (0..64).map(|i| 100.0 + i as f64 * 0.1).collect();
        let low: Vec<f64> = high.iter().map(|v| v - 1.0).collect();
        let close: Vec<f64> = high.iter().map(|v| v - 0.4).collect();
        let output = cci(&high, &low, &close, 14).unwrap();
        assert!(output[..13].iter().all(|value| value.is_nan()));
        assert!(output[13..].iter().all(|value| value.is_finite()));
    }
}
