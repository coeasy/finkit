//! Qlib-compatible Alpha360 price and volume feature set.
//!
//! Alpha360 contains six ordered groups (close, open, high, low, vwap,
//! volume), each with observations from lag 59 through lag 0. Price fields are
//! divided by the current close; volume is divided by current volume plus
//! `1e-12`. Missing history is represented by `NaN`.

use super::{Feature, FeatureMatrix};

/// Alpha360's stable input group and output order.
pub const ALPHA360_GROUPS: [&str; 6] = ["CLOSE", "OPEN", "HIGH", "LOW", "VWAP", "VOLUME"];

/// Number of observations in each Alpha360 field group.
pub const ALPHA360_WINDOW: usize = 60;

/// Error returned when Alpha360 inputs do not share one time axis.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Alpha360Error {
    /// The named series has a different row count from `close`.
    #[error("length mismatch for {name}: expected {expected}, got {actual}")]
    LengthMismatch {
        /// Input series name.
        name: &'static str,
        /// Number of rows in `close`.
        expected: usize,
        /// Number of rows in the mismatched series.
        actual: usize,
    },
}

/// Generate the Qlib Alpha360 feature matrix.
///
/// Columns are ordered as `CLOSE59..CLOSE0`, `OPEN59..OPEN0`,
/// `HIGH59..HIGH0`, `LOW59..LOW0`, `VWAP59..VWAP0`, and
/// `VOLUME59..VOLUME0`. Rows preserve the input time order.
///
/// Price values are divided by the current close without a denominator guard,
/// matching Qlib expressions. Volume values use the denominator
/// `current_volume + 1e-12`. For a lag that predates the first input row, the
/// feature is `NaN`.
///
/// # Errors
///
/// Returns [`Alpha360Error::LengthMismatch`] if any input length differs from
/// `close`.
pub fn alpha360(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    vwap: &[f64],
    volume: &[f64],
) -> Result<FeatureMatrix, Alpha360Error> {
    let expected = close.len();
    for (name, values) in [
        ("open", open),
        ("high", high),
        ("low", low),
        ("vwap", vwap),
        ("volume", volume),
    ] {
        if values.len() != expected {
            return Err(Alpha360Error::LengthMismatch {
                name,
                expected,
                actual: values.len(),
            });
        }
    }

    let mut matrix =
        FeatureMatrix::with_capacity(expected, ALPHA360_GROUPS.len() * ALPHA360_WINDOW);
    for (group, values) in [
        ("CLOSE", close),
        ("OPEN", open),
        ("HIGH", high),
        ("LOW", low),
        ("VWAP", vwap),
        ("VOLUME", volume),
    ] {
        for lag in (0..ALPHA360_WINDOW).rev() {
            let column = (0..expected)
                .map(|row| {
                    if row < lag {
                        f64::NAN
                    } else if group == "VOLUME" {
                        values[row - lag] / (volume[row] + 1e-12)
                    } else {
                        values[row - lag] / close[row]
                    }
                })
                .collect();
            matrix.add_column(
                Feature::new(format!("{group}{lag}"), "alpha360", lag),
                column,
            );
        }
    }
    Ok(matrix)
}

/// Stable Alpha360 feature names in Qlib output order.
#[must_use]
pub fn alpha360_names() -> Vec<String> {
    ALPHA360_GROUPS
        .iter()
        .flat_map(|group| {
            (0..ALPHA360_WINDOW)
                .rev()
                .map(move |lag| format!("{group}{lag}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let close = (1..=62).map(|v| v as f64 * 10.0).collect::<Vec<_>>();
        let open = close.iter().map(|v| v + 1.0).collect();
        let high = close.iter().map(|v| v + 2.0).collect();
        let low = close.iter().map(|v| v - 2.0).collect();
        let vwap = close.iter().map(|v| v + 0.5).collect();
        let volume = (1..=62).map(|v| v as f64 * 100.0).collect();
        (open, high, low, close, vwap, volume)
    }

    #[test]
    fn output_shape_names_and_order_match_qlib_contract() {
        let (open, high, low, close, vwap, volume) = fixture();
        let matrix = alpha360(&open, &high, &low, &close, &vwap, &volume).unwrap();

        assert_eq!(matrix.rows(), close.len());
        assert_eq!(matrix.cols(), 360);
        assert_eq!(
            matrix.column_names(),
            alpha360_names()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );
        assert_eq!(matrix.column_names()[0], "CLOSE59");
        assert_eq!(matrix.column_names()[59], "CLOSE0");
        assert_eq!(matrix.column_names()[60], "OPEN59");
        assert_eq!(matrix.column_names()[359], "VOLUME0");
    }

    #[test]
    fn lag_normalization_and_warmup_match_qlib_expressions() {
        let (open, high, low, close, vwap, volume) = fixture();
        let matrix = alpha360(&open, &high, &low, &close, &vwap, &volume).unwrap();

        assert!(matrix.column_by_name("CLOSE59").unwrap()[0].is_nan());
        assert_eq!(matrix.column_by_name("CLOSE59").unwrap()[59], 10.0 / 600.0);
        assert_eq!(matrix.column_by_name("CLOSE0").unwrap()[61], 1.0);
        assert_eq!(matrix.column_by_name("OPEN1").unwrap()[61], 611.0 / 620.0);
        assert_eq!(matrix.column_by_name("VWAP0").unwrap()[61], 620.5 / 620.0);
        assert!((matrix.column_by_name("VOLUME0").unwrap()[61] - 1.0).abs() < 1e-12);
        assert!(
            (matrix.column_by_name("VOLUME1").unwrap()[61] - (6100.0 / (6200.0 + 1e-12))).abs()
                < 1e-12
        );
    }

    #[test]
    fn zero_denominators_preserve_reference_arithmetic() {
        let open = [1.0, 2.0];
        let high = [1.0, 2.0];
        let low = [1.0, 2.0];
        let close = [0.0, 0.0];
        let vwap = [1.0, 2.0];
        let volume = [0.0, 0.0];
        let matrix = alpha360(&open, &high, &low, &close, &vwap, &volume).unwrap();

        assert!(matrix.column_by_name("CLOSE0").unwrap()[0].is_nan());
        assert!(matrix.column_by_name("OPEN0").unwrap()[1].is_infinite());
        assert_eq!(matrix.column_by_name("VOLUME0").unwrap()[0], 0.0);
    }

    #[test]
    fn rejects_misaligned_series() {
        let (open, high, low, close, vwap, volume) = fixture();
        let error = alpha360(&open[..3], &high, &low, &close, &vwap, &volume).unwrap_err();
        assert_eq!(
            error,
            Alpha360Error::LengthMismatch {
                name: "open",
                expected: close.len(),
                actual: 3,
            }
        );
    }
}
