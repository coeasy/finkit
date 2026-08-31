use crate::streaming::traits::StreamingIndicator;
use crate::{impl_indicator_meta, impl_standard_methods};

/// Streaming Kaufman Efficiency Ratio.
///
/// ER = |Price Change over period| / Sum(|Daily Changes|) over period.
/// Input: single `f64` price per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingEfficiencyRatio {
    period: usize,
    ring: Vec<f64>,
    ring_idx: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingEfficiencyRatio {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            ring: vec![0.0; period + 1],
            ring_idx: 0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<f64, f64> for StreamingEfficiencyRatio {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.ring[self.ring_idx] = input;
        self.ring_idx += 1;
        if self.ring_idx == self.period + 1 {
            self.ring_idx = 0;
        }
        self.count += 1;

        if self.count <= self.period {
            self.last_value = None;
            return None;
        }

        // Current price is at old_idx, oldest price is at self.ring_idx
        let current = input;
        let oldest = self.ring[self.ring_idx];
        let direction = (current - oldest).abs();

        // Sum of |daily changes| over the period
        let mut volatility = 0.0;
        let ring_len = self.period + 1;
        for k in 0..self.period {
            let idx_curr = (self.ring_idx + 1 + k) % ring_len;
            let idx_prev = (self.ring_idx + k) % ring_len;
            volatility += (self.ring[idx_curr] - self.ring[idx_prev]).abs();
        }

        let er = if volatility > 1e-15 {
            direction / volatility
        } else {
            0.0
        };
        self.last_value = Some(er);
        Some(er)
    }

    fn reset(&mut self) {
        self.ring.fill(0.0);
        self.ring_idx = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count > self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(
    StreamingEfficiencyRatio,
    "EfficiencyRatio",
    "overlap",
    "Kaufman Efficiency Ratio: measures trend efficiency from 0 (choppy) to 1 (trending)"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_er_basic() {
        let mut ind = StreamingEfficiencyRatio::new(10);
        // Trending data
        let data: Vec<f64> = (0..20).map(|i| 100.0 + i as f64).collect();
        let mut results = Vec::new();
        for &v in &data {
            if let Some(er) = ind.next(v) {
                results.push(er);
            }
        }
        assert!(!results.is_empty());
        for &er in &results {
            assert!((er - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_streaming_er_reset() {
        let mut ind = StreamingEfficiencyRatio::new(5);
        for i in 0..10 {
            ind.next(100.0 + i as f64);
        }
        assert!(ind.is_ready());

        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), None);
    }

    #[test]
    fn test_streaming_er_meta() {
        let ind = StreamingEfficiencyRatio::new(10);
        assert_eq!(StreamingEfficiencyRatio::name(), "EfficiencyRatio");
        assert_eq!(StreamingEfficiencyRatio::category(), "overlap");
        assert_eq!(ind.warm_up_period(), 10);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..40)
            .map(|i| 100.0 + (i as f64 * 0.3).sin() * 5.0 + i as f64 * 0.1)
            .collect();
        let period = 10;

        let batch = crate::indicators::overlap::efficiency_ratio(&data, period).unwrap();

        let mut streaming = StreamingEfficiencyRatio::new(period);
        for i in 0..data.len() {
            let result = streaming.next(data[i]);
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
