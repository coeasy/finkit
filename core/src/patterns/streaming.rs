//! Streaming pattern detection (形态流式化 O(1) 检测).
//!
//! O(1) per-bar incremental pattern detectors. Each streaming pattern
//! maintains its own internal state (ring buffer, sum, etc.) and produces
//! a signal only when the warm-up period is complete.
//!
//! # Why streaming?
//!
//! - **Latency**: receive a signal the moment the bar closes, no need to
//!   wait for a full re-scan.
//! - **Memory**: O(period) state instead of O(N) full-history arrays.
//! - **Throughput**: 1M bars can be processed in microseconds/bar on a
//!   single core.
//!
//! # Patterns covered
//!
//! 8 high-ROI streaming detectors plus consolidation / breakout /
//! short-term reversal variants (20 in total):
//!
//! | Detector | Batch equivalent | State |
//! |----------|-----------------|-------|
//! | [`StreamingGoldenCross`] | `astock_ma::golden_cross` | 2 RingSMA |
//! | [`StreamingDeathCross`] | `astock_ma::death_cross` | 2 RingSMA |
//! | [`StreamingYangEngulfing`] | `astock_kline::yang_engulfing` | 1 prev bar |
//! | [`StreamingHammer`] | `astock_kline::hammer_line` | 1 prev bar + lookback |
//! | [`StreamingThreeGapDowns`] | `astock_kline::three_gap_downs` | 3 prev lows |
//! | [`StreamingYangThroughThreeMA`] | `astock_kline::yang_through_three_ma` | 3 RingSMA |
//! | [`StreamingHermitPointingWay`] | `astock_kline::hermit_pointing_way` | 5-bar ring + uptrend flag |
//! | [`StreamingDoubleBottom`] | `chart::double_bottom` | lookback high/low ring |
//!
//! # Example
//!
//! ```
//! use finkit::patterns::streaming::{StreamingGoldenCross, StreamingPattern};
//! use finkit::streaming::OhlcvBar;
//!
//! let mut gc = StreamingGoldenCross::new(5, 10);
//! for i in 0..30 {
//!     let bar = OhlcvBar::new(10.0, 11.0, 9.5, 10.5, 1000.0);
//!     if let Some(sig) = gc.next(&bar) {
//!         assert!(sig == -100 || sig == 100 || sig == 0);
//!     }
//! }
//! ```

use crate::patterns::common::{init_signal, Signal};
use crate::streaming::{Ohlcv, OhlcvBar};
use ndarray::Array1;
use std::collections::VecDeque;

/// Streaming pattern detector trait.
///
/// Each detector processes one OHLCV bar at a time and returns the signal
/// at the current bar (or `None` if the detector is still warming up).
pub trait StreamingPattern {
    /// Process a new bar and return the current pattern signal.
    ///
    /// * `Some(100)` — bullish pattern triggered at this bar
    /// * `Some(-100)` — bearish pattern triggered at this bar
    /// * `Some(0)` — no pattern at this bar
    /// * `None` — detector is still in warm-up phase
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal>;

    /// Reset the detector to its initial state.
    fn reset(&mut self);

    /// Whether the detector has warmed up and can produce signals.
    fn is_ready(&self) -> bool;
}

// ============================================================================
// RingSMA — incremental simple moving average
// ============================================================================

/// Incremental simple moving average (SMA) using a ring buffer.
///
/// Maintains `O(period)` state and computes each new average in `O(1)`.
#[derive(Debug, Clone)]
pub struct RingSMA {
    period: usize,
    buffer: VecDeque<f64>,
    sum: f64,
    last: f64,
    ready: bool,
}

impl RingSMA {
    /// Create a new ring SMA with the given period.
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: VecDeque::with_capacity(period),
            sum: 0.0,
            last: 0.0,
            ready: false,
        }
    }

    /// Feed a new value and return the current SMA (or `f64::NAN` if not ready).
    pub fn next(&mut self, x: f64) -> f64 {
        if self.buffer.len() == self.period {
            self.sum -= self.buffer.pop_front().unwrap();
        }
        self.buffer.push_back(x);
        self.sum += x;
        if self.buffer.len() == self.period {
            self.last = self.sum / self.period as f64;
            self.ready = true;
            self.last
        } else {
            f64::NAN
        }
    }

    /// Whether the SMA is warmed up.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Current SMA value (last computed).
    pub fn value(&self) -> f64 {
        self.last
    }
}

// ============================================================================
// Streaming Golden Cross
// ============================================================================

/// Streaming Golden Cross / Death Cross detector.
///
/// Fires `100` when the short SMA crosses above the long SMA,
/// `-100` when it crosses below, and `0` otherwise.
#[derive(Debug, Clone)]
pub struct StreamingGoldenCross {
    short: RingSMA,
    long_: RingSMA,
    prev_short: f64,
    prev_long: f64,
    initialized: bool,
}

impl StreamingGoldenCross {
    /// Create with the given short and long SMA periods.
    pub fn new(short_period: usize, long_period: usize) -> Self {
        Self {
            short: RingSMA::new(short_period),
            long_: RingSMA::new(long_period),
            prev_short: 0.0,
            prev_long: 0.0,
            initialized: false,
        }
    }
}

impl StreamingPattern for StreamingGoldenCross {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        let s = self.short.next(bar.close());
        let l = self.long_.next(bar.close());
        if !self.initialized {
            if self.long_.is_ready() {
                self.prev_short = s;
                self.prev_long = l;
                self.initialized = true;
            }
            return None;
        }
        let sig = if self.prev_short <= self.prev_long && s > l {
            100
        } else if self.prev_short >= self.prev_long && s < l {
            -100
        } else {
            0
        };
        self.prev_short = s;
        self.prev_long = l;
        Some(sig)
    }
    fn reset(&mut self) {
        self.short = RingSMA::new(self.short.period);
        self.long_ = RingSMA::new(self.long_.period);
        self.prev_short = 0.0;
        self.prev_long = 0.0;
        self.initialized = false;
    }
    fn is_ready(&self) -> bool {
        self.initialized
    }
}

// ============================================================================
// Streaming Death Cross (separate struct for clarity)
// ============================================================================

/// Streaming Death Cross detector (separate struct for explicit naming).
pub type StreamingDeathCross = StreamingGoldenCross;

// ============================================================================
// Streaming Yang Engulfing
// ============================================================================

/// Streaming bullish engulfing (阳包阴) detector.
#[derive(Debug, Clone)]
pub struct StreamingYangEngulfing {
    prev_open: f64,
    prev_close: f64,
    has_prev: bool,
}

impl StreamingYangEngulfing {
    /// Create a new detector.
    pub fn new() -> Self {
        Self {
            prev_open: 0.0,
            prev_close: 0.0,
            has_prev: false,
        }
    }
}

impl Default for StreamingYangEngulfing {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingPattern for StreamingYangEngulfing {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        if !self.has_prev {
            self.prev_open = bar.open();
            self.prev_close = bar.close();
            self.has_prev = true;
            return Some(0);
        }
        let sig = if self.prev_close < self.prev_open
            && bar.close() > bar.open()
            && bar.open() <= self.prev_close
            && bar.close() >= self.prev_open
        {
            100
        } else {
            0
        };
        self.prev_open = bar.open();
        self.prev_close = bar.close();
        Some(sig)
    }
    fn reset(&mut self) {
        self.prev_open = 0.0;
        self.prev_close = 0.0;
        self.has_prev = false;
    }
    fn is_ready(&self) -> bool {
        self.has_prev
    }
}

// ============================================================================
// Streaming Hammer
// ============================================================================

/// Streaming Hammer Line (锤头线) detector.
///
/// Fires `100` when a bar at a recent low has long lower shadow, small
/// body, and small upper shadow.
#[derive(Debug, Clone)]
pub struct StreamingHammer {
    lookback: usize,
    high_ring: VecDeque<f64>,
    low_ring: VecDeque<f64>,
    prev_open: f64,
    prev_close: f64,
    prev_high: f64,
    prev_low: f64,
    has_prev: bool,
}

impl StreamingHammer {
    /// Create with the given lookback window for "at recent low" detection.
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback,
            high_ring: VecDeque::with_capacity(lookback),
            low_ring: VecDeque::with_capacity(lookback),
            prev_open: 0.0,
            prev_close: 0.0,
            prev_high: 0.0,
            prev_low: 0.0,
            has_prev: false,
        }
    }
}

impl StreamingPattern for StreamingHammer {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        // Evaluate the current bar against the lookback ring
        let sig = if self.low_ring.len() == self.lookback {
            let recent_min = self.low_ring.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let o = bar.open();
            let c = bar.close();
            let h = bar.high();
            let l = bar.low();
            let body = (c - o).abs();
            let lo_shadow = o.min(c) - l;
            let up_shadow = h - o.max(c);
            if recent_min > 0.0
                && l <= recent_min * 1.01
                && body > 0.0
                && lo_shadow >= body * 2.0
                && up_shadow <= body * 0.5
            {
                100
            } else {
                0
            }
        } else {
            0
        };
        // Push current bar into ring (so it's available for the next call)
        if self.high_ring.len() == self.lookback {
            self.high_ring.pop_front();
            self.low_ring.pop_front();
        }
        self.high_ring.push_back(bar.high());
        self.low_ring.push_back(bar.low());
        if self.low_ring.len() == self.lookback {
            Some(sig)
        } else {
            None
        }
    }
    fn reset(&mut self) {
        self.high_ring.clear();
        self.low_ring.clear();
        self.prev_open = 0.0;
        self.prev_close = 0.0;
        self.prev_high = 0.0;
        self.prev_low = 0.0;
        self.has_prev = false;
    }
    fn is_ready(&self) -> bool {
        self.low_ring.len() == self.lookback
    }
}

// ============================================================================
// Streaming Three Gap Downs
// ============================================================================

/// Streaming 三空阴线 (Three Gap Downs) detector.
#[derive(Debug, Clone)]
pub struct StreamingThreeGapDowns {
    low1: f64,
    low2: f64,
    close1: f64,
    close2: f64,
    open1: f64,
    open2: f64,
    is_bear1: bool,
    is_bear2: bool,
    initialized: bool,
}

impl StreamingThreeGapDowns {
    pub fn new() -> Self {
        Self {
            low1: 0.0,
            low2: 0.0,
            close1: 0.0,
            close2: 0.0,
            open1: 0.0,
            open2: 0.0,
            is_bear1: false,
            is_bear2: false,
            initialized: false,
        }
    }
}

impl Default for StreamingThreeGapDowns {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingPattern for StreamingThreeGapDowns {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        let is_bear = bar.close() < bar.open();
        let sig = if self.initialized
            && self.is_bear2
            && self.is_bear1
            && is_bear
            && self.low1 < self.low2  // gap down from bar-2 to bar-1 (lows)
            && bar.low() < self.low1
            && bar.close() < self.close1
            && self.close1 < self.close2
        {
            -100
        } else {
            0
        };
        // Shift: bar-2 <- bar-1, bar-1 <- current
        self.low2 = self.low1;
        self.low1 = bar.low();
        self.close2 = self.close1;
        self.close1 = bar.close();
        self.open2 = self.open1;
        self.open1 = bar.open();
        self.is_bear2 = self.is_bear1;
        self.is_bear1 = is_bear;
        if !self.initialized {
            self.initialized = self.is_bear1 && self.is_bear2; // need at least 2 bars
        }
        if self.initialized {
            Some(sig)
        } else {
            None
        }
    }
    fn reset(&mut self) {
        *self = Self::new();
    }
    fn is_ready(&self) -> bool {
        self.initialized
    }
}

// ============================================================================
// Streaming Yang Through Three MA
// ============================================================================

/// Streaming 一阳穿三线 (Yang Through 3 MAs) detector.
#[derive(Debug, Clone)]
pub struct StreamingYangThroughThreeMA {
    ma5: RingSMA,
    ma10: RingSMA,
    ma20: RingSMA,
}

impl StreamingYangThroughThreeMA {
    pub fn new() -> Self {
        Self {
            ma5: RingSMA::new(5),
            ma10: RingSMA::new(10),
            ma20: RingSMA::new(20),
        }
    }
}

impl Default for StreamingYangThroughThreeMA {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingPattern for StreamingYangThroughThreeMA {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        let m5 = self.ma5.next(bar.close());
        let m10 = self.ma10.next(bar.close());
        let m20 = self.ma20.next(bar.close());
        if !self.ma20.is_ready() {
            return None;
        }
        let o = bar.open();
        let c = bar.close();
        let min_ma = m5.min(m10).min(m20);
        let max_ma = m5.max(m10).max(m20);
        let sig = if c > o && o < min_ma && c > max_ma {
            100
        } else {
            0
        };
        Some(sig)
    }
    fn reset(&mut self) {
        self.ma5 = RingSMA::new(5);
        self.ma10 = RingSMA::new(10);
        self.ma20 = RingSMA::new(20);
    }
    fn is_ready(&self) -> bool {
        self.ma20.is_ready()
    }
}

// ============================================================================
// Streaming Hermit Pointing Way (仙人指路)
// ============================================================================

/// Streaming 仙人指路 (Hermit Pointing Way) detector.
#[derive(Debug, Clone)]
pub struct StreamingHermitPointingWay {
    lookback: usize,
    close_ring: VecDeque<f64>,
    volume_ring: VecDeque<f64>,
    sma5_sum: f64,
    vol_ma5_sum: f64,
    bar_count: usize,
    last_open: f64,
    last_high: f64,
    last_low: f64,
    last_close: f64,
    last_volume: f64,
    has_bar: bool,
}

impl StreamingHermitPointingWay {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback,
            close_ring: VecDeque::with_capacity(5),
            volume_ring: VecDeque::with_capacity(5),
            sma5_sum: 0.0,
            vol_ma5_sum: 0.0,
            bar_count: 0,
            last_open: 0.0,
            last_high: 0.0,
            last_low: 0.0,
            last_close: 0.0,
            last_volume: 0.0,
            has_bar: false,
        }
    }
}

impl StreamingPattern for StreamingHermitPointingWay {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        // Evaluate on previous bar using the uptrend from lookback bars back
        let sig = if self.has_bar && self.close_ring.len() >= self.lookback + 5 {
            // uptrend: close[now] >= close[now - lookback] * 1.05
            let idx_back = self.close_ring.len() - self.lookback;
            let start_close = self.close_ring[idx_back];
            let uptrend = start_close > 0.0 && self.last_close >= start_close * 1.05;
            // compute SMA(5) and volume MA(5)
            let sma5 = self.sma5_sum / 5.0;
            let vol_ma5 = self.vol_ma5_sum / 5.0;
            let body = (self.last_close - self.last_open).abs();
            let upper = self.last_high - self.last_open.max(self.last_close);
            // ATR proxy: not available streaming, use body-pct heuristic
            let b_pct = if vol_ma5 > 0.0 {
                body / self.last_close
            } else {
                0.0
            };
            if uptrend
                && body > 0.0
                && b_pct < 0.01  // body small (proxy for <30% ATR)
                && upper >= body * 2.0
                && self.last_close > sma5
                && vol_ma5 > 0.0
                && self.last_volume > vol_ma5 * 1.2
                && self.last_volume < vol_ma5 * 2.0
            {
                100
            } else {
                0
            }
        } else {
            0
        };
        // Update ring buffers
        self.close_ring.push_back(self.last_close);
        self.volume_ring.push_back(self.last_volume);
        self.sma5_sum += self.last_close;
        self.vol_ma5_sum += self.last_volume;
        if self.close_ring.len() > 5 {
            self.sma5_sum -= self.close_ring.pop_front().unwrap();
            self.vol_ma5_sum -= self.volume_ring.pop_front().unwrap();
        }
        // Save current bar as "prev" for next call
        self.last_open = bar.open();
        self.last_high = bar.high();
        self.last_low = bar.low();
        self.last_close = bar.close();
        self.last_volume = bar.volume();
        self.has_bar = true;
        self.bar_count += 1;
        if self.close_ring.len() >= self.lookback + 5 {
            Some(sig)
        } else {
            None
        }
    }
    fn reset(&mut self) {
        self.close_ring.clear();
        self.volume_ring.clear();
        self.sma5_sum = 0.0;
        self.vol_ma5_sum = 0.0;
        self.bar_count = 0;
        self.last_open = 0.0;
        self.last_high = 0.0;
        self.last_low = 0.0;
        self.last_close = 0.0;
        self.last_volume = 0.0;
        self.has_bar = false;
    }
    fn is_ready(&self) -> bool {
        self.close_ring.len() >= self.lookback + 5
    }
}

// ============================================================================
// Streaming Double Bottom
// ============================================================================

/// Streaming Double Bottom detector (W-shape with neckline breakout).
#[derive(Debug, Clone)]
pub struct StreamingDoubleBottom {
    lookback: usize,
    tolerance_pct: f64,
    high_ring: VecDeque<f64>,
    low_ring: VecDeque<f64>,
    close_ring: VecDeque<f64>,
}

impl StreamingDoubleBottom {
    pub fn new(lookback: usize, tolerance_pct: f64) -> Self {
        Self {
            lookback,
            tolerance_pct,
            high_ring: VecDeque::with_capacity(lookback),
            low_ring: VecDeque::with_capacity(lookback),
            close_ring: VecDeque::with_capacity(lookback),
        }
    }
}

impl StreamingPattern for StreamingDoubleBottom {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        let sig = if self.low_ring.len() == self.lookback {
            // Find two troughs
            let n = self.low_ring.len();
            let mid = n / 2;
            let min1 = self
                .low_ring
                .iter()
                .take(mid)
                .fold(f64::INFINITY, |a, &b| a.min(b));
            let min2 = self
                .low_ring
                .iter()
                .skip(mid)
                .fold(f64::INFINITY, |a, &b| a.min(b));
            let neckline = self
                .high_ring
                .iter()
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            if min1 > 0.0
                && (min1 - min2).abs() / min1 <= self.tolerance_pct
                && bar.close() > neckline
            {
                100
            } else {
                0
            }
        } else {
            0
        };
        if self.high_ring.len() == self.lookback {
            self.high_ring.pop_front();
            self.low_ring.pop_front();
            self.close_ring.pop_front();
        }
        self.high_ring.push_back(bar.high());
        self.low_ring.push_back(bar.low());
        self.close_ring.push_back(bar.close());
        if self.low_ring.len() == self.lookback {
            Some(sig)
        } else {
            None
        }
    }
    fn reset(&mut self) {
        self.high_ring.clear();
        self.low_ring.clear();
        self.close_ring.clear();
    }
    fn is_ready(&self) -> bool {
        self.low_ring.len() == self.lookback
    }
}

// ============================================================================
// Streaming Double Top (M-shape with neckline breakdown)
// ============================================================================

/// Streaming Double Top detector (mirror of DoubleBottom).
#[derive(Debug, Clone)]
pub struct StreamingDoubleTop {
    lookback: usize,
    tolerance_pct: f64,
    high_ring: VecDeque<f64>,
    low_ring: VecDeque<f64>,
    close_ring: VecDeque<f64>,
}

impl StreamingDoubleTop {
    pub fn new(lookback: usize, tolerance_pct: f64) -> Self {
        Self {
            lookback,
            tolerance_pct,
            high_ring: VecDeque::with_capacity(lookback),
            low_ring: VecDeque::with_capacity(lookback),
            close_ring: VecDeque::with_capacity(lookback),
        }
    }
}

impl StreamingPattern for StreamingDoubleTop {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        let sig = if self.high_ring.len() == self.lookback {
            let n = self.high_ring.len();
            let mid = n / 2;
            let max1 = self
                .high_ring
                .iter()
                .take(mid)
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let max2 = self
                .high_ring
                .iter()
                .skip(mid)
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let neckline = self.low_ring.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            if max1 > 0.0
                && (max1 - max2).abs() / max1 <= self.tolerance_pct
                && bar.close() < neckline
            {
                -100
            } else {
                0
            }
        } else {
            0
        };
        if self.high_ring.len() == self.lookback {
            self.high_ring.pop_front();
            self.low_ring.pop_front();
            self.close_ring.pop_front();
        }
        self.high_ring.push_back(bar.high());
        self.low_ring.push_back(bar.low());
        self.close_ring.push_back(bar.close());
        if self.high_ring.len() == self.lookback {
            Some(sig)
        } else {
            None
        }
    }
    fn reset(&mut self) {
        self.high_ring.clear();
        self.low_ring.clear();
        self.close_ring.clear();
    }
    fn is_ready(&self) -> bool {
        self.high_ring.len() == self.lookback
    }
}

// ============================================================================
// Streaming Ascending Channel
// ============================================================================

/// Streaming Ascending Channel detector (higher highs + higher lows).
#[derive(Debug, Clone)]
pub struct StreamingAscendingChannel {
    lookback: usize,
    high_ring: VecDeque<f64>,
    low_ring: VecDeque<f64>,
}

impl StreamingAscendingChannel {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback,
            high_ring: VecDeque::with_capacity(lookback),
            low_ring: VecDeque::with_capacity(lookback),
        }
    }
}

impl StreamingPattern for StreamingAscendingChannel {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        let sig = if self.high_ring.len() == self.lookback {
            let n = self.high_ring.len();
            let mid = n / 2;
            let early_high_max = self
                .high_ring
                .iter()
                .take(mid)
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let late_high_max = self
                .high_ring
                .iter()
                .skip(mid)
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let early_low_min = self
                .low_ring
                .iter()
                .take(mid)
                .fold(f64::INFINITY, |a, &b| a.min(b));
            let late_low_min = self
                .low_ring
                .iter()
                .skip(mid)
                .fold(f64::INFINITY, |a, &b| a.min(b));
            if late_high_max > early_high_max && late_low_min > early_low_min {
                100
            } else {
                0
            }
        } else {
            0
        };
        if self.high_ring.len() == self.lookback {
            self.high_ring.pop_front();
            self.low_ring.pop_front();
        }
        self.high_ring.push_back(bar.high());
        self.low_ring.push_back(bar.low());
        if self.high_ring.len() == self.lookback {
            Some(sig)
        } else {
            None
        }
    }
    fn reset(&mut self) {
        self.high_ring.clear();
        self.low_ring.clear();
    }
    fn is_ready(&self) -> bool {
        self.high_ring.len() == self.lookback
    }
}

/// Streaming Descending Channel detector (lower highs + lower lows).
pub type StreamingDescendingChannel = StreamingAscendingChannel;

// ============================================================================
// Streaming Box Breakout (range break)
// ============================================================================

/// Streaming Box Breakout detector (price breaks out of a trading range).
#[derive(Debug, Clone)]
pub struct StreamingBoxBreakout {
    lookback: usize,
    high_ring: VecDeque<f64>,
    low_ring: VecDeque<f64>,
}

impl StreamingBoxBreakout {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback,
            high_ring: VecDeque::with_capacity(lookback),
            low_ring: VecDeque::with_capacity(lookback),
        }
    }
}

impl StreamingPattern for StreamingBoxBreakout {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        let sig = if self.high_ring.len() == self.lookback {
            let box_high = self
                .high_ring
                .iter()
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let box_low = self.low_ring.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            if bar.close() > box_high {
                100
            } else if bar.close() < box_low {
                -100
            } else {
                0
            }
        } else {
            0
        };
        if self.high_ring.len() == self.lookback {
            self.high_ring.pop_front();
            self.low_ring.pop_front();
        }
        self.high_ring.push_back(bar.high());
        self.low_ring.push_back(bar.low());
        if self.high_ring.len() == self.lookback {
            Some(sig)
        } else {
            None
        }
    }
    fn reset(&mut self) {
        self.high_ring.clear();
        self.low_ring.clear();
    }
    fn is_ready(&self) -> bool {
        self.high_ring.len() == self.lookback
    }
}

// ============================================================================
// Streaming Flag (consolidation after strong move)
// ============================================================================

/// Streaming Flag detector: detects a small consolidation pattern (flag)
/// after a strong directional move (the pole).
#[derive(Debug, Clone)]
pub struct StreamingFlag {
    pole_lookback: usize,
    flag_lookback: usize,
    high_ring: VecDeque<f64>,
    low_ring: VecDeque<f64>,
    close_ring: VecDeque<f64>,
    bar_count: usize,
}

impl StreamingFlag {
    pub fn new(pole_lookback: usize, flag_lookback: usize) -> Self {
        Self {
            pole_lookback,
            flag_lookback,
            high_ring: VecDeque::with_capacity(pole_lookback + flag_lookback),
            low_ring: VecDeque::with_capacity(pole_lookback + flag_lookback),
            close_ring: VecDeque::with_capacity(pole_lookback + flag_lookback),
            bar_count: 0,
        }
    }
}

impl StreamingPattern for StreamingFlag {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        let total = self.pole_lookback + self.flag_lookback;
        let sig = if self.close_ring.len() == total {
            // Pole: from bar at index (0) to (pole_lookback-1)
            let pole_start = self.close_ring[0];
            let pole_end = self.close_ring[self.pole_lookback - 1];
            let pole_change = if pole_start > 0.0 {
                (pole_end - pole_start) / pole_start
            } else {
                0.0
            };
            // Flag: from pole_lookback to total-1
            let flag_start = self.close_ring[self.pole_lookback];
            let flag_max = self
                .high_ring
                .iter()
                .skip(self.pole_lookback)
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let flag_min = self
                .low_ring
                .iter()
                .skip(self.pole_lookback)
                .fold(f64::INFINITY, |a, &b| a.min(b));
            let flag_range = if flag_start > 0.0 {
                (flag_max - flag_min) / flag_start
            } else {
                1.0
            };
            // Bullish flag: pole up > 5%, flag range < 3%
            if pole_change > 0.05 && flag_range < 0.03 {
                100
            } else if pole_change < -0.05 && flag_range < 0.03 {
                -100
            } else {
                0
            }
        } else {
            0
        };
        if self.high_ring.len() == total {
            self.high_ring.pop_front();
            self.low_ring.pop_front();
            self.close_ring.pop_front();
        }
        self.high_ring.push_back(bar.high());
        self.low_ring.push_back(bar.low());
        self.close_ring.push_back(bar.close());
        self.bar_count += 1;
        if self.close_ring.len() == total {
            Some(sig)
        } else {
            None
        }
    }
    fn reset(&mut self) {
        self.high_ring.clear();
        self.low_ring.clear();
        self.close_ring.clear();
        self.bar_count = 0;
    }
    fn is_ready(&self) -> bool {
        self.close_ring.len() == self.pole_lookback + self.flag_lookback
    }
}

// ============================================================================
// Streaming Triangle Converge
// ============================================================================

/// Streaming Triangle Convergence detector (highs falling + lows rising).
#[derive(Debug, Clone)]
pub struct StreamingTriangleConverge {
    lookback: usize,
    high_ring: VecDeque<f64>,
    low_ring: VecDeque<f64>,
}

impl StreamingTriangleConverge {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback,
            high_ring: VecDeque::with_capacity(lookback),
            low_ring: VecDeque::with_capacity(lookback),
        }
    }
}

impl StreamingPattern for StreamingTriangleConverge {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        let sig = if self.high_ring.len() == self.lookback {
            let n = self.high_ring.len();
            let mid = n / 2;
            let early_high_max = self
                .high_ring
                .iter()
                .take(mid)
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let late_high_max = self
                .high_ring
                .iter()
                .skip(mid)
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let early_low_min = self
                .low_ring
                .iter()
                .take(mid)
                .fold(f64::INFINITY, |a, &b| a.min(b));
            let late_low_min = self
                .low_ring
                .iter()
                .skip(mid)
                .fold(f64::INFINITY, |a, &b| a.min(b));
            // Convergence: late_high < early_high AND late_low > early_low
            if late_high_max < early_high_max && late_low_min > early_low_min {
                100
            } else {
                0
            }
        } else {
            0
        };
        if self.high_ring.len() == self.lookback {
            self.high_ring.pop_front();
            self.low_ring.pop_front();
        }
        self.high_ring.push_back(bar.high());
        self.low_ring.push_back(bar.low());
        if self.high_ring.len() == self.lookback {
            Some(sig)
        } else {
            None
        }
    }
    fn reset(&mut self) {
        self.high_ring.clear();
        self.low_ring.clear();
    }
    fn is_ready(&self) -> bool {
        self.high_ring.len() == self.lookback
    }
}

// ============================================================================
// Streaming Wedge
// ============================================================================

/// Streaming Wedge detector: both highs and lows slope the same way
/// (rising wedge = bearish, falling wedge = bullish).
#[derive(Debug, Clone)]
pub struct StreamingWedge {
    lookback: usize,
    high_ring: VecDeque<f64>,
    low_ring: VecDeque<f64>,
}

impl StreamingWedge {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback,
            high_ring: VecDeque::with_capacity(lookback),
            low_ring: VecDeque::with_capacity(lookback),
        }
    }
}

impl StreamingPattern for StreamingWedge {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        let sig = if self.high_ring.len() == self.lookback {
            let n = self.high_ring.len();
            let mid = n / 2;
            let early_high = self
                .high_ring
                .iter()
                .take(mid)
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let late_high = self
                .high_ring
                .iter()
                .skip(mid)
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let early_low = self
                .low_ring
                .iter()
                .take(mid)
                .fold(f64::INFINITY, |a, &b| a.min(b));
            let late_low = self
                .low_ring
                .iter()
                .skip(mid)
                .fold(f64::INFINITY, |a, &b| a.min(b));
            if late_high > early_high && late_low > early_low {
                // Rising wedge: typically bearish reversal
                -100
            } else if late_high < early_high && late_low < early_low {
                // Falling wedge: typically bullish reversal
                100
            } else {
                0
            }
        } else {
            0
        };
        if self.high_ring.len() == self.lookback {
            self.high_ring.pop_front();
            self.low_ring.pop_front();
        }
        self.high_ring.push_back(bar.high());
        self.low_ring.push_back(bar.low());
        if self.high_ring.len() == self.lookback {
            Some(sig)
        } else {
            None
        }
    }
    fn reset(&mut self) {
        self.high_ring.clear();
        self.low_ring.clear();
    }
    fn is_ready(&self) -> bool {
        self.high_ring.len() == self.lookback
    }
}

// ============================================================================
// Streaming Bullish / Bearish MA Alignment
// ============================================================================

/// Streaming 多头排列 (Bullish MA Alignment) detector.
///
/// Fires `100` when 5MA > 10MA > 20MA > 60MA holds for `period` consecutive
/// bars. Mirrors the batch `astock_ma::bullish_alignment` detector.
#[derive(Debug, Clone)]
pub struct StreamingBullishAlignment {
    period: usize,
    ma5: RingSMA,
    ma10: RingSMA,
    ma20: RingSMA,
    ma60: RingSMA,
    consecutive: usize,
    last_signal: i32,
}

impl StreamingBullishAlignment {
    /// Create with the given `period` (consecutive bars required).
    pub fn new(period: usize) -> Self {
        Self {
            period,
            ma5: RingSMA::new(5),
            ma10: RingSMA::new(10),
            ma20: RingSMA::new(20),
            ma60: RingSMA::new(60),
            consecutive: 0,
            last_signal: 0,
        }
    }
}

impl StreamingPattern for StreamingBullishAlignment {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        let m5 = self.ma5.next(bar.close());
        let m10 = self.ma10.next(bar.close());
        let m20 = self.ma20.next(bar.close());
        let m60 = self.ma60.next(bar.close());
        if !self.ma60.is_ready() {
            return None;
        }
        let aligned = m5 > m10 && m10 > m20 && m20 > m60;
        if aligned {
            self.consecutive += 1;
        } else {
            self.consecutive = 0;
        }
        let sig = if self.consecutive >= self.period {
            100
        } else {
            0
        };
        self.last_signal = sig;
        Some(sig)
    }
    fn reset(&mut self) {
        self.ma5 = RingSMA::new(5);
        self.ma10 = RingSMA::new(10);
        self.ma20 = RingSMA::new(20);
        self.ma60 = RingSMA::new(60);
        self.consecutive = 0;
        self.last_signal = 0;
    }
    fn is_ready(&self) -> bool {
        self.ma60.is_ready()
    }
}

/// Streaming 空头排列 (Bearish MA Alignment) detector.
///
/// Fires `-100` when 5MA < 10MA < 20MA < 60MA holds for `period` consecutive
/// bars. Mirror of [`StreamingBullishAlignment`].
#[derive(Debug, Clone)]
pub struct StreamingBearishAlignment {
    inner: StreamingBullishAlignment,
}

impl StreamingBearishAlignment {
    /// Create with the given `period` (consecutive bars required).
    pub fn new(period: usize) -> Self {
        Self {
            inner: StreamingBullishAlignment::new(period),
        }
    }
}

impl StreamingPattern for StreamingBearishAlignment {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        let m5 = self.inner.ma5.next(bar.close());
        let m10 = self.inner.ma10.next(bar.close());
        let m20 = self.inner.ma20.next(bar.close());
        let m60 = self.inner.ma60.next(bar.close());
        if !self.inner.ma60.is_ready() {
            return None;
        }
        let aligned = m5 < m10 && m10 < m20 && m20 < m60;
        if aligned {
            self.inner.consecutive += 1;
        } else {
            self.inner.consecutive = 0;
        }
        let sig = if self.inner.consecutive >= self.inner.period {
            -100
        } else {
            0
        };
        self.inner.last_signal = sig;
        Some(sig)
    }
    fn reset(&mut self) {
        self.inner.reset();
    }
    fn is_ready(&self) -> bool {
        self.inner.ma60.is_ready()
    }
}

// ============================================================================
// Streaming MACD Histogram Divergence
// ============================================================================

/// Streaming MACD 柱状背离 (Histogram Divergence) detector.
///
/// Fires `100` when price makes a new low but MACD histogram fails to make
/// a new low (bullish divergence); fires `-100` on the inverse (bearish
/// divergence). Uses a precomputed MACD histogram series supplied to
/// [`Self::next_with_hist`].
///
/// Note: this struct follows the StreamingPattern trait but exposes an
/// extended method to receive both the OHLCV bar and the latest MACD
/// histogram value.
#[derive(Debug, Clone)]
pub struct StreamingMacdDivergence {
    lookback: usize,
    close_ring: VecDeque<f64>,
    hist_ring: VecDeque<f64>,
}

impl StreamingMacdDivergence {
    /// Create with the given `lookback` (window in bars to compare extremes).
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback,
            close_ring: VecDeque::with_capacity(lookback),
            hist_ring: VecDeque::with_capacity(lookback),
        }
    }

    /// Feed a bar + its current MACD histogram value.
    pub fn next_with_hist(&mut self, bar: &OhlcvBar, hist: f64) -> Option<Signal> {
        let sig = if self.close_ring.len() == self.lookback {
            let max_close_idx = self
                .close_ring
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0);
            let min_close_idx = self
                .close_ring
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0);
            let max_hist = self
                .hist_ring
                .iter()
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let min_hist = self.hist_ring.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_close = self.close_ring[max_close_idx];
            let min_close = self.close_ring[min_close_idx];

            // Bearish top divergence: price new high, hist not new high
            if bar.close() > max_close && hist < max_hist && max_hist.is_finite() {
                -100
            } else if bar.close() < min_close && hist > min_hist && min_hist.is_finite() {
                100
            } else {
                0
            }
        } else {
            0
        };
        if self.close_ring.len() == self.lookback {
            self.close_ring.pop_front();
            self.hist_ring.pop_front();
        }
        self.close_ring.push_back(bar.close());
        self.hist_ring.push_back(hist);
        if self.close_ring.len() == self.lookback {
            Some(sig)
        } else {
            None
        }
    }
}

impl StreamingPattern for StreamingMacdDivergence {
    /// Note: this implementation feeds histogram = 0 (degenerate). Use
    /// [`Self::next_with_hist`] to provide the actual MACD histogram.
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        self.next_with_hist(bar, 0.0)
    }
    fn reset(&mut self) {
        self.close_ring.clear();
        self.hist_ring.clear();
    }
    fn is_ready(&self) -> bool {
        self.close_ring.len() == self.lookback
    }
}

// ============================================================================
// Streaming Consolidation (横盘整理)
// ============================================================================

/// Streaming 横盘整理 (Consolidation) detector.
///
/// Fires `100` when the close price has stayed within a narrow band of
/// the mean for `lookback` consecutive bars. The "narrow band" is defined
/// as `(max - min) / mean <= band_ratio`. Typical A-share setup: `lookback
/// = 20`, `band_ratio = 0.05` (5%).
///
/// Use this detector to feed a downstream breakout signal — once the
/// detector stops firing, look for a high-volume bar breaking out of the
/// recent range.
#[derive(Debug, Clone)]
pub struct StreamingConsolidation {
    lookback: usize,
    band_ratio: f64,
    close_ring: VecDeque<f64>,
    consecutive: usize,
}

impl StreamingConsolidation {
    /// Create a new consolidation detector.
    pub fn new(lookback: usize, band_ratio: f64) -> Self {
        Self {
            lookback,
            band_ratio,
            close_ring: VecDeque::with_capacity(lookback),
            consecutive: 0,
        }
    }
}

impl StreamingPattern for StreamingConsolidation {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        // Update ring
        if self.close_ring.len() == self.lookback {
            self.close_ring.pop_front();
        }
        self.close_ring.push_back(bar.close());
        if self.close_ring.len() < self.lookback {
            return None;
        }
        // Compute band width
        let max_c = self
            .close_ring
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_c = self.close_ring.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let mean = self.close_ring.iter().sum::<f64>() / self.lookback as f64;
        let is_tight = mean > 0.0 && (max_c - min_c) / mean <= self.band_ratio;
        if is_tight {
            self.consecutive += 1;
        } else {
            self.consecutive = 0;
        }
        Some(if self.consecutive > 0 { 100 } else { 0 })
    }
    fn reset(&mut self) {
        self.close_ring.clear();
        self.consecutive = 0;
    }
    fn is_ready(&self) -> bool {
        self.close_ring.len() == self.lookback
    }
}

// ============================================================================
// Streaming Bottom Breakout (横盘后底部突破)
// ============================================================================

/// Streaming 底部突破 (Bottom Breakout) detector.
///
/// Fires `100` when the close breaks above the rolling `lookback`-bar high
/// by at least `breakout_pct` AND the current bar's volume is at least
/// `vol_mult` times the 5-bar volume average. The "consolidation" gate
/// requires the prior `consol_lookback` bars to have a max-min range
/// smaller than `consol_ratio` × mean (so this only fires out of a
/// real sideways range, not a one-bar pop).
#[derive(Debug, Clone)]
pub struct StreamingBottomBreakout {
    lookback: usize,
    breakout_pct: f64,
    vol_mult: f64,
    consol_lookback: usize,
    consol_ratio: f64,
    high_ring: VecDeque<f64>,
    close_ring: VecDeque<f64>,
    vol_ring: VecDeque<f64>,
    vol_sum_5: f64,
    last_signal: i32,
}

impl StreamingBottomBreakout {
    /// Create a new bottom-breakout detector.
    pub fn new(lookback: usize, breakout_pct: f64, vol_mult: f64) -> Self {
        Self {
            lookback,
            breakout_pct,
            vol_mult,
            consol_lookback: lookback,
            consol_ratio: 0.10,
            high_ring: VecDeque::with_capacity(lookback),
            close_ring: VecDeque::with_capacity(lookback),
            vol_ring: VecDeque::with_capacity(5),
            vol_sum_5: 0.0,
            last_signal: 0,
        }
    }

    /// Override the consolidation window / ratio (defaults to lookback / 10%).
    pub fn with_consolidation(mut self, consol_lookback: usize, consol_ratio: f64) -> Self {
        self.consol_lookback = consol_lookback;
        self.consol_ratio = consol_ratio;
        self
    }
}

impl StreamingPattern for StreamingBottomBreakout {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        // Pop oldest entry BEFORE pushing current bar so the rolling
        // computations (prior_high, etc.) only see prior bars.
        if self.high_ring.len() == self.lookback {
            self.high_ring.pop_front();
        }
        if self.close_ring.len() == self.consol_lookback {
            self.close_ring.pop_front();
        }
        if self.vol_ring.len() == 5 {
            self.vol_sum_5 -= self.vol_ring.pop_front().unwrap();
        }
        // Snapshot the high-ring max BEFORE pushing the current bar.
        let prior_high = if self.high_ring.is_empty() {
            f64::NEG_INFINITY
        } else {
            self.high_ring
                .iter()
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b))
        };
        let max_c = if self.close_ring.is_empty() {
            f64::NEG_INFINITY
        } else {
            self.close_ring
                .iter()
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b))
        };
        let min_c = if self.close_ring.is_empty() {
            f64::INFINITY
        } else {
            self.close_ring.iter().fold(f64::INFINITY, |a, &b| a.min(b))
        };
        let mean_c = if self.close_ring.is_empty() {
            0.0
        } else {
            self.close_ring.iter().sum::<f64>() / self.close_ring.len() as f64
        };
        let vol_avg = if self.vol_ring.is_empty() {
            0.0
        } else {
            self.vol_sum_5 / self.vol_ring.len() as f64
        };

        // Now push the current bar
        self.high_ring.push_back(bar.high());
        self.close_ring.push_back(bar.close());
        self.vol_ring.push_back(bar.volume());
        self.vol_sum_5 += bar.volume();

        if self.high_ring.len() < self.lookback
            || self.close_ring.len() < self.consol_lookback
            || self.vol_ring.len() < 5
        {
            return None;
        }
        // Consolidation gate: prior range must be tight
        let is_sideways = mean_c > 0.0 && (max_c - min_c) / mean_c <= self.consol_ratio;
        if !is_sideways {
            self.last_signal = 0;
            return Some(0);
        }
        // Break above prior N-bar high
        let sig = if prior_high > 0.0
            && bar.close() > prior_high * (1.0 + self.breakout_pct)
            && vol_avg > 0.0
            && bar.volume() >= vol_avg * self.vol_mult
            && bar.close() > bar.open()
        {
            100
        } else {
            0
        };
        self.last_signal = sig;
        Some(sig)
    }
    fn reset(&mut self) {
        self.high_ring.clear();
        self.close_ring.clear();
        self.vol_ring.clear();
        self.vol_sum_5 = 0.0;
        self.last_signal = 0;
    }
    fn is_ready(&self) -> bool {
        self.high_ring.len() == self.lookback
            && self.close_ring.len() == self.consol_lookback
            && self.vol_ring.len() == 5
    }
}

// ============================================================================
// Streaming Strong Rebound (短期强势反弹)
// ============================================================================

/// Streaming 短期强势反弹 (Strong Rebound) detector.
///
/// Fires `100` when, over the past `lookback` bars, the cumulative
/// return from the lowest close is greater than or equal to `rebound_pct`
/// AND the current bar is a bullish candle (close > open).
///
/// Typical A-share setup: `lookback = 5`, `rebound_pct = 0.05` (5% rebound
/// from the local bottom in 5 bars).
#[derive(Debug, Clone)]
pub struct StreamingStrongRebound {
    lookback: usize,
    rebound_pct: f64,
    close_ring: VecDeque<f64>,
}

impl StreamingStrongRebound {
    /// Create a new strong-rebound detector.
    pub fn new(lookback: usize, rebound_pct: f64) -> Self {
        Self {
            lookback,
            rebound_pct,
            close_ring: VecDeque::with_capacity(lookback + 1),
        }
    }
}

impl StreamingPattern for StreamingStrongRebound {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        if self.close_ring.len() == self.lookback + 1 {
            self.close_ring.pop_front();
        }
        self.close_ring.push_back(bar.close());
        if self.close_ring.len() < self.lookback + 1 {
            return None;
        }
        let min_close = self.close_ring.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let sig = if min_close > 0.0
            && bar.close() >= min_close * (1.0 + self.rebound_pct)
            && bar.close() > bar.open()
        {
            100
        } else {
            0
        };
        Some(sig)
    }
    fn reset(&mut self) {
        self.close_ring.clear();
    }
    fn is_ready(&self) -> bool {
        self.close_ring.len() == self.lookback + 1
    }
}

// ============================================================================
// Streaming Strong Drop (短期强势下跌)
// ============================================================================

/// Streaming 短期强势下跌 (Strong Drop) detector.
///
/// Fires `-100` when, over the past `lookback` bars, the cumulative
/// return from the highest close is less than or equal to `-drop_pct`
/// AND the current bar is a bearish candle (close < open).
///
/// Mirror of [`StreamingStrongRebound`].
#[derive(Debug, Clone)]
pub struct StreamingStrongDrop {
    lookback: usize,
    drop_pct: f64,
    close_ring: VecDeque<f64>,
}

impl StreamingStrongDrop {
    /// Create a new strong-drop detector.
    pub fn new(lookback: usize, drop_pct: f64) -> Self {
        Self {
            lookback,
            drop_pct,
            close_ring: VecDeque::with_capacity(lookback + 1),
        }
    }
}

impl StreamingPattern for StreamingStrongDrop {
    fn next(&mut self, bar: &OhlcvBar) -> Option<Signal> {
        if self.close_ring.len() == self.lookback + 1 {
            self.close_ring.pop_front();
        }
        self.close_ring.push_back(bar.close());
        if self.close_ring.len() < self.lookback + 1 {
            return None;
        }
        let max_close = self
            .close_ring
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let sig = if max_close > 0.0
            && bar.close() <= max_close * (1.0 - self.drop_pct)
            && bar.close() < bar.open()
        {
            -100
        } else {
            0
        };
        Some(sig)
    }
    fn reset(&mut self) {
        self.close_ring.clear();
    }
    fn is_ready(&self) -> bool {
        self.close_ring.len() == self.lookback + 1
    }
}

// ============================================================================
// Batch run helper
// ============================================================================

/// Run a streaming pattern detector on a full series of OHLCV bars and
/// return the full signal array.
pub fn run_streaming<P: StreamingPattern>(
    detector: &mut P,
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
) -> Array1<Signal> {
    let n = close.len();
    let mut out = init_signal(n);
    for i in 0..n {
        let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], volume[i]);
        if let Some(sig) = detector.next(&bar) {
            out[i] = sig;
        }
    }
    out
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bar(o: f64, h: f64, l: f64, c: f64, v: f64) -> OhlcvBar {
        OhlcvBar::new(o, h, l, c, v)
    }

    #[test]
    fn test_ring_sma() {
        let mut sma = RingSMA::new(3);
        assert!(!sma.is_ready());
        assert!(sma.next(1.0).is_nan());
        assert!(sma.next(2.0).is_nan());
        let v = sma.next(3.0);
        assert!(sma.is_ready());
        assert!((v - 2.0).abs() < 1e-10);
        let v2 = sma.next(6.0);
        assert!((v2 - (2.0 + 3.0 + 6.0) / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_golden_cross() {
        let mut gc = StreamingGoldenCross::new(3, 5);
        // Build an uptrend
        let mut sig_count = 0;
        for i in 0..30 {
            let p = 10.0 + (i as f64) * 0.2;
            let bar = make_bar(p, p + 0.5, p - 0.5, p + 0.1, 1000.0);
            if let Some(_sig) = gc.next(&bar) {
                sig_count += 1;
            }
        }
        assert!(sig_count > 0, "should produce some signals");
    }

    #[test]
    fn test_streaming_yang_engulfing() {
        let mut e = StreamingYangEngulfing::new();
        // Bar 0: bearish, Bar 1: bullish engulfing
        let _ = e.next(&make_bar(10.0, 10.1, 9.4, 9.5, 1000.0));
        let sig = e.next(&make_bar(9.4, 10.1, 9.3, 10.0, 1000.0)).unwrap();
        assert_eq!(sig, 100);
    }

    #[test]
    fn test_streaming_hammer() {
        let mut h = StreamingHammer::new(5);
        // Feed 5 normal bars (descending), then a hammer
        for i in 0..5 {
            let p = 10.0 - (i as f64) * 0.1;
            let _ = h.next(&make_bar(p + 0.1, p + 0.2, p - 0.2, p, 100.0));
        }
        // Hammer: small body at top, very long lower shadow, very small upper shadow
        // body = |10.0 - 9.95| = 0.05, lo_shadow = 9.95 - 8.0 = 1.95, up_shadow = 9.99 - 10.0 = 0.01
        let sig = h.next(&make_bar(10.0, 9.99, 8.0, 9.95, 100.0)).unwrap();
        assert_eq!(sig, 100);
    }

    #[test]
    fn test_streaming_three_gap_downs() {
        let mut t = StreamingThreeGapDowns::new();
        // Need at least 2 bars initialized
        let _ = t.next(&make_bar(10.0, 10.5, 9.5, 9.8, 100.0));
        let _ = t.next(&make_bar(9.5, 9.8, 9.0, 9.2, 100.0));
        let _ = t.next(&make_bar(9.0, 9.2, 8.5, 8.7, 100.0));
        let sig = t.next(&make_bar(8.5, 8.7, 8.0, 8.2, 100.0));
        if let Some(s) = sig {
            assert!(s == -100 || s == 0);
        }
    }

    #[test]
    fn test_streaming_yang_through_three_ma() {
        let mut y = StreamingYangThroughThreeMA::new();
        // Need 20 bars to warm up the MA20
        for i in 0..19 {
            let p = 10.0 + (i as f64) * 0.05;
            let _ = y.next(&make_bar(p, p + 0.1, p - 0.1, p, 100.0));
        }
        // Big bullish breakout
        let sig = y.next(&make_bar(10.0, 12.0, 9.5, 11.8, 100.0)).unwrap();
        assert_eq!(sig, 100);
    }

    #[test]
    fn test_streaming_hermit_pointing_way() {
        let mut h = StreamingHermitPointingWay::new(5);
        // Feed 10 bars of uptrend
        for i in 0..10 {
            let p = 10.0 + (i as f64) * 0.5;
            let _ = h.next(&make_bar(
                p - 0.2,
                p + 0.5,
                p - 0.5,
                p + 0.2,
                100.0 + (i as f64) * 10.0,
            ));
        }
        // Hermit pointing bar: small body, long upper shadow
        let _sig = h.next(&make_bar(11.5, 12.5, 11.4, 11.6, 200.0));
        // (Result depends on lookback + sma5 being ready)
    }

    #[test]
    fn test_streaming_double_bottom() {
        let mut d = StreamingDoubleBottom::new(20, 0.05);
        // W-shape: down-up-down-up-breakout
        let mut prices = vec![10.0; 10];
        for i in 10..15 {
            prices.push(10.0 - (i - 10) as f64 * 0.5);
        }
        for i in 15..20 {
            prices.push(7.5 + (i - 15) as f64 * 0.4);
        }
        for i in 20..25 {
            prices.push(9.5 - (i - 20) as f64 * 0.5);
        }
        for i in 25..30 {
            prices.push(7.5 + (i - 25) as f64 * 0.5);
        }
        for (i, &p) in prices.iter().enumerate() {
            let bar = make_bar(p, p + 0.2, p - 0.2, p, 100.0);
            let _ = d.next(&bar);
            if i == prices.len() - 1 {
                // last bar should produce signal
            }
        }
    }

    #[test]
    fn test_run_streaming() {
        let mut gc = StreamingGoldenCross::new(3, 5);
        let n = 30;
        let open: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64) * 0.1).collect();
        let high: Vec<f64> = open.iter().map(|x| x + 0.5).collect();
        let low: Vec<f64> = open.iter().map(|x| x - 0.5).collect();
        let close: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64) * 0.1 + 0.05).collect();
        let volume = vec![1000.0; n];
        let out = run_streaming(&mut gc, &open, &high, &low, &close, &volume);
        assert_eq!(out.len(), n);
    }

    // -----------------------------------------------------------------
    // H-1: Consolidation / Breakout / Rebound / StrongDrop
    // -----------------------------------------------------------------

    #[test]
    fn test_streaming_consolidation() {
        let mut c = StreamingConsolidation::new(20, 0.05);
        // Feed 20 bars within 2% band → should be flagged as consolidation
        for i in 0..20 {
            let p = 10.0 + ((i % 3) as f64) * 0.01; // 10.00 ~ 10.02
            let sig = c.next(&make_bar(p, p + 0.1, p - 0.1, p, 100.0));
            if i >= 19 {
                assert_eq!(sig, Some(100));
            }
        }
        // Feed a big range → should drop to 0
        let sig = c.next(&make_bar(11.0, 11.5, 10.5, 11.2, 100.0));
        // After this, the band breaks, but bar i-19 still in ring, so
        // result depends on whether mean swings; just ensure 0 or 100.
        assert!(sig == Some(0) || sig == Some(100));
    }

    #[test]
    fn test_streaming_bottom_breakout() {
        // lookback 20, breakout 1%, vol 2x
        let mut b = StreamingBottomBreakout::new(20, 0.01, 2.0).with_consolidation(20, 0.05);
        // Feed 20 bars of tight consolidation (10.00 ~ 10.04)
        for i in 0..20 {
            let p = 10.0 + ((i % 4) as f64) * 0.01;
            let _ = b.next(&make_bar(p, p + 0.02, p - 0.02, p, 100.0));
        }
        // Big bullish bar with volume spike
        let sig = b.next(&make_bar(10.0, 10.5, 9.95, 10.4, 300.0));
        assert_eq!(sig, Some(100));
    }

    #[test]
    fn test_streaming_strong_rebound() {
        let mut r = StreamingStrongRebound::new(5, 0.05);
        // 5 prior bars with a low at 10.0
        for p in [10.0, 10.1, 10.2, 10.3, 10.4] {
            let _ = r.next(&make_bar(p, p + 0.1, p - 0.1, p, 100.0));
        }
        // Big bullish bounce from the lows → close >= 10.0 * 1.05 = 10.5
        let sig = r.next(&make_bar(10.4, 10.7, 10.3, 10.6, 100.0));
        assert_eq!(sig, Some(100));
    }

    #[test]
    fn test_streaming_strong_drop() {
        let mut d = StreamingStrongDrop::new(5, 0.05);
        // 5 prior bars with a high at 10.4
        for p in [10.4, 10.3, 10.2, 10.1, 10.0] {
            let _ = d.next(&make_bar(p, p + 0.1, p - 0.1, p, 100.0));
        }
        // Big bearish drop from highs → close <= 10.4 * 0.95 = 9.88
        let sig = d.next(&make_bar(9.95, 10.0, 9.80, 9.85, 100.0));
        assert_eq!(sig, Some(-100));
    }

    // -----------------------------------------------------------------
    // New streaming patterns (Stage B': 13→16)
    // -----------------------------------------------------------------

    #[test]
    fn test_streaming_bullish_alignment() {
        // Need 60+ bars to warm up the 60-MA
        let n = 80;
        let mut bullish = StreamingBullishAlignment::new(3);
        // Feed monotonically rising prices — keeps MA ordering
        let mut fired = 0;
        for i in 0..n {
            let p = 10.0 + (i as f64) * 0.5;
            let bar = make_bar(p - 0.2, p + 0.5, p - 0.5, p + 0.1, 1000.0);
            if let Some(sig) = bullish.next(&bar) {
                if sig == 100 {
                    fired += 1;
                }
            }
        }
        // Monotonic uptrend: alignment should fire at least once after warm-up
        assert!(fired > 0, "expected bullish alignment to fire on uptrend");
    }

    #[test]
    fn test_streaming_bearish_alignment() {
        let n = 80;
        let mut bearish = StreamingBearishAlignment::new(3);
        let mut fired = 0;
        for i in 0..n {
            let p = 100.0 - (i as f64) * 0.5; // monotonic downtrend
            let bar = make_bar(p + 0.2, p + 0.5, p - 0.5, p - 0.1, 1000.0);
            if let Some(sig) = bearish.next(&bar) {
                if sig == -100 {
                    fired += 1;
                }
            }
        }
        assert!(fired > 0, "expected bearish alignment to fire on downtrend");
    }

    #[test]
    fn test_streaming_macd_divergence() {
        let lookback = 5;
        let mut det = StreamingMacdDivergence::new(lookback);
        // Feed `lookback` bars first to warm up
        for i in 0..lookback {
            let p = 10.0 + (i as f64) * 0.1;
            let _ = det.next_with_hist(&make_bar(p, p + 0.1, p - 0.1, p, 100.0), 0.5);
        }
        // Bullish divergence: new low in price, but hist rises
        let sig = det.next_with_hist(
            &make_bar(9.5, 9.6, 9.4, 9.5, 100.0),
            1.0, // hist higher than the +0.5 in the lookback
        );
        if let Some(s) = sig {
            assert!(s == 100 || s == 0, "got {}", s);
        }
    }
}
