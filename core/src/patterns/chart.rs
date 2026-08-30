use crate::error::{Result, TaError};
use ndarray::Array1;

/// Chart pattern recognition result
/// Values: 1 for pattern detected, 0 for no pattern
pub type ChartPatternResult = Array1<i32>;

/// Head and Shoulders Top Pattern
///
/// A bearish reversal pattern with three peaks: left shoulder, head (highest), right shoulder.
///
/// # Arguments
/// * `high` - High prices
/// * `min_bars_between_peaks` - Minimum bars between peaks (default: 5)
/// * `head_height_ratio` - How much higher the head must be vs shoulders (default: 1.1)
///
/// # Returns
/// Array with 1 where the right shoulder completes
pub fn head_and_shoulders_top(
    high: &[f64],
    min_bars_between_peaks: usize,
    head_height_ratio: f64,
) -> Result<ChartPatternResult> {
    if high.len() < min_bars_between_peaks * 2 + 3 {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: min_bars_between_peaks * 2 + 3,
        });
    }

    let len = high.len();
    let mut output = Array1::zeros(len);
    let lookback = min_bars_between_peaks * 3;

    for i in lookback..len {
        // Find peaks in the lookback window
        let mut peaks: Vec<(usize, f64)> = Vec::new();

        for j in (i - lookback + 2)..(i - 1) {
            let is_peak = high[j] > high[j - 1] && high[j] > high[j + 1];
            let is_local_high = (j.saturating_sub(min_bars_between_peaks)..j)
                .all(|k| high[j] >= high[k])
                && (j + 1..=(j + min_bars_between_peaks).min(i)).all(|k| high[j] >= high[k]);

            if is_peak && is_local_high {
                peaks.push((j, high[j]));
            }
        }

        if peaks.len() >= 3 {
            let recent: Vec<(usize, f64)> = peaks.iter().rev().take(3).cloned().collect();
            let left = recent[2];
            let head = recent[1];
            let right = recent[0];

            if head.1 > left.1 * head_height_ratio
                && head.1 > right.1 * head_height_ratio
                && (left.1 - right.1).abs() / left.1 < 0.05
                && head.0 - left.0 >= min_bars_between_peaks
                && right.0 - head.0 >= min_bars_between_peaks
            {
                output[i] = 1;
            }
        }
    }

    Ok(output)
}

/// Head and Shoulders Bottom (Inverse) Pattern
///
/// A bullish reversal pattern with three troughs.
pub fn head_and_shoulders_bottom(
    low: &[f64],
    min_bars_between_peaks: usize,
    head_depth_ratio: f64,
) -> Result<ChartPatternResult> {
    if low.len() < min_bars_between_peaks * 2 + 3 {
        return Err(TaError::InsufficientData {
            length: low.len(),
            required: min_bars_between_peaks * 2 + 3,
        });
    }

    let len = low.len();
    let mut output = Array1::zeros(len);
    let lookback = min_bars_between_peaks * 3;

    for i in lookback..len {
        let mut troughs: Vec<(usize, f64)> = Vec::new();

        for j in (i - lookback + 2)..(i - 1) {
            let is_trough = low[j] < low[j - 1] && low[j] < low[j + 1];
            let is_local_low = (j.saturating_sub(min_bars_between_peaks)..j)
                .all(|k| low[j] <= low[k])
                && (j + 1..=(j + min_bars_between_peaks).min(i)).all(|k| low[j] <= low[k]);

            if is_trough && is_local_low {
                troughs.push((j, low[j]));
            }
        }

        if troughs.len() >= 3 {
            let recent: Vec<(usize, f64)> = troughs.iter().rev().take(3).cloned().collect();
            let left = recent[2];
            let head = recent[1];
            let right = recent[0];

            if head.1 < left.1 * head_depth_ratio
                && head.1 < right.1 * head_depth_ratio
                && (left.1 - right.1).abs() / left.1 < 0.05
                && head.0 - left.0 >= min_bars_between_peaks
                && right.0 - head.0 >= min_bars_between_peaks
            {
                output[i] = 1;
            }
        }
    }

    Ok(output)
}

/// Double Top Pattern
///
/// A bearish reversal pattern with two peaks at similar levels.
///
/// # Arguments
/// * `high` - High prices
/// * `lookback` - Lookback period to find the pattern
/// * `tolerance_pct` - Percentage tolerance for peak matching
pub fn double_top(high: &[f64], lookback: usize, tolerance_pct: f64) -> Result<ChartPatternResult> {
    if high.len() < lookback {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: lookback,
        });
    }

    let len = high.len();
    let mut output = Array1::zeros(len);

    for i in lookback..len {
        let window = &high[i - lookback..i];
        let max_idx = window
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        if max_idx == 0 || max_idx == lookback - 1 {
            continue;
        }

        let first_peak = window[max_idx];
        let left_max = window[..max_idx]
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);
        let right_max = window[max_idx + 1..]
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);

        let tolerance = first_peak * tolerance_pct;
        if (left_max - first_peak).abs() < tolerance || (right_max - first_peak).abs() < tolerance {
            let valley = window[max_idx..]
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .copied()
                .unwrap_or(first_peak);

            if valley < first_peak * 0.95 {
                output[i] = 1;
            }
        }
    }

    Ok(output)
}

/// Double Bottom Pattern
///
/// A bullish reversal pattern with two troughs at similar levels.
pub fn double_bottom(
    low: &[f64],
    lookback: usize,
    tolerance_pct: f64,
) -> Result<ChartPatternResult> {
    if low.len() < lookback {
        return Err(TaError::InsufficientData {
            length: low.len(),
            required: lookback,
        });
    }

    let len = low.len();
    let mut output = Array1::zeros(len);

    for i in lookback..len {
        let window = &low[i - lookback..i];
        let min_idx = window
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        if min_idx == 0 || min_idx == lookback - 1 {
            continue;
        }

        let first_trough = window[min_idx];
        let left_min = window[..min_idx]
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f64::MAX);
        let right_min = window[min_idx + 1..]
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f64::MAX);

        let tolerance = first_trough * tolerance_pct;
        if (left_min - first_trough).abs() < tolerance
            || (right_min - first_trough).abs() < tolerance
        {
            let peak = window[min_idx..]
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .copied()
                .unwrap_or(first_trough);

            if peak > first_trough * 1.05 {
                output[i] = 1;
            }
        }
    }

    Ok(output)
}

/// Triple Top Pattern
///
/// A bearish reversal pattern with three peaks at similar levels.
pub fn triple_top(high: &[f64], lookback: usize, tolerance_pct: f64) -> Result<ChartPatternResult> {
    if high.len() < lookback {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: lookback,
        });
    }

    let len = high.len();
    let mut output = Array1::zeros(len);

    for i in lookback..len {
        let window = &high[i - lookback..i];
        let avg = window.iter().sum::<f64>() / window.len() as f64;
        let tolerance = avg * tolerance_pct;

        let mut peak_count = 0;
        for j in 1..window.len() - 1 {
            if window[j] > window[j - 1] && window[j] > window[j + 1] && window[j] > avg - tolerance
            {
                peak_count += 1;
            }
        }

        if peak_count >= 3 {
            output[i] = 1;
        }
    }

    Ok(output)
}

/// Triple Bottom Pattern
///
/// A bullish reversal pattern with three troughs at similar levels.
pub fn triple_bottom(
    low: &[f64],
    lookback: usize,
    tolerance_pct: f64,
) -> Result<ChartPatternResult> {
    if low.len() < lookback {
        return Err(TaError::InsufficientData {
            length: low.len(),
            required: lookback,
        });
    }

    let len = low.len();
    let mut output = Array1::zeros(len);

    for i in lookback..len {
        let window = &low[i - lookback..i];
        let avg = window.iter().sum::<f64>() / window.len() as f64;
        let tolerance = avg * tolerance_pct;

        let mut trough_count = 0;
        for j in 1..window.len() - 1 {
            if window[j] < window[j - 1] && window[j] < window[j + 1] && window[j] < avg + tolerance
            {
                trough_count += 1;
            }
        }

        if trough_count >= 3 {
            output[i] = 1;
        }
    }

    Ok(output)
}

/// Ascending Triangle Pattern
///
/// A bullish continuation pattern with flat resistance and rising support.
pub fn ascending_triangle(
    high: &[f64],
    low: &[f64],
    lookback: usize,
    tolerance_pct: f64,
) -> Result<ChartPatternResult> {
    if high.len() != low.len() || high.len() < lookback {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have same length and sufficient data".to_string(),
        });
    }

    let len = high.len();
    let mut output = Array1::zeros(len);

    for i in lookback..len {
        let high_window = &high[i - lookback..i];
        let low_window = &low[i - lookback..i];

        let resistance = high_window
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);

        let highs_near_resistance = high_window
            .iter()
            .filter(|&&h| (h - resistance).abs() < resistance * tolerance_pct)
            .count();

        let first_half_low = low_window[..lookback / 2]
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f64::MAX);
        let second_half_low = low_window[lookback / 2..]
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f64::MAX);

        if highs_near_resistance >= 2 && second_half_low > first_half_low {
            output[i] = 1;
        }
    }

    Ok(output)
}

/// Descending Triangle Pattern
///
/// A bearish continuation pattern with flat support and declining resistance.
pub fn descending_triangle(
    high: &[f64],
    low: &[f64],
    lookback: usize,
    tolerance_pct: f64,
) -> Result<ChartPatternResult> {
    if high.len() != low.len() || high.len() < lookback {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have same length and sufficient data".to_string(),
        });
    }

    let len = high.len();
    let mut output = Array1::zeros(len);

    for i in lookback..len {
        let high_window = &high[i - lookback..i];
        let low_window = &low[i - lookback..i];

        let support = low_window
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);

        let lows_near_support = low_window
            .iter()
            .filter(|&&l| (l - support).abs() < support * tolerance_pct)
            .count();

        let first_half_high = high_window[..lookback / 2]
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);
        let second_half_high = high_window[lookback / 2..]
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);

        if lows_near_support >= 2 && second_half_high < first_half_high {
            output[i] = 1;
        }
    }

    Ok(output)
}

/// Symmetrical Triangle Pattern
///
/// A continuation pattern with converging trendlines.
pub fn symmetrical_triangle(
    high: &[f64],
    low: &[f64],
    lookback: usize,
) -> Result<ChartPatternResult> {
    if high.len() != low.len() || high.len() < lookback {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have same length and sufficient data".to_string(),
        });
    }

    let len = high.len();
    let mut output = Array1::zeros(len);

    for i in lookback..len {
        let high_window = &high[i - lookback..i];
        let low_window = &low[i - lookback..i];

        let first_high = high_window[..lookback / 3]
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);
        let second_high = high_window[lookback / 3..2 * lookback / 3]
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);
        let third_high = high_window[2 * lookback / 3..]
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);

        let first_low = low_window[..lookback / 3]
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f64::MAX);
        let second_low = low_window[lookback / 3..2 * lookback / 3]
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f64::MAX);
        let third_low = low_window[2 * lookback / 3..]
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f64::MAX);

        if first_high > second_high
            && second_high > third_high
            && first_low < second_low
            && second_low < third_low
        {
            let range = first_high - first_low;
            if range > 0.0 && (third_high - third_low) < range * 0.5 {
                output[i] = 1;
            }
        }
    }

    Ok(output)
}

/// Wedge (Rising/Falling) Pattern
///
/// A reversal pattern with converging trendlines in the same direction.
pub fn rising_wedge(high: &[f64], low: &[f64], lookback: usize) -> Result<ChartPatternResult> {
    if high.len() != low.len() || high.len() < lookback {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have same length and sufficient data".to_string(),
        });
    }

    let len = high.len();
    let mut output = Array1::zeros(len);

    for i in lookback..len {
        let high_window = &high[i - lookback..i];
        let low_window = &low[i - lookback..i];

        let first_high = high_window[..lookback / 2]
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);
        let second_high = high_window[lookback / 2..]
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);

        let first_low = low_window[..lookback / 2]
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f64::MAX);
        let second_low = low_window[lookback / 2..]
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f64::MAX);

        if second_high > first_high && second_low > first_low {
            let first_range = first_high - first_low;
            let second_range = second_high - second_low;
            if second_range < first_range * 0.7 && first_range > 0.0 {
                output[i] = 1;
            }
        }
    }

    Ok(output)
}

/// Falling Wedge Pattern
///
/// A bullish reversal pattern with declining converging trendlines.
pub fn falling_wedge(high: &[f64], low: &[f64], lookback: usize) -> Result<ChartPatternResult> {
    if high.len() != low.len() || high.len() < lookback {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have same length and sufficient data".to_string(),
        });
    }

    let len = high.len();
    let mut output = Array1::zeros(len);

    for i in lookback..len {
        let high_window = &high[i - lookback..i];
        let low_window = &low[i - lookback..i];

        let first_high = high_window[..lookback / 2]
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);
        let second_high = high_window[lookback / 2..]
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);

        let first_low = low_window[..lookback / 2]
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f64::MAX);
        let second_low = low_window[lookback / 2..]
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(f64::MAX);

        if second_high < first_high && second_low < first_low {
            let first_range = first_high - first_low;
            let second_range = second_high - second_low;
            if second_range < first_range * 0.7 && first_range > 0.0 {
                output[i] = 1;
            }
        }
    }

    Ok(output)
}

/// Pennant Pattern
///
/// A continuation pattern with a small symmetrical triangle after a sharp move.
pub fn pennant(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    flagpole_period: usize,
    pennant_period: usize,
) -> Result<ChartPatternResult> {
    if high.len() != low.len()
        || high.len() != close.len()
        || high.len() < flagpole_period + pennant_period
    {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: flagpole_period + pennant_period,
        });
    }

    let len = high.len();
    let mut output = Array1::zeros(len);

    for i in flagpole_period + pennant_period..len {
        let flagpole_start = i - pennant_period - flagpole_period;
        let flagpole_end = i - pennant_period;

        let flagpole_move = (close[flagpole_end] - close[flagpole_start]).abs();
        let flagpole_avg =
            close[flagpole_start..flagpole_end].iter().sum::<f64>() / flagpole_period as f64;

        if flagpole_move < flagpole_avg * 0.05 {
            continue;
        }

        let pennant_highs = &high[i - pennant_period..i];
        let pennant_lows = &low[i - pennant_period..i];

        let pennant_range = pennant_highs
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0)
            - pennant_lows
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .copied()
                .unwrap_or(0.0);

        if pennant_range < flagpole_move * 0.3 {
            output[i] = 1;
        }
    }

    Ok(output)
}

/// Flag Pattern
///
/// A continuation pattern with a small rectangle/slope against the trend.
pub fn flag(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    flagpole_period: usize,
    flag_period: usize,
) -> Result<ChartPatternResult> {
    if high.len() != low.len()
        || high.len() != close.len()
        || high.len() < flagpole_period + flag_period
    {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: flagpole_period + flag_period,
        });
    }

    let len = high.len();
    let mut output = Array1::zeros(len);

    for i in flagpole_period + flag_period..len {
        let flagpole_start = i - flag_period - flagpole_period;
        let flagpole_end = i - flag_period;

        let flagpole_move = close[flagpole_end] - close[flagpole_start];
        let flagpole_avg =
            close[flagpole_start..flagpole_end].iter().sum::<f64>() / flagpole_period as f64;

        if flagpole_move.abs() < flagpole_avg * 0.05 {
            continue;
        }

        let flag_highs = &high[i - flag_period..i];
        let flag_lows = &low[i - flag_period..i];

        let flag_range = flag_highs
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0)
            - flag_lows
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .copied()
                .unwrap_or(0.0);

        if flag_range < flagpole_move.abs() * 0.382 {
            let flag_direction = if flagpole_move > 0.0 {
                close[i - 1] < close[i - flag_period]
            } else {
                close[i - 1] > close[i - flag_period]
            };

            if flag_direction {
                output[i] = 1;
            }
        }
    }

    Ok(output)
}

/// Rectangle Pattern
///
/// A continuation pattern with parallel support and resistance.
pub fn rectangle(
    high: &[f64],
    low: &[f64],
    lookback: usize,
    tolerance_pct: f64,
) -> Result<ChartPatternResult> {
    if high.len() != low.len() || high.len() < lookback {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have same length and sufficient data".to_string(),
        });
    }

    let len = high.len();
    let mut output = Array1::zeros(len);

    for i in lookback..len {
        let high_window = &high[i - lookback..i];
        let low_window = &low[i - lookback..i];

        let resistance = high_window
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);
        let support = low_window
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.0);

        let range = resistance - support;
        if range <= 0.0 {
            continue;
        }

        let touches_resistance = high_window
            .iter()
            .filter(|&&h| (h - resistance).abs() < range * tolerance_pct)
            .count();

        let touches_support = low_window
            .iter()
            .filter(|&&l| (l - support).abs() < range * tolerance_pct)
            .count();

        if touches_resistance >= 2 && touches_support >= 2 {
            output[i] = 1;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_head_and_shoulders_top() {
        let mut high = vec![10.0; 20];
        high[5] = 12.0;
        high[10] = 15.0;
        high[15] = 12.0;
        let result = head_and_shoulders_top(&high, 3, 1.1).unwrap();
        assert_eq!(result.len(), 20);
    }

    #[test]
    fn test_double_top() {
        let mut high = vec![10.0; 20];
        high[5] = 12.0;
        high[15] = 11.9;
        let result = double_top(&high, 18, 0.05).unwrap();
        assert_eq!(result.len(), 20);
    }

    #[test]
    fn test_double_bottom() {
        let mut low = vec![10.0; 20];
        low[5] = 8.0;
        low[15] = 8.1;
        let result = double_bottom(&low, 18, 0.05).unwrap();
        assert_eq!(result.len(), 20);
    }

    #[test]
    fn test_triple_top() {
        let high = vec![10.0; 30];
        let result = triple_top(&high, 25, 0.05).unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_ascending_triangle() {
        let high = vec![12.0; 30];
        let low: Vec<f64> = (0..30).map(|i| 8.0 + i as f64 * 0.05).collect();
        let result = ascending_triangle(&high, &low, 20, 0.05).unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_symmetrical_triangle() {
        let high: Vec<f64> = (0..30).map(|i| 15.0 - i as f64 * 0.1).collect();
        let low: Vec<f64> = (0..30).map(|i| 5.0 + i as f64 * 0.1).collect();
        let result = symmetrical_triangle(&high, &low, 25).unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_rising_wedge() {
        let high: Vec<f64> = (0..30).map(|i| 10.0 + i as f64 * 0.1).collect();
        let low: Vec<f64> = (0..30).map(|i| 8.0 + i as f64 * 0.08).collect();
        let result = rising_wedge(&high, &low, 25).unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_rectangle() {
        let high = vec![12.0; 30];
        let low = vec![8.0; 30];
        let result = rectangle(&high, &low, 20, 0.05).unwrap();
        assert_eq!(result.len(), 30);
    }
}
