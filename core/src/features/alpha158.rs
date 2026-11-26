//! Matrix adapter for Qlib's Alpha158 feature collection.

use super::{Feature, FeatureMatrix};
use crate::factors::{builtin, FactorError};
use crate::formula::FormulaContext;
use ndarray::Array1;

/// Evaluate Alpha158 in Qlib's declared feature order into a time-by-feature matrix.
///
/// This is an ordered matrix adapter over the same precompiled expressions used
/// by [`crate::factors::builtin::factor_library`]. The result has 158 columns in
/// [`crate::factors::builtin::alpha158::EXPRESSIONS`] order.
///
/// # Errors
///
/// Returns [`FactorError::LengthMismatch`] when input series do not share a
/// length, or propagates a factor library build/evaluation failure.
pub fn alpha158_features(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    vwap: &[f64],
    volume: &[f64],
) -> Result<FeatureMatrix, FactorError> {
    let expected = close.len();
    for (name, values) in [
        ("open", open),
        ("high", high),
        ("low", low),
        ("vwap", vwap),
        ("volume", volume),
    ] {
        if values.len() != expected {
            return Err(FactorError::LengthMismatch {
                name: name.to_string(),
                expected,
                actual: values.len(),
            });
        }
    }

    let library = builtin::factor_library("alpha158")?;
    let mut context = FormulaContext::from_borrowed_ohlcv(open, high, low, close, volume, None);
    context.set_variable("VWAP".to_string(), Array1::from_vec(vwap.to_vec()));

    let mut matrix = FeatureMatrix::with_capacity(expected, builtin::alpha158::FACTOR_COUNT);
    for (name, _) in builtin::alpha158::EXPRESSIONS {
        let values = library.evaluate(name, &context)?;
        matrix.add_column(Feature::new(*name, "alpha158", 0), values);
    }
    Ok(matrix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_qlib_alpha158_output_order() {
        let close = (0..100).map(|i| 100.0 + i as f64).collect::<Vec<_>>();
        let volume = (0..100).map(|i| 1000.0 + i as f64).collect::<Vec<_>>();
        let open = close.iter().map(|value| value - 0.5).collect::<Vec<_>>();
        let high = close.iter().map(|value| value + 1.0).collect::<Vec<_>>();
        let low = close.iter().map(|value| value - 1.0).collect::<Vec<_>>();
        let vwap = close.clone();
        let matrix = alpha158_features(&open, &high, &low, &close, &vwap, &volume).unwrap();

        assert_eq!(matrix.cols(), 158);
        assert_eq!(matrix.rows(), close.len());
        assert_eq!(
            matrix.column_names(),
            crate::factors::builtin::alpha158::EXPRESSIONS
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            matrix.column_by_name("OPEN0").unwrap()[99],
            open[99] / close[99]
        );
    }

    #[test]
    fn rejects_misaligned_inputs() {
        let values = vec![1.0, 2.0, 3.0];
        let error = alpha158_features(&values[..2], &values, &values, &values, &values, &values)
            .unwrap_err();
        assert_eq!(
            error,
            FactorError::LengthMismatch {
                name: "open".to_string(),
                expected: 3,
                actual: 2,
            }
        );
    }
}
