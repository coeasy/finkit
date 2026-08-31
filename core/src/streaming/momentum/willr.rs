use crate::impl_indicator_meta;
use crate::impl_standard_methods;
use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::streaming::traits::{Ohlcv, StreamingIndicator};

/// Williams %R streaming indicator with O(1) amortized per-update complexity.
///
/// Uses monotonic deques to maintain rolling max(high) and rolling min(low) in
/// O(1) amortized time, avoiding the previous O(period) linear scan.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingWillR {
    period: usize,
    maxq: RollingMax,
    minq: RollingMin,
    count: usize,
    last_close: f64,
    last_value: Option<f64>,
}

impl StreamingWillR {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "StreamingWillR: period must be > 0");
        Self {
            period,
            maxq: RollingMax::new(),
            minq: RollingMin::new(),
            count: 0,
            last_close: f64::NAN,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingWillR {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, bar))
    )]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        crate::streaming_measure!("willr", self.count, {
            self.count += 1;
            let idx = self.count; // 1-based index for monotonic queues
            self.maxq.push(idx, bar.high());
            self.minq.push(idx, bar.low());
            // Pop everything older than (idx - period) so the queue spans
            // [idx - period + 1, idx] inclusive —exactly `period` bars.
            if idx > self.period {
                self.maxq.pop(idx - self.period);
                self.minq.pop(idx - self.period);
            }
            self.last_close = bar.close();

            if self.count < self.period {
                self.last_value = None;
                return None;
            }

            let highest = self.maxq.current().unwrap_or(f64::NEG_INFINITY);
            let lowest = self.minq.current().unwrap_or(f64::INFINITY);
            let denom = highest - lowest;
            let result = if denom.abs() > 1e-15 {
                Some((highest - self.last_close) / denom * -100.0)
            } else {
                Some(0.0)
            };
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.maxq.reset();
        self.minq.reset();
        self.count = 0;
        self.last_close = f64::NAN;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingWillR, "WILLR", "momentum", "Williams %R");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_willr_basic() {
        let mut w = StreamingWillR::new(3);
        let bars = [
            OhlcvBar::new(9.0, 12.0, 8.0, 10.0, 100.0),
            OhlcvBar::new(10.0, 13.0, 9.0, 11.0, 100.0),
            OhlcvBar::new(11.0, 14.0, 10.0, 12.0, 100.0),
        ];
        for bar in &bars[..2] {
            assert_eq!(w.next(bar), None);
        }
        let v = w.next(&bars[2]).unwrap();
        assert!((-100.0..=0.0).contains(&v));
    }

    #[test]
    fn test_streaming_willr_meta() {
        assert_eq!(StreamingWillR::name(), "WILLR");
    }

    #[test]
    fn test_streaming_willr_reset() {
        let mut w = StreamingWillR::new(3);
        for i in 0..5 {
            w.next(&OhlcvBar::new(
                i as f64,
                i as f64 + 2.0,
                i as f64 - 1.0,
                i as f64 + 1.0,
                100.0,
            ));
        }
        assert!(w.is_ready());
        w.reset();
        assert!(!w.is_ready());
    }

    /// Cross-validated with linear scan: monotonic queue O(1) vs O(period) outputs match
    #[test]
    fn test_streaming_willr_parity_with_linear_scan() {
        let period = 14;
        let mut w = StreamingWillR::new(period);
        let mut lin_highs: Vec<f64> = Vec::new();
        let mut lin_lows: Vec<f64> = Vec::new();
        let bars: Vec<OhlcvBar> = (0..500)
            .map(|i| {
                let h = 100.0 + (i as f64 * 0.13).sin() * 5.0;
                let l = h - 2.0 - (i as f64 * 0.17).cos().abs() * 1.5;
                let c = (h + l) / 2.0;
                OhlcvBar::new(c, h, l, c, 1000.0)
            })
            .collect();

        for bar in &bars {
            // 线性参考实现：扫描最�?period �?
            lin_highs.push(bar.high());
            lin_lows.push(bar.low());
            if lin_highs.len() > period {
                lin_highs.remove(0);
                lin_lows.remove(0);
            }
            let opt = w.next(bar);
            if lin_highs.len() < period {
                assert!(opt.is_none());
                continue;
            }
            let highest = lin_highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let lowest = lin_lows.iter().cloned().fold(f64::INFINITY, f64::min);
            let denom = highest - lowest;
            let expected = if denom.abs() > 1e-15 {
                (highest - bar.close()) / denom * -100.0
            } else {
                0.0
            };
            let actual = opt.unwrap();
            assert!(
                (actual - expected).abs() < 1e-10,
                "WILLR mismatch: actual={actual} expected={expected}"
            );
        }
    }
}
