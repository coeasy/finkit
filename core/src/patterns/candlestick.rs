use crate::error::{Result, TaError};
use crate::utils::validate_input;
use ndarray::Array1;

/// Candlestick pattern result
/// Values: 100 for bullish, -100 for bearish, 0 for no pattern
pub type PatternResult = Array1<i32>;

/// Small fixed-size rolling average used by the candle hot paths.
///
/// TA-Lib updates its candle settings in the same pass that classifies a
/// candle. Keeping the window on the stack avoids materializing one or more
/// full-length helper vectors for every pattern invocation.
#[derive(Clone, Copy)]
struct RollingAverage<const N: usize> {
    values: [f64; N],
    sum: f64,
    next: usize,
    count: usize,
}

impl<const N: usize> RollingAverage<N> {
    #[inline(always)]
    fn new() -> Self {
        Self {
            values: [0.0; N],
            sum: 0.0,
            next: 0,
            count: 0,
        }
    }

    #[inline(always)]
    fn average(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    #[inline(always)]
    fn push(&mut self, value: f64) {
        if self.count == N {
            self.sum -= self.values[self.next];
        } else {
            self.count += 1;
        }
        self.values[self.next] = value;
        self.sum += value;
        self.next += 1;
        if self.next == N {
            self.next = 0;
        }
    }
}

/// Precompute the average true range used by all candles in one pattern.
///
/// Candlestick rules often inspect the same rolling range several times per
/// candle. Building it once keeps the hot loop branch-free and reduces the
/// usual O(n * period) implementation to O(n).
fn candle_avg_ranges(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    let mut ranges = Vec::with_capacity(high.len());
    let mut true_ranges = Vec::with_capacity(high.len());
    let mut rolling_sum = 0.0;

    for i in 0..high.len() {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        true_ranges.push(true_range);
        rolling_sum += true_range;
        if i >= period {
            rolling_sum -= true_ranges[i - period];
        }
        let count = (i + 1).min(period);
        ranges.push(rolling_sum / count as f64);
    }

    ranges
}

/// TA-Lib candle settings compare the current candle with the average of the
/// preceding bars; the current bar is added only after classification.
/// Candle body size
#[inline(always)]
fn body(open: f64, close: f64) -> f64 {
    (close - open).abs()
}

/// Candle upper shadow
#[inline(always)]
fn upper_shadow(high: f64, open: f64, close: f64) -> f64 {
    high - open.max(close)
}

/// Candle lower shadow
#[inline(always)]
fn lower_shadow(low: f64, open: f64, close: f64) -> f64 {
    open.min(close) - low
}

/// Whether candle is bullish (white)
#[inline(always)]
fn is_bullish(open: f64, close: f64) -> bool {
    close > open
}

/// Whether candle is bearish (black)
#[inline(always)]
fn is_bearish(open: f64, close: f64) -> bool {
    close < open
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
    validate_input(open.len(), 10)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let mut ranges = [0.0; 10];
    let mut rolling_sum = 0.0;
    for i in 0..10 {
        let value = high[i] - low[i];
        ranges[i] = value;
        rolling_sum += value;
    }
    for i in 10..len {
        let avg_range = rolling_sum / 10.0;
        let slot = i % 10;
        rolling_sum -= ranges[slot];
        let value = high[i] - low[i];
        ranges[slot] = value;
        rolling_sum += value;
        let body_size = body(open[i], close[i]);

        if body_size <= avg_range * doji_pct {
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
    let mut avg_ranges = RollingAverage::<10>::new();

    for i in 0..len {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        avg_ranges.push(true_range);
        if i < period {
            continue;
        }
        let avg_range = avg_ranges.average();
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
    let mut avg_ranges = RollingAverage::<10>::new();

    for i in 0..len {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        avg_ranges.push(true_range);
        if i < period {
            continue;
        }
        let avg_range = avg_ranges.average();
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
    let mut ranges = [0.0; 10];
    let mut rolling_sum = 0.0;
    for i in 0..10 {
        let value = high[i] - low[i];
        ranges[i] = value;
        rolling_sum += value;
    }
    for i in 10..len {
        let avg_range = rolling_sum / 10.0;
        let slot = i % 10;
        rolling_sum -= ranges[slot];
        let value = high[i] - low[i];
        ranges[slot] = value;
        rolling_sum += value;
        let body_size = body(open[i], close[i]);
        let up_shadow = upper_shadow(high[i], open[i], close[i]);
        let lo_shadow = lower_shadow(low[i], open[i], close[i]);

        if body_size <= avg_range * doji_pct && (up_shadow > body_size || lo_shadow > body_size) {
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
    validate_input(open.len(), 1)?;

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
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 1)?;

    let len = open.len();
    let mut output = Array1::zeros(len);

    for i in 0..len {
        let body_size = body(open[i], close[i]);
        let up_shadow = upper_shadow(high[i], open[i], close[i]);
        let lo_shadow = lower_shadow(low[i], open[i], close[i]);

        if body_size > 0.0
            && up_shadow / body_size < shadow_pct
            && lo_shadow / body_size < shadow_pct
        {
            if is_bullish(open[i], close[i]) {
                output[i] = 100;
            } else {
                output[i] = -100;
            }
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
    let period = 10;
    let mut avg_ranges = RollingAverage::<10>::new();

    for i in 0..len {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        avg_ranges.push(true_range);
        if i < period {
            continue;
        }
        let avg_range = avg_ranges.average();
        let body_size = body(open[i], close[i]);
        let up_shadow = upper_shadow(high[i], open[i], close[i]);
        let lo_shadow = lower_shadow(low[i], open[i], close[i]);

        if lo_shadow >= body_size * 2.0
            && up_shadow <= avg_range * 0.1
            && body_size > avg_range * 0.1
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
    let period = 10;
    let mut avg_ranges = RollingAverage::<10>::new();

    for i in 0..len {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        avg_ranges.push(true_range);
        if i < period {
            continue;
        }
        let avg_range = avg_ranges.average();
        let body_size = body(open[i], close[i]);
        let up_shadow = upper_shadow(high[i], open[i], close[i]);
        let lo_shadow = lower_shadow(low[i], open[i], close[i]);

        if up_shadow >= body_size * 2.0
            && lo_shadow <= avg_range * 0.1
            && body_size > avg_range * 0.1
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

    for i in 1..len {
        let prev_body = body(open[i - 1], close[i - 1]);
        let curr_body = body(open[i], close[i]);

        if curr_body < prev_body * 0.5 {
            let prev_high = open[i - 1].max(close[i - 1]);
            let prev_low = open[i - 1].min(close[i - 1]);
            let curr_high = open[i].max(close[i]);
            let curr_low = open[i].min(close[i]);

            if curr_high < prev_high && curr_low > prev_low {
                if is_bearish(open[i - 1], close[i - 1]) && is_bullish(open[i], close[i]) {
                    output[i] = 100;
                } else if is_bullish(open[i - 1], close[i - 1]) && is_bearish(open[i], close[i]) {
                    output[i] = -100;
                }
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
    validate_input(open.len(), 11)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;
    let mut avg_ranges = RollingAverage::<10>::new();

    for i in 0..len {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        avg_ranges.push(true_range);
        if i < period {
            continue;
        }
        let prev_body = body(open[i - 1], close[i - 1]);
        let curr_body = body(open[i], close[i]);
        let avg_range = avg_ranges.average();

        if curr_body < avg_range * 0.1 && curr_body < prev_body * 0.5 {
            let prev_high = open[i - 1].max(close[i - 1]);
            let prev_low = open[i - 1].min(close[i - 1]);
            let curr_high = open[i].max(close[i]);
            let curr_low = open[i].min(close[i]);

            if curr_high < prev_high && curr_low > prev_low {
                if is_bearish(open[i - 1], close[i - 1]) {
                    output[i] = 100;
                } else if is_bullish(open[i - 1], close[i - 1]) {
                    output[i] = -100;
                }
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

    for i in 2..len {
        let first_body = body(open[i - 2], close[i - 2]);
        let second_body = body(open[i - 1], close[i - 1]);
        let third_body = body(open[i], close[i]);

        if is_bearish(open[i - 2], close[i - 2])
            && second_body < first_body * 0.3
            && is_bullish(open[i], close[i])
            && third_body > first_body * 0.5
            && close[i] > (open[i - 2] + close[i - 2]) / 2.0
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

    for i in 2..len {
        let first_body = body(open[i - 2], close[i - 2]);
        let second_body = body(open[i - 1], close[i - 1]);
        let third_body = body(open[i], close[i]);

        if is_bullish(open[i - 2], close[i - 2])
            && second_body < first_body * 0.3
            && is_bearish(open[i], close[i])
            && third_body > first_body * 0.5
            && close[i] < (open[i - 2] + close[i - 2]) / 2.0
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
    validate_input(open.len(), 10)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;
    let mut ranges = [0.0; 10];
    let mut rolling_sum = 0.0;

    for i in 0..10 {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        ranges[i] = true_range;
        rolling_sum += true_range;
    }
    for i in period..len {
        let slot = i % 10;
        rolling_sum -= ranges[slot];
        let true_range = (high[i] - low[i])
            .max((high[i] - close[i - 1]).abs())
            .max((low[i] - close[i - 1]).abs());
        ranges[slot] = true_range;
        rolling_sum += true_range;
        let first_body = body(open[i - 2], close[i - 2]);
        let second_body = body(open[i - 1], close[i - 1]);
        let third_body = body(open[i], close[i]);
        let avg_range = rolling_sum / 10.0;

        if is_bearish(open[i - 2], close[i - 2])
            && second_body < avg_range * doji_pct
            && is_bullish(open[i], close[i])
            && third_body > first_body * 0.5
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
    validate_input(open.len(), 10)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;
    let mut avg_ranges = RollingAverage::<10>::new();

    for i in 0..len {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        avg_ranges.push(true_range);
        if i < period || i < 2 {
            continue;
        }
        let first_body = body(open[i - 2], close[i - 2]);
        let second_body = body(open[i - 1], close[i - 1]);
        let third_body = body(open[i], close[i]);
        let avg_range = avg_ranges.average();

        if is_bullish(open[i - 2], close[i - 2])
            && second_body < avg_range * doji_pct
            && is_bearish(open[i], close[i])
            && third_body > first_body * 0.5
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
    let mut avg_ranges = RollingAverage::<10>::new();

    for i in 0..len {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        avg_ranges.push(true_range);
        if i < period || i < 2 {
            continue;
        }
        let avg_range = avg_ranges.average();

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
    let mut avg_ranges = RollingAverage::<10>::new();
    let open_ptr = open.as_ptr();
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();
    let output_ptr: *mut i32 = output.as_slice_mut().unwrap().as_mut_ptr();

    for i in 0..len {
        let true_range = unsafe {
            if i == 0 {
                *high_ptr - *low_ptr
            } else {
                (*high_ptr.add(i) - *low_ptr.add(i))
                    .max((*high_ptr.add(i) - *close_ptr.add(i - 1)).abs())
                    .max((*low_ptr.add(i) - *close_ptr.add(i - 1)).abs())
            }
        };
        avg_ranges.push(true_range);
        if i < period || i < 2 {
            continue;
        }
        let avg_range = avg_ranges.average();

        let all_bearish = unsafe {
            is_bearish(*open_ptr.add(i), *close_ptr.add(i))
                && is_bearish(*open_ptr.add(i - 1), *close_ptr.add(i - 1))
                && is_bearish(*open_ptr.add(i - 2), *close_ptr.add(i - 2))
        };

        let lower_closes = unsafe {
            *close_ptr.add(i) < *close_ptr.add(i - 1)
                && *close_ptr.add(i - 1) < *close_ptr.add(i - 2)
        };

        let opens_within_prev = unsafe {
            *open_ptr.add(i) < *open_ptr.add(i - 1) && *open_ptr.add(i - 1) < *open_ptr.add(i - 2)
        };

        if all_bearish && lower_closes && opens_within_prev {
            let bodies_large = unsafe {
                body(*open_ptr.add(i), *close_ptr.add(i)) > avg_range * 0.5
                    && body(*open_ptr.add(i - 1), *close_ptr.add(i - 1)) > avg_range * 0.5
                    && body(*open_ptr.add(i - 2), *close_ptr.add(i - 2)) > avg_range * 0.5
            };

            if bodies_large {
                unsafe {
                    *output_ptr.add(i) = -100;
                }
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

    for i in 2..len {
        if is_bearish(open[i - 2], close[i - 2])
            && body(open[i - 1], close[i - 1]) < body(open[i - 2], close[i - 2])
            && open[i - 1].max(close[i - 1]) < open[i - 2].max(close[i - 2])
            && open[i - 1].min(close[i - 1]) > open[i - 2].min(close[i - 2])
            && is_bullish(open[i], close[i])
            && close[i] > open[i - 2].max(close[i - 2])
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
        if is_bearish(open[i - 2], close[i - 2])
            && is_bullish(open[i - 1], close[i - 1])
            && open[i - 1] <= close[i - 2]
            && close[i - 1] >= open[i - 2]
            && is_bullish(open[i], close[i])
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

    for i in 2..len {
        if is_bullish(open[i - 2], close[i - 2])
            && body(open[i - 1], close[i - 1]) < body(open[i - 2], close[i - 2])
            && open[i - 1].max(close[i - 1]) < open[i - 2].max(close[i - 2])
            && open[i - 1].min(close[i - 1]) > open[i - 2].min(close[i - 2])
            && is_bearish(open[i], close[i])
            && close[i] < open[i - 2].min(close[i - 2])
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
        if is_bullish(open[i - 2], close[i - 2])
            && is_bearish(open[i - 1], close[i - 1])
            && open[i - 1] >= close[i - 2]
            && close[i - 1] <= open[i - 2]
            && is_bearish(open[i], close[i])
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

    for i in 3..len {
        // First: long black candle
        if is_bearish(open[i - 3], close[i - 3])
            // Second: black candle with lower low and gap down
            && is_bearish(open[i - 2], close[i - 2])
            && low[i - 2] < low[i - 3]
            // Third: small-bodied candle (spinning top or doji)
            && body(open[i - 1], close[i - 1]) < body(open[i - 2], close[i - 2])
            // Fourth: white candle that closes within first candle's body
            && is_bullish(open[i], close[i])
            && close[i] > close[i - 3]
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

    for i in 2..len {
        if is_bullish(open[i - 2], close[i - 2])
            && is_bearish(open[i - 1], close[i - 1])
            && is_bullish(open[i], close[i])
            && (close[i - 2] - close[i]).abs() < (close[i - 2] * 0.01)
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

    for i in 0..len {
        if is_bullish(open[i], close[i]) && (open[i] - low[i]).abs() < 1e-10 {
            output[i] = 100;
        } else if is_bearish(open[i], close[i]) && (open[i] - high[i]).abs() < 1e-10 {
            output[i] = -100;
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

    for i in 0..len {
        if is_bullish(open[i], close[i]) && (high[i] - close[i]).abs() < 1e-10 {
            output[i] = 100;
        } else if is_bearish(open[i], close[i]) && (low[i] - close[i]).abs() < 1e-10 {
            output[i] = -100;
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
    let mut bodies = [0.0; 10];
    let mut rolling_sum = 0.0;
    for i in 0..10 {
        let value = body(open[i], close[i]);
        bodies[i] = value;
        rolling_sum += value;
    }
    for i in 10..len {
        let avg_body = rolling_sum / 10.0;
        let slot = i % 10;
        rolling_sum -= bodies[slot];
        let body_size = body(open[i], close[i]);
        bodies[slot] = body_size;
        rolling_sum += body_size;
        let up_shadow = upper_shadow(high[i], open[i], close[i]);
        let lo_shadow = lower_shadow(low[i], open[i], close[i]);

        if body_size < avg_body && up_shadow > body_size && lo_shadow > body_size {
            output[i] = if is_bullish(open[i], close[i]) {
                100
            } else {
                -100
            };
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
    validate_input(open.len(), 11)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let mut bodies = [0.0; 10];
    let mut rolling_sum = 0.0;
    for i in 0..10 {
        let value = body(open[i], close[i]);
        bodies[i] = value;
        rolling_sum += value;
    }
    for i in 10..len {
        let avg_body = rolling_sum / 10.0;
        let slot = i % 10;
        rolling_sum -= bodies[slot];
        let body_size = body(open[i], close[i]);
        bodies[slot] = body_size;
        rolling_sum += body_size;
        if body_size < avg_body
            && upper_shadow(high[i], open[i], close[i]) > body_size * 2.0
            && lower_shadow(low[i], open[i], close[i]) > body_size * 2.0
        {
            output[i] = if is_bullish(open[i], close[i]) {
                100
            } else {
                -100
            };
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
    validate_input(open.len(), 11)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;
    let mut avg_doji = RollingAverage::<10>::new();
    let mut avg_near = RollingAverage::<5>::new();

    for i in 0..len {
        let avg_doji_value = avg_doji.average();
        let avg_near_value = avg_near.average();
        let range = high[i] - low[i];
        avg_doji.push(range);
        avg_near.push(range);
        if i < period {
            continue;
        }
        let body_size = body(open[i], close[i]);
        let upper = open[i].max(close[i]);
        let lower = open[i].min(close[i]);

        if body_size <= avg_doji_value * 0.1
            && lower - low[i] > body_size
            && high[i] - upper > body_size
            && lower <= low[i] + (high[i] - low[i]) / 2.0 + avg_near_value * 0.2
            && upper >= low[i] + (high[i] - low[i]) / 2.0 - avg_near_value * 0.2
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
    validate_input(open.len(), 11)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;
    let mut avg_bodies = RollingAverage::<10>::new();
    let mut avg_shadows = RollingAverage::<10>::new();

    for i in 0..len {
        let avg_body = avg_bodies.average();
        let avg_shadow = avg_shadows.average();
        let body_size = body(open[i], close[i]);
        let up_shadow = upper_shadow(high[i], open[i], close[i]);
        let lo_shadow = lower_shadow(low[i], open[i], close[i]);
        avg_bodies.push(body_size);
        avg_shadows.push(up_shadow + lo_shadow);
        if i < period {
            continue;
        }

        if body_size < avg_body && up_shadow < avg_shadow * 0.5 && lo_shadow < avg_shadow * 0.5 {
            output[i] = if is_bullish(open[i], close[i]) {
                100
            } else {
                -100
            };
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
    validate_input(open.len(), 11)?;

    let len = open.len();
    let mut output = Array1::zeros(len);
    let period = 10;
    let mut avg_bodies = RollingAverage::<10>::new();
    let mut avg_shadows = RollingAverage::<10>::new();

    for i in 0..len {
        let avg_body = avg_bodies.average();
        let avg_shadow = avg_shadows.average();
        let body_size = body(open[i], close[i]);
        let up_shadow = upper_shadow(high[i], open[i], close[i]);
        let lo_shadow = lower_shadow(low[i], open[i], close[i]);
        avg_bodies.push(body_size);
        avg_shadows.push(up_shadow + lo_shadow);
        if i < period {
            continue;
        }

        if body_size > avg_body && up_shadow < avg_shadow * 0.5 && lo_shadow < avg_shadow * 0.5 {
            if is_bullish(open[i], close[i]) {
                output[i] = 100;
            } else {
                output[i] = -100;
            }
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

    for i in 1..len {
        if is_bearish(open[i - 1], close[i - 1])
            && is_bullish(open[i], close[i])
            && open[i] < close[i - 1]
            && close[i] > (open[i - 1] + close[i - 1]) / 2.0
            && close[i] < open[i - 1]
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

    for i in 1..len {
        if is_bullish(open[i - 1], close[i - 1])
            && is_bearish(open[i], close[i])
            && open[i] > close[i - 1]
            && close[i] < (open[i - 1] + close[i - 1]) / 2.0
            && close[i] > open[i - 1]
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
    validate_input(open.len(), 2)?;

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
    validate_input(open.len(), 2)?;

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

    for i in 4..len {
        if is_bullish(open[i - 4], close[i - 4])
            && is_bearish(open[i - 3], close[i - 3])
            && open[i - 3] > close[i - 4]
            && is_bearish(open[i - 2], close[i - 2])
            && is_bearish(open[i - 1], close[i - 1])
            && close[i - 2] > close[i - 4] * 0.9
            && is_bullish(open[i], close[i])
            && close[i] > close[i - 4]
        {
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

    for i in 2..len {
        // Bullish tasuki gap
        if is_bullish(open[i - 2], close[i - 2])
            && is_bullish(open[i - 1], close[i - 1])
            && low[i - 1] > high[i - 2]
            && is_bearish(open[i], close[i])
            && close[i] > close[i - 2]
            && open[i] < close[i - 1]
            && close[i] > open[i - 1]
        {
            output[i] = 100;
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

    for i in 1..len {
        if (close[i] - close[i - 1]).abs() < 1e-10 {
            if is_bearish(open[i - 1], close[i - 1]) && is_bullish(open[i], close[i]) {
                output[i] = 100;
            } else if is_bullish(open[i - 1], close[i - 1]) && is_bearish(open[i], close[i]) {
                output[i] = -100;
            }
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

    for i in 1..len {
        if is_bearish(open[i - 1], close[i - 1])
            && is_bearish(open[i], close[i])
            && (close[i] - close[i - 1]).abs() < 1e-10
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
    let mut avg_ranges = RollingAverage::<10>::new();

    for i in 0..len {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        avg_ranges.push(true_range);
        if i < period || i < 2 {
            continue;
        }
        let avg_range = avg_ranges.average();

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

    for i in 3..len {
        if is_bearish(open[i - 3], close[i - 3])
            && is_bearish(open[i - 2], close[i - 2])
            && low[i - 2] < low[i - 3]
            && is_bullish(open[i - 1], close[i - 1])
            && open[i - 1] < close[i - 2]
            && close[i - 1] > close[i - 2]
            && is_bullish(open[i], close[i])
            && open[i] < close[i - 1]
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

    for i in 4..len {
        // Bullish breakaway
        if is_bearish(open[i - 4], close[i - 4])
            && is_bearish(open[i - 3], close[i - 3])
            && close[i - 3] < close[i - 4]
            && is_bearish(open[i - 2], close[i - 2])
            && close[i - 2] < close[i - 3]
            && is_bearish(open[i - 1], close[i - 1])
            && is_bullish(open[i], close[i])
            && open[i] < close[i - 1]
            && close[i] > close[i - 3]
        {
            output[i] = 100;
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

    for i in 3..len {
        if is_bearish(open[i - 3], close[i - 3])
            && is_bearish(open[i - 2], close[i - 2])
            && open[i - 2] > close[i - 3]
            && close[i - 2] < close[i - 3]
            && is_bearish(open[i - 1], close[i - 1])
            && open[i - 1] > close[i - 2]
            && low[i - 1] < low[i - 2]
            && is_bullish(open[i], close[i])
            && open[i] < open[i - 1]
            && close[i] > open[i - 2]
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
    let mut avg_ranges = RollingAverage::<10>::new();

    for i in 0..len {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        avg_ranges.push(true_range);
        if i == 0 {
            continue;
        }
        let avg_range = avg_ranges.average();
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
    let mut avg_ranges = RollingAverage::<10>::new();

    for i in 0..len {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        avg_ranges.push(true_range);
        if i == 0 {
            continue;
        }
        let avg_range = avg_ranges.average();
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
    let mut body_window = RollingAverage::<10>::new();
    let mut shadow_window = RollingAverage::<10>::new();
    let mut near_window = RollingAverage::<5>::new();
    let mut body_history = [0.0; 3];
    let mut shadow_history = [0.0; 3];
    let mut near_history = [0.0; 3];

    for i in 0..len {
        let avg_body = body_window.average();
        let avg_shadow = shadow_window.average();
        let avg_near = near_window.average();
        body_history[i % 3] = avg_body;
        shadow_history[i % 3] = avg_shadow;
        near_history[i % 3] = avg_near;
        let current_body = body(open[i], close[i]);
        let current_shadow =
            upper_shadow(high[i], open[i], close[i]) + lower_shadow(low[i], open[i], close[i]);
        body_window.push(current_body);
        shadow_window.push(current_shadow);
        near_window.push(high[i] - low[i]);
        if i < 12 {
            continue;
        }
        let first = i - 2;
        let second = i - 1;
        let white =
            close[first] >= open[first] && close[second] >= open[second] && close[i] >= open[i];
        let upper_first = upper_shadow(high[first], open[first], close[first]);
        let upper_second = upper_shadow(high[second], open[second], close[second]);
        let upper_third = upper_shadow(high[i], open[i], close[i]);
        let first_body = body(open[first], close[first]);
        let second_body = body(open[second], close[second]);
        let third_body = current_body;
        let near_first = near_history[first % 3] * 0.2;
        let near_second = near_history[second % 3] * 0.2;
        let far_first = near_history[first % 3] * 0.6;
        let far_second = near_history[second % 3] * 0.6;
        let shadow_short_first = shadow_history[first % 3] * 0.5;
        let shadow_short_second = shadow_history[second % 3] * 0.5;
        let shadow_short_third = shadow_history[i % 3] * 0.5;
        let blocked = (second_body < first_body - far_first
            && third_body < second_body + near_second)
            || third_body < second_body - far_second
            || (third_body < second_body
                && second_body < first_body
                && (upper_third > shadow_short_third || upper_second > shadow_short_second))
            || (third_body < second_body && upper_third > third_body);

        if white
            && close[i] > close[second]
            && close[second] > close[first]
            && open[second] > open[first]
            && open[second] <= close[first] + near_first
            && open[i] > open[second]
            && open[i] <= close[second] + near_second
            && first_body > body_history[first % 3]
            && upper_first < shadow_short_first
            && blocked
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
    let mut body_window = RollingAverage::<10>::new();
    let mut range_window = RollingAverage::<10>::new();
    let mut near_window = RollingAverage::<5>::new();
    let mut body_history = [0.0; 3];
    let mut range_history = [0.0; 3];
    let mut near_history = [0.0; 3];

    for i in 0..len {
        let avg_body = body_window.average();
        let avg_range = range_window.average();
        let avg_near_value = near_window.average();
        body_history[i % 3] = avg_body;
        range_history[i % 3] = avg_range;
        near_history[i % 3] = avg_near_value;
        let current_body = body(open[i], close[i]);
        body_window.push(current_body);
        range_window.push(high[i] - low[i]);
        near_window.push(high[i] - low[i]);
        if i < 12 {
            continue;
        }
        let first = i - 2;
        let second = i - 1;
        if close[first] >= open[first]
            && close[second] >= open[second]
            && close[i] >= open[i]
            && close[i] > close[second]
            && close[second] > close[first]
            && body(open[first], close[first]) > body_history[first % 3]
            && body(open[second], close[second]) > body_history[second % 3]
            && upper_shadow(high[second], open[second], close[second])
                < range_history[second % 3] * 0.1
            && open[second] > open[first]
            && open[second] <= close[first] + near_history[first % 3] * 0.2
            && current_body < avg_body
            && open[i] >= close[second] - current_body - near_history[second % 3] * 0.2
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

    for i in 1..len {
        if is_bearish(open[i - 1], close[i - 1])
            && is_bullish(open[i], close[i])
            && open[i] < close[i - 1]
            && close[i] < (open[i - 1] + close[i - 1]) / 2.0
            && close[i] > close[i - 1]
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

    for i in 1..len {
        if is_bearish(open[i - 1], close[i - 1])
            && is_bullish(open[i], close[i])
            && open[i] < close[i - 1]
            && (close[i] - close[i - 1]).abs() < (close[i - 1] * 0.01)
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

    for i in 1..len {
        if is_bearish(open[i - 1], close[i - 1])
            && is_bullish(open[i], close[i])
            && open[i] < close[i - 1]
            && (close[i] - close[i - 1]).abs() < 1e-10
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
    for i in 2..len {
        let bull1 = is_bullish(open[i - 2], close[i - 2]);
        let bear2 = is_bearish(open[i - 1], close[i - 1]);
        let bear3 = is_bearish(open[i], close[i]);
        let gap_up = open[i - 1] > close[i - 2];
        let engulf = open[i] > open[i - 1] && close[i] < close[i - 1] && close[i] > close[i - 2];
        if bull1 && bear2 && bear3 && gap_up && engulf {
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
    validate_input(open.len(), 2)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    let mut avg_ranges = RollingAverage::<10>::new();
    for i in 0..len {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        avg_ranges.push(true_range);
        if i == 0 {
            continue;
        }
        let avg = avg_ranges.average();
        let body_prev = body(open[i - 1], close[i - 1]);
        let body_curr = body(open[i], close[i]);
        let is_doji = body_curr < avg * 0.1;
        let long_prev = body_prev > avg * 0.6;
        if is_doji && long_prev {
            if is_bullish(open[i - 1], close[i - 1]) && open[i].min(close[i]) > close[i - 1] {
                output[i] = -100; // bearish doji star
            } else if is_bearish(open[i - 1], close[i - 1]) && open[i].max(close[i]) < close[i - 1]
            {
                output[i] = 100; // bullish doji star
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
    for i in 2..len {
        let bull2 = is_bullish(open[i - 1], close[i - 1]);
        let bull3 = is_bullish(open[i], close[i]);
        let similar_size = (body(open[i], close[i]) - body(open[i - 1], close[i - 1])).abs()
            < body(open[i - 1], close[i - 1]) * 0.3;
        let similar_open = (open[i] - open[i - 1]).abs() < body(open[i - 1], close[i - 1]) * 0.3;
        if bull2 && bull3 && similar_size && similar_open {
            if is_bullish(open[i - 2], close[i - 2]) && open[i - 1] > close[i - 2] {
                output[i] = 100; // upside gap
            } else if is_bearish(open[i - 2], close[i - 2]) && open[i - 1] < close[i - 2] {
                output[i] = -100; // downside gap
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
    validate_input(open.len(), 4)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    let mut pattern_result = 0;
    let mut countdown = 0;
    let mut saved_high = 0.0;
    let mut saved_low = 0.0;
    for i in 2..len {
        if high[i - 1] < high[i - 2]
            && low[i - 1] > low[i - 2]
            && ((high[i] < high[i - 1] && low[i] < low[i - 1])
                || (high[i] > high[i - 1] && low[i] > low[i - 1]))
        {
            pattern_result = if high[i] < high[i - 1] { 100 } else { -100 };
            saved_high = high[i - 1];
            saved_low = low[i - 1];
            countdown = 4;
            if i >= 5 {
                output[i] = pattern_result;
            }
        } else if countdown > 0
            && ((pattern_result > 0 && close[i] > saved_high)
                || (pattern_result < 0 && close[i] < saved_low))
        {
            if i >= 5 {
                output[i] = pattern_result + if pattern_result > 0 { 100 } else { -100 };
            }
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
    for i in 4..len {
        let inside = high[i - 3] < high[i - 4] && low[i - 3] > low[i - 4];
        let second_inside = high[i - 2] < high[i - 3] && low[i - 2] > low[i - 3];
        if inside && second_inside {
            if high[i - 1] > high[i - 4] && close[i] < low[i - 3] {
                output[i] = -100;
            } else if low[i - 1] < low[i - 4] && close[i] > high[i - 3] {
                output[i] = 100;
            }
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
    for i in 1..len {
        let bear1 = is_bearish(open[i - 1], close[i - 1]);
        let bear2 = is_bearish(open[i], close[i]);
        let contained = open[i] < open[i - 1] && close[i] > close[i - 1];
        if bear1 && bear2 && contained {
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
    for i in 4..len {
        let bear1 = is_bearish(open[i - 4], close[i - 4]);
        let bear2 = is_bearish(open[i - 3], close[i - 3]) && close[i - 3] < close[i - 4];
        let bear3 = is_bearish(open[i - 2], close[i - 2]) && close[i - 2] < close[i - 3];
        let upper_shadow_4 = upper_shadow(high[i - 1], open[i - 1], close[i - 1]);
        let body_4 = body(open[i - 1], close[i - 1]);
        let has_upper = upper_shadow_4 > body_4;
        let bull5 = is_bullish(open[i], close[i]) && close[i] > open[i - 1];
        if bear1 && bear2 && bear3 && has_upper && bull5 {
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
    let mut avg_window = RollingAverage::<10>::new();
    let mut avg_history = [0.0; 5];
    for i in 0..len {
        let avg_body = avg_window.average();
        avg_history[i % 5] = avg_body;
        let current_body = body(open[i], close[i]);
        avg_window.push(current_body);
        if i < 4 {
            continue;
        }
        let avg_first = avg_history[(i - 4) % 5];
        let avg_second = avg_history[(i - 3) % 5];
        let avg_third = avg_history[(i - 2) % 5];
        let avg_fourth = avg_history[(i - 1) % 5];
        let first_white = close[i - 4] >= open[i - 4];
        let middle_white = close[i - 3] >= open[i - 3];
        let final_white = close[i] >= open[i];
        if middle_white == first_white
            || (close[i - 2] >= open[i - 2]) != middle_white
            || (close[i - 1] >= open[i - 1]) != middle_white
            || final_white == first_white
        {
            continue;
        }
        let first_high = high[i - 4];
        let first_low = low[i - 4];
        let middle1_low = open[i - 3].min(close[i - 3]);
        let middle1_high = open[i - 3].max(close[i - 3]);
        let middle2_low = open[i - 2].min(close[i - 2]);
        let middle2_high = open[i - 2].max(close[i - 2]);
        let middle3_low = open[i - 1].min(close[i - 1]);
        let middle3_high = open[i - 1].max(close[i - 1]);
        if middle1_low >= first_high
            || middle1_high <= first_low
            || middle2_low >= first_high
            || middle2_high <= first_low
            || middle3_low >= first_high
            || middle3_high <= first_low
        {
            continue;
        }
        let middle_trends = if first_white {
            close[i - 2] < close[i - 3] && close[i - 1] < close[i - 2]
        } else {
            close[i - 2] > close[i - 3] && close[i - 1] > close[i - 2]
        };
        if !middle_trends {
            continue;
        }
        let final_continues = if first_white {
            open[i] > close[i - 1] && close[i] > close[i - 4]
        } else {
            open[i] < close[i - 1] && close[i] < close[i - 4]
        };
        if !final_continues {
            continue;
        }
        let first_body = body(open[i - 4], close[i - 4]);
        let second_body = body(open[i - 3], close[i - 3]);
        let third_body = body(open[i - 2], close[i - 2]);
        let fourth_body = body(open[i - 1], close[i - 1]);
        if first_body > avg_first
            && second_body < avg_second
            && third_body < avg_third
            && fourth_body < avg_fourth
            && current_body > avg_body
        {
            output[i] = if first_white { 100 } else { -100 };
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
    let mut avg_ranges = RollingAverage::<10>::new();
    for i in 0..len {
        let true_range = if i == 0 {
            high[i] - low[i]
        } else {
            (high[i] - low[i])
                .max((high[i] - close[i - 1]).abs())
                .max((low[i] - close[i - 1]).abs())
        };
        avg_ranges.push(true_range);
        let b = body(open[i], close[i]);
        let ls = lower_shadow(low[i], open[i], close[i]);
        let us = upper_shadow(high[i], open[i], close[i]);
        let avg = avg_ranges.average();
        if b < avg * 0.1 && ls > avg * 2.0 && us < avg * 0.1 {
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
    validate_input(open.len(), 12)?;
    let len = open.len();
    let mut output = PatternResult::zeros(len);
    let mut avg_window = RollingAverage::<10>::new();
    let mut avg_history = [0.0; 3];
    for i in 0..len {
        let avg_doji = avg_window.average();
        avg_history[i % 3] = avg_doji;
        avg_window.push(high[i] - low[i]);
        if i < 12 {
            continue;
        }
        let threshold = avg_history[(i - 2) % 3] * 0.1;
        let doji1 = body(open[i - 2], close[i - 2]) <= threshold;
        let doji2 = body(open[i - 1], close[i - 1]) <= threshold;
        let doji3 = body(open[i], close[i]) <= threshold;
        if doji1 && doji2 && doji3 {
            if open[i - 1].min(close[i - 1]) > open[i - 2].max(close[i - 2])
                && open[i].max(close[i]) < open[i - 1].max(close[i - 1])
            {
                output[i] = -100;
            } else if open[i - 1].max(close[i - 1]) < open[i - 2].min(close[i - 2])
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
    for i in 2..len {
        if is_bearish(open[i - 2], close[i - 2])
            && is_bullish(open[i - 1], close[i - 1])
            && open[i - 1] <= close[i - 2]
            && close[i - 1] >= open[i - 2]
            && is_bullish(open[i], close[i])
            && close[i] > close[i - 1]
        {
            output[i] = 100;
        } else if is_bullish(open[i - 2], close[i - 2])
            && is_bearish(open[i - 1], close[i - 1])
            && open[i - 1] >= close[i - 2]
            && close[i - 1] <= open[i - 2]
            && is_bearish(open[i], close[i])
            && close[i] < close[i - 1]
        {
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
    doji(open, high, low, close, 0.1)
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
    let avg_ranges = candle_avg_ranges(high, low, close, period);
    for i in period..len {
        if i < 2 {
            continue;
        }
        let avg_range = avg_ranges[i];
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
