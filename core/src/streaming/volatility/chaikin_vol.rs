use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Chaikin Volatility.
///
/// CV = ROC(EMA(High-Low, ema_period), roc_period)
/// Input: `(high, low)` per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingChaikinVol {
    ema_period: usize,
    roc_period: usize,
    alpha: f64,
    ema_val: f64,
    ema_ready: bool,
    ema_count: usize,
    // Ring buffer for EMA values (for ROC lookback)
    ema_ring: Vec<f64>,
    ema_ring_idx: usize,
    ema_ring_count: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingChaikinVol {
    pub fn new(ema_period: usize, roc_period: usize) -> Self {
        let alpha = 2.0 / (ema_period as f64 + 1.0);
        Self {
            ema_period,
            roc_period,
            alpha,
            ema_val: 0.0,
            ema_ready: false,
            ema_count: 0,
            ema_ring: vec![0.0; roc_period + 1],
            ema_ring_idx: 0,
            ema_ring_count: 0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64), f64> for StreamingChaikinVol {
    #[inline]
    fn next(&mut self, input: (f64, f64)) -> Option<f64> {
        let (high, low) = input;
        self.count += 1;
        let hl = high - low;

        // Update EMA
        if !self.ema_ready {
            self.ema_count += 1;
            if self.ema_count == 1 {
                self.ema_val = hl;
            } else {
                self.ema_val = self.alpha * hl + (1.0 - self.alpha) * self.ema_val;
            }
            if self.ema_count >= self.ema_period {
                self.ema_ready = true;
            }
        } else {
            self.ema_val = self.alpha * hl + (1.0 - self.alpha) * self.ema_val;
        }

        if !self.ema_ready {
            self.last_value = None;
            return None;
        }

        // Store EMA value in ring buffer for ROC
        self.ema_ring[self.ema_ring_idx] = self.ema_val;
        self.ema_ring_idx += 1;
        if self.ema_ring_idx == self.roc_period + 1 {
            self.ema_ring_idx = 0;
        }
        if self.ema_ring_count <= self.roc_period {
            self.ema_ring_count += 1;
        }

        // Need roc_period + 1 EMA values for ROC
        if self.ema_ring_count <= self.roc_period {
            self.last_value = None;
            return None;
        }

        // Current is the last stored, prev is roc_period ago
        let prev_idx = self.ema_ring_idx; // next slot = oldest
        let prev = self.ema_ring[prev_idx % (self.roc_period + 1)];

        let cv = if prev.abs() > 1e-15 {
            ((self.ema_val - prev) / prev) * 100.0
        } else {
            0.0
        };

        self.last_value = Some(cv);
        Some(cv)
    }

    fn reset(&mut self) {
        self.ema_val = 0.0;
        self.ema_ready = false;
        self.ema_count = 0;
        self.ema_ring.fill(0.0);
        self.ema_ring_idx = 0;
        self.ema_ring_count = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.ema_ready && self.ema_ring_count > self.roc_period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingChaikinVol {
    fn name() -> &'static str {
        "ChaikinVol"
    }
    fn category() -> &'static str {
        "volatility"
    }
    fn description() -> &'static str {
        "Chaikin Volatility: ROC of EMA-smoothed High-Low range"
    }
    fn warm_up_period(&self) -> usize {
        self.ema_period + self.roc_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_chaikin_vol_basic() {
        let mut ind = StreamingChaikinVol::new(10, 10);
        let n = 40;
        let high: Vec<f64> = (0..n)
            .map(|i| 110.0 + (i as f64 * 0.3).sin() * 5.0)
            .collect();
        let low: Vec<f64> = (0..n)
            .map(|i| 90.0 + (i as f64 * 0.3).sin() * 5.0)
            .collect();

        let mut results = Vec::new();
        for i in 0..n {
            if let Some(val) = ind.next((high[i], low[i])) {
                results.push((i, val));
            }
        }
        assert!(!results.is_empty());
        for (_, v) in &results {
            assert!(v.is_finite());
        }
    }

    #[test]
    fn test_streaming_chaikin_vol_reset() {
        let mut ind = StreamingChaikinVol::new(5, 5);
        for i in 0..20 {
            ind.next((110.0 + i as f64, 90.0 + i as f64));
        }
        assert!(ind.is_ready());

        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), None);
    }

    #[test]
    fn test_streaming_chaikin_vol_meta() {
        let ind = StreamingChaikinVol::new(10, 10);
        assert_eq!(StreamingChaikinVol::name(), "ChaikinVol");
        assert_eq!(StreamingChaikinVol::category(), "volatility");
        assert_eq!(ind.warm_up_period(), 20);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 50;
        let high: Vec<f64> = (0..n)
            .map(|i| 110.0 + (i as f64 * 0.2).sin() * 5.0)
            .collect();
        let low: Vec<f64> = (0..n)
            .map(|i| 90.0 + (i as f64 * 0.2).sin() * 5.0)
            .collect();
        let ema_period = 10;
        let roc_period = 10;

        let batch = crate::indicators::volatility_ext::chaikin_volatility(
            &high, &low, ema_period, roc_period,
        )
        .unwrap();

        let mut streaming = StreamingChaikinVol::new(ema_period, roc_period);
        for i in 0..n {
            let result = streaming.next((high[i], low[i]));
            if let Some(val) = result {
                if !batch[i].is_nan() {
                    assert!(
                        (val - batch[i]).abs() < 1e-8,
                        "Mismatch at {i}: streaming={val}, batch={}",
                        batch[i]
                    );
                }
            }
        }
    }
}
