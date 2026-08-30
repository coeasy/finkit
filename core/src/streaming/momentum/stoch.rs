use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StochOutput {
    pub k: f64,
    pub d: f64,
}

/// Streaming Stochastic Oscillator (%K, %D).
///
/// Uses ring buffers for (high, low) and SMA smoothing buffers.
/// Uses O(1) amortized monotonic deques for rolling max/min.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingStoch {
    k_period: usize,
    k_slow: usize,
    d_period: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    hl_head: usize,
    hl_len: usize,
    fast_k_buf: Vec<f64>,
    fk_head: usize,
    fk_len: usize,
    slow_k_buf: Vec<f64>,
    sk_head: usize,
    sk_len: usize,
    rolling_max: RollingMax,
    rolling_min: RollingMin,
    fk_sum: f64,
    sk_sum: f64,
    count: usize,
    last_value: Option<StochOutput>,
}

impl StreamingStoch {
    pub fn new(k_period: usize, k_slow: usize, d_period: usize) -> Self {
        Self {
            k_period,
            k_slow,
            d_period,
            highs: vec![0.0; k_period],
            lows: vec![0.0; k_period],
            hl_head: 0,
            hl_len: 0,
            fast_k_buf: vec![0.0; k_slow],
            fk_head: 0,
            fk_len: 0,
            slow_k_buf: vec![0.0; d_period],
            sk_head: 0,
            sk_len: 0,
            rolling_max: RollingMax::new(),
            rolling_min: RollingMin::new(),
            fk_sum: 0.0,
            sk_sum: 0.0,
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    fn ring_push(buf: &mut [f64], head: &mut usize, len: &mut usize, val: f64) {
        let cap = buf.len();
        if *len < cap {
            buf[(*head + *len) % cap] = val;
            *len += 1;
        } else {
            buf[*head] = val;
            *head = (*head + 1) % cap;
        }
    }
}

impl StreamingIndicator<(f64, f64, f64), StochOutput> for StreamingStoch {
    #[inline]
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip(self, input)))]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<StochOutput> {
        crate::streaming_measure!("stoch", self.count, {
            let (high, low, close) = input;
            self.count += 1;

            // Push to rolling deques for O(1) amortized max/min
            self.rolling_max.push(self.count, high);
            self.rolling_min.push(self.count, low);
            if self.count > self.k_period {
                self.rolling_max.pop(self.count - self.k_period);
                self.rolling_min.pop(self.count - self.k_period);
            }

            // Ring buffer management for highs/lows (kept for window membership)
            let cap = self.k_period;
            if self.hl_len < cap {
                let idx = (self.hl_head + self.hl_len) % cap;
                self.highs[idx] = high;
                self.lows[idx] = low;
                self.hl_len += 1;
            } else {
                self.highs[self.hl_head] = high;
                self.lows[self.hl_head] = low;
                self.hl_head = (self.hl_head + 1) % cap;
            }

            if self.hl_len < self.k_period {
                self.last_value = None;
                return None;
            }

            let highest = self.rolling_max.current().unwrap_or(f64::NEG_INFINITY);
            let lowest = self.rolling_min.current().unwrap_or(f64::INFINITY);

            let denom = highest - lowest;
            let fast_k = if denom.abs() > 1e-15 {
                (close - lowest) / denom * 100.0
            } else {
                50.0
            };

            // Incremental fk_sum
            if self.fk_len < self.k_slow {
                self.fk_sum += fast_k;
            } else {
                let evicted = self.fast_k_buf[self.fk_head];
                self.fk_sum += fast_k - evicted;
            }
            Self::ring_push(&mut self.fast_k_buf, &mut self.fk_head, &mut self.fk_len, fast_k);

            if self.fk_len < self.k_slow {
                self.last_value = None;
                return None;
            }

            let slow_k = self.fk_sum / self.k_slow as f64;

            // Incremental sk_sum
            if self.sk_len < self.d_period {
                self.sk_sum += slow_k;
            } else {
                let evicted = self.slow_k_buf[self.sk_head];
                self.sk_sum += slow_k - evicted;
            }
            Self::ring_push(&mut self.slow_k_buf, &mut self.sk_head, &mut self.sk_len, slow_k);

            if self.sk_len < self.d_period {
                self.last_value = None;
                return None;
            }

            let d = self.sk_sum / self.d_period as f64;
            let result = Some(StochOutput { k: slow_k, d });
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.hl_head = 0;
        self.hl_len = 0;
        self.fk_head = 0;
        self.fk_len = 0;
        self.sk_head = 0;
        self.sk_len = 0;
        self.rolling_max.reset();
        self.rolling_min.reset();
        self.fk_sum = 0.0;
        self.sk_sum = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.sk_len >= self.d_period
    }

        impl_standard_methods!(output = StochOutput);


}

impl IndicatorMeta for StreamingStoch {
    fn name() -> &'static str { "STOCH" }
    fn category() -> &'static str { "momentum" }
    fn description() -> &'static str { "Stochastic Oscillator" }
    fn warm_up_period(&self) -> usize { self.k_period + self.k_slow + self.d_period - 2 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_stoch_basic() {
        let mut stoch = StreamingStoch::new(5, 3, 3);
        let data: Vec<(f64, f64, f64)> = (0..20)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.3).sin() * 10.0;
                let l = h - 3.0;
                let c = (h + l) / 2.0;
                (h, l, c)
            })
            .collect();

        let mut last = None;
        for &d in &data {
            last = stoch.next(d);
        }
        let last = last.unwrap();
        assert!(!last.k.is_nan());
        assert!(!last.d.is_nan());
        assert!((0.0..=100.0).contains(&last.k));
        assert!((0.0..=100.0).contains(&last.d));
    }

    #[test]
    fn test_streaming_stoch_range() {
        let mut stoch = StreamingStoch::new(14, 3, 3);
        let data: Vec<(f64, f64, f64)> = (0..100)
            .map(|i| {
                let h = 100.0 + (i as f64 * 0.1).sin() * 20.0;
                let l = h - 5.0;
                let c = h - 2.5;
                (h, l, c)
            })
            .collect();

        for &d in &data {
            if let Some(out) = stoch.next(d) {
                assert!(out.k >= -0.01 && out.k <= 100.01, "k out of range: {}", out.k);
                assert!(out.d >= -0.01 && out.d <= 100.01, "d out of range: {}", out.d);
            }
        }
    }

    #[test]
    fn test_streaming_stoch_reset() {
        let mut stoch = StreamingStoch::new(5, 3, 3);
        for i in 0..20 {
            stoch.next((50.0 + i as f64, 45.0 + i as f64, 47.0 + i as f64));
        }
        assert!(stoch.is_ready());
        stoch.reset();
        assert!(!stoch.is_ready());
        assert_eq!(stoch.count(), 0);
    }

    #[test]
    fn test_streaming_stoch_meta() {
        let stoch = StreamingStoch::new(14, 3, 3);
        assert_eq!(StreamingStoch::name(), "STOCH");
        assert_eq!(StreamingStoch::category(), "momentum");
        assert_eq!(stoch.warm_up_period(), 18);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let high: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();
        let close: Vec<f64> = high.iter().zip(low.iter()).map(|(h, l)| (h + l) / 2.0).collect();

        let batch = crate::indicators::momentum::stoch(&high, &low, &close, 14, 3, 3).unwrap();

        let mut streaming = StreamingStoch::new(14, 3, 3);
        for i in 0..n {
            if let Some(out) = streaming.next((high[i], low[i], close[i])) {
                if !batch.k[i].is_nan() {
                    assert!(
                        (out.k - batch.k[i]).abs() < 1e-10,
                        "K mismatch at {i}: streaming={}, batch={}",
                        out.k,
                        batch.k[i]
                    );
                }
                if !batch.d[i].is_nan() {
                    assert!(
                        (out.d - batch.d[i]).abs() < 1e-10,
                        "D mismatch at {i}: streaming={}, batch={}",
                        out.d,
                        batch.d[i]
                    );
                }
            }
        }
    }
}
