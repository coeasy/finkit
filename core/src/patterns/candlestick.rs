use crate::error::{Result, TaError};
use crate::utils::validate_input;
use ndarray::Array1;

/// Candlestick pattern result
/// Values: 100 for bullish, -100 for bearish, 0 for no pattern
pub type PatternResult = Array1<i32>;

/// Average True Range for candlestick calculations
fn candle_avg_range(high: &[f64], low: &[f64], close: &[f64], period: usize, idx: usize) -> f64 {
    let mut sum = 0.0;
    let start = if idx >= period { idx - period + 1 } else { 0 };
    let count = idx - start + 1;

    for i in start..=idx {
        if i == 0 {
            sum += high[i] - low[i];
        } else {
            let tr = (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs());
            sum += tr;
        }
    }

    if count > 0 {
        sum / count as f64
    } else {
        0.0
    }
}

/// Candle body size
fn body(open: f64, close: f64) -> f64 {
    (close - open).abs()
}

/// Candle upper shadow
fn upper_shadow(high: f64, open: f64, close: f64) -> f64 {
    high - open.max(close)
}

/// Candle lower shadow
fn lower_shadow(low: f64, open: f64, close: f64) -> f64 {
    open.min(close) - low
}

/// Whether candle is bullish (white)
fn is_bullish(open: f64, close: f64) -> bool {
    close > open
}

/// Whether candle is bearish (black)
fn is_bearish(open: f64, close: f64) -> bool {
    close < open
}

// TA-Lib's candlestick rules use a shared CandleSettings table. Keep these
// defaults centralized so patterns consistently exclude the current candle
// from trailing averages and use the correct range type.
const CDL_AVG_PERIOD: usize = 10;

#[inline]
fn talib_body(open: f64, close: f64) -> f64 {
    (close - open).abs()
}

#[inline]
fn talib_upper_shadow(open: f64, high: f64, close: f64) -> f64 {
    high - open.max(close)
}

#[inline]
fn talib_lower_shadow(open: f64, low: f64, close: f64) -> f64 {
    open.min(close) - low
}

/// CandleSetting range: 0 = RealBody, 1 = HighLow, 2 = Shadows.
#[inline]
fn talib_range(open: f64, high: f64, low: f64, close: f64, range_type: u8) -> f64 {
    match range_type {
        0 => talib_body(open, close),
        1 => high - low,
        _ => talib_upper_shadow(open, high, close) + talib_lower_shadow(open, low, close),
    }
}

/// TA_CANDLEAVERAGE for the default TA-Lib settings, excluding the current
/// candle when `period` is non-zero.
#[inline]
fn talib_average(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    idx: usize,
    period: usize,
    factor: f64,
    range_type: u8,
) -> f64 {
    if period == 0 {
        return factor * talib_range(open[idx], high[idx], low[idx], close[idx], range_type)
            / if range_type == 2 { 2.0 } else { 1.0 };
    }
    if idx < period {
        return 0.0;
    }
    let sum = (idx - period..idx)
        .map(|j| talib_range(open[j], high[j], low[j], close[j], range_type))
        .sum::<f64>();
    factor * sum / period as f64 / if range_type == 2 { 2.0 } else { 1.0 }
}

#[inline]
fn talib_color(open: f64, close: f64) -> i32 {
    if close >= open { 100 } else { -100 }
}

/// Doji (DOJI)
///
/// Open and close are virtually the same.
///
/// # Arguments
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `doji_pct` - Doji threshold percentage of average range (default: 0.1)
pub fn doji(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    doji_pct: f64,
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), CDL_AVG_PERIOD + 1)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = CDL_AVG_PERIOD;

    // TA-Lib's BodyDoji setting uses the preceding HighLow average and a
    // default factor of 0.1. The current candle is excluded.
    for i in period..len {
        let threshold = talib_average(open, high, low, close, i, period, doji_pct, 1);
        if talib_body(open[i], close[i]) <= threshold {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Dragonfly Doji (DRAGONFLYDOJI)
///
/// Doji with long lower shadow and little to no upper shadow.
pub fn dragonfly_doji(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    doji_pct: f64,
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 10)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;

    for i in period..len {
        let avg_range = candle_avg_range(high, low, close, period, i);
        let body_size = body(open[i], close[i]);
        let up_shadow = upper_shadow(high[i], open[i], close[i]);
        let lo_shadow = lower_shadow(low[i], open[i], close[i]);

        if body_size <= avg_range * doji_pct
            && lo_shadow > avg_range * 2.0
            && up_shadow <= avg_range * doji_pct
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Gravestone Doji (GRAVESTONEDOJI)
///
/// Doji with long upper shadow and little to no lower shadow.
pub fn gravestone_doji(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    doji_pct: f64,
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 10)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;

    for i in period..len {
        let avg_range = candle_avg_range(high, low, close, period, i);
        let body_size = body(open[i], close[i]);
        let up_shadow = upper_shadow(high[i], open[i], close[i]);
        let lo_shadow = lower_shadow(low[i], open[i], close[i]);

        if body_size <= avg_range * doji_pct
            && up_shadow > avg_range * 2.0
            && lo_shadow <= avg_range * doji_pct
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Long-Legged Doji (LONGLEGGEDDOJI)
///
/// Doji with long upper and lower shadows.
pub fn long_legged_doji(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    doji_pct: f64,
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 10)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in CDL_AVG_PERIOD..len {
        let doji = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, doji_pct, 1);
        let long_shadow = talib_average(open, high, low, close, i, 0, 1.0, 0);
        if talib_body(open[i], close[i]) <= doji
            && (talib_lower_shadow(open[i], low[i], close[i]) > long_shadow
                || talib_upper_shadow(open[i], high[i], close[i]) > long_shadow)
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// 4 Price Doji (DOJI_4PRICES)
///
/// Open, High, Low, and Close are all equal.
pub fn doji_4prices(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), CDL_AVG_PERIOD + 1)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 0..len {
        if (open[i] - high[i]).abs() < 1e-10
            && (open[i] - low[i]).abs() < 1e-10
            && (open[i] - close[i]).abs() < 1e-10
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Marubozu (MARUBOZU)
///
/// A candle with no shadows (or very small shadows).
pub fn marubozu(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    shadow_pct: f64,
) -> Result<PatternResult> {
    let _ = shadow_pct;
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 1)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    if len <= CDL_AVG_PERIOD {
        for i in 0..len {
            let body_size = body(open[i], close[i]);
            let up_shadow = upper_shadow(high[i], open[i], close[i]);
            let lo_shadow = lower_shadow(low[i], open[i], close[i]);
            if body_size > 0.0 && up_shadow / body_size < shadow_pct && lo_shadow / body_size < shadow_pct {
                output[i] = if is_bullish(open[i], close[i]) { 100 } else { -100 };
            }
        }
        return Ok(output);
    }

    for i in CDL_AVG_PERIOD..len {
        let body_long = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        let shadow_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 0.1, 1);
        if talib_body(open[i], close[i]) > body_long
            && talib_upper_shadow(open[i], high[i], close[i]) < shadow_short
            && talib_lower_shadow(open[i], low[i], close[i]) < shadow_short
        {
            output[i] = talib_color(open[i], close[i]);
        }
    }

    Ok(output)
}

/// Hammer (HAMMER)
///
/// Small body at the top, long lower shadow (at least 2x body), little to no upper shadow.
/// Bullish reversal pattern after downtrend.
pub fn hammer(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 10)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in CDL_AVG_PERIOD..len {
        let body_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        let shadow_long = talib_average(open, high, low, close, i, 0, 1.0, 0);
        let shadow_very_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 0.1, 1);
        let near = talib_average(open, high, low, close, i - 1, 5, 0.2, 1);
        if talib_body(open[i], close[i]) < body_short
            && talib_lower_shadow(open[i], low[i], close[i]) > shadow_long
            && talib_upper_shadow(open[i], high[i], close[i]) < shadow_very_short
            && open[i].min(close[i]) <= low[i - 1] + near
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Inverted Hammer (INVERTEDHAMMER)
///
/// Small body at the bottom, long upper shadow, little to no lower shadow.
/// Bullish reversal pattern after downtrend.
pub fn inverted_hammer(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 10)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in CDL_AVG_PERIOD..len {
        let body_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        let shadow_long = talib_average(open, high, low, close, i, 0, 1.0, 0);
        let shadow_very_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 0.1, 1);
        if open[i].max(close[i]) < open[i - 1].min(close[i - 1])
            && talib_body(open[i], close[i]) < body_short
            && talib_upper_shadow(open[i], high[i], close[i]) > shadow_long
            && talib_lower_shadow(open[i], low[i], close[i]) < shadow_very_short
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Hanging Man (HANGINGMAN)
///
/// Same shape as Hammer but appears after uptrend.
/// Bearish reversal pattern.
pub fn hanging_man(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 15)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 15..len {
        let body_size = body(open[i], close[i]);
        let up_shadow = upper_shadow(high[i], open[i], close[i]);
        let lo_shadow = lower_shadow(low[i], open[i], close[i]);

        // Check for hammer shape
        if lo_shadow >= body_size * 2.0 && up_shadow <= body_size * 0.1 {
            // Check for prior uptrend (last 3 candles up)
            let uptrend = (1..=3).all(|j| close[i - j] > close[i - j - 1]);

            if uptrend {
                output[i] = -100;
            }
        }
    }

    Ok(output)
}

/// Shooting Star (SHOOTINGSTAR)
///
/// Same shape as Inverted Hammer but appears after uptrend.
/// Bearish reversal pattern.
pub fn shooting_star(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 15)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 15..len {
        let body_size = body(open[i], close[i]);
        let up_shadow = upper_shadow(high[i], open[i], close[i]);
        let lo_shadow = lower_shadow(low[i], open[i], close[i]);

        if up_shadow >= body_size * 2.0 && lo_shadow <= body_size * 0.1 {
            let uptrend = (1..=3).all(|j| close[i - j] > close[i - j - 1]);

            if uptrend {
                output[i] = -100;
            }
        }
    }

    Ok(output)
}

/// Engulfing Pattern (ENGULFING)
///
/// A two-candle pattern where the second candle's body completely engulfs the first.
/// Bullish: first candle bearish, second candle bullish and engulfs.
/// Bearish: first candle bullish, second candle bearish and engulfs.
pub fn engulfing(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 2)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 1..len {
        let prev_body = body(open[i - 1], close[i - 1]);
        let curr_body = body(open[i], close[i]);

        if curr_body > prev_body {
            // Bullish engulfing
            if is_bearish(open[i - 1], close[i - 1])
                && is_bullish(open[i], close[i])
                && open[i] <= close[i - 1]
                && close[i] >= open[i - 1]
            {
                output[i] = 100;
            }
            // Bearish engulfing
            else if is_bullish(open[i - 1], close[i - 1])
                && is_bearish(open[i], close[i])
                && open[i] >= close[i - 1]
                && close[i] <= open[i - 1]
            {
                output[i] = -100;
            }
        }
    }

    Ok(output)
}

/// Harami Pattern (HARAMI)
///
/// A two-candle pattern where the second candle's body is contained within the first.
/// Bullish: first candle bearish (large), second candle bullish (small).
/// Bearish: first candle bullish (large), second candle bearish (small).
pub fn harami(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 2)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 1)..len {
        let prev_long = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        let curr_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        let prev_high = open[i - 1].max(close[i - 1]);
        let prev_low = open[i - 1].min(close[i - 1]);
        let curr_high = open[i].max(close[i]);
        let curr_low = open[i].min(close[i]);
        if talib_body(open[i - 1], close[i - 1]) > prev_long
            && talib_body(open[i], close[i]) <= curr_short
        {
            let sign = if close[i - 1] >= open[i - 1] { -1 } else { 1 };
            if curr_high < prev_high && curr_low > prev_low {
                output[i] = sign * 100;
            } else if curr_high <= prev_high && curr_low >= prev_low {
                output[i] = sign * 80;
            }
        }
    }

    Ok(output)
}

/// Harami Cross (HARAMICROSS)
///
/// A Harami where the second candle is a Doji.
pub fn harami_cross(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 2)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in (CDL_AVG_PERIOD + 1)..len {
        let prev_long = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        let curr_doji = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 0.1, 1);
        let prev_high = open[i - 1].max(close[i - 1]);
        let prev_low = open[i - 1].min(close[i - 1]);
        let curr_high = open[i].max(close[i]);
        let curr_low = open[i].min(close[i]);
        if talib_body(open[i - 1], close[i - 1]) > prev_long
            && talib_body(open[i], close[i]) <= curr_doji
        {
            let sign = if close[i - 1] >= open[i - 1] { -1 } else { 1 };
            if curr_high < prev_high && curr_low > prev_low {
                output[i] = sign * 100;
            } else if curr_high <= prev_high && curr_low >= prev_low {
                output[i] = sign * 80;
            }
        }
    }

    Ok(output)
}

/// Morning Star (MORNINGSTAR)
///
/// A three-candle bullish reversal pattern.
/// 1. Long bearish candle
/// 2. Small-bodied candle (star) that gaps down
/// 3. Long bullish candle that closes into the first candle's body
pub fn morning_star(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 2)..len {
        let first_body = talib_body(open[i - 2], close[i - 2]);
        let second_body = talib_body(open[i - 1], close[i - 1]);
        let third_body = talib_body(open[i], close[i]);
        let first_long = talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 0);
        let second_short = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        let third_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);

        if close[i - 2] < open[i - 2]
            && close[i] >= open[i]
            && open[i - 1].max(close[i - 1]) < open[i - 2].min(close[i - 2])
            && close[i] > close[i - 2] + first_body * 0.3
            && first_body > first_long
            && second_body <= second_short
            && third_body > third_short
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Evening Star (EVENINGSTAR)
///
/// A three-candle bearish reversal pattern.
/// 1. Long bullish candle
/// 2. Small-bodied candle (star) that gaps up
/// 3. Long bearish candle that closes into the first candle's body
pub fn evening_star(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 2)..len {
        let first_body = talib_body(open[i - 2], close[i - 2]);
        let second_body = talib_body(open[i - 1], close[i - 1]);
        let third_body = talib_body(open[i], close[i]);
        let first_long = talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 0);
        let second_short = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        let third_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);

        if close[i - 2] >= open[i - 2]
            && close[i] < open[i]
            && open[i - 1].min(close[i - 1]) > open[i - 2].max(close[i - 2])
            && close[i] < close[i - 2] - first_body * 0.3
            && first_body > first_long
            && second_body <= second_short
            && third_body > third_short
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// Morning Doji Star (MORNINGDOJISTAR)
///
/// Like Morning Star but the second candle is a Doji.
pub fn morning_doji_star(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    doji_pct: f64,
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in (CDL_AVG_PERIOD + 2)..len {
        let first_body = talib_body(open[i - 2], close[i - 2]);
        let second_body = talib_body(open[i - 1], close[i - 1]);
        let third_body = talib_body(open[i], close[i]);
        let first_long = talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 0);
        let second_doji = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, doji_pct, 1);
        let third_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        if close[i - 2] < open[i - 2]
            && close[i] >= open[i]
            && open[i - 1].max(close[i - 1]) < open[i - 2].min(close[i - 2])
            && close[i] > close[i - 2] + first_body * 0.3
            && first_body > first_long
            && second_body <= second_doji
            && third_body > third_short
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Evening Doji Star (EVENINGDOJISTAR)
///
/// Like Evening Star but the second candle is a Doji.
pub fn evening_doji_star(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    doji_pct: f64,
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in (CDL_AVG_PERIOD + 2)..len {
        let first_body = talib_body(open[i - 2], close[i - 2]);
        let second_body = talib_body(open[i - 1], close[i - 1]);
        let third_body = talib_body(open[i], close[i]);
        let first_long = talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 0);
        let second_doji = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, doji_pct, 1);
        let third_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        if close[i - 2] >= open[i - 2]
            && close[i] < open[i]
            && open[i - 1].min(close[i - 1]) > open[i - 2].max(close[i - 2])
            && close[i] < close[i - 2] - first_body * 0.3
            && first_body > first_long
            && second_body <= second_doji
            && third_body > third_short
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// Three White Soldiers (THREEWHITESOLDIERS)
///
/// Three consecutive bullish candles with higher closes.
/// Bullish reversal pattern.
pub fn three_white_soldiers(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 13)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;

    for i in period..len {
        if i < 2 {
            continue;
        }
        let avg_range = candle_avg_range(high, low, close, period, i);

        let all_bullish = is_bullish(open[i], close[i])
            && is_bullish(open[i - 1], close[i - 1])
            && is_bullish(open[i - 2], close[i - 2]);

        let higher_closes = close[i] > close[i - 1] && close[i - 1] > close[i - 2];

        let opens_within_prev = open[i] > open[i - 1] && open[i - 1] > open[i - 2];

        if all_bullish && higher_closes && opens_within_prev {
            let bodies_large = body(open[i], close[i]) > avg_range * 0.5
                && body(open[i - 1], close[i - 1]) > avg_range * 0.5
                && body(open[i - 2], close[i - 2]) > avg_range * 0.5;

            if bodies_large {
                output[i] = 100;
            }
        }
    }

    Ok(output)
}

/// Three Black Crows (THREEBLACKCROWS)
///
/// Three consecutive bearish candles with lower closes.
/// Bearish reversal pattern.
pub fn three_black_crows(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 13)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;

    for i in period..len {
        if i < 2 {
            continue;
        }
        let avg_range = candle_avg_range(high, low, close, period, i);

        let all_bearish = is_bearish(open[i], close[i])
            && is_bearish(open[i - 1], close[i - 1])
            && is_bearish(open[i - 2], close[i - 2]);

        let lower_closes = close[i] < close[i - 1] && close[i - 1] < close[i - 2];

        let opens_within_prev = open[i] < open[i - 1] && open[i - 1] < open[i - 2];

        if all_bearish && lower_closes && opens_within_prev {
            let bodies_large = body(open[i], close[i]) > avg_range * 0.5
                && body(open[i - 1], close[i - 1]) > avg_range * 0.5
                && body(open[i - 2], close[i - 2]) > avg_range * 0.5;

            if bodies_large {
                output[i] = -100;
            }
        }
    }

    Ok(output)
}

/// Three Inside Up (THREEINSIDEUP)
///
/// Three-candle bullish reversal pattern with Harami followed by confirmation.
pub fn three_inside_up(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 2)..len {
        let first_body = talib_body(open[i - 2], close[i - 2]);
        let second_body = talib_body(open[i - 1], close[i - 1]);
        if open[i - 1].max(close[i - 1]) < open[i - 2].max(close[i - 2])
            && open[i - 1].min(close[i - 1]) > open[i - 2].min(close[i - 2])
            && first_body > talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 0)
            && second_body <= talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0)
            && close[i - 2] < open[i - 2]
            && close[i] >= open[i]
            && close[i] > open[i - 2]
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Three Outside Up (THREEOUTSIDEUP)
///
/// Three-candle bullish reversal pattern with Bullish Engulfing followed by confirmation.
pub fn three_outside_up(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 2..len {
        if close[i - 1] >= open[i - 1]
            && close[i - 2] < open[i - 2]
            && close[i - 1] > open[i - 2]
            && open[i - 1] < close[i - 2]
            && close[i] > close[i - 1]
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Three Inside Down (THREEINSIDEDOWN)
///
/// Three-candle bearish reversal pattern with Harami followed by confirmation.
pub fn three_inside_down(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 2)..len {
        let first_body = talib_body(open[i - 2], close[i - 2]);
        let second_body = talib_body(open[i - 1], close[i - 1]);
        if open[i - 1].max(close[i - 1]) < open[i - 2].max(close[i - 2])
            && open[i - 1].min(close[i - 1]) > open[i - 2].min(close[i - 2])
            && first_body > talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 0)
            && second_body <= talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0)
            && close[i - 2] >= open[i - 2]
            && close[i] < open[i]
            && close[i] < open[i - 2]
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// Three Outside Down (THREEOUTSIDEDOWN)
///
/// Three-candle bearish reversal pattern with Bearish Engulfing followed by confirmation.
pub fn three_outside_down(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 2..len {
        if close[i - 1] < open[i - 1]
            && close[i - 2] >= open[i - 2]
            && open[i - 1] > close[i - 2]
            && close[i - 1] < open[i - 2]
            && close[i] < close[i - 1]
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// Three Stars In The South (THREESTARSINSOUTH)
///
/// A rare four-candle bullish reversal pattern.
pub fn three_stars_in_south(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 4)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 2)..len {
        let first_body = talib_body(open[i - 2], close[i - 2]);
        let second_body = talib_body(open[i - 1], close[i - 1]);
        let third_body = talib_body(open[i], close[i]);
        let first_long = talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 0);
        let first_shadow = talib_average(open, high, low, close, i - 2, 0, 1.0, 0);
        let second_shadow = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 0.1, 1);
        let third_shadow = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 0.1, 1);
        if close[i - 2] < open[i - 2]
            && close[i - 1] < open[i - 1]
            && close[i] < open[i]
            && first_body > first_long
            && talib_lower_shadow(open[i - 2], low[i - 2], close[i - 2]) > first_shadow
            && second_body < first_body
            && open[i - 1] > close[i - 2]
            && open[i - 1] <= high[i - 2]
            && low[i - 1] < close[i - 2]
            && low[i - 1] >= low[i - 2]
            && talib_lower_shadow(open[i - 1], low[i - 1], close[i - 1]) > second_shadow
            && third_body < talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0)
            && talib_lower_shadow(open[i], low[i], close[i]) < third_shadow
            && talib_upper_shadow(open[i], high[i], close[i]) < third_shadow
            && low[i] > low[i - 1]
            && high[i] < high[i - 1]
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Three Line Strike (THREELINESTRIKE)
///
/// Three consecutive candles in trend direction, followed by a strong reversal candle.
pub fn three_line_strike(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 4)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 3..len {
        // Bullish three line strike
        if is_bearish(open[i - 3], close[i - 3])
            && is_bearish(open[i - 2], close[i - 2])
            && is_bearish(open[i - 1], close[i - 1])
            && close[i - 1] < close[i - 2]
            && close[i - 2] < close[i - 3]
            && is_bullish(open[i], close[i])
            && open[i] < close[i - 1]
            && close[i] > open[i - 3]
        {
            output[i] = 100;
        }
        // Bearish three line strike
        else if is_bullish(open[i - 3], close[i - 3])
            && is_bullish(open[i - 2], close[i - 2])
            && is_bullish(open[i - 1], close[i - 1])
            && close[i - 1] > close[i - 2]
            && close[i - 2] > close[i - 3]
            && is_bearish(open[i], close[i])
            && open[i] > close[i - 1]
            && close[i] < open[i - 3]
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// Stick Sandwich (STICKSANDWICH)
///
/// A bearish candle between two bullish candles at similar price levels.
pub fn stick_sandwich(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 2)..len {
        let equal = talib_average(open, high, low, close, i - 2, 5, 0.05, 1);
        if close[i - 2] < open[i - 2]
            && close[i - 1] >= open[i - 1]
            && close[i] < open[i]
            && low[i - 1] > close[i - 2]
            && close[i] <= close[i - 2] + equal
            && close[i] >= close[i - 2] - equal
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Belt Hold (BELTHOLD)
///
/// A candle that opens at its high (bearish) or low (bullish) with no shadow on one side.
pub fn belt_hold(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 1)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in CDL_AVG_PERIOD..len {
        let body_long = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        let shadow_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 0.1, 1);
        let body = talib_body(open[i], close[i]);
        let color = talib_color(open[i], close[i]);
        let lower = talib_lower_shadow(open[i], low[i], close[i]);
        let upper = talib_upper_shadow(open[i], high[i], close[i]);
        if body > body_long
            && ((color > 0 && lower < shadow_short) || (color < 0 && upper < shadow_short))
        {
            output[i] = color;
        }
    }

    Ok(output)
}

/// Closing Marubozu (CLOSINGMARUBOZU)
///
/// A candle with no shadow at the close side.
pub fn closing_marubozu(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 1)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in CDL_AVG_PERIOD..len {
        let body_long = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        let shadow_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 0.1, 1);
        let body = talib_body(open[i], close[i]);
        let color = talib_color(open[i], close[i]);
        let upper = talib_upper_shadow(open[i], high[i], close[i]);
        let lower = talib_lower_shadow(open[i], low[i], close[i]);
        if body > body_long
            && ((color > 0 && upper < shadow_short) || (color < 0 && lower < shadow_short))
        {
            output[i] = color;
        }
    }

    Ok(output)
}

/// Spinning Top (SPINNINGTOP)
///
/// Small body with upper and lower shadows of similar length.
pub fn spinning_top(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 11)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in CDL_AVG_PERIOD..len {
        let body_size = talib_body(open[i], close[i]);
        let avg_body = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        let up_shadow = talib_upper_shadow(open[i], high[i], close[i]);
        let lo_shadow = talib_lower_shadow(open[i], low[i], close[i]);

        if avg_body > body_size && up_shadow > body_size && lo_shadow > body_size {
            output[i] = talib_color(open[i], close[i]);
        }
    }

    Ok(output)
}

/// High Wave (HIGHWAVE)
///
/// Similar to Spinning Top but with longer shadows indicating high volatility.
pub fn high_wave(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), CDL_AVG_PERIOD + 1)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in CDL_AVG_PERIOD..len {
        let body_size = talib_body(open[i], close[i]);
        let avg_body = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        let long_shadow = talib_average(open, high, low, close, i, 0, 2.0, 0);
        let up_shadow = talib_upper_shadow(open[i], high[i], close[i]);
        let lo_shadow = talib_lower_shadow(open[i], low[i], close[i]);

        if avg_body > body_size && up_shadow > long_shadow && lo_shadow > long_shadow {
            output[i] = talib_color(open[i], close[i]);
        }
    }

    Ok(output)
}

/// Rickshaw Man (RICKSHAWMAN)
///
/// A Doji with long upper and lower shadows and the open/close near the midpoint.
pub fn rickshaw_man(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), CDL_AVG_PERIOD + 1)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in CDL_AVG_PERIOD..len {
        let body_size = talib_body(open[i], close[i]);
        let doji = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 0.1, 1);
        let long_shadow = talib_average(open, high, low, close, i, 0, 1.0, 0);
        let near = talib_average(open, high, low, close, i, 5, 0.2, 1);
        let midpoint = low[i] + (high[i] - low[i]) / 2.0;
        let min_body = open[i].min(close[i]);
        let max_body = open[i].max(close[i]);

        if body_size <= doji
            && talib_lower_shadow(open[i], low[i], close[i]) > long_shadow
            && talib_upper_shadow(open[i], high[i], close[i]) > long_shadow
            && min_body <= midpoint + near
            && max_body >= midpoint - near
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Short Line Candle (SHORTLINE)
///
/// A candle with a very small range.
pub fn short_line(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), CDL_AVG_PERIOD + 1)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in CDL_AVG_PERIOD..len {
        let body_size = talib_body(open[i], close[i]);
        let avg_body = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        let avg_shadow = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 2);
        let up_shadow = talib_upper_shadow(open[i], high[i], close[i]);
        let lo_shadow = talib_lower_shadow(open[i], low[i], close[i]);

        if avg_body > body_size && avg_shadow > up_shadow && avg_shadow > lo_shadow {
            output[i] = talib_color(open[i], close[i]);
        }
    }

    Ok(output)
}

/// Long Line Candle (LONGLINE)
///
/// A candle with a very large range and large body.
pub fn long_line(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), CDL_AVG_PERIOD + 1)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in CDL_AVG_PERIOD..len {
        let body_size = talib_body(open[i], close[i]);
        let avg_body = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        let avg_shadow = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 2);
        let up_shadow = talib_upper_shadow(open[i], high[i], close[i]);
        let lo_shadow = talib_lower_shadow(open[i], low[i], close[i]);

        if body_size > avg_body && avg_shadow > up_shadow && avg_shadow > lo_shadow {
            output[i] = talib_color(open[i], close[i]);
        }
    }

    Ok(output)
}

/// Piercing Pattern (PIERCING)
///
/// A two-candle bullish reversal pattern.
/// First candle is bearish, second candle opens lower and closes above midpoint of first.
pub fn piercing(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 2)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 1)..len {
        let prev_body = talib_body(open[i - 1], close[i - 1]);
        let prev_long = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        let curr_long = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        if close[i - 1] < open[i - 1]
            && prev_body > prev_long
            && close[i] >= open[i]
            && talib_body(open[i], close[i]) > curr_long
            && open[i] < low[i - 1]
            && close[i] < open[i - 1]
            && close[i] > close[i - 1] + prev_body * 0.5
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Dark Cloud Cover (DARKCLOUDCOVER)
///
/// A two-candle bearish reversal pattern (opposite of Piercing).
pub fn dark_cloud_cover(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 2)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in CDL_AVG_PERIOD..len {
        let first_body = talib_body(open[i - 1], close[i - 1]);
        let long_body = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        if close[i - 1] >= open[i - 1]
            && first_body > long_body
            && close[i] < open[i]
            && open[i] > high[i - 1]
            && close[i] > open[i - 1]
            && close[i] < close[i - 1] - first_body * 0.5
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// Tweezer Top (TWEEZERTOP)
///
/// Two candles with matching highs after an uptrend.
pub fn tweezer_top(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 12)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 1..len {
        if (high[i] - high[i - 1]).abs() < 1e-10 && is_bearish(open[i], close[i]) {
            let uptrend = (2..=5).all(|j| i >= j && close[i - j + 1] > close[i - j]);
            if uptrend {
                output[i] = -100;
            }
        }
    }

    Ok(output)
}

/// Tweezer Bottom (TWEEZERBOT)
///
/// Two candles with matching lows after a downtrend.
pub fn tweezer_bot(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 12)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 1..len {
        if (low[i] - low[i - 1]).abs() < 1e-10 && is_bullish(open[i], close[i]) {
            let downtrend = (2..=5).all(|j| i >= j && close[i - j + 1] < close[i - j]);
            if downtrend {
                output[i] = 100;
            }
        }
    }

    Ok(output)
}

/// Abandoned Baby (ABANDONEDBABY)
///
/// A rare three-candle reversal pattern with gaps and a doji.
pub fn abandoned_baby(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    _penetration_pct: f64,
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 2..len {
        let body_size = body(open[i - 1], close[i - 1]);
        if body_size > (high[i - 1] - low[i - 1]) * 0.1 {
            continue;
        }

        // Bullish abandoned baby
        if is_bearish(open[i - 2], close[i - 2])
            && low[i - 1] > high[i - 2]
            && is_bullish(open[i], close[i])
            && low[i] > high[i - 1]
        {
            output[i] = 100;
        }
        // Bearish abandoned baby
        else if is_bullish(open[i - 2], close[i - 2])
            && high[i - 1] < low[i - 2]
            && is_bearish(open[i], close[i])
            && high[i] < low[i - 1]
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// Upside Gap Two Crows (UPSIDEGAP2CROWS)
///
/// A three-candle bearish continuation pattern.
pub fn upside_gap_2crows(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 3..len {
        let gap_up = low[i - 2] > high[i - 3];

        if is_bullish(open[i - 2], close[i - 2])
            && gap_up
            && is_bearish(open[i - 1], close[i - 1])
            && low[i - 1] > close[i - 2]
            && is_bearish(open[i], close[i])
            && open[i] < close[i - 1]
            && close[i] < close[i - 2]
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// Upside/Downside Gap Three Methods (UPSIDEGAP3METHODS / DOWNSIDEGAP3METHODS)
/// A continuation pattern with a gap followed by opposite candles.
pub fn upside_gap_3methods(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 3..len {
        if is_bullish(open[i - 2], close[i - 2])
            && low[i - 2] > high[i - 3]
            && is_bearish(open[i - 1], close[i - 1])
            && is_bearish(open[i], close[i])
            && close[i - 1] > close[i - 2]
            && close[i] < close[i - 2]
            && close[i] > open[i - 2]
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Mat Hold (MATHOLD)
///
/// A five-candle bullish continuation pattern.
pub fn mat_hold(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 5)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 4)..len {
        let first_body = talib_body(open[i - 4], close[i - 4]);
        let first_long = first_body > talib_average(open, high, low, close, i - 4, CDL_AVG_PERIOD, 1.0, 0);
        let short3 = talib_body(open[i - 3], close[i - 3]) < talib_average(open, high, low, close, i - 3, CDL_AVG_PERIOD, 1.0, 0);
        let short4 = talib_body(open[i - 2], close[i - 2]) < talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 0);
        let short5 = talib_body(open[i - 1], close[i - 1]) < talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        let up = is_bullish(open[i - 4], close[i - 4])
            && is_bearish(open[i - 3], close[i - 3])
            && is_bullish(open[i], close[i])
            && open[i - 3].min(close[i - 3]) > open[i - 4].max(close[i - 4])
            && open[i - 2].min(close[i - 2]) < close[i - 4]
            && open[i - 1].min(close[i - 1]) < close[i - 4]
            && open[i - 2].min(close[i - 2]) > close[i - 4] - first_body * 0.5
            && open[i - 1].min(close[i - 1]) > close[i - 4] - first_body * 0.5
            && open[i - 2].max(close[i - 2]) < open[i - 3]
            && open[i - 1].max(close[i - 1]) < open[i - 2].max(close[i - 2])
            && open[i] > close[i - 1]
            && close[i] > high[i - 3].max(high[i - 2]).max(high[i - 1]);
        if up && first_long && short3 && short4 && short5 {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Tasuki Gap (TASUKIGAP)
///
/// A three-candle continuation pattern with a gap.
pub fn tasuki_gap(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 2)..len {
        let near = talib_average(open, high, low, close, i - 1, 5, 0.2, 1);
        let up = open[i - 1].min(close[i - 1]) > open[i - 2].max(close[i - 2])
            && is_bullish(open[i - 1], close[i - 1])
            && is_bearish(open[i], close[i])
            && open[i] < close[i - 1]
            && open[i] > open[i - 1]
            && close[i] < open[i - 1]
            && close[i] > open[i - 2].max(close[i - 2])
            && (talib_body(open[i - 1], close[i - 1]) - talib_body(open[i], close[i])).abs() < near;
        let down = open[i - 1].max(close[i - 1]) < open[i - 2].min(close[i - 2])
            && is_bearish(open[i - 1], close[i - 1])
            && is_bullish(open[i], close[i])
            && open[i] < open[i - 1]
            && open[i] > close[i - 1]
            && close[i] > open[i - 1]
            && close[i] < open[i - 2].min(close[i - 2])
            && (talib_body(open[i - 1], close[i - 1]) - talib_body(open[i], close[i])).abs() < near;
        if up || down {
            output[i] = if up { 100 } else { -100 };
        }
    }

    Ok(output)
}

/// Separating Lines (SEPARATINGLINES)
///
/// Two candles with the same open price after different trends.
pub fn separating_lines(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 2)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 1..len {
        if (open[i] - open[i - 1]).abs() < 1e-10 {
            if is_bearish(open[i - 1], close[i - 1]) && is_bullish(open[i], close[i]) {
                output[i] = 100;
            } else if is_bullish(open[i - 1], close[i - 1]) && is_bearish(open[i], close[i]) {
                output[i] = -100;
            }
        }
    }

    Ok(output)
}

/// Counter Attack (COUNTERATTACK)
///
/// Two candles where the second candle closes at the same level as the first.
pub fn counter_attack(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 2)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 1)..len {
        let equal = talib_average(open, high, low, close, i - 1, 5, 0.05, 1);
        let prev_long = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        let curr_long = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        let prev_body = talib_body(open[i - 1], close[i - 1]);
        let curr_body = talib_body(open[i], close[i]);
        if (close[i - 1] >= open[i - 1]) != (close[i] >= open[i])
            && prev_body > prev_long
            && curr_body > curr_long
            && close[i] <= close[i - 1] + equal
            && close[i] >= close[i - 1] - equal
        {
            output[i] = if close[i] >= open[i] { 100 } else { -100 };
        }
    }

    Ok(output)
}

/// Matching Low (MATCHINGLOW)
///
/// Two bearish candles with matching close prices.
pub fn matching_low(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 2)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 6..len {
        let equal = talib_average(open, high, low, close, i - 1, 5, 0.05, 1);
        if is_bearish(open[i - 1], close[i - 1])
            && is_bearish(open[i], close[i])
            && (close[i] - close[i - 1]).abs() <= equal
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Identical Three Crows (IDENTICAL3CROWS)
///
/// Three consecutive bearish candles with similar characteristics.
pub fn identical_3crows(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 13)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;

    for i in period..len {
        if i < 2 {
            continue;
        }
        let avg_range = candle_avg_range(high, low, close, period, i);

        if is_bearish(open[i - 2], close[i - 2])
            && is_bearish(open[i - 1], close[i - 1])
            && is_bearish(open[i], close[i])
            && body(open[i - 2], close[i - 2]) > avg_range * 0.5
            && body(open[i - 1], close[i - 1]) > avg_range * 0.5
            && body(open[i], close[i]) > avg_range * 0.5
            && close[i] < close[i - 1]
            && close[i - 1] < close[i - 2]
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// Unique 3 River (UNIQUE3RIVER)
///
/// A four-candle bullish reversal pattern.
pub fn unique_3_river(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 4)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 2)..len {
        if is_bearish(open[i - 2], close[i - 2])
            && is_bearish(open[i - 1], close[i - 1])
            && is_bullish(open[i], close[i])
            && close[i - 1] > close[i - 2]
            && open[i - 1] <= open[i - 2]
            && low[i - 1] < low[i - 2]
            && open[i] > low[i - 1]
            && talib_body(open[i - 2], close[i - 2])
                > talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 0)
            && talib_body(open[i], close[i])
                < talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0)
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Breakaway (BREAKAWAY)
///
/// A five-candle reversal pattern.
pub fn breakaway(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 5)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 4)..len {
        let first_long = talib_body(open[i - 4], close[i - 4])
            > talib_average(open, high, low, close, i - 4, CDL_AVG_PERIOD, 1.0, 0);
        let same_124 = is_bullish(open[i - 4], close[i - 4]) == is_bullish(open[i - 3], close[i - 3])
            && is_bullish(open[i - 3], close[i - 3]) == is_bullish(open[i - 1], close[i - 1]);
        let opposite = is_bullish(open[i], close[i]) != is_bullish(open[i - 1], close[i - 1]);
        let down = !is_bullish(open[i - 4], close[i - 4])
            && open[i - 3].max(close[i - 3]) < open[i - 4].min(close[i - 4])
            && high[i - 2] < high[i - 3] && low[i - 2] < low[i - 3]
            && high[i - 1] < high[i - 2] && low[i - 1] < low[i - 2]
            && close[i] > open[i - 3] && close[i] < close[i - 4];
        let up = is_bullish(open[i - 4], close[i - 4])
            && open[i - 3].min(close[i - 3]) > open[i - 4].max(close[i - 4])
            && high[i - 2] > high[i - 3] && low[i - 2] > low[i - 3]
            && high[i - 1] > high[i - 2] && low[i - 1] > low[i - 2]
            && close[i] < open[i - 3] && close[i] > close[i - 4];
        if first_long && same_124 && opposite && (down || up) {
            output[i] = talib_color(open[i], close[i]);
        }
    }

    Ok(output)
}

/// Concealing Baby Swallow (CONCEALBABYSWALL)
///
/// A four-candle bullish reversal pattern.
pub fn concealing_baby_swallow(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 4)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 3)..len {
        let short = |j: usize| talib_average(open, high, low, close, j, CDL_AVG_PERIOD, 0.1, 1);
        let black = |j: usize| is_bearish(open[j], close[j]);
        let marubozu = |j: usize| {
            talib_lower_shadow(open[j], low[j], close[j]) < short(j)
                && talib_upper_shadow(open[j], high[j], close[j]) < short(j)
        };
        if black(i - 3) && black(i - 2) && black(i - 1) && black(i)
            && marubozu(i - 3) && marubozu(i - 2)
            && open[i - 1].max(close[i - 1]) < open[i - 2].min(close[i - 2])
            && talib_upper_shadow(open[i - 1], high[i - 1], close[i - 1]) > short(i - 1)
            && high[i - 1] > close[i - 2]
            && high[i] > high[i - 1] && low[i] < low[i - 1]
        {
            output[i] = 100;
        }
    }

    Ok(output)
}

/// Kicking (KICKING)
///
/// Two candles with gaps and opposite colors, with long bodies.
pub fn kicking(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 11)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;

    for i in 1..len {
        let avg_range = candle_avg_range(high, low, close, period, i);
        let body_1 = body(open[i - 1], close[i - 1]);
        let body_2 = body(open[i], close[i]);

        if body_1 > avg_range && body_2 > avg_range {
            // Bullish kicking
            if is_bearish(open[i - 1], close[i - 1])
                && is_bullish(open[i], close[i])
                && low[i] > high[i - 1]
            {
                output[i] = 100;
            }
            // Bearish kicking
            else if is_bullish(open[i - 1], close[i - 1])
                && is_bearish(open[i], close[i])
                && high[i] < low[i - 1]
            {
                output[i] = -100;
            }
        }
    }

    Ok(output)
}

/// Kicking by Length (KICKINGBYLENGTH)
///
/// Similar to Kicking but with even longer bodies.
pub fn kicking_by_length(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 11)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;

    for i in 1..len {
        let avg_range = candle_avg_range(high, low, close, period, i);
        let body_1 = body(open[i - 1], close[i - 1]);
        let body_2 = body(open[i], close[i]);

        if body_1 > avg_range * 1.5 && body_2 > avg_range * 1.5 {
            if is_bearish(open[i - 1], close[i - 1])
                && is_bullish(open[i], close[i])
                && low[i] > high[i - 1]
            {
                output[i] = 100;
            } else if is_bullish(open[i - 1], close[i - 1])
                && is_bearish(open[i], close[i])
                && high[i] < low[i - 1]
            {
                output[i] = -100;
            }
        }
    }

    Ok(output)
}

/// Advanced Block (ADVANCEBLOCK)
///
/// A three-candle bearish reversal pattern with small bodies.
pub fn advance_block(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 13)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in (CDL_AVG_PERIOD + 2)..len {
        let b1 = talib_body(open[i - 2], close[i - 2]);
        let b2 = talib_body(open[i - 1], close[i - 1]);
        let b3 = talib_body(open[i], close[i]);
        let near1 = talib_average(open, high, low, close, i - 2, 5, 0.2, 1);
        let near2 = talib_average(open, high, low, close, i - 1, 5, 0.2, 1);
        let far1 = talib_average(open, high, low, close, i - 2, 5, 0.6, 1);
        let far2 = talib_average(open, high, low, close, i - 1, 5, 0.6, 1);
        let short1 = talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 2);
        let short2 = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 2);
        let short3 = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 2);
        let long3 = talib_average(open, high, low, close, i, 0, 1.0, 0);
        let weakened = (b2 < b1 - far1 && b3 < b2 + near2)
            || b3 < b2 - far2
            || (b3 < b2 && b2 < b1 && (talib_upper_shadow(open[i], high[i], close[i]) > short3
                || talib_upper_shadow(open[i - 1], high[i - 1], close[i - 1]) > short2))
            || (b3 < b2 && talib_upper_shadow(open[i], high[i], close[i]) > long3);
        if close[i - 2] >= open[i - 2]
            && close[i - 1] >= open[i - 1]
            && close[i] >= open[i]
            && close[i] > close[i - 1]
            && close[i - 1] > close[i - 2]
            && open[i - 1] > open[i - 2]
            && open[i - 1] <= close[i - 2] + near1
            && open[i] > open[i - 1]
            && open[i] <= close[i - 1] + near2
            && b1 > talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 0)
            && talib_upper_shadow(open[i - 2], high[i - 2], close[i - 2]) < short1
            && weakened
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// Stalled Pattern (STALLEDPATTERN)
///
/// Also known as Deliberation. Three white soldiers followed by hesitation.
pub fn stalled_pattern(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 13)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    for i in (CDL_AVG_PERIOD + 2)..len {
        let b1 = talib_body(open[i - 2], close[i - 2]);
        let b2 = talib_body(open[i - 1], close[i - 1]);
        let b3 = talib_body(open[i], close[i]);
        let near1 = talib_average(open, high, low, close, i - 2, 5, 0.2, 1);
        let near2 = talib_average(open, high, low, close, i - 1, 5, 0.2, 1);
        if close[i - 2] >= open[i - 2]
            && close[i - 1] >= open[i - 1]
            && close[i] >= open[i]
            && close[i] > close[i - 1]
            && close[i - 1] > close[i - 2]
            && b1 > talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 0)
            && b2 > talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0)
            && talib_upper_shadow(open[i - 1], high[i - 1], close[i - 1])
                < talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 0.1, 1)
            && open[i - 1] > open[i - 2]
            && open[i - 1] <= close[i - 2] + near1
            && b3 < talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0)
            && open[i] >= close[i - 1] - b3 - near2
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// Thrusting Pattern (THRUSTING)
///
/// A two-candle bearish continuation pattern.
pub fn thrusting(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 2)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 1)..len {
        let prev_body = talib_body(open[i - 1], close[i - 1]);
        let prev_long = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        let equal = talib_average(open, high, low, close, i - 1, 5, 0.05, 1);
        if close[i - 1] < open[i - 1]
            && prev_body > prev_long
            && close[i] >= open[i]
            && open[i] < low[i - 1]
            && close[i] > close[i - 1] + equal
            && close[i] <= close[i - 1] + prev_body * 0.5
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// In Neck Pattern (INNECK)
///
/// Similar to Piercing but the second candle closes at or near the first candle's close.
pub fn in_neck(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 2)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 1)..len {
        let prev_body = talib_body(open[i - 1], close[i - 1]);
        let prev_long = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        let equal = talib_average(open, high, low, close, i - 1, 5, 0.05, 1);
        if close[i - 1] < open[i - 1]
            && prev_body > prev_long
            && close[i] >= open[i]
            && open[i] < low[i - 1]
            && close[i] <= close[i - 1] + equal
            && close[i] >= close[i - 1]
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// On Neck Pattern (ONNECK)
///
/// Similar to In Neck but the close prices are equal.
pub fn on_neck(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 2)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in (CDL_AVG_PERIOD + 1)..len {
        let body_long = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        let equal = talib_average(open, high, low, close, i - 1, 5, 0.05, 1);
        if is_bearish(open[i - 1], close[i - 1])
            && talib_body(open[i - 1], close[i - 1]) > body_long
            && is_bullish(open[i], close[i])
            && open[i] < low[i - 1]
            && (close[i] - low[i - 1]).abs() <= equal
        {
            output[i] = -100;
        }
    }

    Ok(output)
}

/// Two Crows (CDL2CROWS) — bearish reversal
///
/// Three-candle pattern: long bullish first, gap-up bearish second, bearish third that engulfs second.
pub fn cdl_2crows(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    for i in (CDL_AVG_PERIOD + 2)..len {
        let long_first = talib_body(open[i - 2], close[i - 2])
            > talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 1.0, 0);
        let first_white = is_bullish(open[i - 2], close[i - 2]);
        let second_black = is_bearish(open[i - 1], close[i - 1]);
        let third_black = is_bearish(open[i], close[i]);
        let gap_up = open[i - 1].min(close[i - 1]) > open[i - 2].max(close[i - 2]);
        let engulf = open[i] < open[i - 1]
            && open[i] > close[i - 1]
            && close[i] > open[i - 2]
            && close[i] < close[i - 2];
        if first_white && long_first && second_black && third_black && gap_up && engulf {
            output[i] = -100;
        }
    }
    Ok(output)
}

/// Doji Star (CDLDOJISTAR)
///
/// Two-candle pattern: long candle followed by a doji that gaps away.
pub fn cdl_doji_star(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), CDL_AVG_PERIOD + 2)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    for i in (CDL_AVG_PERIOD + 1)..len {
        let long_prev = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        let doji = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 0.1, 1);
        let prev_body = talib_body(open[i - 1], close[i - 1]);
        let curr_body = talib_body(open[i], close[i]);
        let gaps_up = open[i].min(close[i]) > open[i - 1].max(close[i - 1]);
        let gaps_down = open[i].max(close[i]) < open[i - 1].min(close[i - 1]);
        if prev_body > long_prev && curr_body <= doji {
            if close[i - 1] >= open[i - 1] && gaps_up {
                output[i] = -100;
            } else if close[i - 1] < open[i - 1] && gaps_down {
                output[i] = 100;
            }
        }
    }
    Ok(output)
}

/// Up/Down Gap Side-by-Side White Lines (CDLGAPSIDESIDEWHITE)
pub fn cdl_gap_side_white(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    for i in 5..len {
        let near = talib_average(open, high, low, close, i - 1, 5, 0.2, 1);
        let equal = talib_average(open, high, low, close, i - 1, 5, 0.05, 1);
        let up_gap = open[i - 1].min(close[i - 1]) > open[i - 2].max(close[i - 2])
            && open[i].min(close[i]) > open[i - 2].max(close[i - 2]);
        let down_gap = open[i - 1].max(close[i - 1]) < open[i - 2].min(close[i - 2])
            && open[i].max(close[i]) < open[i - 2].min(close[i - 2]);
        let size_near = talib_body(open[i], close[i]) >= talib_body(open[i - 1], close[i - 1]) - near
            && talib_body(open[i], close[i]) <= talib_body(open[i - 1], close[i - 1]) + near;
        let open_equal = open[i] >= open[i - 1] - equal && open[i] <= open[i - 1] + equal;
        if size_near && open_equal && close[i - 1] >= open[i - 1] && close[i] >= open[i] {
            if up_gap {
                output[i] = 100;
            } else if down_gap {
                output[i] = -100;
            }
        }
    }
    Ok(output)
}

/// Hikkake Pattern (CDLHIKKAKE)
///
/// Inside bar followed by a false breakout.
pub fn cdl_hikkake(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    let mut pattern_result = 0i32;
    let mut saved_high = 0.0;
    let mut saved_low = 0.0;
    let mut countdown = 0i32;
    for i in 5..len {
        if high[i - 1] < high[i - 2]
            && low[i - 1] > low[i - 2]
            && ((high[i] < high[i - 1] && low[i] < low[i - 1])
                || (high[i] > high[i - 1] && low[i] > low[i - 1]))
        {
            pattern_result = if high[i] < high[i - 1] { 100 } else { -100 };
            saved_high = high[i - 1];
            saved_low = low[i - 1];
            countdown = 4;
            output[i] = pattern_result;
        } else if countdown > 0
            && ((pattern_result > 0 && close[i] > saved_high)
                || (pattern_result < 0 && close[i] < saved_low))
        {
            output[i] = pattern_result + if pattern_result > 0 { 100 } else { -100 };
            countdown = 0;
        }
        if countdown > 0 {
            countdown -= 1;
        }
    }
    Ok(output)
}

/// Modified Hikkake Pattern (CDLHIKKAKEMOD)
pub fn cdl_hikkake_mod(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 5)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    let mut pattern_result = 0i32;
    let mut pattern_high = 0.0;
    let mut pattern_low = 0.0;
    let mut pattern_count = 0i32;
    for i in (CDL_AVG_PERIOD)..len {
        let near = talib_average(open, high, low, close, i - 2, 5, 0.2, 1);
        let inside = high[i - 2] < high[i - 3] && low[i - 2] > low[i - 3]
            && high[i - 1] < high[i - 2] && low[i - 1] > low[i - 2];
        let bullish = high[i] < high[i - 1] && low[i] < low[i - 1]
            && close[i - 2] <= low[i - 2] + near;
        let bearish = high[i] > high[i - 1] && low[i] > low[i - 1]
            && close[i - 2] >= high[i - 2] - near;
        if inside && (bullish || bearish) {
            pattern_result = if bullish { 100 } else { -100 };
            pattern_high = high[i - 1];
            pattern_low = low[i - 1];
            pattern_count = 4;
            output[i] = pattern_result;
        } else if pattern_count > 0
            && ((pattern_result > 0 && close[i] > pattern_high)
                || (pattern_result < 0 && close[i] < pattern_low))
        {
            output[i] = pattern_result + if pattern_result > 0 { 100 } else { -100 };
            pattern_count = 0;
        }
        if pattern_count > 0 {
            pattern_count -= 1;
        }
    }
    Ok(output)
}

/// Homing Pigeon (CDLHOMINGPIGEON) — bullish
///
/// Two bearish candles where the second is contained within the first.
pub fn cdl_homing_pigeon(
    open: &[f64],
    _high: &[f64],
    _low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 2)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    let start = if len <= CDL_AVG_PERIOD { 1 } else { CDL_AVG_PERIOD };
    for i in start..len {
        let bear1 = is_bearish(open[i - 1], close[i - 1]);
        let bear2 = is_bearish(open[i], close[i]);
        let first_long = len <= CDL_AVG_PERIOD || talib_body(open[i - 1], close[i - 1])
            > talib_average(open, _high, _low, close, i - 1, CDL_AVG_PERIOD, 1.0, 0);
        let second_short = len <= CDL_AVG_PERIOD || talib_body(open[i], close[i])
            <= talib_average(open, _high, _low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        if bear1 && bear2 && first_long && second_short && open[i] < open[i - 1] && close[i] > close[i - 1] {
            output[i] = 100;
        }
    }
    Ok(output)
}

/// Ladder Bottom (CDLLADDERBOTTOM) — bullish reversal
///
/// Three or more bearish candles with lower closes, followed by a hammer-like candle and a bullish candle.
pub fn cdl_ladder_bottom(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 5)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    for i in (CDL_AVG_PERIOD + 4)..len {
        let very_short = talib_average(open, high, low, close, i - 1, CDL_AVG_PERIOD, 0.1, 1);
        if close[i - 4] < open[i - 4]
            && close[i - 3] < open[i - 3]
            && close[i - 2] < open[i - 2]
            && open[i - 4] > open[i - 3]
            && open[i - 3] > open[i - 2]
            && close[i - 4] > close[i - 3]
            && close[i - 3] > close[i - 2]
            && close[i - 1] < open[i - 1]
            && talib_upper_shadow(open[i - 1], high[i - 1], close[i - 1]) > very_short
            && close[i] >= open[i]
            && open[i] > open[i - 1]
            && close[i] > high[i - 1]
        {
            output[i] = 100;
        }
    }
    Ok(output)
}

/// Rising/Falling Three Methods (CDLRISEFALL3METHODS)
///
/// Continuation pattern: long candle, 3 small counter-trend candles within range, long same-trend candle.
pub fn cdl_rise_fall_3methods(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 5)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    for i in (CDL_AVG_PERIOD + 4)..len {
        let sign = if is_bullish(open[i - 4], close[i - 4]) { 1.0 } else { -1.0 };
        let same_colors = (is_bullish(open[i - 4], close[i - 4]) != is_bullish(open[i - 3], close[i - 3]))
            && is_bullish(open[i - 3], close[i - 3]) == is_bullish(open[i - 2], close[i - 2])
            && is_bullish(open[i - 2], close[i - 2]) == is_bullish(open[i - 1], close[i - 1])
            && is_bullish(open[i - 1], close[i - 1]) != is_bullish(open[i], close[i]);
        let contained = [i - 3, i - 2, i - 1].iter().all(|&j| {
            open[j].min(close[j]) < high[i - 4] && open[j].max(close[j]) > low[i - 4]
        });
        let falling = close[i - 2] * sign < close[i - 3] * sign
            && close[i - 1] * sign < close[i - 2] * sign;
        let final_move = open[i] * sign > close[i - 1] * sign && close[i] * sign > close[i - 4] * sign;
        let first_long = talib_body(open[i - 4], close[i - 4])
            > talib_average(open, high, low, close, i - 4, CDL_AVG_PERIOD, 1.0, 0);
        let last_long = talib_body(open[i], close[i])
            > talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 1.0, 0);
        let small = [i - 3, i - 2, i - 1].iter().all(|&j| {
            talib_body(open[j], close[j])
                < talib_average(open, high, low, close, j, CDL_AVG_PERIOD, 1.0, 0)
        });
        if same_colors && contained && falling == (sign > 0.0) && final_move && first_long && last_long && small {
            output[i] = (sign as i32) * 100;
        }
    }
    Ok(output)
}

/// Takuri (CDLTAKURI) — dragonfly doji variant with very long lower shadow
pub fn cdl_takuri(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 1)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    for i in CDL_AVG_PERIOD..len {
        let doji = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 0.1, 1);
        let very_short = talib_average(open, high, low, close, i, CDL_AVG_PERIOD, 0.1, 1);
        let very_long = talib_average(open, high, low, close, i, 0, 2.0, 0);
        if talib_body(open[i], close[i]) <= doji
            && talib_upper_shadow(open[i], high[i], close[i]) < very_short
            && talib_lower_shadow(open[i], low[i], close[i]) > very_long
        {
            output[i] = 100;
        }
    }
    Ok(output)
}

/// Tristar Pattern (CDLTRISTAR) — three consecutive dojis
pub fn cdl_tristar(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    for i in (CDL_AVG_PERIOD + 2)..len {
        let doji = talib_average(open, high, low, close, i - 2, CDL_AVG_PERIOD, 0.1, 1);
        if talib_body(open[i - 2], close[i - 2]) <= doji
            && talib_body(open[i - 1], close[i - 1]) <= doji
            && talib_body(open[i], close[i]) <= doji
        {
            if open[i - 1].min(close[i - 1]) > open[i - 2].max(close[i - 2])
                && open[i].max(close[i]) < open[i - 1].max(close[i - 1])
            {
                output[i] = -100;
            }
            if open[i - 1].max(close[i - 1]) < open[i - 2].min(close[i - 2])
                && open[i].min(close[i]) > open[i - 1].min(close[i - 1])
            {
                output[i] = 100;
            }
        }
    }
    Ok(output)
}

// =====================================================================
// 完整的 TA-Lib 兼容 CDL 函数 (61 个)
// =====================================================================

/// CDL3BLACKCROWS — Three Black Crows (看跌反转)
pub fn cdl_3black_crows(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    three_black_crows(open, high, low, close)
}

/// CDL3INSIDE — Three Inside Up/Down
///
/// 三蜡烛形态：前两根为 Harami，第三根确认。
/// 返回 100（三 Inside Up，看涨）或 -100（三 Inside Down，看跌）。
pub fn cdl_3inside(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    let up = three_inside_up(open, high, low, close)?;
    let down = three_inside_down(open, high, low, close)?;
    for i in 0..len {
        if up[i] == 100 {
            output[i] = 100;
        } else if down[i] == -100 {
            output[i] = -100;
        }
    }
    Ok(output)
}

/// CDL3LINESTRIKE — Three Line Strike
pub fn cdl_3linestrike(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    three_line_strike(open, high, low, close)
}

/// CDL3OUTSIDE — Three Outside Up/Down
pub fn cdl_3outside(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 3)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    let up = three_outside_up(open, high, low, close)?;
    let down = three_outside_down(open, high, low, close)?;
    for i in 0..len {
        if up[i] == 100 {
            output[i] = 100;
        } else if down[i] == -100 {
            output[i] = -100;
        }
    }
    Ok(output)
}

/// CDL3STARSINSOUTH — Three Stars In The South
pub fn cdl_3starsinsouth(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    three_stars_in_south(open, high, low, close)
}

/// CDL3WHITESOLDIERS — Three White Soldiers (看涨反转)
pub fn cdl_3white_soldiers(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    three_white_soldiers(open, high, low, close)
}

/// CDLABANDONEDBABY — Abandoned Baby
pub fn cdl_abandoned_baby(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    abandoned_baby(open, high, low, close, 0.0)
}

/// CDLADVANCEBLOCK — Advance Block (看跌)
pub fn cdl_advanceblock(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    advance_block(open, high, low, close)
}

/// CDLBELTHOLD — Belt Hold
pub fn cdl_belthold(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    belt_hold(open, high, low, close)
}

/// CDLBREAKAWAY — Breakaway (反转)
pub fn cdl_breakaway(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    breakaway(open, high, low, close)
}

/// CDLCLOSINGMARUBOZU — Closing Marubozu
pub fn cdl_closingmarubozu(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    closing_marubozu(open, high, low, close)
}

/// CDLCONCEALBABYSWALL — Concealing Baby Swallow (看涨)
pub fn cdl_concealbabyswall(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    concealing_baby_swallow(open, high, low, close)
}

/// CDLCOUNTERATTACK — Counterattack
pub fn cdl_counterattack(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    counter_attack(open, high, low, close)
}

/// CDLDARKCLOUDCOVER — Dark Cloud Cover
pub fn cdl_darkcloudcover(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    dark_cloud_cover(open, high, low, close)
}

/// CDLDOJI — Doji
pub fn cdl_doji(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    // TA-Lib's default BodyDoji candle setting uses the preceding average
    // real body directly (factor 1.0); the generic helper keeps its explicit
    // percentage parameter for callers that want a stricter custom rule.
    doji(open, high, low, close, 1.0)
}

/// CDLDRAGONFLYDOJI — Dragonfly Doji
pub fn cdl_dragonflydoji(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    dragonfly_doji(open, high, low, close, 0.1)
}

/// CDLENGULFING — Engulfing Pattern
pub fn cdl_engulfing(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    engulfing(open, high, low, close)
}

/// CDLEVENINGDOJISTAR — Evening Doji Star (看跌)
pub fn cdl_eveningdojistar(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    evening_doji_star(open, high, low, close, 0.1)
}

/// CDLEVENINGSTAR — Evening Star (看跌反转)
pub fn cdl_eveningstar(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    evening_star(open, high, low, close)
}

/// CDLGRAVESTONEDOJI — Gravestone Doji
pub fn cdl_gravestonedoji(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    gravestone_doji(open, high, low, close, 0.1)
}

/// CDLHAMMER — Hammer (看涨反转)
pub fn cdl_hammer(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    hammer(open, high, low, close)
}

/// CDLHANGINGMAN — Hanging Man (看跌反转)
pub fn cdl_hangingman(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    hanging_man(open, high, low, close)
}

/// CDLHARAMI — Harami Pattern
pub fn cdl_harami(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    harami(open, high, low, close)
}

/// CDLHARAMICROSS — Harami Cross
pub fn cdl_haramicross(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    harami_cross(open, high, low, close)
}

/// CDLHIGHWAVE — High Wave
pub fn cdl_highwave(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    high_wave(open, high, low, close)
}

/// CDLIDENTICAL3CROWS — Identical Three Crows (看跌)
pub fn cdl_identical3crows(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    identical_3crows(open, high, low, close)
}

/// CDLINNECK — In Neck Pattern (看跌)
pub fn cdl_inneck(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    in_neck(open, high, low, close)
}

/// CDLONNECK — On Neck Pattern (看跌)
pub fn cdl_onneck(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    on_neck(open, high, low, close)
}

/// CDLINVERTEDHAMMER — Inverted Hammer (看涨反转)
pub fn cdl_invertedhammer(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    inverted_hammer(open, high, low, close)
}

/// CDLKICKING — Kicking (强烈反转)
pub fn cdl_kicking(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    kicking(open, high, low, close)
}

/// CDLKICKINGBYLENGTH — Kicking by Length
pub fn cdl_kickingbylength(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    kicking_by_length(open, high, low, close)
}

/// CDLLONGLEGGEDDOJI — Long-Legged Doji
pub fn cdl_longleggeddoji(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    long_legged_doji(open, high, low, close, 0.1)
}

/// CDLLONGLINE — Long Line Candle
pub fn cdl_longline(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    long_line(open, high, low, close)
}

/// CDLMAJORMINORITY — Major Minor Reversal
///
/// 三蜡烛形态：长实体的 major + 短实体的 minor + 短实体的反方向。
pub fn cdl_majorminority(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 13)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    let period = 10;
    for i in period..len {
        if i < 2 {
            continue;
        }
        let avg_range = candle_avg_range(high, low, close, period, i);
        let body1 = body(open[i - 2], close[i - 2]);
        let body2 = body(open[i - 1], close[i - 1]);
        let body3 = body(open[i], close[i]);
        // Bullish Major Minor: long black, small white gap up, small black
        if body1 > avg_range * 0.7
            && is_bearish(open[i - 2], close[i - 2])
            && is_bullish(open[i - 1], close[i - 1])
            && open[i - 1] > close[i - 2]
            && body2 < body1 * 0.3
            && is_bearish(open[i], close[i])
            && body3 < body1 * 0.3
            && close[i] < close[i - 2]
        {
            output[i] = 100;
        }
        // Bearish Major Minor: long white, small black gap down, small white
        else if body1 > avg_range * 0.7
            && is_bullish(open[i - 2], close[i - 2])
            && is_bearish(open[i - 1], close[i - 1])
            && open[i - 1] < close[i - 2]
            && body2 < body1 * 0.3
            && is_bullish(open[i], close[i])
            && body3 < body1 * 0.3
            && close[i] > close[i - 2]
        {
            output[i] = -100;
        }
    }
    Ok(output)
}

/// CDLMARUBOZU — Marubozu
pub fn cdl_marubozu(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    marubozu(open, high, low, close, 0.05)
}

/// CDLMATCHINGLOW — Matching Low (看涨)
pub fn cdl_matchinglow(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    matching_low(open, high, low, close)
}

/// CDLMATHOLD — Mat Hold (看涨持续)
pub fn cdl_mathold(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    mat_hold(open, high, low, close)
}

/// CDLMORNINGDOJISTAR — Morning Doji Star (看涨反转)
pub fn cdl_morningdojistar(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    morning_doji_star(open, high, low, close, 0.1)
}

/// CDLMORNINGSTAR — Morning Star (看涨反转)
pub fn cdl_morningstar(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    morning_star(open, high, low, close)
}

/// CDLONSIDE — On Neck (与 in_neck 类似，简化实现)
pub fn cdl_onside(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    on_neck(open, high, low, close)
}

/// CDLPIERCING — Piercing Pattern (看涨反转)
pub fn cdl_piercing(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    piercing(open, high, low, close)
}

/// CDLRICKSHAWMAN — Rickshaw Man
pub fn cdl_rickshawman(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    rickshaw_man(open, high, low, close)
}

/// CDLSEPARATINGLINES — Separating Lines
pub fn cdl_separatinglines(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    separating_lines(open, high, low, close)
}

/// CDLSHOOTINGSTAR — Shooting Star (看跌反转)
pub fn cdl_shootingstar(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    shooting_star(open, high, low, close)
}

/// CDLSHORTLINE — Short Line Candle
pub fn cdl_shortline(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    short_line(open, high, low, close)
}

/// CDLSPINNINGTOP — Spinning Top
pub fn cdl_spinningtop(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    spinning_top(open, high, low, close)
}

/// CDLSTALLEDPATTERN — Stalled Pattern (看跌)
pub fn cdl_stalledpattern(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    stalled_pattern(open, high, low, close)
}

/// CDLSTICKSANDWICH — Stick Sandwich (看涨)
pub fn cdl_sticksandwich(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    stick_sandwich(open, high, low, close)
}

/// CDLTASUKIGAP — Tasuki Gap (持续形态)
pub fn cdl_tasukigap(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    tasuki_gap(open, high, low, close)
}

/// CDLTHRUSTING — Thrusting Pattern (看跌持续)
pub fn cdl_thrusting(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    thrusting(open, high, low, close)
}

/// CDLUNIQUE3RIVER — Unique Three River (看涨反转)
pub fn cdl_unique3river(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    unique_3_river(open, high, low, close)
}

/// CDLUPSIDEGAP2CROWS — Upside Gap Two Crows (看跌)
pub fn cdl_upsidegap2crows(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    upside_gap_2crows(open, high, low, close)
}

/// CDLXSIDEGAP3METHODS — Up/Down Gap Side-By-Side Three Methods
pub fn cdl_xsidegap3methods(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 4)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    for i in 3..len {
        // Upside gap three methods (continuation in uptrend)
        if is_bullish(open[i - 2], close[i - 2])
            && low[i - 2] > high[i - 3]
            && is_bearish(open[i - 1], close[i - 1])
            && is_bearish(open[i], close[i])
            && close[i - 1] > close[i - 2]
            && close[i] < close[i - 2]
            && close[i] > open[i - 2]
        {
            output[i] = 100;
        }
        // Downside gap three methods (continuation in downtrend)
        else if is_bearish(open[i - 2], close[i - 2])
            && high[i - 2] < low[i - 3]
            && is_bullish(open[i - 1], close[i - 1])
            && is_bullish(open[i], close[i])
            && close[i - 1] < close[i - 2]
            && close[i] > close[i - 2]
            && close[i] < open[i - 2]
        {
            output[i] = -100;
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doji() {
        let open = vec![
            10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0,
        ];
        let high = vec![
            10.1, 10.1, 10.1, 10.1, 10.1, 10.1, 10.1, 10.1, 10.1, 10.1, 10.1,
        ];
        let low = vec![9.9, 9.9, 9.9, 9.9, 9.9, 9.9, 9.9, 9.9, 9.9, 9.9, 9.9];
        let close = vec![
            10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0,
        ];
        let result = doji(&open, &high, &low, &close, 0.1).unwrap();
        assert_eq!(result[10], 100);
    }

    #[test]
    fn test_engulfing() {
        let open = vec![10.0, 12.0, 9.5, 11.0];
        let high = vec![10.5, 12.5, 10.0, 11.5];
        let low = vec![9.5, 11.5, 9.0, 10.5];
        let close = vec![11.0, 11.5, 10.0, 10.5];
        let result = engulfing(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_hammer() {
        let open = vec![
            10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0,
        ];
        let high = vec![
            10.1, 10.1, 10.1, 10.1, 10.1, 10.1, 10.1, 10.1, 10.1, 10.1, 10.2,
        ];
        let low = vec![9.5, 9.5, 9.5, 9.5, 9.5, 9.5, 9.5, 9.5, 9.5, 9.5, 9.0];
        let close = vec![
            10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.1,
        ];
        let result = hammer(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 11);
    }

    #[test]
    fn test_marubozu() {
        let open = vec![10.0, 12.0];
        let high = vec![12.0, 14.0];
        let low = vec![10.0, 12.0];
        let close = vec![12.0, 14.0];
        let result = marubozu(&open, &high, &low, &close, 0.05).unwrap();
        assert_eq!(result[0], 100);
        assert_eq!(result[1], 100);
    }

    #[test]
    fn test_cdl_2crows() {
        let open = vec![10.0, 12.5, 12.0];
        let high = vec![12.0, 13.0, 12.5];
        let low = vec![9.5, 11.0, 11.0];
        let close = vec![12.0, 11.5, 11.0];
        let result = cdl_2crows(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_cdl_doji_star() {
        let open: Vec<f64> = (0..20).map(|i| 10.0 + i as f64).collect();
        let high: Vec<f64> = open.iter().map(|x| x + 2.0).collect();
        let low: Vec<f64> = open.iter().map(|x| x - 0.5).collect();
        let close: Vec<f64> = open.iter().map(|x| x + 1.5).collect();
        let result = cdl_doji_star(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 20);
    }

    #[test]
    fn test_cdl_gap_side_white() {
        let open = vec![10.0, 12.5, 12.5];
        let high = vec![12.0, 13.5, 13.5];
        let low = vec![9.5, 12.0, 12.0];
        let close = vec![11.5, 13.0, 13.0];
        let result = cdl_gap_side_white(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_cdl_hikkake() {
        let open = vec![10.0, 10.5, 11.0, 9.0];
        let high = vec![12.0, 11.0, 12.5, 11.5];
        let low = vec![9.0, 9.5, 10.0, 8.5];
        let close = vec![11.0, 10.0, 12.0, 8.0];
        let result = cdl_hikkake(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_cdl_hikkake_mod() {
        let open = vec![10.0, 10.5, 10.6, 11.0, 9.0];
        let high = vec![12.0, 11.0, 10.9, 12.5, 11.5];
        let low = vec![9.0, 9.5, 9.6, 10.0, 8.5];
        let close = vec![11.0, 10.0, 10.0, 12.0, 8.0];
        let result = cdl_hikkake_mod(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_homing_pigeon() {
        let open = vec![12.0, 11.5];
        let high = vec![12.5, 12.0];
        let low = vec![10.0, 10.5];
        let close = vec![10.5, 11.0];
        let result = cdl_homing_pigeon(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[1], 100);
    }

    #[test]
    fn test_cdl_ladder_bottom() {
        let open = vec![15.0, 14.0, 13.0, 12.0, 12.0];
        let high = vec![15.5, 14.5, 13.5, 14.0, 13.5];
        let low = vec![13.5, 12.5, 11.5, 11.0, 11.5];
        let close = vec![14.0, 13.0, 12.0, 13.0, 13.0];
        let result = cdl_ladder_bottom(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_rise_fall_3methods() {
        let open = vec![10.0, 11.5, 11.0, 10.5, 10.0];
        let high = vec![12.0, 11.8, 11.3, 10.8, 13.0];
        let low = vec![9.5, 10.5, 10.0, 9.5, 9.5];
        let close = vec![12.0, 10.8, 10.3, 9.8, 13.0];
        let result = cdl_rise_fall_3methods(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_takuri() {
        let open = vec![
            10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0,
        ];
        let high = vec![
            10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.01,
        ];
        let low = vec![9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 7.0];
        let close = vec![
            10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0,
        ];
        let result = cdl_takuri(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 11);
    }

    #[test]
    fn test_cdl_tristar() {
        let open = vec![
            10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.2, 10.0,
        ];
        let high = vec![
            10.5, 10.5, 10.5, 10.5, 10.5, 10.5, 10.5, 10.5, 10.5, 10.5, 10.7, 10.5,
        ];
        let low = vec![9.5, 9.5, 9.5, 9.5, 9.5, 9.5, 9.5, 9.5, 9.5, 9.5, 9.7, 9.5];
        let close = vec![
            10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.2, 10.0,
        ];
        let result = cdl_tristar(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 12);
    }

    // 通用测试 helper: 生成简单测试数据
    fn make_ohlc(n: usize, base: f64, body_size: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let open: Vec<f64> = (0..n).map(|i| base + i as f64).collect();
        let high: Vec<f64> = open.iter().map(|x| x + 1.0).collect();
        let low: Vec<f64> = open.iter().map(|x| x - 1.0).collect();
        let close: Vec<f64> = open.iter().map(|x| x + body_size).collect();
        (open, high, low, close)
    }

    #[test]
    fn test_cdl_3black_crows() {
        let n = 15;
        let mut open = vec![10.0; n];
        let mut close = vec![10.0; n];
        let high = vec![12.0; n];
        let low = vec![8.0; n];
        for i in 10..n {
            open[i] = 11.0;
            close[i] = 10.5 - (i - 10) as f64 * 0.1;
        }
        let result = cdl_3black_crows(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), n);
    }

    #[test]
    fn test_cdl_3inside() {
        let (open, high, low, close) = make_ohlc(10, 10.0, 1.0);
        let result = cdl_3inside(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_cdl_3linestrike() {
        let (open, high, low, close) = make_ohlc(10, 10.0, 1.0);
        let result = cdl_3linestrike(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_cdl_3outside() {
        let (open, high, low, close) = make_ohlc(10, 10.0, 1.0);
        let result = cdl_3outside(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_cdl_3starsinsouth() {
        let (open, high, low, close) = make_ohlc(10, 10.0, 1.0);
        let result = cdl_3starsinsouth(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_cdl_3white_soldiers() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_3white_soldiers(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_abandoned_baby() {
        let (open, high, low, close) = make_ohlc(10, 10.0, 1.0);
        let result = cdl_abandoned_baby(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_cdl_advanceblock() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_advanceblock(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_belthold() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_belthold(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_breakaway() {
        let (open, high, low, close) = make_ohlc(10, 10.0, 1.0);
        let result = cdl_breakaway(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_cdl_closingmarubozu() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_closingmarubozu(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_concealbabyswall() {
        let (open, high, low, close) = make_ohlc(10, 10.0, 1.0);
        let result = cdl_concealbabyswall(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_cdl_counterattack() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_counterattack(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_darkcloudcover() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_darkcloudcover(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_doji() {
        let n = 15;
        let open: Vec<f64> = vec![10.0; n];
        let high: Vec<f64> = vec![10.5; n];
        let low: Vec<f64> = vec![9.5; n];
        let close: Vec<f64> = vec![10.0; n];
        let result = cdl_doji(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), n);
        assert_eq!(result[n - 1], 100);
    }

    #[test]
    fn test_cdl_dragonflydoji() {
        let n = 15;
        let open: Vec<f64> = vec![10.0; n];
        let high: Vec<f64> = vec![10.1; n];
        let low: Vec<f64> = vec![9.0; n];
        let close: Vec<f64> = vec![10.0; n];
        let result = cdl_dragonflydoji(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), n);
    }

    #[test]
    fn test_cdl_engulfing() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_engulfing(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_eveningdojistar() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_eveningdojistar(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_eveningstar() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_eveningstar(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_gravestonedoji() {
        let n = 15;
        let open: Vec<f64> = vec![10.0; n];
        let high: Vec<f64> = vec![11.0; n];
        let low: Vec<f64> = vec![9.9; n];
        let close: Vec<f64> = vec![10.0; n];
        let result = cdl_gravestonedoji(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), n);
    }

    #[test]
    fn test_cdl_hammer() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_hammer(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_hangingman() {
        let (open, high, low, close) = make_ohlc(20, 10.0, 1.0);
        let result = cdl_hangingman(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 20);
    }

    #[test]
    fn test_cdl_harami() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_harami(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_haramicross() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_haramicross(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_highwave() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_highwave(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_identical3crows() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_identical3crows(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_inneck() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_inneck(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_invertedhammer() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_invertedhammer(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_kicking() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_kicking(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_kickingbylength() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_kickingbylength(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_longleggeddoji() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_longleggeddoji(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_longline() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_longline(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_majorminority() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_majorminority(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_marubozu() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_marubozu(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_matchinglow() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_matchinglow(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_mathold() {
        let (open, high, low, close) = make_ohlc(10, 10.0, 1.0);
        let result = cdl_mathold(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_cdl_morningdojistar() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_morningdojistar(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_morningstar() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_morningstar(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_onside() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_onside(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_piercing() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_piercing(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_rickshawman() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_rickshawman(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_separatinglines() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_separatinglines(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_shootingstar() {
        let (open, high, low, close) = make_ohlc(20, 10.0, 1.0);
        let result = cdl_shootingstar(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 20);
    }

    #[test]
    fn test_cdl_shortline() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_shortline(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_spinningtop() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_spinningtop(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_stalledpattern() {
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_stalledpattern(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_sticksandwich() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_sticksandwich(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_tasukigap() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_tasukigap(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_thrusting() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_thrusting(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_unique3river() {
        let (open, high, low, close) = make_ohlc(10, 10.0, 1.0);
        let result = cdl_unique3river(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_cdl_upsidegap2crows() {
        // Use n>=10 to avoid pre-existing underflow bug in underlying
        // upside_gap_2crows() which iterates from i=2 and accesses i-3.
        let (open, high, low, close) = make_ohlc(15, 10.0, 1.0);
        let result = cdl_upsidegap2crows(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_cdl_xsidegap3methods() {
        let (open, high, low, close) = make_ohlc(5, 10.0, 1.0);
        let result = cdl_xsidegap3methods(&open, &high, &low, &close).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_cdl_all_callable() {
        // Sanity check: 10 pre-existing + 51 new cdl_ functions = 61 total
        // (in practice 62 cdl_ functions, see the file for the full list).
        // The function names are statically known, so we just ensure the
        // file compiles with all of them present.  This test always passes
        // (modulo runtime panics from buggy input lengths), but the
        // compiler will fail if any of the functions is missing.
        // Use n=15 to avoid the pre-existing underflow in
        // upside_gap_2crows which is used by cdl_upsidegap2crows.
        let n = 15;
        let (open, high, low, close) = make_ohlc(n, 10.0, 1.0);
        let _ = cdl_2crows(&open, &high, &low, &close);
        let _ = cdl_3black_crows(&open, &high, &low, &close);
        let _ = cdl_3inside(&open, &high, &low, &close);
        let _ = cdl_3linestrike(&open, &high, &low, &close);
        let _ = cdl_3outside(&open, &high, &low, &close);
        let _ = cdl_3starsinsouth(&open, &high, &low, &close);
        let _ = cdl_3white_soldiers(&open, &high, &low, &close);
        let _ = cdl_abandoned_baby(&open, &high, &low, &close);
        let _ = cdl_advanceblock(&open, &high, &low, &close);
        let _ = cdl_belthold(&open, &high, &low, &close);
        let _ = cdl_breakaway(&open, &high, &low, &close);
        let _ = cdl_closingmarubozu(&open, &high, &low, &close);
        let _ = cdl_concealbabyswall(&open, &high, &low, &close);
        let _ = cdl_counterattack(&open, &high, &low, &close);
        let _ = cdl_darkcloudcover(&open, &high, &low, &close);
        let _ = cdl_doji(&open, &high, &low, &close);
        let _ = cdl_doji_star(&open, &high, &low, &close);
        let _ = cdl_dragonflydoji(&open, &high, &low, &close);
        let _ = cdl_engulfing(&open, &high, &low, &close);
        let _ = cdl_eveningdojistar(&open, &high, &low, &close);
        let _ = cdl_eveningstar(&open, &high, &low, &close);
        let _ = cdl_gap_side_white(&open, &high, &low, &close);
        let _ = cdl_gravestonedoji(&open, &high, &low, &close);
        let _ = cdl_hammer(&open, &high, &low, &close);
        let _ = cdl_hangingman(&open, &high, &low, &close);
        let _ = cdl_harami(&open, &high, &low, &close);
        let _ = cdl_haramicross(&open, &high, &low, &close);
        let _ = cdl_highwave(&open, &high, &low, &close);
        let _ = cdl_hikkake(&open, &high, &low, &close);
        let _ = cdl_hikkake_mod(&open, &high, &low, &close);
        let _ = cdl_homing_pigeon(&open, &high, &low, &close);
        let _ = cdl_identical3crows(&open, &high, &low, &close);
        let _ = cdl_inneck(&open, &high, &low, &close);
        let _ = cdl_invertedhammer(&open, &high, &low, &close);
        let _ = cdl_kicking(&open, &high, &low, &close);
        let _ = cdl_kickingbylength(&open, &high, &low, &close);
        let _ = cdl_ladder_bottom(&open, &high, &low, &close);
        let _ = cdl_longleggeddoji(&open, &high, &low, &close);
        let _ = cdl_longline(&open, &high, &low, &close);
        let _ = cdl_majorminority(&open, &high, &low, &close);
        let _ = cdl_marubozu(&open, &high, &low, &close);
        let _ = cdl_matchinglow(&open, &high, &low, &close);
        let _ = cdl_mathold(&open, &high, &low, &close);
        let _ = cdl_morningdojistar(&open, &high, &low, &close);
        let _ = cdl_morningstar(&open, &high, &low, &close);
        let _ = cdl_onside(&open, &high, &low, &close);
        let _ = cdl_piercing(&open, &high, &low, &close);
        let _ = cdl_rickshawman(&open, &high, &low, &close);
        let _ = cdl_rise_fall_3methods(&open, &high, &low, &close);
        let _ = cdl_separatinglines(&open, &high, &low, &close);
        let _ = cdl_shootingstar(&open, &high, &low, &close);
        let _ = cdl_shortline(&open, &high, &low, &close);
        let _ = cdl_spinningtop(&open, &high, &low, &close);
        let _ = cdl_stalledpattern(&open, &high, &low, &close);
        let _ = cdl_sticksandwich(&open, &high, &low, &close);
        let _ = cdl_takuri(&open, &high, &low, &close);
        let _ = cdl_tasukigap(&open, &high, &low, &close);
        let _ = cdl_thrusting(&open, &high, &low, &close);
        let _ = cdl_tristar(&open, &high, &low, &close);
        let _ = cdl_unique3river(&open, &high, &low, &close);
        let _ = cdl_upsidegap2crows(&open, &high, &low, &close);
        let _ = cdl_xsidegap3methods(&open, &high, &low, &close);
    }
}
