use crate::impl_standard_methods;
use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KdjOutput {
    pub k: f64,
    pub d: f64,
    pub j: f64,
}

/// Streaming KDJ (Chinese Stochastic Oscillator).
///
/// Uses recursive china_sma smoothing matching the batch `china::kdj` implementation.
/// Uses O(1) amortized monotonic deques for rolling max/min.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingKdj {
    n: usize,
    m1: usize,
    m2: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    head: usize,
    len: usize,
    k_prev: f64,
    d_prev: f64,
    k_valid: bool,
    d_valid: bool,
    rolling_max: RollingMax,
    rolling_min: RollingMin,
    count: usize,
    last_value: Option<KdjOutput>,
}

#[inline]
fn china_sma_step(input: f64, prev: f64, period: usize, m: usize) -> f64 {
    let period_f = period as f64;
    let m_f = m as f64;
    (m_f * input + (period_f - m_f) * prev) / period_f
}

impl StreamingKdj {
    pub fn new(n: usize, m1: usize, m2: usize) -> Self {
        Self {
            n,
            m1,
            m2,
            highs: vec![0.0; n],
            lows: vec![0.0; n],
            head: 0,
            len: 0,
            k_prev: 50.0,
            d_prev: 50.0,
            k_valid: false,
            d_valid: false,
            rolling_max: RollingMax::new(),
            rolling_min: RollingMin::new(),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64), KdjOutput> for StreamingKdj {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<KdjOutput> {
        let (high, low, close) = input;
        self.count += 1;
        let cap = self.n;

        // Push to rolling deques for O(1) amortized max/min
        self.rolling_max.push(self.count, high);
        self.rolling_min.push(self.count, low);
        if self.count > self.n {
            self.rolling_max.pop(self.count - self.n);
            self.rolling_min.pop(self.count - self.n);
        }

        // Ring buffer management for highs/lows (kept for window membership)
        if self.len < cap {
            let idx = (self.head + self.len) % cap;
            self.highs[idx] = high;
            self.lows[idx] = low;
            self.len += 1;
        } else {
            self.highs[self.head] = high;
            self.lows[self.head] = low;
            self.head = (self.head + 1) % cap;
        }

        if self.len < self.n {
            self.last_value = None;
            return None;
        }

        let highest = self.rolling_max.current().unwrap_or(f64::NEG_INFINITY);
        let lowest = self.rolling_min.current().unwrap_or(f64::INFINITY);

        let denom = highest - lowest;
        let rsv = if denom.abs() > 1e-15 {
            (close - lowest) / denom * 100.0
        } else {
            50.0
        };

        self.k_prev = china_sma_step(rsv, self.k_prev, self.m1, 1);
        self.k_valid = true;

        self.d_prev = china_sma_step(self.k_prev, self.d_prev, self.m2, 1);
        self.d_valid = true;

        let j = 3.0 * self.k_prev - 2.0 * self.d_prev;
        let result = Some(KdjOutput {
            k: self.k_prev,
            d: self.d_prev,
            j,
        });
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.k_prev = 50.0;
        self.d_prev = 50.0;
        self.k_valid = false;
        self.d_valid = false;
        self.rolling_max.reset();
        self.rolling_min.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.k_valid && self.d_valid
    }

    impl_standard_methods!(output = KdjOutput);
}

impl IndicatorMeta for StreamingKdj {
    fn name() -> &'static str {
        "KDJ"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Chinese Stochastic Oscillator (KDJ)"
    }
    fn warm_up_period(&self) -> usize {
        self.n - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_kdj_basic() {
        let mut kdj = StreamingKdj::new(9, 3, 3);
        let data: Vec<(f64, f64, f64)> = (0..30)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.3).sin() * 10.0;
                let l = h - 3.0;
                let c = (h + l) / 2.0;
                (h, l, c)
            })
            .collect();

        let mut last = None;
        for &d in &data {
            last = kdj.next(d);
        }
        let last = last.unwrap();
        assert!(!last.k.is_nan());
        assert!(!last.d.is_nan());
        assert!(!last.j.is_nan());
    }

    #[test]
    fn test_streaming_kdj_reset() {
        let mut kdj = StreamingKdj::new(9, 3, 3);
        for i in 0..20 {
            kdj.next((50.0 + i as f64, 45.0 + i as f64, 47.0 + i as f64));
        }
        assert!(kdj.is_ready());
        kdj.reset();
        assert!(!kdj.is_ready());
        assert_eq!(kdj.count(), 0);
    }

    #[test]
    fn test_streaming_kdj_meta() {
        let kdj = StreamingKdj::new(9, 3, 3);
        assert_eq!(StreamingKdj::name(), "KDJ");
        assert_eq!(StreamingKdj::category(), "momentum");
        assert_eq!(kdj.warm_up_period(), 8);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let high: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();
        let close: Vec<f64> = high
            .iter()
            .zip(low.iter())
            .map(|(h, l)| (h + l) / 2.0)
            .collect();

        let batch = crate::indicators::china::kdj(&high, &low, &close, 9, 3, 3).unwrap();

        let mut streaming = StreamingKdj::new(9, 3, 3);
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
                if !batch.j[i].is_nan() {
                    assert!(
                        (out.j - batch.j[i]).abs() < 1e-10,
                        "J mismatch at {i}: streaming={}, batch={}",
                        out.j,
                        batch.j[i]
                    );
                }
            }
        }
    }
}
