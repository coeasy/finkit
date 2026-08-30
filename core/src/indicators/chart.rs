use crate::error::{Result, TaError};
use crate::utils::{init_output, validate_input, validate_param};
use ndarray::Array1;

/// Heikin-Ashi candlestick result
#[derive(Debug, Clone)]
pub struct HeikinAshiResult {
    pub ha_open: Array1<f64>,
    pub ha_high: Array1<f64>,
    pub ha_low: Array1<f64>,
    pub ha_close: Array1<f64>,
}

/// Heikin-Ashi (HA) candlestick transformation
///
/// Converts standard OHLC data into Heikin-Ashi smoothed candles.
///
/// # Formulas
/// - HA_Close = (Open + High + Low + Close) / 4
/// - HA_Open\[0\] = (Open\[0\] + Close\[0\]) / 2
/// - HA_Open\[i\] = (HA_Open\[i-1\] + HA_Close\[i-1\]) / 2
/// - HA_High = max(High, HA_Open, HA_Close)
/// - HA_Low = min(Low, HA_Open, HA_Close)
#[inline]
pub fn heikin_ashi(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<HeikinAshiResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 1)?;

    let len = open.len();
    let mut ha_o = vec![f64::NAN; len];
    let mut ha_h = vec![f64::NAN; len];
    let mut ha_l = vec![f64::NAN; len];
    let mut ha_c = vec![f64::NAN; len];

    let mut prev_open = f64::NAN;
    let mut prev_close = f64::NAN;
    for i in 0..len {
        let o = open[i]; let h = high[i]; let l = low[i]; let c = close[i];
        if o.is_nan() || h.is_nan() || l.is_nan() || c.is_nan() {
            prev_open = f64::NAN;
            prev_close = f64::NAN;
            continue;
        }
        let hc = (o + h + l + c) * 0.25;
        let ho = if prev_open.is_nan() {
            (o + c) * 0.5
        } else {
            (prev_open + prev_close) * 0.5
        };
        ha_c[i] = hc;
        ha_o[i] = ho;
        ha_h[i] = h.max(ho).max(hc);
        ha_l[i] = l.min(ho).min(hc);
        prev_open = ho;
        prev_close = hc;
    }

    Ok(HeikinAshiResult {
        ha_open: Array1::from_vec(ha_o),
        ha_high: Array1::from_vec(ha_h),
        ha_low: Array1::from_vec(ha_l),
        ha_close: Array1::from_vec(ha_c),
    })
}

/// ZigZag indicator result
#[derive(Debug, Clone)]
pub struct ZigZagResult {
    /// ZigZag line values (NaN except at pivot points)
    pub zigzag: Array1<f64>,
    /// Pivot points as (index, price) tuples
    pub pivots: Vec<(usize, f64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZigZagTrend {
    Up,
    Down,
}

/// ZigZag indicator
///
/// Identifies significant price reversals based on a percentage threshold.
/// Only pivot points are populated in the output array; all other values are NaN.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `threshold` - Reversal threshold in percent (e.g. 5.0 = 5%)
pub fn zigzag(high: &[f64], low: &[f64], threshold: f64) -> Result<ZigZagResult> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;
    validate_param("threshold", "greater than 0", || threshold > 0.0)?;

    let len = high.len();
    let mut zigzag = init_output(len);
    let mut pivots = Vec::new();

    #[inline(always)]
    fn is_valid(high: &[f64], low: &[f64], i: usize) -> bool {
        !high[i].is_nan() && !low[i].is_nan() && high[i].is_finite() && low[i].is_finite()
    }

    #[inline(always)]
    fn safe_pct(diff: f64, base: f64) -> f64 {
        if base.abs() < f64::EPSILON { 0.0 } else { diff / base * 100.0 }
    }

    if len == 1 {
        if is_valid(high, low, 0) {
            let mid = (high[0] + low[0]) / 2.0;
            pivots.push((0, mid));
            zigzag[0] = mid;
        }
        return Ok(ZigZagResult { zigzag, pivots });
    }

    let first_valid = (0..len).find(|&i| is_valid(high, low, i));
    let first = match first_valid {
        Some(idx) => idx,
        None => return Ok(ZigZagResult { zigzag, pivots }),
    };

    let mut trend: Option<ZigZagTrend> = None;
    let mut ext_idx = first;
    let mut ext_val = 0.0_f64;

    for i in (first + 1)..len {
        if !is_valid(high, low, i) {
            continue;
        }

        match trend {
            None => {
                let rise_pct = safe_pct(high[i] - low[first], low[first]);
                let fall_pct = safe_pct(high[first] - low[i], high[first]);

                if rise_pct >= threshold {
                    let pivot_price = low[first];
                    pivots.push((first, pivot_price));
                    zigzag[first] = pivot_price;
                    trend = Some(ZigZagTrend::Up);
                    ext_idx = i;
                    ext_val = high[i];
                } else if fall_pct >= threshold {
                    let pivot_price = high[first];
                    pivots.push((first, pivot_price));
                    zigzag[first] = pivot_price;
                    trend = Some(ZigZagTrend::Down);
                    ext_idx = i;
                    ext_val = low[i];
                }
            }
            Some(ZigZagTrend::Up) => {
                if high[i] > ext_val {
                    ext_val = high[i];
                    ext_idx = i;
                }

                if i > ext_idx {
                    let reversal_pct = safe_pct(ext_val - low[i], ext_val);
                    if reversal_pct >= threshold {
                        pivots.push((ext_idx, ext_val));
                        zigzag[ext_idx] = ext_val;
                        trend = Some(ZigZagTrend::Down);
                        ext_idx = i;
                        ext_val = low[i];
                        continue;
                    }
                }
            }
            Some(ZigZagTrend::Down) => {
                if low[i] < ext_val {
                    ext_val = low[i];
                    ext_idx = i;
                }

                if i > ext_idx {
                    let reversal_pct = safe_pct(high[i] - ext_val, ext_val);
                    if reversal_pct >= threshold {
                        pivots.push((ext_idx, ext_val));
                        zigzag[ext_idx] = ext_val;
                        trend = Some(ZigZagTrend::Up);
                        ext_idx = i;
                        ext_val = high[i];
                        continue;
                    }
                }
            }
        }
    }

    if let Some(_current_trend) = trend {
        let last_pivot = pivots.last().map(|(idx, _)| *idx);
        if last_pivot != Some(ext_idx) {
            pivots.push((ext_idx, ext_val));
            zigzag[ext_idx] = ext_val;
        }
    } else if is_valid(high, low, first) {
        let mid = (high[first] + low[first]) / 2.0;
        pivots.push((first, mid));
        zigzag[first] = mid;
    }

    Ok(ZigZagResult { zigzag, pivots })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_heikin_ashi_basic() {
        let open = vec![10.0, 11.0, 12.0];
        let high = vec![12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![11.0, 12.0, 13.0];

        let result = heikin_ashi(&open, &high, &low, &close).unwrap();

        assert_eq!(result.ha_close.len(), 3);
        assert_relative_eq!(result.ha_close[0], (10.0 + 12.0 + 9.0 + 11.0) / 4.0, epsilon = 1e-10);
        assert_relative_eq!(result.ha_open[0], (10.0 + 11.0) / 2.0, epsilon = 1e-10);
        assert_relative_eq!(
            result.ha_open[1],
            (result.ha_open[0] + result.ha_close[0]) / 2.0,
            epsilon = 1e-10
        );

        for i in 0..3 {
            assert!(result.ha_high[i] >= result.ha_open[i]);
            assert!(result.ha_high[i] >= result.ha_close[i]);
            assert!(result.ha_low[i] <= result.ha_open[i]);
            assert!(result.ha_low[i] <= result.ha_close[i]);
        }
    }

    #[test]
    fn test_heikin_ashi_invalid_lengths() {
        let open = vec![10.0, 11.0];
        let high = vec![12.0];
        let low = vec![9.0, 10.0];
        let close = vec![11.0, 12.0];

        assert!(heikin_ashi(&open, &high, &low, &close).is_err());
    }

    #[test]
    fn test_heikin_ashi_empty_input() {
        let open: Vec<f64> = vec![];
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let close: Vec<f64> = vec![];

        assert!(heikin_ashi(&open, &high, &low, &close).is_err());
    }

    #[test]
    fn test_zigzag_basic_reversals() {
        let high = vec![10.0, 12.0, 15.0, 14.0, 11.0, 13.0, 16.0];
        let low = vec![8.0, 10.0, 13.0, 12.0, 9.0, 11.0, 14.0];

        let result = zigzag(&high, &low, 5.0).unwrap();

        assert_eq!(result.pivots.len(), 4);
        assert_eq!(result.pivots[0], (0, 8.0));
        assert_eq!(result.pivots[1], (2, 15.0));
        assert_eq!(result.pivots[2], (4, 9.0));
        assert_eq!(result.pivots[3], (6, 16.0));

        assert!(result.zigzag[0].is_finite());
        assert!(result.zigzag[2].is_finite());
        assert!(result.zigzag[4].is_finite());
        assert!(result.zigzag[6].is_finite());
        assert!(result.zigzag[1].is_nan());
        assert!(result.zigzag[3].is_nan());
        assert!(result.zigzag[5].is_nan());
    }

    #[test]
    fn test_zigzag_single_bar() {
        let high = vec![105.0];
        let low = vec![100.0];

        let result = zigzag(&high, &low, 5.0).unwrap();

        assert_eq!(result.pivots.len(), 1);
        assert_eq!(result.pivots[0], (0, 102.5));
        assert_relative_eq!(result.zigzag[0], 102.5, epsilon = 1e-10);
    }

    #[test]
    fn test_zigzag_no_reversal() {
        let high = vec![10.0, 10.1, 10.2];
        let low = vec![9.9, 10.0, 10.1];

        let result = zigzag(&high, &low, 5.0).unwrap();

        assert_eq!(result.pivots.len(), 1);
        assert_relative_eq!(result.pivots[0].1, 9.95, epsilon = 1e-10);
        assert!(result.zigzag[1].is_nan());
        assert!(result.zigzag[2].is_nan());
    }

    #[test]
    fn test_zigzag_invalid_lengths() {
        let high = vec![10.0, 12.0];
        let low = vec![8.0];

        assert!(zigzag(&high, &low, 5.0).is_err());
    }

    #[test]
    fn test_zigzag_invalid_threshold() {
        let high = vec![10.0, 12.0];
        let low = vec![8.0, 10.0];

        assert!(zigzag(&high, &low, 0.0).is_err());
        assert!(zigzag(&high, &low, -1.0).is_err());
    }

    #[test]
    fn test_zigzag_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];

        assert!(zigzag(&high, &low, 5.0).is_err());
    }

    #[test]
    fn test_zigzag_zero_prices() {
        let high = vec![0.0, 0.0, 10.0, 12.0, 8.0];
        let low = vec![0.0, 0.0, 8.0, 10.0, 6.0];

        let result = zigzag(&high, &low, 5.0).unwrap();
        for val in result.zigzag.iter() {
            assert!(!val.is_infinite(), "zigzag should never produce inf");
        }
        for (_, price) in &result.pivots {
            assert!(price.is_finite(), "pivot price should be finite");
        }
    }

    #[test]
    fn test_zigzag_all_zero_prices() {
        let high = vec![0.0, 0.0, 0.0];
        let low = vec![0.0, 0.0, 0.0];

        let result = zigzag(&high, &low, 5.0).unwrap();
        for val in result.zigzag.iter() {
            assert!(!val.is_infinite(), "zigzag should never produce inf");
        }
    }

    #[test]
    fn test_zigzag_nan_inputs() {
        let high = vec![10.0, f64::NAN, 15.0, 14.0, 11.0, f64::NAN, 16.0];
        let low = vec![8.0, f64::NAN, 13.0, 12.0, 9.0, f64::NAN, 14.0];

        let result = zigzag(&high, &low, 5.0).unwrap();
        for val in result.zigzag.iter() {
            assert!(!val.is_infinite(), "zigzag should never produce inf with NaN inputs");
        }
        for (_, price) in &result.pivots {
            assert!(price.is_finite(), "pivot price should be finite with NaN inputs");
        }
    }

    #[test]
    fn test_zigzag_all_nan() {
        let high = vec![f64::NAN, f64::NAN, f64::NAN];
        let low = vec![f64::NAN, f64::NAN, f64::NAN];

        let result = zigzag(&high, &low, 5.0).unwrap();
        assert!(result.pivots.is_empty(), "all NaN input should produce no pivots");
    }

    #[test]
    fn test_zigzag_leading_nan() {
        let high = vec![f64::NAN, f64::NAN, 10.0, 15.0, 14.0, 9.0, 16.0];
        let low = vec![f64::NAN, f64::NAN, 8.0, 13.0, 12.0, 7.0, 14.0];

        let result = zigzag(&high, &low, 5.0).unwrap();
        for (_, price) in &result.pivots {
            assert!(price.is_finite());
        }
        assert!(result.zigzag[0].is_nan());
        assert!(result.zigzag[1].is_nan());
    }

    #[test]
    fn test_zigzag_single_bar_zero() {
        let high = vec![0.0];
        let low = vec![0.0];

        let result = zigzag(&high, &low, 5.0).unwrap();
        assert_eq!(result.pivots.len(), 1);
        assert_relative_eq!(result.pivots[0].1, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_zigzag_single_bar_nan() {
        let high = vec![f64::NAN];
        let low = vec![f64::NAN];

        let result = zigzag(&high, &low, 5.0).unwrap();
        assert!(result.pivots.is_empty());
    }

    #[test]
    fn test_heikin_ashi_nan_gap() {
        let open = vec![10.0, f64::NAN, 12.0, 13.0];
        let high = vec![12.0, f64::NAN, 14.0, 15.0];
        let low = vec![9.0, f64::NAN, 11.0, 12.0];
        let close = vec![11.0, f64::NAN, 13.0, 14.0];

        let result = heikin_ashi(&open, &high, &low, &close).unwrap();
        assert!(result.ha_close[0].is_finite());
        assert!(result.ha_close[1].is_nan());
        assert!(result.ha_close[2].is_finite());
        assert!(result.ha_open[2].is_finite());
    }

    #[test]
    fn test_heikin_ashi_zero_prices() {
        let open = vec![0.0, 0.0, 10.0];
        let high = vec![0.0, 0.0, 12.0];
        let low = vec![0.0, 0.0, 8.0];
        let close = vec![0.0, 0.0, 11.0];

        let result = heikin_ashi(&open, &high, &low, &close).unwrap();
        for i in 0..3 {
            assert!(result.ha_close[i].is_finite());
            assert!(result.ha_open[i].is_finite());
        }
    }
}
