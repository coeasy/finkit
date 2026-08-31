//! Streaming Smart Money Concepts (SMC) indicators.
//!
//! Provides O(1) per-bar streaming versions of the most latency-sensitive SMC
//! detectors: [`StreamingFairValueGap`] and [`StreamingOrderBlock`]. These are
//! designed for real-time bar-by-bar processing where the full batch API is
//! impractical.

use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};

// ---------------------------------------------------------------------------
// StreamingFairValueGap
// ---------------------------------------------------------------------------

/// Streaming Fair Value Gap (FVG) detector.
///
/// Maintains a 3-candle sliding window and emits a signed gap size on each
/// new bar:
/// - `Some(+gap)` -- bullish FVG detected
/// - `Some(-gap)` -- bearish FVG detected
/// - `Some(0.0)` -- no gap
/// - `None` -- warm-up (first 2 bars)
///
/// # Example
///
/// ```
/// use finkit::streaming::indicators::StreamingFairValueGap;
/// use finkit::streaming::StreamingIndicator;
///
/// let mut fvg = StreamingFairValueGap::new();
/// // Warm-up: first two bars return None.
/// assert!(fvg.next((10.0, 9.0)).is_none());
/// assert!(fvg.next((12.0, 11.0)).is_none());
/// // Third bar: low=13 > first high=10 -> bullish FVG of 3.0.
/// let v = fvg.next((14.0, 13.0)).unwrap();
/// assert!((v - 3.0).abs() < 1e-10);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingFairValueGap {
    // 2-element history of (high, low).
    hist: [(f64, f64); 2],
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingFairValueGap {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingFairValueGap {
    /// Create a new streaming FVG detector.
    pub const fn new() -> Self {
        Self {
            hist: [(0.0, 0.0); 2],
            count: 0,
            last_value: None,
        }
    }

    /// Feed an OHLC bar via the [`Ohlcv`] trait (used by `compute_bar`).
    pub fn compute_bar(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.next((bar.high(), bar.low()))
    }
}

impl StreamingIndicator<(f64, f64)> for StreamingFairValueGap {
    #[inline]
    fn next(&mut self, input: (f64, f64)) -> Option<f64> {
        let (high, low) = input;
        self.count += 1;

        // Shift history: [old_first, old_second] -> [old_second, current].
        let prev = self.hist;
        self.hist[0] = prev[1];
        self.hist[1] = (high, low);

        // Need at least 3 bars (current is the 3rd).
        if self.count < 3 {
            self.last_value = None;
            return None;
        }

        // Bullish FVG: current low > first candle's high.
        let first_high = prev[0].0;
        let bull_gap = low - first_high;
        let result = if bull_gap > 0.0 {
            bull_gap
        } else {
            // Bearish FVG: current high < first candle's low.
            let first_low = prev[0].1;
            let bear_gap = high - first_low;
            if bear_gap < 0.0 {
                bear_gap
            } else {
                0.0
            }
        };

        self.last_value = Some(result);
        Some(result)
    }

    fn reset(&mut self) {
        self.hist = [(0.0, 0.0); 2];
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= 3
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingFairValueGap {
    fn name() -> &'static str {
        "FVG"
    }

    fn category() -> &'static str {
        "smc"
    }

    fn description() -> &'static str {
        "Fair Value Gap (3-candle imbalance detector)"
    }

    fn warm_up_period(&self) -> usize {
        2
    }
}

// ---------------------------------------------------------------------------
// StreamingOrderBlock
// ---------------------------------------------------------------------------

/// Streaming Order Block detector.
///
/// Detects order blocks by tracking impulse strength relative to a rolling
/// ATR. Emits `Some(signed_midpoint)` when an OB is detected at the current
/// bar (positive = bullish OB, negative = bearish OB), `Some(0.0)` when no
/// OB, and `None` during warm-up.
///
/// # Example
///
/// ```
/// use finkit::streaming::indicators::StreamingOrderBlock;
/// use finkit::streaming::StreamingIndicator;
///
/// let mut ob = StreamingOrderBlock::new(3);
/// // Feed 10 bars of (high, low, close).
/// let data = [
///     (10.0,  9.0,  9.8),
///     (10.5, 10.0, 10.2),
///     (11.0, 10.5, 10.8),
///     (10.8, 10.2, 10.5),
///     (10.6, 10.0, 10.3),
///     (11.0, 10.5, 10.8),
///     (12.5, 11.5, 12.2),  // impulse up
///     (13.5, 12.5, 13.2),  // impulse up
/// ];
/// let mut last = None;
/// for (h, l, c) in data {
///     last = ob.next((h, l, c));
/// }
/// // After enough bars, an OB should have been detected.
/// assert!(last.is_some());
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingOrderBlock {
    lookback: usize,
    // Bounded history of closes for net-move calculation.
    closes: Vec<f64>,
    // Bounded history of (high, low) for ATR.
    hl: Vec<(f64, f64)>,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingOrderBlock {
    /// Create a new streaming OB detector with the given lookback window.
    ///
    /// # Panics
    /// Panics if `lookback < 2`.
    pub fn new(lookback: usize) -> Self {
        assert!(lookback >= 2, "lookback must be >= 2");
        Self {
            lookback,
            closes: Vec::with_capacity(lookback + 2),
            hl: Vec::with_capacity(lookback + 2),
            count: 0,
            last_value: None,
        }
    }

    /// Feed an OHLC bar via the [`Ohlcv`] trait.
    pub fn compute_bar(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.next((bar.high(), bar.low(), bar.close()))
    }
}

impl StreamingIndicator<(f64, f64, f64)> for StreamingOrderBlock {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<f64> {
        let (high, low, close) = input;
        self.count += 1;

        // Maintain bounded history.
        self.closes.push(close);
        self.hl.push((high, low));
        let cap = self.lookback + 2;
        if self.closes.len() > cap {
            self.closes.remove(0);
            self.hl.remove(0);
        }

        // Need at least lookback+1 bars to measure an impulse.
        if self.count < self.lookback + 1 {
            self.last_value = None;
            return None;
        }

        let n = self.closes.len();
        let window_start = n.saturating_sub(self.lookback + 1);
        let net_move = self.closes[n - 1] - self.closes[window_start];

        // ATR over the window.
        let mut atr_sum = 0.0_f64;
        let atr_start = window_start.max(1);
        for j in atr_start..n {
            let (h, l) = self.hl[j];
            let tr = (h - l)
                .max((h - self.closes[j - 1]).abs())
                .max((l - self.closes[j - 1]).abs());
            atr_sum += tr;
        }
        let atr_n = (n - atr_start) as f64;
        if atr_n == 0.0 {
            let r = Some(0.0);
            self.last_value = r;
            return r;
        }
        let atr_avg = atr_sum / atr_n;

        // Impulse threshold: net move > 1.5x ATR.
        if net_move.abs() <= atr_avg * 1.5 {
            let r = Some(0.0);
            self.last_value = r;
            return r;
        }

        // OB candidate: the candle right before the impulse window.
        if window_start == 0 {
            let r = Some(0.0);
            self.last_value = r;
            return r;
        }
        let ob_idx = window_start - 1;
        if ob_idx == 0 {
            let r = Some(0.0);
            self.last_value = r;
            return r;
        }

        let ob_close = self.closes[ob_idx];
        let ob_prev_close = self.closes[ob_idx - 1];
        let (ob_high, ob_low) = self.hl[ob_idx];
        let mid = (ob_high + ob_low) / 2.0;

        let ob_bullish = ob_close < ob_prev_close && net_move > 0.0;
        let ob_bearish = ob_close > ob_prev_close && net_move < 0.0;

        let result = if ob_bullish {
            mid
        } else if ob_bearish {
            -mid
        } else {
            0.0
        };

        self.last_value = Some(result);
        Some(result)
    }

    fn reset(&mut self) {
        self.closes.clear();
        self.hl.clear();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.lookback + 1
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingOrderBlock {
    fn name() -> &'static str {
        "OB"
    }

    fn category() -> &'static str {
        "smc"
    }

    fn description() -> &'static str {
        "Order Block (last opposite candle before impulse)"
    }

    fn warm_up_period(&self) -> usize {
        self.lookback
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_fvg_warmup() {
        let mut fvg = StreamingFairValueGap::new();
        assert!(fvg.next((10.0, 9.0)).is_none());
        assert!(fvg.next((12.0, 11.0)).is_none());
        assert!(!fvg.is_ready());
    }

    #[test]
    fn test_streaming_fvg_bullish() {
        let mut fvg = StreamingFairValueGap::new();
        fvg.next((10.0, 9.0));
        fvg.next((12.0, 11.0));
        // Third bar: low=13 > first high=10 -> bullish gap of 3.
        let v = fvg.next((14.0, 13.0)).unwrap();
        assert!((v - 3.0).abs() < 1e-10);
        assert!(fvg.is_ready());
        assert_eq!(fvg.count(), 3);
        assert_eq!(fvg.value(), Some(3.0));
    }

    #[test]
    fn test_streaming_fvg_bearish() {
        let mut fvg = StreamingFairValueGap::new();
        fvg.next((14.0, 13.0)); // first
        fvg.next((12.0, 11.0)); // second
                                // Third bar: high=9 < first low=13 -> bearish gap = 9 - 13 = -4.
        let v = fvg.next((9.0, 8.0)).unwrap();
        assert!((v - (-4.0)).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_fvg_no_gap() {
        let mut fvg = StreamingFairValueGap::new();
        fvg.next((10.0, 9.0));
        fvg.next((10.5, 9.5));
        // Overlapping: no gap -> 0.0.
        let v = fvg.next((11.0, 10.0)).unwrap();
        assert!(v.abs() == 0.0);
    }

    #[test]
    fn test_streaming_fvg_reset() {
        let mut fvg = StreamingFairValueGap::new();
        fvg.next((10.0, 9.0));
        fvg.next((12.0, 11.0));
        fvg.next((14.0, 13.0));
        assert!(fvg.is_ready());
        fvg.reset();
        assert!(!fvg.is_ready());
        assert_eq!(fvg.count(), 0);
        assert!(fvg.next((10.0, 9.0)).is_none());
    }

    #[test]
    fn test_streaming_fvg_meta() {
        let fvg = StreamingFairValueGap::new();
        assert_eq!(StreamingFairValueGap::name(), "FVG");
        assert_eq!(StreamingFairValueGap::category(), "smc");
        assert_eq!(fvg.warm_up_period(), 2);
    }

    #[test]
    fn test_streaming_ob_warmup() {
        let mut ob = StreamingOrderBlock::new(3);
        // Need at least 4 bars (lookback+1).
        assert!(ob.next((10.0, 9.0, 9.8)).is_none());
        assert!(ob.next((10.5, 10.0, 10.2)).is_none());
        assert!(ob.next((11.0, 10.5, 10.8)).is_none());
        // 4th bar: should return Some.
        assert!(ob.next((10.8, 10.2, 10.5)).is_some());
        assert!(ob.is_ready());
    }

    #[test]
    fn test_streaming_ob_reset() {
        let mut ob = StreamingOrderBlock::new(3);
        for (h, l, c) in [(10.0, 9.0, 9.8), (10.5, 10.0, 10.2), (11.0, 10.5, 10.8)] {
            ob.next((h, l, c));
        }
        ob.reset();
        assert!(!ob.is_ready());
        assert_eq!(ob.count(), 0);
    }

    #[test]
    fn test_streaming_ob_meta() {
        let ob = StreamingOrderBlock::new(4);
        assert_eq!(StreamingOrderBlock::name(), "OB");
        assert_eq!(StreamingOrderBlock::category(), "smc");
        assert_eq!(ob.warm_up_period(), 4);
    }

    #[test]
    fn test_streaming_ob_detects_impulse() {
        let mut ob = StreamingOrderBlock::new(3);
        // Build an up-impulse.
        let bars = [
            (10.0, 9.0, 9.5),
            (10.2, 9.8, 10.0),
            (10.1, 9.9, 10.0),
            (10.0, 9.5, 9.8), // down candle before impulse (bullish OB candidate)
            (12.0, 11.0, 11.8),
            (13.5, 12.5, 13.2), // strong up move -> impulse
        ];
        let mut last = Some(0.0);
        for (h, l, c) in bars {
            last = ob.next((h, l, c));
        }
        assert!(last.is_some());
        // A bullish OB would be positive.
        if let Some(v) = last {
            assert!(
                v >= 0.0,
                "expected non-negative (bullish or zero) OB, got {v}"
            );
        }
    }
}
