use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming Historical Volatility (Close-to-Close).
///
/// HV = StdDev(ln(C[i]/C[i-1]), period) × sqrt(annualization)
/// Input: single `f64` close price per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingHv {
    period: usize,
    ann_sqrt: f64,
    inv_pm1: f64,
    prev_close: f64,
    ring: Vec<f64>,
    ring_idx: usize,
    count: usize,
    sum: f64,
    sum_sq: f64,
    last_value: Option<f64>,
}

impl StreamingHv {
    pub fn new(period: usize, annualization: f64) -> Self {
        Self {
            period,
            ann_sqrt: annualization.sqrt() * 100.0,
            inv_pm1: 1.0 / (period - 1).max(1) as f64,
            prev_close: f64::NAN,
            ring: vec![0.0; period],
            ring_idx: 0,
            count: 0,
            sum: 0.0,
            sum_sq: 0.0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<f64, f64> for StreamingHv {
    #[inline]
    fn next(&mut self, close: f64) -> Option<f64> {
        if self.prev_close.is_nan() || self.prev_close <= 0.0 || close <= 0.0 {
            self.prev_close = close;
            self.count += 1;
            self.last_value = None;
            return None;
        }

        let log_ret = (close / self.prev_close).ln();
        self.prev_close = close;

        // If window is full, evict the oldest log return
        if self.count >= self.period {
            let old = self.ring[self.ring_idx];
            self.sum -= old;
            self.sum_sq -= old * old;
        }

        // Store new log return
        self.ring[self.ring_idx] = log_ret;
        self.ring_idx += 1;
        if self.ring_idx == self.period {
            self.ring_idx = 0;
        }
        self.count += 1;

        // Accumulate new value
        self.sum += log_ret;
        self.sum_sq += log_ret * log_ret;

        // Need period log returns (which requires period+1 prices)
        if self.count <= self.period {
            self.last_value = None;
            return None;
        }

        // Compute sample stddev incrementally: Var = (Σx² - (Σx)²/n) / (n-1)
        let n = self.period as f64;
        let variance = (self.sum_sq - self.sum * self.sum / n) * self.inv_pm1;
        let stddev = variance.max(0.0).sqrt();
        let hv = stddev * self.ann_sqrt;

        self.last_value = Some(hv);
        Some(hv)
    }

    fn reset(&mut self) {
        self.prev_close = f64::NAN;
        self.ring.fill(0.0);
        self.ring_idx = 0;
        self.count = 0;
        self.sum = 0.0;
        self.sum_sq = 0.0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count > self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingHv {
    fn name() -> &'static str {
        "HV"
    }
    fn category() -> &'static str {
        "volatility"
    }
    fn description() -> &'static str {
        "Historical Volatility: annualized standard deviation of log returns"
    }
    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_hv_basic() {
        let mut ind = StreamingHv::new(20, 252.0);
        let close: Vec<f64> = (0..50)
            .map(|i| 100.0 * (1.0 + 0.01 * (i as f64 * 0.5).sin()))
            .collect();

        let mut results = Vec::new();
        for &c in &close {
            if let Some(val) = ind.next(c) {
                results.push(val);
            }
        }
        assert!(!results.is_empty());
        for v in &results {
            assert!(v.is_finite());
            assert!(*v >= 0.0);
        }
    }

    #[test]
    fn test_streaming_hv_reset() {
        let mut ind = StreamingHv::new(5, 252.0);
        for i in 0..15 {
            ind.next(100.0 + i as f64);
        }
        assert!(ind.is_ready());

        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), None);
    }

    #[test]
    fn test_streaming_hv_meta() {
        let ind = StreamingHv::new(20, 252.0);
        assert_eq!(StreamingHv::name(), "HV");
        assert_eq!(StreamingHv::category(), "volatility");
        assert_eq!(ind.warm_up_period(), 21);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let close: Vec<f64> = (0..50)
            .map(|i| 100.0 * (1.0 + 0.01 * (i as f64 * 0.5).sin()))
            .collect();
        let period = 20;
        let ann = 252.0;

        let batch = crate::indicators::volatility_ext::historical_volatility(&close, period, ann).unwrap();

        let mut streaming = StreamingHv::new(period, ann);
        for i in 0..close.len() {
            let result = streaming.next(close[i]);
            if let Some(val) = result {
                if !batch[i].is_nan() {
                    assert!(
                        (val - batch[i]).abs() < 1e-10,
                        "Mismatch at {i}: streaming={val}, batch={}",
                        batch[i]
                    );
                }
            }
        }
    }
}
