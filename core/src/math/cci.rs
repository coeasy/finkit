//! TA-Lib 0.7.1-compatible Commodity Channel Index kernel.
//!
//! For the small periods used by CCI, the reference circular buffer plus two
//! tight linear scans is both faster than maintaining a sorted Vec and exactly
//! preserves TA-Lib's floating-point operation order.

use crate::error::{Result, TaError};
use ndarray::Array1;

#[inline]
fn typical_price(high: f64, low: f64, close: f64) -> f64 {
    (high + low + close) / 3.0
}

pub fn cci(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if period < 2 {
        return Err(TaError::InvalidParameter {
            name: "timeperiod".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    if high.len() < period {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: period,
        });
    }

    let len = high.len();
    let lookback = period - 1;
    let mut output = vec![f64::NAN; len];
    let mut circ = vec![0.0; period];
    let mut circ_idx = 0usize;

    let mut i = 0usize;
    while i < lookback {
        circ[circ_idx] = typical_price(high[i], low[i], close[i]);
        i += 1;
        circ_idx += 1;
        if circ_idx >= period {
            circ_idx = 0;
        }
    }

    while i < len {
        let last_value = typical_price(high[i], low[i], close[i]);
        circ[circ_idx] = last_value;

        // Keep the same j=0..period accumulation order as TA_CCI 0.7.1.
        let mut average = 0.0;
        for &value in &circ {
            average += value;
        }
        average /= period as f64;

        let mut deviation_sum = 0.0;
        for &value in &circ {
            deviation_sum += (value - average).abs();
        }

        let delta = last_value - average;
        output[i] = if delta != 0.0 && deviation_sum != 0.0 {
            delta / (0.015 * (deviation_sum / period as f64))
        } else {
            0.0
        };

        circ_idx += 1;
        if circ_idx >= period {
            circ_idx = 0;
        }
        i += 1;
    }

    Ok(Array1::from_vec(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warmup_matches_period_minus_one() {
        let high = [10.0, 11.0, 12.0, 13.0, 14.0];
        let low = [9.0, 10.0, 11.0, 12.0, 13.0];
        let close = [9.5, 10.5, 11.5, 12.5, 13.5];
        let result = cci(&high, &low, &close, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_finite());
    }

    #[test]
    fn flat_series_returns_zero_after_warmup() {
        let high = vec![10.0; 32];
        let low = vec![10.0; 32];
        let close = vec![10.0; 32];
        let result = cci(&high, &low, &close, 14).unwrap();
        assert!(result.iter().take(13).all(|value| value.is_nan()));
        assert!(result.iter().skip(13).all(|value| *value == 0.0));
    }
}
