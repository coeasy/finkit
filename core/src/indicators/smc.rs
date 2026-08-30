//! Smart Money Concepts (SMC) indicators.
//!
//! Implements the core ICT/SMC toolkit used by modern price-action traders:
//! order blocks, fair value gaps, break of structure, change of character,
//! and liquidity zones. These indicators identify institutional footprints
//! on the chart.
//!
//! # References
//!
//! - ICT (Inner Circle Trader) concepts
//! - TradingView SMC implementations
//!
//! # Example
//!
//! ```
//! use alpha_ta_core::indicators;
//!
//! let high  = vec![10.0, 11.0, 12.0, 11.5, 10.5, 9.5, 10.0, 11.0, 12.5, 13.0];
//! let low   = vec![ 9.0, 10.0, 11.0, 10.5,  9.5,  8.5,  9.5, 10.0, 11.5, 12.0];
//! let close = vec![10.5, 10.5, 11.5, 11.0, 10.0,  9.0,  9.8, 10.8, 12.0, 12.8];
//! let ob = indicators::order_block(&high, &low, &close, 3).unwrap();
//! assert_eq!(ob.len(), 10);
//! ```

use crate::error::{Result, TaError};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

/// Order block result: `> 0` marks a bullish OB level, `< 0` a bearish OB level,
/// `0.0` means no OB detected at that bar. `NaN` fills the warm-up window.
pub type OrderBlockResult = Array1<f64>;

/// Detect **Order Blocks** (OB).
///
/// An order block is the last opposite-colour candle before a strong impulse
/// move. A bullish OB is the last down-candle before an up-impulse; a bearish
/// OB is the last up-candle before a down-impulse. The returned value is the
/// candle's midpoint (reference price) when an OB is detected, signed by
/// direction (`+` bullish, `-` bearish), `0.0` otherwise.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `lookback` - Window used to gauge impulse strength (≥ 2)
///
/// # Examples
///
/// ```
/// use alpha_ta_core::indicators;
///
/// let high  = vec![10.0, 11.0, 12.0, 11.5, 10.5, 9.5, 10.0, 11.0, 12.5, 13.0];
/// let low   = vec![ 9.0, 10.0, 11.0, 10.5,  9.5,  8.5,  9.5, 10.0, 11.5, 12.0];
/// let close = vec![10.5, 10.5, 11.5, 11.0, 10.0,  9.0,  9.8, 10.8, 12.0, 12.8];
/// let ob = indicators::order_block(&high, &low, &close, 3).unwrap();
/// assert_eq!(ob.len(), 10);
/// ```
pub fn order_block(high: &[f64], low: &[f64], close: &[f64], lookback: usize) -> Result<OrderBlockResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if lookback < 2 {
        return Err(TaError::InvalidParameter {
            name: "lookback".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(high.len(), lookback + 1)?;

    let len = high.len();
    let mut output = init_output(len);

    // An "impulse" over a window of `lookback` bars is defined as a net move
    // whose absolute magnitude exceeds the average true range of the window.
    // The OB is the candle immediately preceding the impulse that has the
    // opposite colour.
    for i in lookback..len {
        // Window [i-lookback, i]: measure net move.
        let window_start = i - lookback;
        let net_move = close[i] - close[window_start];

        // Average true range over the window as a significance threshold.
        let mut atr_sum = 0.0_f64;
        for j in (window_start + 1)..=i {
            let tr = (high[j] - low[j])
                .max((high[j] - close[j - 1]).abs())
                .max((low[j] - close[j - 1]).abs());
            atr_sum += tr;
        }
        let atr_avg = atr_sum / lookback as f64;

        // Impulse threshold: net move exceeds 1.5× ATR.
        if net_move.abs() <= atr_avg * 1.5 {
            continue;
        }

        // The candle right before the impulse window is the OB candidate.
        let ob_idx = window_start;
        if ob_idx == 0 {
            continue;
        }
        // OB candle colour: down candle (close < open-approx = prev close) for bullish OB.
        let ob_bullish = close[ob_idx] < close[ob_idx - 1] && net_move > 0.0;
        let ob_bearish = close[ob_idx] > close[ob_idx - 1] && net_move < 0.0;

        if ob_bullish {
            // Store midpoint as a positive reference price.
            output[i] = (high[ob_idx] + low[ob_idx]) / 2.0;
        } else if ob_bearish {
            // Store midpoint as a negative reference price.
            output[i] = -(high[ob_idx] + low[ob_idx]) / 2.0;
        }
    }

    Ok(output)
}

/// Fair Value Gap (FVG) result.
///
/// Each element is a signed gap size: `> 0` for a bullish FVG, `< 0` for a
/// bearish FVG, `0.0` when no gap. `NaN` fills the warm-up window.
pub type FairValueGapResult = Array1<f64>;

/// Detect **Fair Value Gaps** (FVG / Imbalance).
///
/// A three-candle pattern:
/// - **Bullish FVG**: `low[i] > high[i-2]` — gap between candle 1's high and candle 3's low.
/// - **Bearish FVG**: `high[i] < low[i-2]` — gap between candle 1's low and candle 3's high.
///
/// The returned magnitude is the gap size (price units), signed by direction.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
///
/// # Examples
///
/// ```
/// use alpha_ta_core::indicators;
///
/// let high = vec![10.0, 12.0, 14.0, 13.5, 11.0,  9.0,  8.0, 10.0, 12.0];
/// let low  = vec![ 9.0, 11.0, 13.0, 12.5, 10.0,  8.0,  7.0,  9.0, 11.0];
/// let fvg  = indicators::fair_value_gap(&high, &low).unwrap();
/// assert_eq!(fvg.len(), 9);
/// ```
pub fn fair_value_gap(high: &[f64], low: &[f64]) -> Result<FairValueGapResult> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 3)?;

    let len = high.len();
    let mut output = init_output(len);

    // Need at least 3 candles: compare i (current), i-1 (middle), i-2 (first).
    for i in 2..len {
        let first_high = high[i - 2];
        let first_low = low[i - 2];
        let third_low = low[i];
        let third_high = high[i];

        // Bullish FVG: third candle's low > first candle's high.
        let bull_gap = third_low - first_high;
        if bull_gap > 0.0 {
            output[i] = bull_gap;
            continue;
        }

        // Bearish FVG: third candle's high < first candle's low.
        let bear_gap = third_high - first_low;
        if bear_gap < 0.0 {
            output[i] = bear_gap;
        }
    }

    Ok(output)
}

/// Break of Structure (BOS) result.
///
/// Each element is an integer signal encoded as `f64`:
/// - `1.0` — bullish BOS (price broke above a prior swing high)
/// - `-1.0` — bearish BOS (price broke below a prior swing low)
/// - `0.0` — no BOS
/// - `NaN` — warm-up
pub type BreakOfStructureResult = Array1<f64>;

/// Detect **Break of Structure** (BOS).
///
/// A bullish BOS occurs when price closes above the highest high of the prior
/// `lookback` swing window. A bearish BOS occurs when price closes below the
/// lowest low of the prior window. BOS confirms trend continuation.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `lookback` - Swing window length (≥ 2)
///
/// # Examples
///
/// ```
/// use alpha_ta_core::indicators;
///
/// let high  = vec![10.0, 11.0, 12.0, 11.5, 13.0, 12.5, 14.0, 13.5, 15.0, 14.5];
/// let low   = vec![ 9.0, 10.0, 11.0, 10.5, 12.0, 11.5, 13.0, 12.5, 14.0, 13.5];
/// let close = vec![ 9.5, 10.5, 11.5, 11.0, 12.8, 12.0, 13.8, 13.0, 14.8, 14.0];
/// let bos = indicators::break_of_structure(&high, &low, &close, 3).unwrap();
/// assert_eq!(bos.len(), 10);
/// ```
pub fn break_of_structure(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<BreakOfStructureResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if lookback < 2 {
        return Err(TaError::InvalidParameter {
            name: "lookback".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(high.len(), lookback + 1)?;

    let len = high.len();
    let mut output = init_output(len);

    for i in lookback..len {
        // Find the highest high and lowest low in [i-lookback, i-1].
        let mut swing_high = f64::NEG_INFINITY;
        let mut swing_low = f64::INFINITY;
        for j in (i - lookback)..i {
            if high[j] > swing_high {
                swing_high = high[j];
            }
            if low[j] < swing_low {
                swing_low = low[j];
            }
        }

        // Bullish BOS: close breaks above swing high.
        if close[i] > swing_high {
            output[i] = 1.0;
        }
        // Bearish BOS: close breaks below swing low.
        else if close[i] < swing_low {
            output[i] = -1.0;
        }
    }

    Ok(output)
}

/// Change of Character (CHoCH) result.
///
/// Same encoding as [`BreakOfStructureResult`]: `1.0` bullish CHoCH, `-1.0`
/// bearish CHoCH, `0.0` none, `NaN` warm-up.
pub type ChangeOfCharacterResult = Array1<f64>;

/// Detect **Change of Character** (CHoCH).
///
/// CHoCH is an early trend-reversal signal: the first bar in an uptrend that
/// closes below the prior swing low (bearish CHoCH), or the first bar in a
/// downtrend that closes above the prior swing high (bullish CHoCH). Unlike
/// BOS (continuation), CHoCH signals a potential reversal.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `lookback` - Swing window length (≥ 2)
///
/// # Examples
///
/// ```
/// use alpha_ta_core::indicators;
///
/// let high  = vec![14.0, 13.5, 15.0, 14.5, 13.0, 11.5, 10.0, 11.0, 10.5,  9.0];
/// let low   = vec![13.0, 12.5, 14.0, 13.5, 12.0, 10.5,  9.0,  9.5,  9.0,  8.0];
/// let close = vec![13.5, 13.0, 14.8, 14.0, 12.5, 10.8,  9.5, 10.5,  9.8,  8.5];
/// let choch = indicators::change_of_character(&high, &low, &close, 3).unwrap();
/// assert_eq!(choch.len(), 10);
/// ```
pub fn change_of_character(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<ChangeOfCharacterResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if lookback < 2 {
        return Err(TaError::InvalidParameter {
            name: "lookback".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(high.len(), lookback + 1)?;

    let len = high.len();
    let mut output = init_output(len);

    // Track the prevailing trend. A CHoCH fires when price breaks against the
    // prevailing trend: bullish CHoCH = close breaks above swing high while in
    // a downtrend; bearish CHoCH = close breaks below swing low while in an
    // uptrend. The swing is computed over [i-lookback, i-1] and compared
    // against close[i] (the current bar), so the comparison bar is never part
    // of the swing window.
    let mut trend: f64 = 0.0; // 1 = up, -1 = down, 0 = unknown

    for i in lookback..len {
        // Swing high/low over [i-lookback, i-1].
        let mut swing_high = f64::NEG_INFINITY;
        let mut swing_low = f64::INFINITY;
        for j in (i - lookback)..i {
            if high[j] > swing_high {
                swing_high = high[j];
            }
            if low[j] < swing_low {
                swing_low = low[j];
            }
        }

        // Check if current close breaks the swing.
        if close[i] > swing_high {
            // Bullish break.
            if trend < 0.0 {
                // Was downtrend → bullish CHoCH (reversal).
                output[i] = 1.0;
            }
            trend = 1.0;
        } else if close[i] < swing_low {
            // Bearish break.
            if trend > 0.0 {
                // Was uptrend → bearish CHoCH (reversal).
                output[i] = -1.0;
            }
            trend = -1.0;
        }
    }

    Ok(output)
}

/// Liquidity zone result.
///
/// Each element is the liquidity-zone midpoint price when a zone is detected
/// at that bar, `0.0` otherwise, `NaN` during warm-up.
pub type LiquidityZonesResult = Array1<f64>;

/// Detect **Liquidity Zones**.
///
/// Liquidity zones are price areas where the market previously consolidated
/// (low range relative to surrounding bars), trapping stop orders. We detect
/// them as windows of `lookback` bars whose combined range is below a
/// fraction of the surrounding ATR — i.e. "tight" clusters.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `lookback` - Cluster window length (≥ 2)
///
/// # Examples
///
/// ```
/// use alpha_ta_core::indicators;
///
/// let high = vec![10.0, 10.2, 10.1, 10.3, 10.2, 12.0, 13.0, 12.5, 13.5, 13.2];
/// let low  = vec![ 9.8, 10.0,  9.9, 10.1, 10.0, 11.5, 12.5, 12.0, 13.0, 12.8];
/// let lz   = indicators::liquidity_zones(&high, &low, 3).unwrap();
/// assert_eq!(lz.len(), 10);
/// ```
pub fn liquidity_zones(high: &[f64], low: &[f64], lookback: usize) -> Result<LiquidityZonesResult> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if lookback < 2 {
        return Err(TaError::InvalidParameter {
            name: "lookback".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(high.len(), lookback + 1)?;

    let len = high.len();
    let mut output = init_output(len);

    // A liquidity zone is a tight consolidation: the cluster's price range
    // (highest high − lowest low over `lookback` bars) is less than 5% of the
    // cluster midpoint. This price-relative metric is robust across all price
    // levels and does not depend on a separate ATR estimate (which would be
    // artificially small inside the consolidation itself).
    const TIGHT_PCT: f64 = 0.05;

    for i in lookback..len {
        // Cluster range over [i-lookback, i-1].
        let cluster_start = i - lookback;
        let mut cluster_high = f64::NEG_INFINITY;
        let mut cluster_low = f64::INFINITY;
        for j in cluster_start..i {
            if high[j] > cluster_high {
                cluster_high = high[j];
            }
            if low[j] < cluster_low {
                cluster_low = low[j];
            }
        }
        let cluster_range = cluster_high - cluster_low;
        let cluster_mid = (cluster_high + cluster_low) / 2.0;

        if cluster_mid > 0.0 && cluster_range / cluster_mid < TIGHT_PCT {
            output[i] = cluster_mid;
        }
    }

    Ok(output)
}

/// Comprehensive SMC signal bundle.
#[derive(Debug, Clone)]
pub struct SmcSignals {
    /// Order block levels (signed midpoint, see [`order_block`]).
    pub order_blocks: Array1<f64>,
    /// Fair value gap sizes (signed, see [`fair_value_gap`]).
    pub fair_value_gaps: Array1<f64>,
    /// Break of structure signals (±1 / 0, see [`break_of_structure`]).
    pub break_of_structure: Array1<f64>,
    /// Change of character signals (±1 / 0, see [`change_of_character`]).
    pub change_of_character: Array1<f64>,
    /// Liquidity zone midpoints (see [`liquidity_zones`]).
    pub liquidity_zones: Array1<f64>,
}

/// Compute all SMC signals in a single pass.
///
/// Convenience wrapper that calls each SMC detector with the same `lookback`
/// (where applicable) and bundles the results. FVG always uses a fixed
/// 3-candle window regardless of `lookback`.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `lookback` - Swing window for OB/BOS/CHoCH/Liquidity (≥ 2)
///
/// # Examples
///
/// ```
/// use alpha_ta_core::indicators;
///
/// let high  = vec![10.0, 11.0, 12.0, 11.5, 13.0, 12.5, 14.0, 13.5, 15.0, 14.5];
/// let low   = vec![ 9.0, 10.0, 11.0, 10.5, 12.0, 11.5, 13.0, 12.5, 14.0, 13.5];
/// let close = vec![ 9.5, 10.5, 11.5, 11.0, 12.8, 12.0, 13.8, 13.0, 14.8, 14.0];
/// let sig = indicators::smc_signals(&high, &low, &close, 3).unwrap();
/// assert_eq!(sig.order_blocks.len(), 10);
/// assert_eq!(sig.fair_value_gaps.len(), 10);
/// ```
pub fn smc_signals(high: &[f64], low: &[f64], close: &[f64], lookback: usize) -> Result<SmcSignals> {
    Ok(SmcSignals {
        order_blocks: order_block(high, low, close, lookback)?,
        fair_value_gaps: fair_value_gap(high, low)?,
        break_of_structure: break_of_structure(high, low, close, lookback)?,
        change_of_character: change_of_character(high, low, close, lookback)?,
        liquidity_zones: liquidity_zones(high, low, lookback)?,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ohlcv() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        // 15-bar series with a clear up-impulse and a down-impulse.
        let high = vec![
            10.0, 10.5, 11.0, 10.8, 10.6, 11.0, 12.5, 13.5, 13.2, 13.0, 12.0, 11.0, 10.0, 9.5,
            10.5,
        ];
        let low = vec![
            9.5, 10.0, 10.5, 10.2, 10.0, 10.5, 11.5, 12.5, 12.8, 12.5, 11.5, 10.5, 9.5, 9.0, 9.8,
        ];
        let close = vec![
            9.8, 10.2, 10.8, 10.5, 10.3, 10.8, 12.2, 13.2, 13.0, 12.8, 11.8, 10.8, 9.8, 9.2, 10.2,
        ];
        (high, low, close)
    }

    #[test]
    fn test_order_block_basic() {
        let (high, low, close) = sample_ohlcv();
        let ob = order_block(&high, &low, &close, 3).unwrap();
        assert_eq!(ob.len(), 15);
        // Warm-up is NaN.
        assert!(ob[0].is_nan() && ob[1].is_nan() && ob[2].is_nan());
        // At least one non-zero OB in the impulse region.
        let has_ob = ob.iter().any(|&v| v.abs() > 0.0);
        assert!(has_ob, "expected at least one order block");
    }

    #[test]
    fn test_order_block_invalid_lookback() {
        let (high, low, close) = sample_ohlcv();
        assert!(order_block(&high, &low, &close, 1).is_err());
    }

    #[test]
    fn test_order_block_length_mismatch() {
        let (high, low, close) = sample_ohlcv();
        let short = &high[..3];
        assert!(order_block(short, &low, &close, 3).is_err());
    }

    #[test]
    fn test_fair_value_gap_basic() {
        // Explicit bullish FVG: low[2] > high[0].
        let high = vec![10.0, 12.0, 14.0, 13.5, 11.0, 9.0, 8.0, 10.0, 12.0];
        let low = vec![9.0, 11.0, 13.0, 12.5, 10.0, 8.0, 7.0, 9.0, 11.0];
        let fvg = fair_value_gap(&high, &low).unwrap();
        assert_eq!(fvg.len(), 9);
        // Bullish FVG at index 2: low[2] - high[0] = 13 - 10 = 3.
        assert!((fvg[2] - 3.0).abs() < 1e-10);
        // Bearish FVG: high[6] < low[4] → 8 < 10 → gap = 8 - 10 = -2.
        assert!((fvg[6] - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_fair_value_gap_no_gap() {
        // Overlapping candles: no FVG.
        let high = vec![10.0, 10.5, 11.0, 10.8, 10.5];
        let low = vec![9.0, 9.5, 10.0, 9.8, 9.5];
        let fvg = fair_value_gap(&high, &low).unwrap();
        for v in fvg.iter() {
            assert!(v.is_nan() || v.abs() == 0.0);
        }
    }

    #[test]
    fn test_break_of_structure_basic() {
        let (high, low, close) = sample_ohlcv();
        let bos = break_of_structure(&high, &low, &close, 3).unwrap();
        assert_eq!(bos.len(), 15);
        // The up-impulse around index 7 should trigger a bullish BOS.
        let has_bull = bos.iter().any(|&v| v == 1.0);
        assert!(has_bull, "expected at least one bullish BOS");
        // The down move near the end should trigger a bearish BOS.
        let has_bear = bos.iter().any(|&v| v == -1.0);
        assert!(has_bear, "expected at least one bearish BOS");
    }

    #[test]
    fn test_break_of_structure_invalid_lookback() {
        let (high, low, close) = sample_ohlcv();
        assert!(break_of_structure(&high, &low, &close, 1).is_err());
    }

    #[test]
    fn test_change_of_character_basic() {
        let (high, low, close) = sample_ohlcv();
        let choch = change_of_character(&high, &low, &close, 3).unwrap();
        assert_eq!(choch.len(), 15);
        // After the peak, a close below the prior swing low should fire bearish CHoCH.
        let has_signal = choch.iter().any(|&v| v.abs() == 1.0);
        assert!(has_signal, "expected at least one CHoCH signal");
    }

    #[test]
    fn test_liquidity_zones_basic() {
        // Tight consolidation in first 4 bars (range 0.4), then breakout to 12+.
        let high = vec![10.2, 10.1, 10.3, 10.2, 12.0, 13.5, 12.5, 14.0, 13.5, 15.0];
        let low = vec![10.0,  9.9, 10.1, 10.0, 11.5, 12.5, 12.0, 13.0, 12.8, 14.0];
        let lz = liquidity_zones(&high, &low, 3).unwrap();
        assert_eq!(lz.len(), 10);
        // At least one zone detected in the tight consolidation region.
        let has_zone = lz.iter().any(|&v| v.abs() > 0.0);
        assert!(has_zone, "expected at least one liquidity zone");
    }

    #[test]
    fn test_liquidity_zones_invalid_lookback() {
        let (high, low, _) = sample_ohlcv();
        assert!(liquidity_zones(&high, &low, 1).is_err());
    }

    #[test]
    fn test_smc_signals_bundle() {
        let (high, low, close) = sample_ohlcv();
        let sig = smc_signals(&high, &low, &close, 3).unwrap();
        assert_eq!(sig.order_blocks.len(), 15);
        assert_eq!(sig.fair_value_gaps.len(), 15);
        assert_eq!(sig.break_of_structure.len(), 15);
        assert_eq!(sig.change_of_character.len(), 15);
        assert_eq!(sig.liquidity_zones.len(), 15);
    }
}
