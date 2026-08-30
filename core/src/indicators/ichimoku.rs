use crate::error::{Result, TaError};
use crate::math::statistics::{rolling_max, rolling_min};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

/// Ichimoku Cloud Result
#[derive(Debug, Clone)]
pub struct IchimokuResult {
    /// Tenkan-sen (Conversion Line) = (9-period high + 9-period low) / 2
    pub tenkan_sen: Array1<f64>,
    /// Kijun-sen (Base Line) = (26-period high + 26-period low) / 2
    pub kijun_sen: Array1<f64>,
    /// Senkou Span A (Leading Span A) = (Tenkan-sen + Kijun-sen) / 2, shifted forward 26 periods
    pub senkou_span_a: Array1<f64>,
    /// Senkou Span B (Leading Span B) = (52-period high + 52-period low) / 2, shifted forward 26 periods
    pub senkou_span_b: Array1<f64>,
    /// Chikou Span (Lagging Span) = Close price, shifted backward 26 periods
    pub chikou_span: Array1<f64>,
}

/// Ichimoku Cloud (Ichimoku Kinko Hyo)
///
/// A comprehensive indicator that shows support and resistance, identifies trend direction,
/// gauges momentum, and provides trading signals.
///
/// # Components
/// - **Tenkan-sen (Conversion Line)**: (9-period high + 9-period low) / 2
/// - **Kijun-sen (Base Line)**: (26-period high + 26-period low) / 2
/// - **Senkou Span A (Leading Span A)**: (Tenkan-sen + Kijun-sen) / 2, plotted 26 periods ahead
/// - **Senkou Span B (Leading Span B)**: (52-period high + 52-period low) / 2, plotted 26 periods ahead
/// - **Chikou Span (Lagging Span)**: Close price, plotted 26 periods behind
///
/// The area between Senkou Span A and Senkou Span B is called the "cloud" (Kumo).
/// - Price above the cloud indicates an uptrend
/// - Price below the cloud indicates a downtrend
/// - Price inside the cloud indicates a ranging market
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `tenkan_period` - Tenkan-sen period (default: 9)
/// * `kijun_period` - Kijun-sen period (default: 26)
/// * `senkou_b_period` - Senkou Span B period (default: 52)
/// * `displacement` - Forward/backward displacement period (default: 26)
///
/// # Returns
/// IchimokuResult containing all five lines
///
/// # Example
/// ```
/// use finkit::indicators::ichimoku;
///
/// let high: Vec<f64> = (0..60).map(|i| 100.0 + i as f64).collect();
/// let low: Vec<f64> = (0..60).map(|i| 98.0 + i as f64).collect();
/// let close: Vec<f64> = (0..60).map(|i| 99.0 + i as f64).collect();
///
/// let result = ichimoku(&high, &low, &close, 9, 26, 52, 26).unwrap();
/// assert_eq!(result.tenkan_sen.len(), 60);
/// assert_eq!(result.kijun_sen.len(), 60);
/// assert_eq!(result.senkou_span_a.len(), 60);
/// assert_eq!(result.senkou_span_b.len(), 60);
/// assert_eq!(result.chikou_span.len(), 60);
/// ```
pub fn ichimoku(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    tenkan_period: usize,
    kijun_period: usize,
    senkou_b_period: usize,
    displacement: usize,
) -> Result<IchimokuResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }

    let max_period = tenkan_period.max(kijun_period).max(senkou_b_period);
    validate_input(high.len(), max_period)?;

    let len = high.len();

    // Calculate Tenkan-sen: (9-period high + 9-period low) / 2
    let high_rolling = rolling_max(high, tenkan_period)?;
    let low_rolling = rolling_min(low, tenkan_period)?;
    let mut tenkan_sen = init_output(len);
    for i in 0..len {
        if !high_rolling[i].is_nan() && !low_rolling[i].is_nan() {
            tenkan_sen[i] = (high_rolling[i] + low_rolling[i]) / 2.0;
        }
    }

    // Calculate Kijun-sen: (26-period high + 26-period low) / 2
    let high_rolling_k = rolling_max(high, kijun_period)?;
    let low_rolling_k = rolling_min(low, kijun_period)?;
    let mut kijun_sen = init_output(len);
    for i in 0..len {
        if !high_rolling_k[i].is_nan() && !low_rolling_k[i].is_nan() {
            kijun_sen[i] = (high_rolling_k[i] + low_rolling_k[i]) / 2.0;
        }
    }

    // Calculate Senkou Span A: (Tenkan-sen + Kijun-sen) / 2, shifted forward by displacement
    let mut senkou_span_a = init_output(len);
    for i in 0..len {
        if !tenkan_sen[i].is_nan() && !kijun_sen[i].is_nan() {
            let span_a_value = (tenkan_sen[i] + kijun_sen[i]) / 2.0;
            // Shift forward: value at time i is plotted at time i + displacement
            if i + displacement < len {
                senkou_span_a[i + displacement] = span_a_value;
            }
        }
    }

    // Calculate Senkou Span B: (52-period high + 52-period low) / 2, shifted forward by displacement
    let high_rolling_sb = rolling_max(high, senkou_b_period)?;
    let low_rolling_sb = rolling_min(low, senkou_b_period)?;
    let mut senkou_span_b = init_output(len);
    for i in 0..len {
        if !high_rolling_sb[i].is_nan() && !low_rolling_sb[i].is_nan() {
            let span_b_value = (high_rolling_sb[i] + low_rolling_sb[i]) / 2.0;
            // Shift forward: value at time i is plotted at time i + displacement
            if i + displacement < len {
                senkou_span_b[i + displacement] = span_b_value;
            }
        }
    }

    // Calculate Chikou Span: Close price, shifted backward by displacement
    let mut chikou_span = init_output(len);
    for i in displacement..len {
        chikou_span[i - displacement] = close[i];
    }

    Ok(IchimokuResult {
        tenkan_sen,
        kijun_sen,
        senkou_span_a,
        senkou_span_b,
        chikou_span,
    })
}

/// Ichimoku Cloud with default parameters
///
/// Uses standard periods: Tenkan=9, Kijun=26, Senkou B=52, Displacement=26
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
///
/// # Returns
/// IchimokuResult containing all five lines
pub fn ichimoku_default(high: &[f64], low: &[f64], close: &[f64]) -> Result<IchimokuResult> {
    ichimoku(high, low, close, 9, 26, 52, 26)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn create_test_data(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = (0..len).map(|i| 100.0 + i as f64).collect();
        let low: Vec<f64> = (0..len).map(|i| 98.0 + i as f64).collect();
        let close: Vec<f64> = (0..len).map(|i| 99.0 + i as f64).collect();
        (high, low, close)
    }

    #[test]
    fn test_ichimoku_basic() {
        let (high, low, close) = create_test_data(60);
        let result = ichimoku(&high, &low, &close, 9, 26, 52, 26).unwrap();

        assert_eq!(result.tenkan_sen.len(), 60);
        assert_eq!(result.kijun_sen.len(), 60);
        assert_eq!(result.senkou_span_a.len(), 60);
        assert_eq!(result.senkou_span_b.len(), 60);
        assert_eq!(result.chikou_span.len(), 60);
    }

    #[test]
    fn test_tenkan_sen_calculation() {
        let len = 60;
        let high: Vec<f64> = (0..len).map(|i| 10.0 + i as f64 * 0.1).collect();
        let low: Vec<f64> = (0..len).map(|i| 8.0 + i as f64 * 0.1).collect();
        let close: Vec<f64> = (0..len).map(|i| 9.0 + i as f64 * 0.1).collect();

        let result = ichimoku(&high, &low, &close, 9, 26, 52, 26).unwrap();

        // At index 8 (9th element), tenkan = (max of 9 highs + min of 9 lows) / 2
        assert!(!result.tenkan_sen[8].is_nan());
        // First 8 values should be NaN (need 9 periods)
        for i in 0..8 {
            assert!(result.tenkan_sen[i].is_nan());
        }
    }

    #[test]
    fn test_kijun_sen_initial_nan() {
        let (high, low, close) = create_test_data(60);
        let result = ichimoku(&high, &low, &close, 9, 26, 52, 26).unwrap();

        // First 25 values should be NaN (need 26 periods)
        for i in 0..25 {
            assert!(result.kijun_sen[i].is_nan());
        }
        // At index 25, should have a value
        assert!(!result.kijun_sen[25].is_nan());
    }

    #[test]
    fn test_senkou_span_a_forward_shift() {
        let (high, low, close) = create_test_data(80);
        let result = ichimoku(&high, &low, &close, 9, 26, 52, 26).unwrap();

        // First valid tenkan at index 8, first valid kijun at index 25
        // First valid senkou_span_a = value at index 25 shifted to index 51
        for i in 0..51 {
            assert!(result.senkou_span_a[i].is_nan());
        }
        // At index 51, should have the first valid value
        assert!(!result.senkou_span_a[51].is_nan());
    }

    #[test]
    fn test_senkou_span_b_forward_shift() {
        let (high, low, close) = create_test_data(80);
        let result = ichimoku(&high, &low, &close, 9, 26, 52, 26).unwrap();

        // First valid senkou_b at index 51, shifted forward 26 gives index 77
        for i in 0..77 {
            assert!(result.senkou_span_b[i].is_nan());
        }
        assert!(!result.senkou_span_b[77].is_nan());
    }

    #[test]
    fn test_chikou_span_backward_shift() {
        let (high, low, close) = create_test_data(60);
        let result = ichimoku(&high, &low, &close, 9, 26, 52, 26).unwrap();

        // Last 26 values should be NaN (backward shift)
        for i in (60 - 26)..60 {
            assert!(result.chikou_span[i].is_nan());
        }
        // At index 0, should equal close[26]
        assert_relative_eq!(result.chikou_span[0], close[26], epsilon = 1e-10);
        // At index 33, should equal close[59]
        assert_relative_eq!(result.chikou_span[33], close[59], epsilon = 1e-10);
    }

    #[test]
    fn test_ichimoku_default() {
        let (high, low, close) = create_test_data(60);
        let result = ichimoku_default(&high, &low, &close).unwrap();

        assert_eq!(result.tenkan_sen.len(), 60);
        assert!(!result.tenkan_sen[8].is_nan());
        assert!(!result.kijun_sen[25].is_nan());
    }

    #[test]
    fn test_insufficient_data() {
        let high = vec![100.0, 101.0];
        let low = vec![98.0, 99.0];
        let close = vec![99.0, 100.0];

        let result = ichimoku(&high, &low, &close, 9, 26, 52, 26);
        assert!(result.is_err());
    }

    #[test]
    fn test_mismatched_lengths() {
        let high = vec![100.0, 101.0, 102.0];
        let low = vec![98.0, 99.0];
        let close = vec![99.0, 100.0, 101.0];

        let result = ichimoku(&high, &low, &close, 9, 26, 52, 26);
        assert!(result.is_err());
    }

    #[test]
    fn test_senkou_span_a_formula() {
        // Create data where we can verify Senkou Span A = (Tenkan + Kijun) / 2
        let len = 60;
        let high: Vec<f64> = vec![110.0; len];
        let low: Vec<f64> = vec![90.0; len];
        let close: Vec<f64> = vec![100.0; len];

        let result = ichimoku(&high, &low, &close, 9, 26, 52, 26).unwrap();

        // Tenkan = (110 + 90) / 2 = 100
        // Kijun = (110 + 90) / 2 = 100
        // Senkou Span A = (100 + 100) / 2 = 100
        assert_relative_eq!(result.tenkan_sen[8], 100.0, epsilon = 1e-10);
        assert_relative_eq!(result.kijun_sen[25], 100.0, epsilon = 1e-10);
        // Senkou Span A at index 26+25=51 uses values from index 25
        assert_relative_eq!(result.senkou_span_a[51], 100.0, epsilon = 1e-10);
    }

    #[test]
    fn test_custom_periods() {
        let (high, low, close) = create_test_data(100);
        let result = ichimoku(&high, &low, &close, 5, 15, 30, 10).unwrap();

        assert_eq!(result.tenkan_sen.len(), 100);
        // First valid tenkan at index 4 (5 periods)
        assert!(!result.tenkan_sen[4].is_nan());
        // First valid kijun at index 14 (15 periods)
        assert!(!result.kijun_sen[14].is_nan());
        // First valid senkou_b at index 29 (30 periods), shifted forward 10 gives index 39
        assert!(!result.senkou_span_b[39].is_nan());
    }
}
