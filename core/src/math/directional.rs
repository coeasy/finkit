//! Single-output directional indicator kernels.
//!
//! The legacy `compute_di_only` path computes and allocates both +DI and -DI
//! even when the caller requests only one side.  Installed-wheel calls are
//! independent, so maintain only the requested Wilder DM state plus the shared
//! true-range state and write one output buffer exactly once.

use crate::error::{Result, TaError};
use crate::utils::validate_input;
use ndarray::Array1;

#[derive(Clone, Copy)]
enum Direction {
    Plus,
    Minus,
}

#[inline]
fn directional_di(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    direction: Direction,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    // Keep the established public contract used by compute_di_only.
    validate_input(high.len(), period * 2)?;

    let len = high.len();
    let p = period as f64;
    let mut smooth_dm = 0.0;
    let mut smooth_tr = 0.0;

    // TA-Lib warm-up: accumulate period-1 DM/TR values, then process bar
    // `period` with the Wilder recurrence before emitting the first DI value.
    unsafe {
        let high_ptr = high.as_ptr();
        let low_ptr = low.as_ptr();
        let close_ptr = close.as_ptr();

        for i in 1..period {
            let current_high = *high_ptr.add(i);
            let previous_high = *high_ptr.add(i - 1);
            let current_low = *low_ptr.add(i);
            let previous_low = *low_ptr.add(i - 1);
            let previous_close = *close_ptr.add(i - 1);

            let up_move = current_high - previous_high;
            let down_move = previous_low - current_low;
            smooth_tr += crate::utils::true_range(current_high, current_low, previous_close);
            smooth_dm += match direction {
                Direction::Plus if up_move > down_move && up_move > 0.0 => up_move,
                Direction::Minus if down_move > up_move && down_move > 0.0 => down_move,
                _ => 0.0,
            };
        }

        let mut output = Vec::with_capacity(len);
        output.resize(period, f64::NAN);

        for i in period..len {
            let current_high = *high_ptr.add(i);
            let previous_high = *high_ptr.add(i - 1);
            let current_low = *low_ptr.add(i);
            let previous_low = *low_ptr.add(i - 1);
            let previous_close = *close_ptr.add(i - 1);

            let up_move = current_high - previous_high;
            let down_move = previous_low - current_low;
            let tr = crate::utils::true_range(current_high, current_low, previous_close);
            let dm = match direction {
                Direction::Plus if up_move > down_move && up_move > 0.0 => up_move,
                Direction::Minus if down_move > up_move && down_move > 0.0 => down_move,
                _ => 0.0,
            };

            // Keep division form and update order bit-compatible with the
            // established TA-Lib-compatible implementation.
            smooth_dm = smooth_dm - smooth_dm / p + dm;
            smooth_tr = smooth_tr - smooth_tr / p + tr;
            output.push(if smooth_tr.abs() > 1e-15 {
                smooth_dm / smooth_tr * 100.0
            } else {
                0.0
            });
        }

        debug_assert_eq!(output.len(), len);
        Ok(Array1::from_vec(output))
    }
}

/// Plus Directional Indicator (+DI).
pub fn plus_di(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    directional_di(high, low, close, period, Direction::Plus)
}

/// Minus Directional Indicator (-DI).
pub fn minus_di(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    directional_di(high, low, close, period, Direction::Minus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outputs_share_talib_warmup() {
        let high: Vec<f64> = (0..40).map(|i| 100.0 + i as f64 * 0.5).collect();
        let low: Vec<f64> = high.iter().map(|v| v - 2.0).collect();
        let close: Vec<f64> = high.iter().map(|v| v - 0.7).collect();
        let plus = plus_di(&high, &low, &close, 14).unwrap();
        let minus = minus_di(&high, &low, &close, 14).unwrap();
        assert!(plus.iter().take(14).all(|value| value.is_nan()));
        assert!(minus.iter().take(14).all(|value| value.is_nan()));
        assert!(plus[14].is_finite());
        assert!(minus[14].is_finite());
    }
}
