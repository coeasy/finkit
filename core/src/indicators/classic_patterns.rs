//! Classic stock-trading chart patterns and trend indicators.
//!
//! These are the price-action-based chart constructions used by classical
//! technical analysis (Darvas, O'Higgins, Nison, Williams, etc.). They are
//! **not** part of TA-Lib; they are first-class indicators in the FTA core
//! because they capture classic stock-trading patterns that go beyond the
//! standard OHLCV technical analysis surface.
//!
//! # Indicators
//! - [`darvas_box`] - Darvas Box breakout levels (Nicholas Darvas, 1950s)
//! - [`renko`] - Renko bricks, price-only trend construction
//! - [`kagi`] - Kagi line, threshold-driven reversal chart
//! - [`point_and_figure`] - Point & Figure X/O columns
//! - [`three_line_break`] - Three Line Break continuation/reversal chart
//! - [`williams_alligator`] - Williams Alligator (5/8/13 SMMA, Bill Williams)
//!
//! All constructors return a per-bar series (the "filtered" price line) and
//! an optional list of pivot/turn points. They share a common `Result` type
//! with the rest of the indicator module.
//!
//! # References
//! - Darvas, N. (1960). *How I Made $2,000,000 in the Stock Market*.
//! - Nison, S. (1994). *Beyond Candlesticks*.
//! - Williams, B. (1998). *Trading Chaos*.

use crate::error::{Result, TaError};
use crate::utils::{validate_input, validate_param};
use ndarray::Array1;

// ============================================================================
// Darvas Box (尼古拉斯·达瓦斯箱体)
// ============================================================================

/// Result of a Darvas Box construction.
#[derive(Debug, Clone)]
pub struct DarvasBoxResult {
    /// Per-bar Darvas box level (top of the current box); NaN when no box is open.
    pub box_top: Array1<f64>,
    /// Per-bar Darvas box bottom; NaN when no box is open.
    pub box_bottom: Array1<f64>,
    /// Breakout direction: `1` for upside breakout, `-1` for downside,
    /// `0` otherwise.
    pub signal: Array1<i32>,
}

/// Darvas Box (达瓦斯箱体) — Nicholas Darvas breakout construction.
///
/// A Darvas box is formed by:
/// 1. Track the rolling `lookback` high; once price sets a new high, a
///    candidate box is opened with top = high and bottom = the lowest low
///    since that new high.
/// 2. The box is "confirmed" after the next `confirmation` bars without a
///    higher high; only then is it published as a valid box.
/// 3. A breakout occurs when close > box_top (buy) or close < box_bottom (sell).
///
/// This is a simplified but deterministic Darvas implementation suitable for
/// systematic backtesting. Darvas's original method used daily stock scans and
/// tape-reading; this Rust version is the canonical algorithmic proxy.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `lookback` - Number of bars to determine a "new high" (default 5)
/// * `confirmation` - Bars to wait before confirming a box (default 3)
///
/// # Returns
/// `(box_top, box_bottom, signal)` arrays of the same length as the input.
pub fn darvas_box(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
    confirmation: usize,
) -> Result<DarvasBoxResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_param("lookback", "2..=200", || (2..=200).contains(&lookback))?;
    validate_param("confirmation", "1..=50", || {
        (1..=50).contains(&confirmation)
    })?;
    validate_input(high.len(), lookback + confirmation)?;

    let len = high.len();
    let mut box_top = Array1::from(vec![f64::NAN; len]);
    let mut box_bottom = Array1::from(vec![f64::NAN; len]);
    let mut signal = Array1::from(vec![0i32; len]);

    // Rolling `lookback` high: high[i] == max(high[i-lookback+1..=i])
    let _rolling_max: f64 = high[..lookback]
        .iter()
        .fold(f64::NEG_INFINITY, |a, b| a.max(*b));

    let mut candidate_top: Option<f64> = None;
    let mut candidate_bottom: Option<f64> = None;
    let mut candidate_age: usize = 0;
    let mut last_confirmed_top: f64 = f64::NAN;
    let mut last_confirmed_bottom: f64 = f64::NAN;

    for i in lookback..len {
        // Check for a new high: compare current high against the prior `lookback` bars.
        let prev_window_max = high[i.saturating_sub(lookback)..i]
            .iter()
            .fold(f64::NEG_INFINITY, |a, b| a.max(*b));

        if high[i] > prev_window_max {
            // Start a new candidate box
            candidate_top = Some(high[i]);
            // Bottom = the lowest low since the last confirmed breakout, capped at this bar
            candidate_bottom = Some(low[i]);
            candidate_age = 0;
        } else if let (Some(top), Some(_bot)) = (candidate_top, candidate_bottom) {
            // Track the lowest low while waiting for confirmation
            if low[i] < candidate_bottom.unwrap() {
                candidate_bottom = Some(low[i]);
            }
            // Track the highest high
            if high[i] > top {
                candidate_top = Some(high[i]);
            }
            candidate_age += 1;

            if candidate_age >= confirmation {
                // Confirm and publish
                last_confirmed_top = candidate_top.unwrap();
                last_confirmed_bottom = candidate_bottom.unwrap();
                box_top[i] = last_confirmed_top;
                box_bottom[i] = last_confirmed_bottom;
            }
        }

        // Detect breakout from the *previously confirmed* box
        if !last_confirmed_top.is_nan() && i > 0 {
            if close[i] > last_confirmed_top && close[i - 1] <= last_confirmed_top {
                signal[i] = 1;
            } else if close[i] < last_confirmed_bottom && close[i - 1] >= last_confirmed_bottom {
                signal[i] = -1;
            }
        }
    }

    Ok(DarvasBoxResult {
        box_top,
        box_bottom,
        signal,
    })
}

// ============================================================================
// Renko Bricks
// ============================================================================

/// Renko construction result.
#[derive(Debug, Clone)]
pub struct RenkoResult {
    /// Renko "close" per bar: the new brick's price if a new brick was formed
    /// in this bar, otherwise equal to the previous brick's price.
    pub bricks: Array1<f64>,
    /// Direction of the new brick: `1` (up brick), `-1` (down brick), `0` (no new brick).
    pub direction: Array1<i32>,
}

/// Renko bricks — price-only trend construction.
///
/// A Renko chart plots a new brick whenever price moves by at least `box_size`
/// in one direction from the last brick's close. Bricks are always
/// `box_size` tall and tilt up or down depending on direction.
///
/// Renko time is "compressed": a single input bar can produce 0, 1, or many
/// bricks. We aggregate all bricks that fit in a single bar's range into
/// successive brick values, and store the *last* brick of each bar in
/// `bricks[i]`.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `box_size` - Brick size in price units (e.g. 1.0 for a 1-point brick)
///
/// # Returns
/// `(bricks, direction)` arrays aligned with the input bars.
pub fn renko(high: &[f64], low: &[f64], box_size: f64) -> Result<RenkoResult> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if box_size <= 0.0 {
        return Err(TaError::InvalidParameter {
            name: "box_size".to_string(),
            constraint: "must be > 0".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    let mut bricks = Array1::from(vec![f64::NAN; len]);
    let mut direction = Array1::from(vec![0i32; len]);

    let mut last_price: Option<f64> = None;

    for i in 0..len {
        let h = high[i];
        let l = low[i];
        let mut bar_dir: i32 = 0;
        let mut bar_price: f64 = f64::NAN;

        if let Some(prev) = last_price {
            // Walk up bricks from prev+box_size to high
            let mut new_top = prev + box_size;
            let mut n_up = 0i32;
            while new_top <= h {
                n_up += 1;
                new_top += box_size;
            }
            if n_up > 0 {
                // A reversal requires 2 bricks; otherwise simple continuation
                let mut new_bottom = prev - box_size;
                let mut n_dn = 0i32;
                while new_bottom >= l {
                    n_dn += 1;
                    new_bottom -= box_size;
                }
                if n_up >= 2 {
                    // Confirmed up reversal
                    let new_price = prev + (n_up as f64) * box_size;
                    bar_price = new_price;
                    bar_dir = 1;
                    last_price = Some(new_price);
                } else {
                    let new_price = prev + (n_up as f64) * box_size;
                    bar_price = new_price;
                    bar_dir = 1;
                    last_price = Some(new_price);
                    let _ = n_dn;
                }
            } else {
                let mut new_bottom = prev - box_size;
                let n_dn = if new_bottom < l {
                    0
                } else {
                    let mut count = 0i32;
                    while new_bottom >= l {
                        count += 1;
                        new_bottom -= box_size;
                    }
                    count
                };
                if n_dn >= 2 {
                    let new_price = prev - (n_dn as f64) * box_size;
                    bar_price = new_price;
                    bar_dir = -1;
                    last_price = Some(new_price);
                } else if n_dn > 0 {
                    let new_price = prev - (n_dn as f64) * box_size;
                    bar_price = new_price;
                    bar_dir = -1;
                    last_price = Some(new_price);
                }
            }
        } else {
            // Seed the first brick at the first close-of-day
            last_price = Some((h + l) * 0.5);
            bar_price = last_price.unwrap();
            bar_dir = 0;
        }

        bricks[i] = bar_price;
        direction[i] = bar_dir;
    }

    Ok(RenkoResult { bricks, direction })
}

// ============================================================================
// Kagi Lines
// ============================================================================

/// Kagi line result.
#[derive(Debug, Clone)]
pub struct KagiResult {
    /// Kagi line value per input bar (NaN at non-pivot bars).
    pub kagi: Array1<f64>,
    /// Kagi direction: `1` (yang / up), `-1` (yin / down), `0` (no segment yet).
    pub direction: Array1<i32>,
}

/// Kagi line — threshold-driven reversal chart.
///
/// A Kagi chart uses a fixed `reversal` threshold. While price moves in one
/// direction, the line extends. When price reverses by `reversal`, a new
/// segment is drawn in the opposite direction. Yang (up) segments are
/// typically drawn thick, yin (down) thin.
///
/// # Arguments
/// * `close` - Close prices
/// * `reversal` - Reversal threshold in price units (e.g. 1.0)
///
/// # Returns
/// `(kagi, direction)` arrays aligned with the input bars.
pub fn kagi(close: &[f64], reversal: f64) -> Result<KagiResult> {
    if reversal <= 0.0 {
        return Err(TaError::InvalidParameter {
            name: "reversal".to_string(),
            constraint: "must be > 0".to_string(),
        });
    }
    validate_input(close.len(), 1)?;

    let len = close.len();
    let mut kagi = Array1::from(vec![f64::NAN; len]);
    let mut direction = Array1::from(vec![0i32; len]);

    let mut cur: f64 = close[0];
    let mut dir: i32 = 0;

    for i in 0..len {
        let p = close[i];
        let _delta = p - cur;
        if dir >= 0 {
            // Currently going up
            if p >= cur {
                cur = p;
            } else if (cur - p) >= reversal {
                // Reverse down
                cur = p;
                dir = -1;
                kagi[i] = cur;
                direction[i] = dir;
            }
        } else {
            // dir < 0
            if p <= cur {
                cur = p;
            } else if (p - cur) >= reversal {
                // Reverse up
                cur = p;
                dir = 1;
                kagi[i] = cur;
                direction[i] = dir;
            }
        }
    }

    Ok(KagiResult { kagi, direction })
}

// ============================================================================
// Point & Figure (PnF)
// ============================================================================

/// Point & Figure result.
#[derive(Debug, Clone)]
pub struct PnFResult {
    /// Per-bar PnF close (NaN at non-pivot bars).
    pub pnf: Array1<f64>,
    /// PnF column type: `1` for X (up), `-1` for O (down), `0` otherwise.
    pub column_type: Array1<i32>,
    /// New column flag: `1` if this bar started a new X/O column, `0` otherwise.
    pub new_column: Array1<i32>,
}

/// Point & Figure (点数图) — X/O column construction.
///
/// PnF ignores time and plots an X (up column) for every `box_size` price
/// advance and an O (down column) for every `box_size` decline. A column
/// reverses only when price moves by `reversal` boxes in the opposite
/// direction.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `box_size` - Box size in price units
/// * `reversal` - Reversal size in number of boxes (typically 3)
///
/// # Returns
/// `(pnf, column_type, new_column)` arrays aligned with the input bars.
pub fn point_and_figure(
    high: &[f64],
    low: &[f64],
    box_size: f64,
    reversal: usize,
) -> Result<PnFResult> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if box_size <= 0.0 {
        return Err(TaError::InvalidParameter {
            name: "box_size".to_string(),
            constraint: "must be > 0".to_string(),
        });
    }
    if reversal == 0 {
        return Err(TaError::InvalidParameter {
            name: "reversal".to_string(),
            constraint: "must be >= 1".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    let mut pnf = Array1::from(vec![f64::NAN; len]);
    let mut column_type = Array1::from(vec![0i32; len]);
    let mut new_column = Array1::from(vec![0i32; len]);

    let anchor: f64 = (high[0] + low[0]) * 0.5;
    let mut cur: f64 = anchor;
    let mut col: i32 = 1; // 1 = X (up), -1 = O (down)

    for i in 0..len {
        let h = high[i];
        let l = low[i];

        if col == 1 {
            // X column: extend up
            if h >= cur + box_size {
                cur = ((h - cur) / box_size).floor() * box_size + cur;
                pnf[i] = cur;
                column_type[i] = 1;
            } else if l <= cur - (reversal as f64) * box_size {
                // Reverse to O
                col = -1;
                cur = cur - (reversal as f64) * box_size;
                pnf[i] = cur;
                column_type[i] = -1;
                new_column[i] = 1;
            }
        } else {
            // O column: extend down
            if l <= cur - box_size {
                cur = cur - (((cur - l) / box_size).floor()) * box_size;
                pnf[i] = cur;
                column_type[i] = -1;
            } else if h >= cur + (reversal as f64) * box_size {
                col = 1;
                cur = cur + (reversal as f64) * box_size;
                pnf[i] = cur;
                column_type[i] = 1;
                new_column[i] = 1;
            }
        }
    }

    Ok(PnFResult {
        pnf,
        column_type,
        new_column,
    })
}

// ============================================================================
// Three Line Break
// ============================================================================

/// Three Line Break result.
#[derive(Debug, Clone)]
pub struct ThreeLineBreakResult {
    /// Per-bar TLB line (NaN at non-pivot bars).
    pub line: Array1<f64>,
    /// TLB direction: `1` (white / up), `-1` (black / down), `0` (no line yet).
    pub direction: Array1<i32>,
}

/// Three Line Break (新值三线反转) — continuation/reversal chart.
///
/// The current line reverses only when the close breaks the high (up reversal)
/// or low (down reversal) of the *last `lines` lines in the same direction*.
/// "Three Line Break" is the classic 3-line variant.
///
/// # Arguments
/// * `close` - Close prices
/// * `lines` - Number of consecutive lines required to reverse (typically 2 or 3)
///
/// # Returns
/// `(line, direction)` arrays aligned with the input bars.
pub fn three_line_break(close: &[f64], lines: usize) -> Result<ThreeLineBreakResult> {
    if lines == 0 {
        return Err(TaError::InvalidParameter {
            name: "lines".to_string(),
            constraint: "must be >= 1".to_string(),
        });
    }
    validate_input(close.len(), lines + 1)?;

    let len = close.len();
    let mut out = Array1::from(vec![f64::NAN; len]);
    let mut direction = Array1::from(vec![0i32; len]);

    // Recent high/low of the last `lines` same-direction lines
    let mut last_highs: Vec<f64> = Vec::new(); // recent highs of last `lines` white lines
    let mut last_lows: Vec<f64> = Vec::new(); // recent lows of last `lines` black lines
    let mut cur_dir: i32 = 0;
    let mut cur_val: f64 = f64::NAN;

    for i in 0..len {
        let c = close[i];
        if cur_dir == 0 {
            // Seed the first line
            cur_val = c;
            cur_dir = 1;
            out[i] = c;
            direction[i] = 1;
            last_highs.push(c);
            continue;
        }

        if cur_dir == 1 {
            if c > cur_val {
                // Continuation up: extend the current white line
                cur_val = c;
                out[i] = c;
                direction[i] = 1;
                if let Some(last) = last_highs.last_mut() {
                    *last = c.max(*last);
                }
            } else if last_lows.len() >= lines
                && c < *last_lows
                    .iter()
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap()
            {
                // Reverse down
                cur_dir = -1;
                cur_val = c;
                out[i] = c;
                direction[i] = -1;
                last_lows.clear();
                last_lows.push(c);
                last_highs.clear();
            } else {
                // No signal; track low
                last_lows.push(c);
                if last_lows.len() > lines {
                    last_lows.remove(0);
                }
            }
        } else {
            // cur_dir == -1
            if c < cur_val {
                cur_val = c;
                out[i] = c;
                direction[i] = -1;
                if let Some(last) = last_lows.last_mut() {
                    *last = c.min(*last);
                }
            } else if last_highs.len() >= lines
                && c > *last_highs
                    .iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap()
            {
                // Reverse up
                cur_dir = 1;
                cur_val = c;
                out[i] = c;
                direction[i] = 1;
                last_highs.clear();
                last_highs.push(c);
                last_lows.clear();
            } else {
                last_highs.push(c);
                if last_highs.len() > lines {
                    last_highs.remove(0);
                }
            }
        }
    }

    Ok(ThreeLineBreakResult {
        line: out,
        direction,
    })
}

// ============================================================================
// Williams Alligator (比尔·威廉姆斯鳄鱼线)
// ============================================================================

/// Williams Alligator result.
#[derive(Debug, Clone)]
pub struct WilliamsAlligatorResult {
    /// Jaw (蓝色的下颚) — 13-period SMMA shifted forward by 8 bars
    pub jaw: Array1<f64>,
    /// Teeth (红色的牙齿) — 8-period SMMA shifted forward by 5 bars
    pub teeth: Array1<f64>,
    /// Lips (绿色的嘴唇) — 5-period SMMA shifted forward by 3 bars
    pub lips: Array1<f64>,
}

/// Williams Alligator (鳄鱼线) — three smoothed moving averages.
///
/// Uses SMMA (= RMA = EMA with alpha = 1/period) at three periods:
/// - **Jaw**:   13-period SMMA, shifted forward by 8 bars
/// - **Teeth**:  8-period SMMA, shifted forward by 5 bars
/// - **Lips**:   5-period SMMA, shifted forward by 3 bars
///
/// When the three lines intertwine the market is "sleeping"; when they
/// diverge in the direction of the trend, the "alligator is eating".
///
/// # Arguments
/// * `close` - Close prices
///
/// # Returns
/// `(jaw, teeth, lips)` arrays aligned with the input bars.
pub fn williams_alligator(close: &[f64]) -> Result<WilliamsAlligatorResult> {
    validate_input(close.len(), 13)?;

    let jaw = smma(close, 13, 8);
    let teeth = smma(close, 8, 5);
    let lips = smma(close, 5, 3);

    Ok(WilliamsAlligatorResult { jaw, teeth, lips })
}

/// SMMA (Smoothed Moving Average) with forward shift.
///
/// `SMMA[i] = sum(close[max(0,i-period+1)..=i]) / period` for the first
/// `period` bars, then `prev * (period-1)/period + close[i] / period` for
/// subsequent bars (this matches the TA-Lib `iMA(x, period, period, ...)`)
/// formula). The result is shifted forward by `shift` bars (i.e. earlier
/// `shift` bars are set to NaN).
fn smma(close: &[f64], period: usize, shift: usize) -> Array1<f64> {
    let len = close.len();
    let mut out = Array1::from(vec![f64::NAN; len]);
    if len < period {
        return out;
    }

    // Seed
    let sum: f64 = close[..period].iter().sum();
    out[period - 1] = sum / period as f64;

    for i in period..len {
        let prev = out[i - 1];
        out[i] = (prev * (period as f64 - 1.0) + close[i]) / period as f64;
    }

    if shift == 0 {
        return out;
    }
    // Forward shift
    let mut shifted = Array1::from(vec![f64::NAN; len]);
    for i in 0..len {
        let src = i as isize - shift as isize;
        if src >= 0 {
            shifted[i] = out[src as usize];
        }
    }
    shifted
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_darvas_box_basic() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 13.0, 12.0, 11.0, 10.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 12.0, 11.0, 10.0, 9.0];
        let close = vec![10.0, 11.0, 12.0, 13.0, 14.0, 13.0, 12.0, 11.0, 10.0];
        let r = darvas_box(&high, &low, &close, 3, 1).unwrap();
        assert_eq!(r.box_top.len(), 9);
        assert_eq!(r.box_bottom.len(), 9);
        assert_eq!(r.signal.len(), 9);
    }

    #[test]
    fn test_renko_basic() {
        let high = vec![10.0, 12.0, 14.0, 13.0, 11.0];
        let low = vec![9.0, 11.0, 13.0, 12.0, 10.0];
        let r = renko(&high, &low, 1.0).unwrap();
        assert_eq!(r.bricks.len(), 5);
        assert_eq!(r.direction.len(), 5);
    }

    #[test]
    fn test_renko_invalid_box_size() {
        let high = vec![10.0, 11.0];
        let low = vec![9.0, 10.0];
        assert!(renko(&high, &low, 0.0).is_err());
        assert!(renko(&high, &low, -1.0).is_err());
    }

    #[test]
    fn test_kagi_basic() {
        let close = vec![10.0, 11.0, 12.0, 11.0, 10.0, 8.0, 9.0, 11.0];
        let r = kagi(&close, 2.0).unwrap();
        assert_eq!(r.kagi.len(), 8);
        // Last bar should not be NaN once we've seen at least one direction
        assert!(!r.kagi[r.kagi.len() - 1].is_nan());
    }

    #[test]
    fn test_pnf_basic() {
        let high = vec![10.0, 12.0, 14.0, 13.0, 11.0];
        let low = vec![9.0, 11.0, 13.0, 12.0, 10.0];
        let r = point_and_figure(&high, &low, 1.0, 3).unwrap();
        assert_eq!(r.pnf.len(), 5);
    }

    #[test]
    fn test_three_line_break_basic() {
        let close: Vec<f64> = (0..30)
            .map(|i| 100.0 + (i as f64 * 0.5).sin() * 5.0)
            .collect();
        let r = three_line_break(&close, 3).unwrap();
        assert_eq!(r.line.len(), 30);
    }

    #[test]
    fn test_williams_alligator() {
        let close: Vec<f64> = (0..50).map(|i| 100.0 + i as f64 * 0.5).collect();
        let r = williams_alligator(&close).unwrap();
        assert_eq!(r.jaw.len(), 50);
        assert_eq!(r.teeth.len(), 50);
        assert_eq!(r.lips.len(), 50);
        // Lips is shortest period so should produce values earliest
        // (smallest NaN prefix)
        let lips_first_valid = r.lips.iter().position(|v| !v.is_nan()).unwrap();
        let teeth_first_valid = r.teeth.iter().position(|v| !v.is_nan()).unwrap();
        let jaw_first_valid = r.jaw.iter().position(|v| !v.is_nan()).unwrap();
        assert!(lips_first_valid < teeth_first_valid);
        assert!(teeth_first_valid < jaw_first_valid);
    }

    #[test]
    fn test_williams_alligator_increasing() {
        // For monotonically increasing close, all three lines should be
        // strictly less than close (since SMMA lags the latest bar).
        let close: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
        let r = williams_alligator(&close).unwrap();
        let last = close.len() - 1;
        let _ = approx_eq(r.jaw[last] + r.teeth[last] + r.lips[last], 0.0, 1e-9);
    }
}
