//! Volume Profile family indicators.
//!
//! This module complements [`crate::indicators::volume`] with advanced
//! volume-auction analytics:
//!
//! - [`market_profile_tpo`]: Market Profile / Time-Price-Opportunity counts.
//! - [`vwap_anchored_session`]: VWAP that auto-resets on session boundaries
//!   derived from timestamps (extends the boolean-reset [`crate::indicators::vwap_mtf`]).
//! - [`volume_nodes`]: High-Volume-Node / Low-Volume-Node detection.
//!
//! The classic [`volume_profile`] lives in [`crate::indicators::volume`] and is
//! re-exported here for convenience.

use crate::error::{Result, TaError};
use crate::utils::validate_input;
use ndarray::Array1;

// Re-export the classic volume profile so callers can reach every
// volume-auction primitive from one module.
pub use crate::indicators::volume::{VolumeProfileResult, volume_profile};

/// Market Profile (TPO) result.
///
/// Time-Price-Opportunity analysis counts how many distinct time periods
/// "touch" each price bin, ignoring volume. The resulting histogram reveals
/// acceptance/rejection price levels independently of traded size.
#[derive(Debug, Clone)]
pub struct MarketProfileTpoResult {
    /// Point of Control — price bin touched by the most TPO periods.
    pub poc: f64,
    /// TPO count per price bin (one count per period that touched the bin).
    pub profile: Vec<usize>,
    /// Centre price of each bin (same length as `profile`).
    pub bin_prices: Vec<f64>,
}

/// Volume Nodes result.
///
/// Classifies each price bin of a volume profile as a High-Volume Node (HVN),
/// Low-Volume Node (LVN), or normal. HVNs are acceptance zones where price
/// spends disproportionate time; LVNs are rejection zones.
#[derive(Debug, Clone)]
pub struct VolumeNodesResult {
    /// Raw volume per bin (copied from the underlying profile).
    pub profile: Vec<f64>,
    /// Centre price of each bin.
    pub bin_prices: Vec<f64>,
    /// Per-bin classification: `1` = HVN, `-1` = LVN, `0` = normal.
    pub nodes: Vec<i8>,
    /// Prices of every High-Volume Node (ascending).
    pub hvn_prices: Vec<f64>,
    /// Prices of every Low-Volume Node (ascending).
    pub lvn_prices: Vec<f64>,
}

/// Market Profile — Time Price Opportunity (TPO).
///
/// Groups bars into TPO periods of `period` bars each and, for every price bin,
/// counts how many distinct periods have at least one bar whose `[low, high]`
/// range overlaps the bin. Unlike [`volume_profile`], every bar contributes
/// equally regardless of traded volume.
///
/// # Arguments
/// * `high`  - High prices.
/// * `low`   - Low prices.
/// * `close` - Close prices (used only to validate length; TPO ignores close).
/// * `period`- Bars per TPO period (e.g. 6 for 30-min periods on 5-min bars).
/// * `num_bins` - Number of price bins across the full `[min_low, max_high]` range.
///
/// # Errors
/// Returns [`TaError::InvalidParameter`] when lengths mismatch, `period == 0`,
/// `num_bins == 0`, or the input is empty.
///
/// # Example
///
/// ```
/// use finkit::indicators::market_profile_tpo;
///
/// let high  = vec![15.0, 15.0, 15.0, 15.0, 15.0, 15.0];
/// let low   = vec![ 5.0,  5.0,  5.0,  5.0,  5.0,  5.0];
/// let close = vec![10.0, 10.0, 10.0, 10.0, 10.0, 10.0];
/// let r = market_profile_tpo(&high, &low, &close, 3, 5).unwrap();
/// assert_eq!(r.profile.len(), 5);
/// assert!(r.poc.is_finite());
/// ```
pub fn market_profile_tpo(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    num_bins: usize,
) -> Result<MarketProfileTpoResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    if num_bins == 0 {
        return Err(TaError::InvalidParameter {
            name: "num_bins".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }

    let len = high.len();
    let mut min_price = f64::INFINITY;
    let mut max_price = f64::NEG_INFINITY;
    for i in 0..len {
        min_price = min_price.min(low[i]);
        max_price = max_price.max(high[i]);
    }
    if (max_price - min_price).abs() < 1e-15 {
        max_price = min_price + 1.0;
    }
    let price_range = max_price - min_price;
    let bin_size = price_range / num_bins as f64;
    let inv_bin_size = num_bins as f64 / price_range;

    let mut profile = vec![0usize; num_bins];
    let bin_prices: Vec<f64> = (0..num_bins)
        .map(|i| min_price + (i as f64 + 0.5) * bin_size)
        .collect();

    // Walk the data in TPO-period blocks. For each block, mark which bins are
    // touched by any bar in the block; each touched bin earns one TPO count.
    let mut start = 0usize;
    while start < len {
        let end = (start + period).min(len);

        // Determine the bin span touched by the whole block first to bound the
        // inner loop, then do the precise per-bar marking.
        let block_lo = (low[start..end].iter().copied()).fold(f64::INFINITY, f64::min);
        let block_hi = (high[start..end].iter().copied()).fold(f64::NEG_INFINITY, f64::max);
        let first_bin = ((block_lo - min_price) * inv_bin_size).floor() as isize;
        let last_bin = ((block_hi - min_price) * inv_bin_size).floor() as isize;
        let first_bin = first_bin.clamp(0, (num_bins - 1) as isize) as usize;
        let last_bin = last_bin.clamp(0, (num_bins - 1) as isize) as usize;

        // Tally touched bins for this block. A bin `b` is touched if any bar's
        // [low, high] straddles the bin centre (the conventional TPO convention
        // is "price traded at or through this level").
        let mut touched = vec![false; num_bins];
        for j in first_bin..=last_bin {
            let bin_lower = min_price + j as f64 * bin_size;
            let bin_upper = bin_lower + bin_size;
            for k in start..end {
                // Bar touches bin if ranges overlap: low <= bin_upper && high >= bin_lower
                if low[k] <= bin_upper && high[k] >= bin_lower {
                    touched[j] = true;
                    break;
                }
            }
        }
        for (j, &t) in touched.iter().enumerate() {
            if t {
                profile[j] += 1;
            }
        }

        start = end;
    }

    let (poc_index, _) = profile
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.cmp(b))
        .unwrap_or((0, &0));
    let poc = bin_prices[poc_index];

    Ok(MarketProfileTpoResult {
        poc,
        profile,
        bin_prices,
    })
}

/// Anchored VWAP with automatic session-boundary reset.
///
/// Unlike [`crate::indicators::vwap_mtf`], which takes a precomputed boolean
/// `session_start` mask, this variant derives session boundaries directly from
/// bar timestamps. A new session begins whenever `floor((ts - offset) / period)`
/// changes, where `period` and `offset` are expressed in seconds.
///
/// Typical configurations:
/// - Daily UTC: `session_period_secs = 86_400`, `session_offset_secs = 0`
/// - Daily 9:30 ET: `session_period_secs = 86_400`, `session_offset_secs = 34_200`
/// - Weekly: `session_period_secs = 604_800`, `session_offset_secs = 0`
///
/// # Arguments
/// * `high`, `low`, `close`, `volume` - OHLCV slices of equal length.
/// * `timestamps` - Bar open timestamps in **epoch milliseconds**.
/// * `session_period_secs` - Length of one session in seconds (e.g. 86_400).
/// * `session_offset_secs` - Offset of the session start within the period, in seconds.
///
/// # Errors
/// Returns [`TaError::InvalidParameter`] on length mismatch or non-positive period.
///
/// # Example
///
/// ```
/// use finkit::indicators::vwap_anchored_session;
///
/// let high  = vec![10.0, 11.0, 12.0, 11.5, 12.5, 13.0];
/// let low   = vec![ 9.0, 10.0, 11.0, 10.5, 11.5, 12.0];
/// let close = vec![ 9.5, 10.5, 11.5, 11.0, 12.0, 12.5];
/// let vol   = vec![100.0, 200.0, 150.0, 180.0, 220.0, 260.0];
/// // Two sessions: bars 0-2 and bars 3-5 (each session = 3h = 10800s).
/// let ts    = vec![0i64, 3_600_000, 7_200_000, 10_800_000, 14_400_000, 18_000_000];
/// let r = vwap_anchored_session(&high, &low, &close, &vol, &ts, 10_800, 0).unwrap();
/// assert_eq!(r.len(), 6);
/// ```
pub fn vwap_anchored_session(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    timestamps: &[i64],
    session_period_secs: i64,
    session_offset_secs: i64,
) -> Result<Array1<f64>> {
    let len = high.len();
    if len != low.len() || len != close.len() || len != volume.len() || len != timestamps.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close, volume, timestamps".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(len, 1)?;
    if session_period_secs <= 0 {
        return Err(TaError::InvalidParameter {
            name: "session_period_secs".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }

    let mut output = Array1::zeros(len);
    let mut cum_tp_vol = 0.0f64;
    let mut cum_volume = 0.0f64;
    let mut current_session = i64::MIN;

    let period_ms = (session_period_secs as i64) * 1000;
    let offset_ms = (session_offset_secs as i64) * 1000;

    for i in 0..len {
        // Determine which session this bar belongs to.
        // Negative timestamps are handled by floor division semantics.
        let session = (timestamps[i].saturating_sub(offset_ms)).div_euclid(period_ms);

        if session != current_session {
            cum_tp_vol = 0.0;
            cum_volume = 0.0;
            current_session = session;
        }

        let typical_price = (high[i] + low[i] + close[i]) / 3.0;
        cum_tp_vol += typical_price * volume[i];
        cum_volume += volume[i];

        if cum_volume.abs() > 1e-15 {
            output[i] = cum_tp_vol / cum_volume;
        }
    }

    Ok(output)
}

/// High-Volume-Node / Low-Volume-Node detector.
///
/// Builds a volume profile (price-bin histogram of traded volume) and flags
/// each bin whose volume deviates from the mean by more than `k` standard
/// deviations:
/// - HVN (High-Volume Node): `vol > mean + k * std`
/// - LVN (Low-Volume Node):  `vol < mean - k * std`
///
/// HVNs are price levels accepted by the market (consolidation/magnet zones);
/// LVNs are fast-move rejection zones.
///
/// # Arguments
/// * `high`, `low`, `close`, `volume` - OHLCV slices of equal length.
/// * `num_bins` - Number of price bins.
/// * `k` - Standard-deviation multiplier (typical: 1.0–2.0).
///
/// # Errors
/// Returns [`TaError::InvalidParameter`] on length mismatch, zero `num_bins`, or empty input.
///
/// # Example
///
/// ```
/// use finkit::indicators::volume_nodes;
///
/// let high  = vec![15.0, 15.0, 15.0, 15.0, 15.0, 15.0];
/// let low   = vec![ 5.0,  5.0,  5.0,  5.0,  5.0,  5.0];
/// let close = vec![10.0, 10.0, 12.0,  8.0, 10.0, 10.0];
/// let vol   = vec![ 50.0, 60.0, 500.0, 5.0, 55.0, 50.0];
/// let r = volume_nodes(&high, &low, &close, &vol, 5, 1.0).unwrap();
/// assert_eq!(r.nodes.len(), 5);
/// assert!(!r.hvn_prices.is_empty() || !r.lvn_prices.is_empty());
/// ```
pub fn volume_nodes(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    num_bins: usize,
    k: f64,
) -> Result<VolumeNodesResult> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;
    if num_bins == 0 {
        return Err(TaError::InvalidParameter {
            name: "num_bins".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    if !k.is_finite() || k < 0.0 {
        return Err(TaError::InvalidParameter {
            name: "k".to_string(),
            constraint: "must be a non-negative finite number".to_string(),
        });
    }

    // Build the volume profile (typical-price allocation, identical to
    // `volume_profile` but inlined to avoid the extra result-struct copy).
    let len = high.len();
    let mut min_price = f64::INFINITY;
    let mut max_price = f64::NEG_INFINITY;
    for i in 0..len {
        min_price = min_price.min(low[i]);
        max_price = max_price.max(high[i]);
    }
    if (max_price - min_price).abs() < 1e-15 {
        max_price = min_price + 1.0;
    }
    let price_range = max_price - min_price;
    let bin_size = price_range / num_bins as f64;
    let inv_bin_size = num_bins as f64 / price_range;

    let mut profile = vec![0.0f64; num_bins];
    let bin_prices: Vec<f64> = (0..num_bins)
        .map(|i| min_price + (i as f64 + 0.5) * bin_size)
        .collect();

    for i in 0..len {
        let typical_price = (high[i] + low[i] + close[i]) / 3.0;
        let mut bin_index = ((typical_price - min_price) * inv_bin_size).floor() as isize;
        if bin_index < 0 {
            bin_index = 0;
        } else if bin_index >= num_bins as isize {
            bin_index = num_bins as isize - 1;
        }
        profile[bin_index as usize] += volume[i];
    }

    // Mean & population std of bin volumes.
    let n = num_bins as f64;
    let mean: f64 = profile.iter().sum::<f64>() / n;
    let var: f64 = profile.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n;
    let std = var.sqrt();
    let high_thr = mean + k * std;
    let low_thr = mean - k * std;

    let mut nodes = vec![0i8; num_bins];
    let mut hvn_prices = Vec::new();
    let mut lvn_prices = Vec::new();
    for (i, &vol) in profile.iter().enumerate() {
        if vol > high_thr {
            nodes[i] = 1;
            hvn_prices.push(bin_prices[i]);
        } else if vol < low_thr {
            nodes[i] = -1;
            lvn_prices.push(bin_prices[i]);
        }
    }

    Ok(VolumeNodesResult {
        profile,
        bin_prices,
        nodes,
        hvn_prices,
        lvn_prices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- market_profile_tpo ----------

    #[test]
    fn test_market_profile_tpo_basic() {
        // 6 bars split into 2 TPO periods of 3 bars each.
        // Period A (bars 0-2): touches [5, 15] -> all 5 bins touched (1 TPO each).
        // Period B (bars 3-5): touches [5, 15] -> all 5 bins touched (another TPO).
        let high = vec![15.0, 15.0, 15.0, 15.0, 15.0, 15.0];
        let low = vec![5.0, 5.0, 5.0, 5.0, 5.0, 5.0];
        let close = vec![10.0, 10.0, 10.0, 10.0, 10.0, 10.0];
        let r = market_profile_tpo(&high, &low, &close, 3, 5).unwrap();
        assert_eq!(r.profile.len(), 5);
        // Every bin was touched by both periods -> TPO count of 2.
        for &count in &r.profile {
            assert_eq!(count, 2);
        }
        assert!(r.poc.is_finite());
    }

    #[test]
    fn test_market_profile_tpo_poc_skew() {
        // Two periods. Period A touches only low bins; period B touches all bins.
        // The low bins should have TPO=2 (touched by both), high bins TPO=1.
        // POC must fall in the lower region.
        let high = vec![15.0, 15.0, 15.0, 8.0, 8.0, 8.0];
        let low = vec![5.0, 5.0, 5.0, 5.0, 5.0, 5.0];
        let close = vec![10.0, 10.0, 10.0, 6.0, 6.0, 6.0];
        let r = market_profile_tpo(&high, &low, &close, 3, 5).unwrap();
        // The lowest 2 bins (prices ~6, ~8) are touched by both periods.
        let max_count = *r.profile.iter().max().unwrap();
        assert!(max_count >= 2);
        // POC should be in the lower half (smaller price).
        let midpoint = (r.bin_prices.first().unwrap() + r.bin_prices.last().unwrap()) / 2.0;
        assert!(
            r.poc <= midpoint,
            "POC {} should be in lower half (<= {})",
            r.poc,
            midpoint
        );
    }

    #[test]
    fn test_market_profile_tpo_invalid_params() {
        let high = vec![10.0, 12.0];
        let low = vec![8.0, 10.0];
        let close = vec![9.0, 11.0];
        assert!(market_profile_tpo(&high, &low, &close, 0, 5).is_err());
        assert!(market_profile_tpo(&high, &low, &close, 3, 0).is_err());
        let short_low = vec![8.0];
        assert!(market_profile_tpo(&high, &short_low, &close, 3, 5).is_err());
    }

    #[test]
    fn test_market_profile_tpo_single_period() {
        // All bars in one TPO period -> each bin touched at most once.
        let high = vec![15.0, 15.0, 15.0];
        let low = vec![5.0, 5.0, 5.0];
        let close = vec![10.0, 10.0, 10.0];
        let r = market_profile_tpo(&high, &low, &close, 10, 5).unwrap();
        for &count in &r.profile {
            assert!(count == 0 || count == 1, "expected 0 or 1, got {count}");
        }
    }

    // ---------- vwap_anchored_session ----------

    #[test]
    fn test_vwap_anchored_session_resets() {
        // Two daily sessions (period=86400s). Bar 0 and bar 3 start new sessions.
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let vol = vec![100.0, 100.0, 100.0, 100.0, 100.0, 100.0];
        // Sessions: bar0 @ day0 00:00, bar1 @ day0 06:00, bar2 @ day0 12:00,
        //           bar3 @ day1 00:00, bar4 @ day1 06:00, bar5 @ day1 12:00.
        let ts = vec![
            0i64,
            6 * 3_600_000,
            12 * 3_600_000,
            24 * 3_600_000,
            30 * 3_600_000,
            36 * 3_600_000,
        ];
        let r = vwap_anchored_session(&high, &low, &close, &vol, &ts, 86_400, 0).unwrap();

        // Session 1 VWAP at bar 2 (typical prices 9.5, 10.5, 11.5; equal vol).
        let expected_s1 = (9.5 + 10.5 + 11.5) / 3.0;
        assert!(
            (r[2] - expected_s1).abs() < 1e-9,
            "session1 VWAP {} != expected {}",
            r[2],
            expected_s1
        );
        // Session 2 resets: bar 3 VWAP = typical price of bar 3.
        let tp3 = (13.0 + 12.0 + 12.5) / 3.0;
        assert!(
            (r[3] - tp3).abs() < 1e-9,
            "session2 reset VWAP {} != {}",
            r[3],
            tp3
        );
    }

    #[test]
    fn test_vwap_anchored_session_single_session() {
        let high = vec![10.0, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];
        let vol = vec![100.0, 200.0, 150.0];
        let ts = vec![0i64, 3_600_000, 7_200_000];
        // Large period so all bars are in one session.
        let r = vwap_anchored_session(&high, &low, &close, &vol, &ts, 86_400, 0).unwrap();
        // Should match plain cumulative VWAP.
        let tp: Vec<f64> = high
            .iter()
            .zip(low.iter().zip(close.iter()))
            .map(|(h, (l, c))| (h + l + c) / 3.0)
            .collect();
        let mut cum_pv = 0.0;
        let mut cum_v = 0.0;
        for i in 0..3 {
            cum_pv += tp[i] * vol[i];
            cum_v += vol[i];
            assert!((r[i] - cum_pv / cum_v).abs() < 1e-10);
        }
    }

    #[test]
    fn test_vwap_anchored_session_invalid_params() {
        let high = vec![10.0, 11.0];
        let low = vec![9.0, 10.0];
        let close = vec![9.5, 10.5];
        let vol = vec![100.0, 200.0];
        let ts = vec![0i64, 3_600_000];
        // Length mismatch.
        let short_vol = vec![100.0];
        assert!(vwap_anchored_session(&high, &low, &close, &short_vol, &ts, 86_400, 0).is_err());
        // Non-positive period.
        assert!(vwap_anchored_session(&high, &low, &close, &vol, &ts, 0, 0).is_err());
    }

    // ---------- volume_nodes ----------

    #[test]
    fn test_volume_nodes_detects_hvn_and_lvn() {
        // 6 bars, each mapping to a distinct price bin (typical prices 5,15,25,35,45,55).
        // Volume profile = [5, 100, 100, 100, 100, 300].
        //   mean ≈ 117.5, std ≈ 88.7, k=1.0
        //   high_thr ≈ 206.2  -> bin 5 (vol=300) is HVN
        //   low_thr  ≈ 28.8   -> bin 0 (vol=5)   is LVN
        let high  = vec![ 6.0, 16.0, 26.0, 36.0, 46.0, 56.0];
        let low   = vec![ 4.0, 14.0, 24.0, 34.0, 44.0, 54.0];
        let close = vec![ 5.0, 15.0, 25.0, 35.0, 45.0, 55.0];
        let vol   = vec![ 5.0, 100.0, 100.0, 100.0, 100.0, 300.0];
        let r = volume_nodes(&high, &low, &close, &vol, 6, 1.0).unwrap();
        assert_eq!(r.nodes.len(), 6);
        assert!(!r.hvn_prices.is_empty(), "should detect at least one HVN");
        assert!(
            !r.lvn_prices.is_empty(),
            "should detect at least one LVN"
        );
        // Nodes must be in {-1, 0, 1}.
        for &n in &r.nodes {
            assert!(n == -1 || n == 0 || n == 1, "unexpected node {n}");
        }
        // Sanity: the highest-volume bin must be an HVN.
        let max_vol_idx = r
            .profile
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert_eq!(
            r.nodes[max_vol_idx], 1,
            "max-volume bin (price {}) should be an HVN",
            r.bin_prices[max_vol_idx]
        );
        // The lowest-volume bin must be an LVN.
        let min_vol_idx = r
            .profile
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert_eq!(
            r.nodes[min_vol_idx], -1,
            "min-volume bin (price {}) should be an LVN",
            r.bin_prices[min_vol_idx]
        );
    }

    #[test]
    fn test_volume_nodes_all_normal_when_uniform() {
        // Uniform volume -> no HVN/LVN.
        let high = vec![15.0, 15.0, 15.0, 15.0];
        let low = vec![5.0, 5.0, 5.0, 5.0];
        let close = vec![7.0, 9.0, 11.0, 13.0];
        let vol = vec![100.0, 100.0, 100.0, 100.0];
        let r = volume_nodes(&high, &low, &close, &vol, 4, 1.0).unwrap();
        assert!(r.hvn_prices.is_empty());
        assert!(r.lvn_prices.is_empty());
        for &n in &r.nodes {
            assert_eq!(n, 0);
        }
    }

    #[test]
    fn test_volume_nodes_invalid_params() {
        let high = vec![10.0, 12.0];
        let low = vec![8.0, 10.0];
        let close = vec![9.0, 11.0];
        let vol = vec![100.0, 200.0];
        assert!(volume_nodes(&high, &low, &close, &vol, 0, 1.0).is_err());
        assert!(volume_nodes(&high, &low, &close, &vol, 5, -1.0).is_err());
        assert!(volume_nodes(&high, &low, &close, &vol, 5, f64::NAN).is_err());
        let short_vol = vec![100.0];
        assert!(volume_nodes(&high, &low, &close, &short_vol, 5, 1.0).is_err());
    }
}
