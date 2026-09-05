//! Single-output directional indicator kernels backed by the shared OHLC family state.
//!
//! Architecture v3.1 requires +DI/-DI/DX/ADX/ATR/NATR to consume one canonical
//! Wilder transition model instead of maintaining near-duplicate state machines.
//! Standalone +DI/-DI keep their public API while projecting only the requested
//! output from [`OhlcFamilyState`].

use crate::error::{Result, TaError};
use crate::math::ohlc_family_state::OhlcFamilyState;
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
    // Preserve the established public minimum-input contract.
    validate_input(high.len(), period * 2)?;

    let mut state = OhlcFamilyState::new(period).expect("period validated above");
    let mut output = Vec::with_capacity(high.len());
    for index in 0..high.len() {
        let sample = state.update(high[index], low[index], close[index]);
        output.push(match direction {
            Direction::Plus => sample.plus_di,
            Direction::Minus => sample.minus_di,
        });
    }
    Ok(Array1::from_vec(output))
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

    #[test]
    fn standalone_projections_use_one_family_transition() {
        let high: Vec<f64> = (0..64)
            .map(|i| 100.0 + i as f64 * 0.17 + ((i % 5) as f64 - 2.0) * 0.13)
            .collect();
        let low: Vec<f64> = high
            .iter()
            .enumerate()
            .map(|(i, value)| value - 1.4 - (i % 3) as f64 * 0.03)
            .collect();
        let close: Vec<f64> = high
            .iter()
            .zip(low.iter())
            .map(|(h, l)| (h + l) * 0.5)
            .collect();
        let period = 14;
        let plus = plus_di(&high, &low, &close, period).unwrap();
        let minus = minus_di(&high, &low, &close, period).unwrap();
        let mut state = OhlcFamilyState::new(period).unwrap();

        for index in 0..high.len() {
            let sample = state.update(high[index], low[index], close[index]);
            if sample.plus_di.is_nan() {
                assert!(plus[index].is_nan());
                assert!(minus[index].is_nan());
            } else {
                assert_eq!(plus[index], sample.plus_di);
                assert_eq!(minus[index], sample.minus_di);
            }
        }
    }
}
