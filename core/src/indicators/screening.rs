//! Cross-market screening primitives.
//!
//! These functions deliberately use only price, volume, and an optional
//! benchmark.  They are therefore suitable for A-shares, Hong Kong stocks,
//! US equities, ETFs, futures, and crypto without embedding exchange-specific
//! assumptions such as price limits or trading sessions.

use crate::error::{Result, TaError};
use crate::math::moving_avg::ema;
use crate::utils::{init_output, validate_input};
use ndarray::Array1;
use std::collections::VecDeque;

fn same_len(name: &str, series: &[(&str, &[f64])]) -> Result<usize> {
    let len = series.first().map(|(_, values)| values.len()).unwrap_or(0);
    for &(field, values) in series.iter().skip(1) {
        if values.len() != len {
            return Err(TaError::InvalidParameter {
                name: name.to_string(),
                constraint: format!("{field} must have the same length as the first series"),
            });
        }
    }
    Ok(len)
}

fn validate_period(name: &str, period: usize, len: usize) -> Result<()> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: name.to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    validate_input(len, period + 1)
}

fn cross(fast: &[f64], slow: &[f64], upward: bool) -> Result<Array1<f64>> {
    let len = same_len("fast, slow", &[("fast", fast), ("slow", slow)])?;
    validate_input(len, 1)?;
    let mut output = Array1::zeros(len);
    for i in 1..len {
        let (previous_fast, previous_slow) = (fast[i - 1], slow[i - 1]);
        let (current_fast, current_slow) = (fast[i], slow[i]);
        if !previous_fast.is_finite()
            || !previous_slow.is_finite()
            || !current_fast.is_finite()
            || !current_slow.is_finite()
        {
            continue;
        }
        output[i] = if upward {
            (current_fast > current_slow && previous_fast <= previous_slow) as u8 as f64
        } else {
            (current_fast < current_slow && previous_fast >= previous_slow) as u8 as f64
        };
    }
    Ok(output)
}

/// Upward crossover signal: `1` on the first bar where `fast` crosses above
/// `slow`, otherwise `0`.
pub fn golden_cross(fast: &[f64], slow: &[f64]) -> Result<Array1<f64>> {
    cross(fast, slow, true)
}

/// Downward crossover signal: `1` on the first bar where `fast` crosses below
/// `slow`, otherwise `0`.
pub fn dead_cross(fast: &[f64], slow: &[f64]) -> Result<Array1<f64>> {
    cross(fast, slow, false)
}

fn prior_extreme(values: &[f64], period: usize, maximum: bool) -> Result<Array1<f64>> {
    validate_period("period", period, values.len())?;
    let mut output = init_output(values.len());
    let mut deque = VecDeque::<usize>::with_capacity(period);

    for i in 0..values.len() {
        if i > 0 && values[i - 1].is_finite() {
            while let Some(&back) = deque.back() {
                let dominated = if maximum {
                    values[back] <= values[i - 1]
                } else {
                    values[back] >= values[i - 1]
                };
                if dominated {
                    deque.pop_back();
                } else {
                    break;
                }
            }
            deque.push_back(i - 1);
        }
        while let Some(&front) = deque.front() {
            if front + period < i {
                deque.pop_front();
            } else {
                break;
            }
        }
        if i >= period {
            if let Some(&front) = deque.front() {
                output[i] = values[front];
            }
        }
    }
    Ok(output)
}

/// Breakout above the previous `period` bars' high, excluding the current bar.
pub fn breakout_up(close: &[f64], high: &[f64], period: usize) -> Result<Array1<f64>> {
    let len = same_len("close, high", &[("close", close), ("high", high)])?;
    validate_period("period", period, len)?;
    let previous_high = prior_extreme(high, period, true)?;
    let mut output = Array1::zeros(len);
    for i in period..len {
        if close[i].is_finite() && previous_high[i].is_finite() && close[i] > previous_high[i] {
            output[i] = 1.0;
        }
    }
    Ok(output)
}

/// Breakdown below the previous `period` bars' low, excluding the current bar.
pub fn breakout_down(close: &[f64], low: &[f64], period: usize) -> Result<Array1<f64>> {
    let len = same_len("close, low", &[("close", close), ("low", low)])?;
    validate_period("period", period, len)?;
    let previous_low = prior_extreme(low, period, false)?;
    let mut output = Array1::zeros(len);
    for i in period..len {
        if close[i].is_finite() && previous_low[i].is_finite() && close[i] < previous_low[i] {
            output[i] = 1.0;
        }
    }
    Ok(output)
}

/// Volume expansion signal: current volume is at least `multiplier` times the
/// average of the preceding `period` bars.
pub fn volume_surge(volume: &[f64], period: usize, multiplier: f64) -> Result<Array1<f64>> {
    validate_period("period", period, volume.len())?;
    if !multiplier.is_finite() || multiplier < 0.0 {
        return Err(TaError::InvalidParameter {
            name: "multiplier".to_string(),
            constraint: "must be finite and non-negative".to_string(),
        });
    }
    let mut output = Array1::zeros(volume.len());
    let mut sum = 0.0;
    let mut finite_count = 0usize;
    for i in 0..volume.len() {
        if i > 0 {
            let previous = volume[i - 1];
            if previous.is_finite() {
                sum += previous;
                finite_count += 1;
            }
        }
        if i > period {
            let expired = volume[i - period - 1];
            if expired.is_finite() {
                sum -= expired;
                finite_count -= 1;
            }
        }
        if i >= period && finite_count == period && volume[i].is_finite() {
            let average = sum / period as f64;
            if volume[i] >= average * multiplier {
                output[i] = 1.0;
            }
        }
    }
    Ok(output)
}

/// Moving-average alignment: `1` for bullish `fast > mid > slow`, `-1` for
/// bearish `fast < mid < slow`, and `0` otherwise.
pub fn ma_alignment(
    close: &[f64],
    fast_period: usize,
    mid_period: usize,
    slow_period: usize,
) -> Result<Array1<f64>> {
    if fast_period == 0 || mid_period == 0 || slow_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "all periods must be greater than 0".to_string(),
        });
    }
    if !(fast_period < mid_period && mid_period < slow_period) {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "expected fast < mid < slow".to_string(),
        });
    }
    validate_input(close.len(), slow_period)?;
    let fast = ema(close, fast_period)?;
    let mid = ema(close, mid_period)?;
    let slow = ema(close, slow_period)?;
    let mut output = Array1::zeros(close.len());
    for i in 0..close.len() {
        if fast[i].is_finite() && mid[i].is_finite() && slow[i].is_finite() {
            output[i] = if fast[i] > mid[i] && mid[i] > slow[i] {
                1.0
            } else if fast[i] < mid[i] && mid[i] < slow[i] {
                -1.0
            } else {
                0.0
            };
        }
    }
    Ok(output)
}

/// Excess return versus a benchmark over `period`, in percentage points.
pub fn relative_strength(close: &[f64], benchmark: &[f64], period: usize) -> Result<Array1<f64>> {
    let len = same_len(
        "close, benchmark",
        &[("close", close), ("benchmark", benchmark)],
    )?;
    validate_period("period", period, len)?;
    let mut output = init_output(len);
    for i in period..len {
        let (stock_base, benchmark_base) = (close[i - period], benchmark[i - period]);
        if close[i].is_finite()
            && benchmark[i].is_finite()
            && stock_base.is_finite()
            && benchmark_base.is_finite()
            && stock_base.abs() > f64::EPSILON
            && benchmark_base.abs() > f64::EPSILON
        {
            output[i] = ((close[i] / stock_base) - (benchmark[i] / benchmark_base)) * 100.0;
        }
    }
    Ok(output)
}

/// Overnight/session gap signal: `1` for an upward gap, `-1` for a downward
/// gap, and `0` when the absolute gap is below `threshold`.
pub fn gap_signal(open: &[f64], close: &[f64], threshold: f64) -> Result<Array1<f64>> {
    let len = same_len("open, close", &[("open", open), ("close", close)])?;
    validate_input(len, 1)?;
    if !threshold.is_finite() || threshold < 0.0 {
        return Err(TaError::InvalidParameter {
            name: "threshold".to_string(),
            constraint: "must be finite and non-negative".to_string(),
        });
    }
    let mut output = Array1::zeros(len);
    for i in 1..len {
        if open[i].is_finite() && close[i - 1].is_finite() && close[i - 1].abs() > f64::EPSILON {
            let gap = open[i] / close[i - 1] - 1.0;
            if gap >= threshold {
                output[i] = 1.0;
            } else if gap <= -threshold {
                output[i] = -1.0;
            }
        }
    }
    Ok(output)
}

/// Trend-following breakout screen. Returns `1` for a bullish signal, `-1`
/// for a bearish signal, and `0` otherwise. A signal requires aligned EMAs,
/// a breakout/breakdown, and a volume surge, which keeps it useful across
/// equities, ETFs, and 24/7 crypto bars.
pub fn trend_breakout_signal(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    fast_period: usize,
    slow_period: usize,
    breakout_period: usize,
    volume_period: usize,
    volume_multiplier: f64,
) -> Result<Array1<f64>> {
    let len = same_len(
        "high, low, close, volume",
        &[
            ("high", high),
            ("low", low),
            ("close", close),
            ("volume", volume),
        ],
    )?;
    if fast_period == 0 || slow_period == 0 || fast_period >= slow_period {
        return Err(TaError::InvalidParameter {
            name: "fast_period, slow_period".to_string(),
            constraint: "must satisfy 0 < fast_period < slow_period".to_string(),
        });
    }
    validate_period("breakout_period", breakout_period, len)?;
    validate_period("volume_period", volume_period, len)?;
    let fast = ema(close, fast_period)?;
    let slow = ema(close, slow_period)?;
    let up = breakout_up(close, high, breakout_period)?;
    let down = breakout_down(close, low, breakout_period)?;
    let surge = volume_surge(volume, volume_period, volume_multiplier)?;
    let mut output = Array1::zeros(len);
    for i in 0..len {
        if !fast[i].is_finite() || !slow[i].is_finite() || surge[i] == 0.0 {
            continue;
        }
        if up[i] > 0.0 && close[i] > fast[i] && fast[i] > slow[i] {
            output[i] = 1.0;
        } else if down[i] > 0.0 && close[i] < fast[i] && fast[i] < slow[i] {
            output[i] = -1.0;
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_signals_fire_once_at_transition() {
        let fast = [1.0, 1.0, 2.0, 3.0, 1.0];
        let slow = [2.0, 1.0, 1.0, 2.0, 2.0];
        let up = golden_cross(&fast, &slow).unwrap();
        let down = dead_cross(&fast, &slow).unwrap();
        assert_eq!(up.to_vec(), vec![0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(down.to_vec(), vec![0.0, 0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn breakout_excludes_current_bar() {
        let high = [10.0, 11.0, 12.0, 13.0, 14.0, 20.0];
        let low = [8.0, 9.0, 10.0, 11.0, 12.0, 13.0];
        let close = [9.0, 10.0, 11.0, 13.0, 14.0, 19.0];
        let up = breakout_up(&close, &high, 3).unwrap();
        assert_eq!(up[3], 1.0);
        assert_eq!(up[4], 1.0);
        assert_eq!(up[5], 1.0);
        let down_close = [9.0, 8.0, 7.0, 6.0, 5.0, 1.0];
        let down = breakout_down(&down_close, &low, 3).unwrap();
        assert_eq!(down[3], 1.0);
        assert_eq!(down[5], 1.0);
    }

    #[test]
    fn volume_surge_uses_previous_window() {
        let volume = [10.0, 10.0, 10.0, 30.0, 10.0];
        let signal = volume_surge(&volume, 3, 2.0).unwrap();
        assert_eq!(signal[2], 0.0);
        assert_eq!(signal[3], 1.0);
        assert_eq!(signal[4], 0.0);
    }

    #[test]
    fn relative_strength_and_gap_are_market_neutral() {
        let rs = relative_strength(&[100.0, 110.0, 121.0], &[100.0, 108.0, 116.0], 2).unwrap();
        assert!((rs[2] - 5.0).abs() < 1e-12);
        let gap = gap_signal(&[100.0, 105.0, 94.0], &[100.0, 100.0, 100.0], 0.04).unwrap();
        assert_eq!(gap.to_vec(), vec![0.0, 1.0, -1.0]);
    }
}
