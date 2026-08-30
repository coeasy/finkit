use crate::error::{Result, TaError};
use crate::indicators::volatility::atr;
use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

/// SuperTrend 趋势跟踪指标结果
///
/// 包含趋势方向、趋势线以及上下轨道带
pub struct SuperTrendResult {
    /// 趋势方向：1 表示看涨（上升），-1 表示看跌（下降）
    pub direction: Array1<i32>,
    /// SuperTrend 趋势线（当前趋势的支撑/阻力线）
    pub trend_line: Array1<f64>,
    /// 上轨道带
    pub upper_band: Array1<f64>,
    /// 下轨道带
    pub lower_band: Array1<f64>,
}

/// SuperTrend 趋势跟踪指标
///
/// SuperTrend 是一种基于波动率（ATR）的趋势跟踪指标，通过计算价格的中点
/// 并加减 ATR 的倍数来生成上下轨道带，然后根据价格与轨道带的关系判断趋势方向。
///
/// # 算法说明
///
/// 1. 计算 ATR（平均真实波幅）
/// 2. 计算基本带：
///    - Basic Upper Band = (High + Low) / 2 + multiplier * ATR
///    - Basic Lower Band = (High + Low) / 2 - multiplier * ATR
/// 3. 计算最终带（考虑前一期值，保持带的连续性）：
///    - Final Upper Band：如果当前 Basic Upper Band 低于前一期 Final Upper Band，
///      或前一期收盘价高于前一期 Final Upper Band，则使用 Basic Upper Band
///    - Final Lower Band：如果当前 Basic Lower Band 高于前一期 Final Lower Band，
///      或前一期收盘价低于前一期 Final Lower Band，则使用 Basic Lower Band
/// 4. 判断趋势方向并确定 SuperTrend 线：
///    - 看涨时，SuperTrend = Final Lower Band，direction = 1
///    - 看跌时，SuperTrend = Final Upper Band，direction = -1
///
/// # 参数
/// * `high` - 最高价序列
/// * `low` - 最低价序列
/// * `close` - 收盘价序列
/// * `atr_period` - ATR 计算周期（默认 10）
/// * `multiplier` - ATR 乘数（默认 3.0）
///
/// # 返回值
/// 返回 `SuperTrendResult` 结构体，包含方向、趋势线和上下轨道带
///
/// # 示例
/// ```
/// use alpha_ta_core::indicators::supertrend;
///
/// let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 30.0];
/// let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0];
/// let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0, 25.0, 27.0, 29.0];
///
/// let result = supertrend(&high, &low, &close, 10, 3.0).unwrap();
/// assert_eq!(result.direction.len(), 11);
/// ```
pub fn supertrend(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    atr_period: usize,
    multiplier: f64,
) -> Result<SuperTrendResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), atr_period)?;

    if multiplier <= 0.0 {
        return Err(TaError::InvalidParameter {
            name: "multiplier".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }

    let len = high.len();
    let atr_values = atr(high, low, close, atr_period)?;

    let mut direction = Array1::from_elem(len, 0);
    let mut trend_line = init_output(len);
    let mut upper_band = init_output(len);
    let mut lower_band = init_output(len);

    let mut final_upper = f64::NAN;
    let mut final_lower = f64::NAN;
    let mut current_trend = 1;

    for i in 0..len {
        if atr_values[i].is_nan() {
            continue;
        }

        let basic_upper = (high[i] + low[i]) / 2.0 + multiplier * atr_values[i];
        let basic_lower = (high[i] + low[i]) / 2.0 - multiplier * atr_values[i];

        let new_final_upper = if i == 0
            || final_upper.is_nan()
            || basic_upper < final_upper
            || close[i - 1] > final_upper
        {
            basic_upper
        } else {
            final_upper
        };

        let new_final_lower = if i == 0
            || final_lower.is_nan()
            || basic_lower > final_lower
            || close[i - 1] < final_lower
        {
            basic_lower
        } else {
            final_lower
        };

        final_upper = new_final_upper;
        final_lower = new_final_lower;

        upper_band[i] = final_upper;
        lower_band[i] = final_lower;

        if i > 0 && !trend_line[i - 1].is_nan() {
            if close[i] > final_upper {
                current_trend = 1;
            } else if close[i] < final_lower {
                current_trend = -1;
            }
        }

        direction[i] = current_trend;
        trend_line[i] = if current_trend == 1 {
            final_lower
        } else {
            final_upper
        };
    }

    Ok(SuperTrendResult {
        direction,
        trend_line,
        upper_band,
        lower_band,
    })
}

/// SuperTrend 趋势跟踪指标（使用默认参数）
///
/// 默认参数：ATR 周期 = 10，ATR 乘数 = 3.0
///
/// # 参数
/// * `high` - 最高价序列
/// * `low` - 最低价序列
/// * `close` - 收盘价序列
///
/// # 返回值
/// 返回 `SuperTrendResult` 结构体
///
/// # 示例
/// ```
/// use alpha_ta_core::indicators::supertrend_default;
///
/// let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 30.0];
/// let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0];
/// let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0, 25.0, 27.0, 29.0];
///
/// let result = supertrend_default(&high, &low, &close).unwrap();
/// assert_eq!(result.direction.len(), 11);
/// ```
pub fn supertrend_default(high: &[f64], low: &[f64], close: &[f64]) -> Result<SuperTrendResult> {
    supertrend(high, low, close, 10, 3.0)
}

// ========================================================================
// A3 — Multi-config SuperTrend & Wizard Wave direction
// ========================================================================

/// Result of [`supertrend_multi`]: one entry per `(period, multiplier)` config.
#[derive(Debug, Clone)]
pub struct SuperTrendMultiResult {
    /// The `(atr_period, multiplier)` configuration for each entry.
    pub configs: Vec<(usize, f64)>,
    /// Per-config trend direction arrays (`1` bullish, `-1` bearish).
    pub directions: Vec<Array1<i32>>,
    /// Per-config SuperTrend trend-line arrays.
    pub trend_lines: Vec<Array1<f64>>,
}

/// Batch-compute multiple SuperTrend configurations in one call.
///
/// Each entry in `configs` is an `(atr_period, multiplier)` tuple. The function
/// runs [`supertrend`] for every config and bundles the results. This is a
/// convenience wrapper — useful for building confluence dashboards that need
/// several SuperTrend lengths simultaneously.
///
/// # Example
///
/// ```
/// use alpha_ta_core::indicators::supertrend_multi;
///
/// let high  = vec![10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 30.0, 32.0, 34.0, 36.0, 38.0];
/// let low   = vec![ 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 30.0, 32.0, 34.0, 36.0];
/// let close = vec![ 9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0, 25.0, 27.0, 29.0, 31.0, 33.0, 35.0, 37.0];
/// let configs = vec![(7, 2.0), (10, 3.0), (14, 3.5)];
/// let r = supertrend_multi(&high, &low, &close, &configs).unwrap();
/// assert_eq!(r.directions.len(), 3);
/// ```
pub fn supertrend_multi(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    configs: &[(usize, f64)],
) -> Result<SuperTrendMultiResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if configs.is_empty() {
        return Err(TaError::InvalidParameter {
            name: "configs".to_string(),
            constraint: "must not be empty".to_string(),
        });
    }

    let mut directions = Vec::with_capacity(configs.len());
    let mut trend_lines = Vec::with_capacity(configs.len());

    for &(atr_period, multiplier) in configs {
        let r = supertrend(high, low, close, atr_period, multiplier)?;
        directions.push(r.direction);
        trend_lines.push(r.trend_line);
    }

    Ok(SuperTrendMultiResult {
        configs: configs.to_vec(),
        directions,
        trend_lines,
    })
}

/// Wizard Wave direction indicator.
///
/// A Donchian-style breakout system: price closing above the prior `period`-bar
/// highest-high signals bullish (`1`); closing below the prior `period`-bar
/// lowest-low signals bearish (`-1`). Between breakouts the previous direction
/// is carried forward. The "prior" window excludes the current bar to avoid
/// trivially triggering on the current high/low.
///
/// # Arguments
/// * `high`, `low`, `close` - OHLC slices of equal length.
/// * `period` - Lookback window for the breakout channel.
///
/// # Example
///
/// ```
/// use alpha_ta_core::indicators::wow_direction;
///
/// let high  = vec![10.0, 11.0, 12.0, 11.5, 13.0, 14.5, 14.0, 15.5, 17.0, 16.5];
/// let low   = vec![ 9.0, 10.0, 11.0, 10.5, 12.0, 13.5, 13.0, 14.5, 16.0, 15.5];
/// let close = vec![ 9.5, 10.5, 11.5, 11.0, 12.5, 14.0, 13.5, 15.0, 16.5, 16.0];
/// let dir = wow_direction(&high, &low, &close, 3).unwrap();
/// assert_eq!(dir.len(), 10);
/// ```
pub fn wow_direction(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<i32>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    validate_input(high.len(), period + 1)?;

    let len = high.len();
    let mut direction = Array1::zeros(len);

    let mut rmax = RollingMax::new();
    let mut rmin = RollingMin::new();
    let mut dir: i32 = 0;

    // We need the channel built from bars [i-period, i-1] (prior window),
    // then compare close[i] against it.
    // Seed the rolling structures with the first `period` bars (indices 0..period-1)
    // so that at i = period the channel represents bars [0, period-1].
    for i in 0..(len.min(period)) {
        rmax.push(i, high[i]);
        rmin.push(i, low[i]);
    }

    for i in period..len {
        // Channel currently covers [i-period, i-1].
        let hh = rmax.current().unwrap_or(f64::NEG_INFINITY);
        let ll = rmin.current().unwrap_or(f64::INFINITY);

        if close[i] > hh {
            dir = 1;
        } else if close[i] < ll {
            dir = -1;
        }
        direction[i] = dir;

        // Advance the channel: add current bar, evict bar that fell out.
        rmax.push(i, high[i]);
        rmin.push(i, low[i]);
        rmax.pop(i - period);
        rmin.pop(i - period);
    }

    Ok(direction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_supertrend_basic() {
        let high = vec![
            10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 30.0,
        ];
        let low = vec![
            8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0,
        ];
        let close = vec![
            9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0, 25.0, 27.0, 29.0,
        ];

        let result = supertrend(&high, &low, &close, 10, 3.0).unwrap();

        assert_eq!(result.direction.len(), 11);
        assert_eq!(result.trend_line.len(), 11);
        assert_eq!(result.upper_band.len(), 11);
        assert_eq!(result.lower_band.len(), 11);

        for i in 0..10 {
            assert!(
                result.direction[i] == 0 || result.direction[i] == 1 || result.direction[i] == -1
            );
        }
    }

    #[test]
    fn test_supertrend_uptrend() {
        let high = vec![
            100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 130.0, 135.0, 140.0, 145.0, 150.0,
        ];
        let low = vec![
            95.0, 100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 130.0, 135.0, 140.0, 145.0,
        ];
        let close = vec![
            98.0, 103.0, 108.0, 113.0, 118.0, 123.0, 128.0, 133.0, 138.0, 143.0, 148.0,
        ];

        let result = supertrend(&high, &low, &close, 5, 2.0).unwrap();

        assert!(result.direction[10] == 1, "上升趋势中方向应为 1");
    }

    #[test]
    fn test_supertrend_downtrend() {
        let high = vec![
            150.0, 145.0, 140.0, 135.0, 130.0, 125.0, 120.0, 115.0, 110.0, 105.0, 100.0,
        ];
        let low = vec![
            145.0, 140.0, 135.0, 130.0, 125.0, 120.0, 115.0, 110.0, 105.0, 100.0, 95.0,
        ];
        let close = vec![
            148.0, 143.0, 138.0, 133.0, 128.0, 123.0, 118.0, 113.0, 108.0, 103.0, 98.0,
        ];

        let result = supertrend(&high, &low, &close, 5, 2.0).unwrap();

        assert!(result.direction[10] == -1, "下降趋势中方向应为 -1");
    }

    #[test]
    fn test_supertrend_trend_reversal() {
        let high = vec![
            100.0, 102.0, 104.0, 106.0, 108.0, 110.0, 112.0, 114.0, 116.0, 118.0, 120.0, 118.0,
            116.0, 114.0, 112.0, 110.0, 108.0, 106.0, 104.0, 102.0, 100.0,
        ];
        let low = vec![
            98.0, 100.0, 102.0, 104.0, 106.0, 108.0, 110.0, 112.0, 114.0, 116.0, 118.0, 116.0,
            114.0, 112.0, 110.0, 108.0, 106.0, 104.0, 102.0, 100.0, 98.0,
        ];
        let close = vec![
            99.0, 101.0, 103.0, 105.0, 107.0, 109.0, 111.0, 113.0, 115.0, 117.0, 119.0, 117.0,
            115.0, 113.0, 111.0, 109.0, 107.0, 105.0, 103.0, 101.0, 99.0,
        ];

        let result = supertrend(&high, &low, &close, 5, 2.0).unwrap();

        let mut direction_changes = 0;
        let mut prev_direction = 0;

        for i in 0..result.direction.len() {
            if !result.trend_line[i].is_nan() && result.direction[i] != 0 {
                if prev_direction != 0 && result.direction[i] != prev_direction {
                    direction_changes += 1;
                }
                if prev_direction == 0 {
                    prev_direction = result.direction[i];
                }
            }
        }

        assert!(
            direction_changes >= 1,
            "在包含趋势反转的数据中应至少检测到一次趋势方向变化"
        );
    }

    #[test]
    fn test_supertrend_trend_line_follows_price() {
        let high = vec![
            10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 30.0,
        ];
        let low = vec![
            8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0,
        ];
        let close = vec![
            9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0, 25.0, 27.0, 29.0,
        ];

        let result = supertrend(&high, &low, &close, 5, 2.0).unwrap();

        let mut valid_count = 0;
        for i in 0..result.trend_line.len() {
            if !result.trend_line[i].is_nan() {
                valid_count += 1;
            }
        }

        assert!(valid_count > 0, "应至少有一个有效的趋势线值");
    }

    #[test]
    fn test_supertrend_bands_relationship() {
        let high = vec![
            100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 130.0, 135.0, 140.0, 145.0, 150.0,
        ];
        let low = vec![
            95.0, 100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 130.0, 135.0, 140.0, 145.0,
        ];
        let close = vec![
            98.0, 103.0, 108.0, 113.0, 118.0, 123.0, 128.0, 133.0, 138.0, 143.0, 148.0,
        ];

        let result = supertrend(&high, &low, &close, 5, 2.0).unwrap();

        for i in 0..result.upper_band.len() {
            if !result.upper_band[i].is_nan() && !result.lower_band[i].is_nan() {
                assert!(
                    result.upper_band[i] > result.lower_band[i],
                    "上轨带应始终大于下轨带"
                );
            }
        }
    }

    #[test]
    fn test_supertrend_default_params() {
        let high = vec![
            10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 30.0,
        ];
        let low = vec![
            8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0,
        ];
        let close = vec![
            9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0, 25.0, 27.0, 29.0,
        ];

        let result = supertrend_default(&high, &low, &close).unwrap();

        assert_eq!(result.direction.len(), 11);
        assert_eq!(result.trend_line.len(), 11);
    }

    #[test]
    fn test_supertrend_insufficient_data() {
        let high = vec![10.0, 12.0];
        let low = vec![8.0, 10.0];
        let close = vec![9.0, 11.0];

        let result = supertrend(&high, &low, &close, 5, 2.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_supertrend_mismatched_lengths() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0];
        let close = vec![9.0, 11.0, 13.0];

        let result = supertrend(&high, &low, &close, 2, 2.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_supertrend_empty_input() {
        let result = supertrend(&[], &[], &[], 10, 3.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_supertrend_invalid_multiplier() {
        let high = vec![
            10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 30.0,
        ];
        let low = vec![
            8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0,
        ];
        let close = vec![
            9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0, 25.0, 27.0, 29.0,
        ];

        let result = supertrend(&high, &low, &close, 5, 0.0);
        assert!(result.is_err());

        let result = supertrend(&high, &low, &close, 5, -1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_supertrend_different_multipliers() {
        let high = vec![
            100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 130.0, 135.0, 140.0, 145.0, 150.0,
        ];
        let low = vec![
            95.0, 100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 130.0, 135.0, 140.0, 145.0,
        ];
        let close = vec![
            98.0, 103.0, 108.0, 113.0, 118.0, 123.0, 128.0, 133.0, 138.0, 143.0, 148.0,
        ];

        let result1 = supertrend(&high, &low, &close, 5, 1.0).unwrap();
        let result2 = supertrend(&high, &low, &close, 5, 3.0).unwrap();

        for i in 0..result1.upper_band.len() {
            if !result1.upper_band[i].is_nan() && !result2.upper_band[i].is_nan() {
                let diff1 = result1.upper_band[i] - result1.lower_band[i];
                let diff2 = result2.upper_band[i] - result2.lower_band[i];
                assert!(diff2 > diff1, "较大乘数应产生较宽的轨道带");
            }
        }
    }

    #[test]
    fn test_supertrend_trend_line_at_band() {
        let high = vec![
            100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 130.0, 135.0, 140.0, 145.0, 150.0,
        ];
        let low = vec![
            95.0, 100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 130.0, 135.0, 140.0, 145.0,
        ];
        let close = vec![
            98.0, 103.0, 108.0, 113.0, 118.0, 123.0, 128.0, 133.0, 138.0, 143.0, 148.0,
        ];

        let result = supertrend(&high, &low, &close, 5, 2.0).unwrap();

        for i in 0..result.trend_line.len() {
            if !result.trend_line[i].is_nan() {
                if result.direction[i] == 1 {
                    assert_relative_eq!(
                        result.trend_line[i],
                        result.lower_band[i],
                        epsilon = 1e-10
                    );
                } else if result.direction[i] == -1 {
                    assert_relative_eq!(
                        result.trend_line[i],
                        result.upper_band[i],
                        epsilon = 1e-10
                    );
                }
            }
        }
    }

    // ---- A3: supertrend_multi & wow_direction ----

    #[test]
    fn test_supertrend_multi_basic() {
        let high: Vec<f64> = (0..20).map(|i| 10.0 + i as f64).collect();
        let low: Vec<f64> = (0..20).map(|i| 8.0 + i as f64).collect();
        let close: Vec<f64> = (0..20).map(|i| 9.0 + i as f64).collect();
        let configs = vec![(5, 2.0), (7, 3.0), (10, 3.0)];
        let r = supertrend_multi(&high, &low, &close, &configs).unwrap();
        assert_eq!(r.directions.len(), 3);
        assert_eq!(r.trend_lines.len(), 3);
        assert_eq!(r.configs, configs);
        for d in &r.directions {
            assert_eq!(d.len(), 20);
        }
    }

    #[test]
    fn test_supertrend_multi_matches_single() {
        let high: Vec<f64> = (0..15).map(|i| 10.0 + i as f64).collect();
        let low: Vec<f64> = (0..15).map(|i| 8.0 + i as f64).collect();
        let close: Vec<f64> = (0..15).map(|i| 9.0 + i as f64).collect();
        let single = supertrend(&high, &low, &close, 5, 2.0).unwrap();
        let multi = supertrend_multi(&high, &low, &close, &[(5, 2.0)]).unwrap();
        for i in 0..15 {
            assert_eq!(multi.directions[0][i], single.direction[i]);
            if single.trend_line[i].is_nan() {
                assert!(multi.trend_lines[0][i].is_nan(), "expected NaN at {i}");
            } else {
                assert!((multi.trend_lines[0][i] - single.trend_line[i]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_supertrend_multi_invalid_params() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0, 12.0];
        let close = vec![9.0, 11.0, 13.0];
        assert!(supertrend_multi(&high, &low, &close, &[]).is_err());
        let short_low = vec![8.0];
        assert!(supertrend_multi(&high, &short_low, &close, &[(5, 2.0)]).is_err());
    }

    #[test]
    fn test_wow_direction_uptrend_breakout() {
        // 3-bar channel. After the first 3 bars, each new close makes a higher high.
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5];
        let dir = wow_direction(&high, &low, &close, 3).unwrap();
        // Bar 3: close 12.5 vs prior channel [10,11,12] high=12 -> 12.5 > 12 -> bullish.
        assert_eq!(dir[3], 1);
        // Should stay bullish through the uptrend.
        for i in 3..8 {
            assert!(dir[i] == 1 || dir[i] == 0, "dir[{i}] = {}", dir[i]);
        }
        assert_eq!(dir[7], 1);
    }

    #[test]
    fn test_wow_direction_bearish_breakout() {
        let high = vec![17.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0, 10.0];
        let low = vec![16.0, 15.0, 14.0, 13.0, 12.0, 11.0, 10.0, 9.0];
        let close = vec![16.5, 15.5, 14.5, 13.5, 12.5, 11.5, 10.5, 9.5];
        let dir = wow_direction(&high, &low, &close, 3).unwrap();
        // Bar 3: close 13.5 vs prior channel [14,15,16] low=14 -> 13.5 < 14 -> bearish.
        assert_eq!(dir[3], -1);
        assert_eq!(dir[7], -1);
    }

    #[test]
    fn test_wow_direction_invalid_params() {
        let high = vec![10.0, 12.0];
        let low = vec![8.0, 10.0];
        let close = vec![9.0, 11.0];
        assert!(wow_direction(&high, &low, &close, 0).is_err());
        // Need at least period+1 bars.
        assert!(wow_direction(&high, &low, &close, 5).is_err());
        let short_low = vec![8.0];
        assert!(wow_direction(&high, &short_low, &close, 2).is_err());
    }
}
