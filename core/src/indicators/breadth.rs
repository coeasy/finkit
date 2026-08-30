use crate::error::{Result, TaError};
use crate::math::moving_avg::ema;
use crate::utils::validate_input;
use ndarray::Array1;

/// Advance/Decline Line (AD Line)
///
/// A cumulative indicator that measures the number of advancing stocks minus declining stocks.
/// It helps assess the overall health of a market by tracking whether more stocks are rising or falling.
///
/// # Formula
/// AD Line\[i\] = AD Line\[i-1\] + (advances\[i\] - declines\[i\])
///
/// # Arguments
/// * `advances` - Number (or proportion) of advancing stocks per period
/// * `declines` - Number (or proportion) of declining stocks per period
///
/// # Returns
/// Array of cumulative AD Line values
///
/// # Errors
/// * When input arrays have different lengths
/// * When input data is empty
///
/// # Example
/// ```rust
/// use finkit::indicators::advance_decline_line;
///
/// let advances = vec![1500.0, 1200.0, 1800.0, 1100.0, 1600.0];
/// let declines = vec![1000.0, 1300.0, 900.0, 1400.0, 1100.0];
/// let result = advance_decline_line(&advances, &declines).unwrap();
/// assert_eq!(result.len(), 5);
/// assert_eq!(result[0], 500.0);
/// assert_eq!(result[1], 400.0);
/// ```
pub fn advance_decline_line(advances: &[f64], declines: &[f64]) -> Result<Array1<f64>> {
    if advances.len() != declines.len() {
        return Err(TaError::InvalidParameter {
            name: "advances and declines".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(advances.len(), 1)?;

    let len = advances.len();
    let mut output = Array1::zeros(len);
    let mut cumulative = 0.0;

    for i in 0..len {
        cumulative += advances[i] - declines[i];
        output[i] = cumulative;
    }

    Ok(output)
}

/// Advance/Decline Ratio (AD Ratio)
///
/// Measures the ratio of advancing stocks to declining stocks for each period.
/// Values above 1.0 indicate more stocks advancing, while values below 1.0 indicate more stocks declining.
///
/// # Formula
/// AD Ratio\[i\] = advances\[i\] / declines\[i\]
///
/// # Arguments
/// * `advances` - Number (or proportion) of advancing stocks per period
/// * `declines` - Number (or proportion) of declining stocks per period
///
/// # Returns
/// Array of AD Ratio values
///
/// # Errors
/// * When input arrays have different lengths
/// * When input data is empty
///
/// # Example
/// ```rust
/// use finkit::indicators::advance_decline_ratio;
///
/// let advances = vec![1500.0, 1200.0, 1800.0];
/// let declines = vec![1000.0, 1300.0, 900.0];
/// let result = advance_decline_ratio(&advances, &declines).unwrap();
/// assert!((result[0] - 1.5).abs() < 1e-10);
/// assert!((result[1] - 12.0 / 13.0).abs() < 1e-10);
/// ```
pub fn advance_decline_ratio(advances: &[f64], declines: &[f64]) -> Result<Array1<f64>> {
    if advances.len() != declines.len() {
        return Err(TaError::InvalidParameter {
            name: "advances and declines".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(advances.len(), 1)?;

    let len = advances.len();
    let mut output = Array1::zeros(len);

    for i in 0..len {
        if declines[i].abs() > 1e-15 {
            output[i] = advances[i] / declines[i];
        } else {
            output[i] = f64::NAN;
        }
    }

    Ok(output)
}

/// McClellan Oscillator
///
/// A market breadth indicator calculated as the difference between two EMAs of
/// the daily advance-decline difference. It helps identify overbought/oversold conditions
/// in the broader market.
///
/// # Formula
/// McClellan Osc = EMA(AD Difference, short_period) - EMA(AD Difference, long_period)
///
/// # Arguments
/// * `ad_diff` - Daily advance-decline difference (advances - declines)
/// * `short_period` - Short EMA period (default: 19)
/// * `long_period` - Long EMA period (default: 39)
///
/// # Returns
/// Array of McClellan Oscillator values
///
/// # Errors
/// * When input data length is insufficient for the long period
/// * When input data is empty
///
/// # Example
/// ```rust
/// use finkit::indicators::mcclellan_oscillator;
///
/// let ad_diff: Vec<f64> = (0..50).map(|i| (i as f64) % 10.0 - 5.0).collect();
/// let result = mcclellan_oscillator(&ad_diff, 19, 39).unwrap();
/// assert_eq!(result.len(), 50);
/// ```
pub fn mcclellan_oscillator(
    ad_diff: &[f64],
    short_period: usize,
    long_period: usize,
) -> Result<Array1<f64>> {
    if short_period == 0 || long_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "short_period and long_period".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    if short_period >= long_period {
        return Err(TaError::InvalidParameter {
            name: "short_period".to_string(),
            constraint: "must be less than long_period".to_string(),
        });
    }
    validate_input(ad_diff.len(), long_period)?;

    let len = ad_diff.len();

    let short_ema = ema(ad_diff, short_period)?;
    let long_ema = ema(ad_diff, long_period)?;

    let mut output = Array1::zeros(len);

    for i in 0..len {
        if !short_ema[i].is_nan() && !long_ema[i].is_nan() {
            output[i] = short_ema[i] - long_ema[i];
        } else {
            output[i] = f64::NAN;
        }
    }

    Ok(output)
}

/// McClellan Summation Index
///
/// A cumulative indicator derived from the McClellan Oscillator. It represents
/// the running sum of McClellan Oscillator values and is used to identify major
/// market tops and bottoms.
///
/// # Formula
/// McClellan Summation\[i\] = McClellan Summation\[i-1\] + McClellan Osc\[i\]
///
/// # Arguments
/// * `mcclellan_osc` - McClellan Oscillator values
///
/// # Returns
/// Array of cumulative McClellan Summation Index values
///
/// # Errors
/// * When input data is empty
///
/// # Example
/// ```rust
/// use finkit::indicators::mcclellan_summation;
///
/// let osc = vec![50.0, -30.0, 80.0, -20.0, 40.0];
/// let result = mcclellan_summation(&osc).unwrap();
/// assert_eq!(result.len(), 5);
/// assert_eq!(result[0], 50.0);
/// assert_eq!(result[1], 20.0);
/// assert_eq!(result[2], 100.0);
/// ```
pub fn mcclellan_summation(mcclellan_osc: &[f64]) -> Result<Array1<f64>> {
    validate_input(mcclellan_osc.len(), 1)?;

    let len = mcclellan_osc.len();
    let mut output = Array1::zeros(len);
    let mut cumulative = 0.0;

    for i in 0..len {
        if mcclellan_osc[i].is_finite() {
            cumulative += mcclellan_osc[i];
        }
        output[i] = cumulative;
    }

    Ok(output)
}

/// TRIN (Arms Index)
///
/// A short-term trading indicator that compares the advance/decline ratio
/// to the advance/decline volume ratio. It helps assess whether volume is
/// flowing into advancing or declining stocks.
///
/// # Interpretation
/// * TRIN < 1.0: Bullish (more volume in advancing stocks)
/// * TRIN = 1.0: Neutral
/// * TRIN > 1.0: Bearish (more volume in declining stocks)
/// * TRIN > 1.2: Oversold condition
/// * TRIN < 0.8: Overbought condition
///
/// # Formula
/// TRIN = (advances / declines) / (adv_volume / dec_volume)
///
/// # Arguments
/// * `advances` - Number of advancing stocks per period
/// * `declines` - Number of declining stocks per period
/// * `adv_volume` - Volume of advancing stocks per period
/// * `dec_volume` - Volume of declining stocks per period
///
/// # Returns
/// Array of TRIN values
///
/// # Errors
/// * When input arrays have different lengths
/// * When input data is empty
///
/// # Example
/// ```rust
/// use finkit::indicators::trin;
///
/// let advances = vec![1500.0, 1200.0, 800.0];
/// let declines = vec![1000.0, 1300.0, 1500.0];
/// let adv_volume = vec![500000.0, 400000.0, 300000.0];
/// let dec_volume = vec![400000.0, 500000.0, 600000.0];
/// let result = trin(&advances, &declines, &adv_volume, &dec_volume).unwrap();
/// assert_eq!(result.len(), 3);
/// ```
pub fn trin(
    advances: &[f64],
    declines: &[f64],
    adv_volume: &[f64],
    dec_volume: &[f64],
) -> Result<Array1<f64>> {
    let len = advances.len();
    if declines.len() != len || adv_volume.len() != len || dec_volume.len() != len {
        return Err(TaError::InvalidParameter {
            name: "advances, declines, adv_volume, dec_volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(len, 1)?;

    let mut output = Array1::zeros(len);

    for i in 0..len {
        let ad_ratio = if declines[i].abs() > 1e-15 {
            advances[i] / declines[i]
        } else {
            output[i] = f64::NAN;
            continue;
        };

        let volume_ratio = if dec_volume[i].abs() > 1e-15 {
            adv_volume[i] / dec_volume[i]
        } else {
            output[i] = f64::NAN;
            continue;
        };

        if volume_ratio.abs() > 1e-15 {
            output[i] = ad_ratio / volume_ratio;
        } else {
            output[i] = f64::NAN;
        }
    }

    Ok(output)
}

/// New Highs - New Lows Index
///
/// A market breadth indicator that subtracts the number of stocks making
/// new lows from those making new highs. Positive values indicate bullish
/// breadth, while negative values indicate bearish breadth.
///
/// # Formula
/// NH-NL\[i\] = new_highs\[i\] - new_lows\[i\]
///
/// # Arguments
/// * `highs` - Number of stocks making new highs per period
/// * `lows` - Number of stocks making new lows per period
///
/// # Returns
/// Array of NH-NL values
///
/// # Errors
/// * When input arrays have different lengths
/// * When input data is empty
///
/// # Example
/// ```rust
/// use finkit::indicators::new_highs_lows;
///
/// let highs = vec![200.0, 150.0, 50.0, 100.0, 300.0];
/// let lows = vec![50.0, 100.0, 200.0, 80.0, 30.0];
/// let result = new_highs_lows(&highs, &lows).unwrap();
/// assert_eq!(result[0], 150.0);
/// assert_eq!(result[2], -150.0);
/// ```
pub fn new_highs_lows(highs: &[f64], lows: &[f64]) -> Result<Array1<f64>> {
    if highs.len() != lows.len() {
        return Err(TaError::InvalidParameter {
            name: "highs and lows".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(highs.len(), 1)?;

    let len = highs.len();
    let mut output = Array1::zeros(len);

    for i in 0..len {
        output[i] = highs[i] - lows[i];
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_advance_decline_line_basic() {
        let advances = vec![1500.0, 1200.0, 1800.0, 1100.0, 1600.0];
        let declines = vec![1000.0, 1300.0, 900.0, 1400.0, 1100.0];
        let result = advance_decline_line(&advances, &declines).unwrap();
        assert_eq!(result.len(), 5);
        assert_relative_eq!(result[0], 500.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], 400.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 1300.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 1000.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 1500.0, epsilon = 1e-10);
    }

    #[test]
    fn test_advance_decline_line_negative() {
        let advances = vec![1000.0, 800.0, 500.0];
        let declines = vec![1500.0, 1200.0, 1000.0];
        let result = advance_decline_line(&advances, &declines).unwrap();
        assert_relative_eq!(result[0], -500.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], -900.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], -1400.0, epsilon = 1e-10);
    }

    #[test]
    fn test_advance_decline_line_length_mismatch() {
        let advances = vec![1000.0, 800.0];
        let declines = vec![500.0];
        assert!(advance_decline_line(&advances, &declines).is_err());
    }

    #[test]
    fn test_advance_decline_line_empty() {
        let empty: Vec<f64> = vec![];
        assert!(advance_decline_line(&empty, &empty).is_err());
    }

    #[test]
    fn test_advance_decline_ratio_basic() {
        let advances = vec![1500.0, 1200.0, 1800.0, 1100.0];
        let declines = vec![1000.0, 1300.0, 900.0, 1400.0];
        let result = advance_decline_ratio(&advances, &declines).unwrap();
        assert_eq!(result.len(), 4);
        assert_relative_eq!(result[0], 1.5, epsilon = 1e-10);
        assert_relative_eq!(result[1], 12.0 / 13.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 2.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 11.0 / 14.0, epsilon = 1e-10);
    }

    #[test]
    fn test_advance_decline_ratio_zero_denominator() {
        let advances = vec![1000.0, 1500.0];
        let declines = vec![0.0, 1000.0];
        let result = advance_decline_ratio(&advances, &declines).unwrap();
        assert!(result[0].is_nan());
        assert_relative_eq!(result[1], 1.5, epsilon = 1e-10);
    }

    #[test]
    fn test_mcclellan_oscillator_basic() {
        let ad_diff: Vec<f64> = (0..50).map(|i| (i as f64) % 10.0 - 5.0).collect();
        let result = mcclellan_oscillator(&ad_diff, 19, 39).unwrap();
        assert_eq!(result.len(), 50);
        assert!(result[37].is_nan());
        assert!(result[38].is_finite());
        assert!(result[49].is_finite());
    }

    #[test]
    fn test_mcclellan_oscillator_invalid_periods() {
        let ad_diff = vec![1.0, 2.0, 3.0];
        assert!(mcclellan_oscillator(&ad_diff, 0, 39).is_err());
        assert!(mcclellan_oscillator(&ad_diff, 39, 19).is_err());
        assert!(mcclellan_oscillator(&ad_diff, 39, 39).is_err());
    }

    #[test]
    fn test_mcclellan_oscillator_insufficient_data() {
        let ad_diff: Vec<f64> = (0..20).map(|i| i as f64).collect();
        assert!(mcclellan_oscillator(&ad_diff, 19, 39).is_err());
    }

    #[test]
    fn test_mcclellan_summation_basic() {
        let osc = vec![50.0, -30.0, 80.0, -20.0, 40.0];
        let result = mcclellan_summation(&osc).unwrap();
        assert_eq!(result.len(), 5);
        assert_relative_eq!(result[0], 50.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], 20.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 100.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 80.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 120.0, epsilon = 1e-10);
    }

    #[test]
    fn test_mcclellan_summation_negative() {
        let osc = vec![-50.0, -30.0, -80.0, -20.0];
        let result = mcclellan_summation(&osc).unwrap();
        assert_relative_eq!(result[0], -50.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], -80.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], -160.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], -180.0, epsilon = 1e-10);
    }

    #[test]
    fn test_mcclellan_summation_empty() {
        let empty: Vec<f64> = vec![];
        assert!(mcclellan_summation(&empty).is_err());
    }

    #[test]
    fn test_trin_basic() {
        let advances = vec![1500.0, 1200.0, 800.0];
        let declines = vec![1000.0, 1300.0, 1500.0];
        let adv_volume = vec![500000.0, 400000.0, 300000.0];
        let dec_volume = vec![400000.0, 500000.0, 600000.0];
        let result = trin(&advances, &declines, &adv_volume, &dec_volume).unwrap();
        assert_eq!(result.len(), 3);
        assert_relative_eq!(result[0], 1.5 / 1.25, epsilon = 1e-10);
        assert_relative_eq!(result[1], (12.0 / 13.0) / 0.8, epsilon = 1e-10);
        assert_relative_eq!(result[2], (8.0 / 15.0) / 0.5, epsilon = 1e-10);
    }

    #[test]
    fn test_trin_bullish_bearish() {
        let advances = vec![2000.0, 800.0];
        let declines = vec![1000.0, 1500.0];
        let adv_volume = vec![800000.0, 300000.0];
        let dec_volume = vec![100000.0, 200000.0];
        let result = trin(&advances, &declines, &adv_volume, &dec_volume).unwrap();
        let bullish_trin = (2000.0 / 1000.0) / (800000.0 / 100000.0);
        assert!(bullish_trin < 1.0, "Bullish: TRIN should be < 1.0");
        assert_relative_eq!(result[0], bullish_trin, epsilon = 1e-10);
    }

    #[test]
    fn test_trin_zero_denominator() {
        let advances = vec![1000.0, 1500.0, 1200.0];
        let declines = vec![0.0, 1000.0, 800.0];
        let adv_volume = vec![500000.0, 400000.0, 300000.0];
        let dec_volume = vec![400000.0, 0.0, 600000.0];
        let result = trin(&advances, &declines, &adv_volume, &dec_volume).unwrap();
        assert!(result[0].is_nan(), "Declines is 0");
        assert!(result[1].is_nan(), "dec_volume is 0");
        assert!(result[2].is_finite());
    }

    #[test]
    fn test_trin_length_mismatch() {
        let advances = vec![1000.0, 800.0];
        let declines = vec![500.0, 600.0];
        let adv_volume = vec![400000.0];
        let dec_volume = vec![300000.0, 200000.0];
        assert!(trin(&advances, &declines, &adv_volume, &dec_volume).is_err());
    }

    #[test]
    fn test_new_highs_lows_basic() {
        let highs = vec![200.0, 150.0, 50.0, 100.0, 300.0];
        let lows = vec![50.0, 100.0, 200.0, 80.0, 30.0];
        let result = new_highs_lows(&highs, &lows).unwrap();
        assert_eq!(result.len(), 5);
        assert_relative_eq!(result[0], 150.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], 50.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], -150.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 20.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 270.0, epsilon = 1e-10);
    }

    #[test]
    fn test_new_highs_lows_zero_values() {
        let highs = vec![0.0, 0.0, 0.0];
        let lows = vec![0.0, 0.0, 0.0];
        let result = new_highs_lows(&highs, &lows).unwrap();
        assert_eq!(result.len(), 3);
        for val in result.iter() {
            assert_relative_eq!(*val, 0.0, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_new_highs_lows_length_mismatch() {
        let highs = vec![100.0, 50.0];
        let lows = vec![30.0];
        assert!(new_highs_lows(&highs, &lows).is_err());
    }

    #[test]
    fn test_advance_decline_ratio_length_mismatch() {
        let advances = vec![1000.0, 800.0];
        let declines = vec![500.0];
        assert!(advance_decline_ratio(&advances, &declines).is_err());
    }

    #[test]
    fn test_advance_decline_ratio_empty() {
        let empty: Vec<f64> = vec![];
        assert!(advance_decline_ratio(&empty, &empty).is_err());
    }

    #[test]
    fn test_mcclellan_oscillator_constant_diff() {
        let ad_diff = vec![100.0; 50];
        let result = mcclellan_oscillator(&ad_diff, 19, 39).unwrap();
        assert!(result[37].is_nan());
        assert!(result[38].is_finite());
        let osc_value = result[49];
        assert!(osc_value.abs() < 1.0, "Constant diff should converge to ~0");
    }

    #[test]
    fn test_mcclellan_summation_with_nan() {
        let osc = vec![50.0, f64::NAN, 80.0, f64::NAN, 40.0];
        let result = mcclellan_summation(&osc).unwrap();
        assert_relative_eq!(result[0], 50.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], 50.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 130.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 130.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 170.0, epsilon = 1e-10);
    }

    #[test]
    fn test_trin_all_equal() {
        let advances = vec![1000.0, 1000.0, 1000.0];
        let declines = vec![1000.0, 1000.0, 1000.0];
        let adv_volume = vec![500000.0, 500000.0, 500000.0];
        let dec_volume = vec![500000.0, 500000.0, 500000.0];
        let result = trin(&advances, &declines, &adv_volume, &dec_volume).unwrap();
        for i in 0..result.len() {
            assert_relative_eq!(result[i], 1.0, epsilon = 1e-10);
        }
    }
}
