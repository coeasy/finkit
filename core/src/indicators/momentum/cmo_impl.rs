//! Chande momentum oscillator.

use super::prelude::*;

/// Chande Momentum Oscillator (CMO)
///
/// A momentum indicator that measures the percentage of sum of up days vs sum of down days.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of CMO values (-100 to 100 range)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::cmo(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn cmo(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period + 1)?;

    let len = input.len();
    let mut output = init_output(len);

    // Calculate changes
    let mut changes = Vec::with_capacity(len);
    changes.push(0.0);
    for i in 1..len {
        changes.push(input[i] - input[i - 1]);
    }

    // Initial sum for first period values
    let mut sum_up = 0.0;
    let mut sum_down = 0.0;
    for i in 1..=period {
        let ch = changes[i];
        if ch > 0.0 {
            sum_up += ch;
        } else {
            sum_down -= ch;
        }
    }

    let denom = sum_up + sum_down;
    if !crate::utils::is_zero(denom) {
        output[period] = (sum_up - sum_down) / denom * 100.0;
    }

    // Convert sums to averages for RMA initialization (like TA-Lib)
    let inv_period = 1.0 / period as f64;
    sum_up *= inv_period;
    sum_down *= inv_period;

    // Use RMA (Recursive Moving Average) for subsequent values
    for i in (period + 1)..len {
        let ch = changes[i];
        let up = if ch > 0.0 { ch } else { 0.0 };
        let down = if ch < 0.0 { -ch } else { 0.0 };

        // RMA: new_value = (old_value * (period - 1) + new_value) / period
        sum_up = (sum_up * (period as f64 - 1.0) + up) * inv_period;
        sum_down = (sum_down * (period as f64 - 1.0) + down) * inv_period;

        let denom = sum_up + sum_down;
        if !crate::utils::is_zero(denom) {
            output[i] = (sum_up - sum_down) / denom * 100.0;
        }
    }

    Ok(output)
}

/// Caller-owned CMO kernel for the Python public fast path. It keeps the
/// canonical RMA recurrence but avoids the temporary full-length changes
/// vector and the allocating `Array1` wrapper used by [`cmo`].
pub fn cmo_fast_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    validate_input(input.len(), period + 1)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.fill(f64::NAN);

    let mut sum_up = 0.0;
    let mut sum_down = 0.0;
    for i in 1..=period {
        let change = input[i] - input[i - 1];
        if change > 0.0 {
            sum_up += change;
        } else {
            sum_down -= change;
        }
    }

    let denominator = sum_up + sum_down;
    if !crate::utils::is_zero(denominator) {
        output[period] = (sum_up - sum_down) / denominator * 100.0;
    }

    let inv_period = 1.0 / period as f64;
    let period_minus_one = period as f64 - 1.0;
    sum_up *= inv_period;
    sum_down *= inv_period;
    for i in period + 1..input.len() {
        let change = input[i] - input[i - 1];
        let up = if change > 0.0 { change } else { 0.0 };
        let down = if change < 0.0 { -change } else { 0.0 };
        sum_up = (sum_up * period_minus_one + up) * inv_period;
        sum_down = (sum_down * period_minus_one + down) * inv_period;
        let denominator = sum_up + sum_down;
        if !crate::utils::is_zero(denominator) {
            output[i] = (sum_up - sum_down) / denominator * 100.0;
        }
    }
    Ok(())
}
