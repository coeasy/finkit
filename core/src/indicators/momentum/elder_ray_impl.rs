//! Elder-ray (bull/bear power).

use super::prelude::*;

/// Elder-Ray Indicator Result
#[derive(Debug, Clone)]
pub struct ElderRayResult {
    /// Force Index: (Close - Close\[1\]) * Volume
    pub force_index: Array1<f64>,
    /// Bull Power: High - EMA(Close, period)
    pub bull_power: Array1<f64>,
    /// Bear Power: Low - EMA(Close, period)
    pub bear_power: Array1<f64>,
}

/// Elder-Ray Indicator (ELDER-RAY)
///
/// Developed by Alexander Elder, this indicator uses three components to evaluate
/// the balance of power between bulls and bears in the market.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `period` - EMA lookback period for Bull/Bear Power calculation
///
/// # Returns
/// ElderRayResult containing Force Index, Bull Power, and Bear Power
///
/// # Formula
/// * Force Index = (Close\[i\] - Close\[i-1\]) * Volume\[i\]
/// * Bull Power = High\[i\] - EMA(Close, period)\[i\]
/// * Bear Power = Low\[i\] - EMA(Close, period)\[i\]
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let volume = vec![100.0, 110.0, 120.0, 115.0, 130.0, 125.0, 105.0, 95.0, 110.0, 115.0];
/// let result = indicators::elder_ray(&high, &low, &close, &volume, 5).unwrap();
/// assert_eq!(result.bull_power.len(), 10);
/// ```
pub fn elder_ray(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
) -> Result<ElderRayResult> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period + 1)?;

    let len = close.len();

    // Calculate Force Index: (Close[i] - Close[i-1]) * Volume[i]
    let mut force_index = init_output(len);
    for i in 1..len {
        force_index[i] = (close[i] - close[i - 1]) * volume[i];
    }

    // Calculate EMA of close prices
    let ema_close = ema(close, period)?;

    // Calculate Bull Power: High - EMA(Close)
    let mut bull_power = init_output(len);
    for i in 0..len {
        if !ema_close[i].is_nan() {
            bull_power[i] = high[i] - ema_close[i];
        }
    }

    // Calculate Bear Power: Low - EMA(Close)
    let mut bear_power = init_output(len);
    for i in 0..len {
        if !ema_close[i].is_nan() {
            bear_power[i] = low[i] - ema_close[i];
        }
    }

    Ok(ElderRayResult {
        force_index,
        bull_power,
        bear_power,
    })
}
