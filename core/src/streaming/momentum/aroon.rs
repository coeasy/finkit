use crate::impl_standard_methods;
use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AroonOutput {
    pub aroon_up: f64,
    pub aroon_down: f64,
}

/// Streaming Aroon Up/Down indicator using monotonic deque.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAroon {
    period: usize,
    count: usize,
    rolling_max: RollingMax,
    rolling_min: RollingMin,
    last_value: Option<AroonOutput>,
}

impl StreamingAroon {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            count: 0,
            rolling_max: RollingMax::new(),
            rolling_min: RollingMin::new(),
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64), AroonOutput> for StreamingAroon {
    #[inline]
    fn next(&mut self, input: (f64, f64)) -> Option<AroonOutput> {
        let (high, low) = input;
        self.count += 1;
        let idx = self.count - 1;

        // Window is `period + 1` elements: [idx-period, idx], including index 0.
        // This matches TA-Lib's AROON window semantics (first output at
        // idx == period). Push every bar, then evict entries that have slid
        // out of the window.
        self.rolling_max.push(idx, high);
        self.rolling_min.push(idx, low);

        if idx >= self.period {
            let oldest = idx - self.period;
            // Evict expired entries independently per deque: a stale low could
            // live past the window even when the max is recent, and vice versa.
            while let Some((front_idx, _)) = self.rolling_max.front() {
                if front_idx < oldest {
                    self.rolling_max.pop(front_idx);
                } else {
                    break;
                }
            }
            while let Some((front_idx, _)) = self.rolling_min.front() {
                if front_idx < oldest {
                    self.rolling_min.pop(front_idx);
                } else {
                    break;
                }
            }
        }

        if self.count <= self.period {
            self.last_value = None;
            return None;
        }

        let (max_idx, _) = self.rolling_max.front().unwrap();
        let (min_idx, _) = self.rolling_min.front().unwrap();

        let bars_since_high = idx - max_idx;
        let bars_since_low = idx - min_idx;

        let aroon_up = ((self.period - bars_since_high) as f64 / self.period as f64) * 100.0;
        let aroon_down = ((self.period - bars_since_low) as f64 / self.period as f64) * 100.0;

        let result = Some(AroonOutput {
            aroon_up,
            aroon_down,
        });
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.count = 0;
        self.rolling_max.reset();
        self.rolling_min.reset();
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count > self.period
    }

    impl_standard_methods!(output = AroonOutput);
}

impl IndicatorMeta for StreamingAroon {
    fn name() -> &'static str {
        "AROON"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Aroon Up/Down"
    }
    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_aroon_basic() {
        let mut aroon = StreamingAroon::new(5);
        let data: Vec<(f64, f64)> = (0..10)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.5).sin() * 10.0;
                (h, h - 3.0)
            })
            .collect();

        let mut last = None;
        for &d in &data {
            last = aroon.next(d);
        }
        let last = last.unwrap();
        assert!(!last.aroon_up.is_nan());
        assert!(!last.aroon_down.is_nan());
        assert!((0.0..=100.0).contains(&last.aroon_up));
        assert!((0.0..=100.0).contains(&last.aroon_down));
    }

    #[test]
    fn test_streaming_aroon_uptrend() {
        let mut aroon = StreamingAroon::new(5);
        for i in 0..10 {
            let h = 100.0 + i as f64 * 5.0;
            let out = aroon.next((h, h - 2.0));
            if aroon.is_ready() {
                let out = out.unwrap();
                assert_eq!(out.aroon_up, 100.0, "In uptrend, aroon_up should be 100");
            }
        }
    }

    #[test]
    fn test_streaming_aroon_downtrend() {
        let mut aroon = StreamingAroon::new(5);
        for i in 0..10 {
            let h = 100.0 - i as f64 * 5.0;
            let out = aroon.next((h, h - 2.0));
            if aroon.is_ready() {
                let out = out.unwrap();
                assert_eq!(
                    out.aroon_down, 100.0,
                    "In downtrend, aroon_down should be 100"
                );
            }
        }
    }

    #[test]
    fn test_streaming_aroon_reset() {
        let mut aroon = StreamingAroon::new(5);
        for i in 0..10 {
            aroon.next((50.0 + i as f64, 45.0 + i as f64));
        }
        assert!(aroon.is_ready());
        aroon.reset();
        assert!(!aroon.is_ready());
        assert_eq!(aroon.count(), 0);
    }

    #[test]
    fn test_streaming_aroon_meta() {
        let aroon = StreamingAroon::new(25);
        assert_eq!(StreamingAroon::name(), "AROON");
        assert_eq!(StreamingAroon::category(), "momentum");
        assert_eq!(aroon.warm_up_period(), 26);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let high: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();
        let period = 14;

        let batch = crate::indicators::momentum::aroon(&high, &low, period).unwrap();

        let mut streaming = StreamingAroon::new(period);
        for i in 0..n {
            if let Some(out) = streaming.next((high[i], low[i])) {
                if !batch.aroon_up[i].is_nan() {
                    assert!(
                        (out.aroon_up - batch.aroon_up[i]).abs() < 1e-10,
                        "AroonUp mismatch at {i}: streaming={}, batch={}",
                        out.aroon_up,
                        batch.aroon_up[i]
                    );
                }
                if !batch.aroon_down[i].is_nan() {
                    assert!(
                        (out.aroon_down - batch.aroon_down[i]).abs() < 1e-10,
                        "AroonDown mismatch at {i}: streaming={}, batch={}",
                        out.aroon_down,
                        batch.aroon_down[i]
                    );
                }
            }
        }
    }
}
