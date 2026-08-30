use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming Chande Forecast Oscillator.
///
/// CFO = ((Close - TSF) / Close) * 100
/// Uses a ring buffer of prices and computes linear regression inline.
/// Input: single `f64` price per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingCfo {
    period: usize,
    ring: Vec<f64>,
    ring_idx: usize,
    count: usize,
    sx: f64,
    n: f64,
    denom: f64,
    sum_y: f64,
    sum_xy: f64,
    last_value: Option<f64>,
}

impl StreamingCfo {
    pub fn new(period: usize) -> Self {
        let n = period as f64;
        let sx: f64 = (0..period).map(|j| j as f64).sum();
        let sx2: f64 = (0..period).map(|j| (j as f64) * (j as f64)).sum();
        let denom = n * sx2 - sx * sx;

        Self {
            period,
            ring: vec![0.0; period],
            ring_idx: 0,
            count: 0,
            sx,
            n,
            denom,
            sum_y: 0.0,
            sum_xy: 0.0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<f64, f64> for StreamingCfo {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        let old_y = self.ring[self.ring_idx];
        self.sum_xy = self.sum_xy - self.sum_y + old_y + (self.n - 1.0) * input;
        self.sum_y = self.sum_y - old_y + input;

        self.ring[self.ring_idx] = input;
        self.ring_idx = (self.ring_idx + 1) % self.period;
        self.count += 1;

        if self.count < self.period {
            self.last_value = None;
            return None;
        }

        if self.denom.abs() < 1e-15 {
            self.last_value = Some(0.0);
            return self.last_value;
        }

        let slope = (self.n * self.sum_xy - self.sx * self.sum_y) / self.denom;
        let intercept = (self.sum_y - slope * self.sx) / self.n;
        let tsf_val = intercept + slope * self.n;

        let cfo = if input.abs() > 1e-15 {
            ((input - tsf_val) / input) * 100.0
        } else {
            0.0
        };

        self.last_value = Some(cfo);
        Some(cfo)
    }

    fn reset(&mut self) {
        self.ring.fill(0.0);
        self.ring_idx = 0;
        self.count = 0;
        self.sum_y = 0.0;
        self.sum_xy = 0.0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingCfo {
    fn name() -> &'static str {
        "CFO"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Chande Forecast Oscillator: percentage deviation of price from TSF"
    }
    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_cfo_basic() {
        let mut ind = StreamingCfo::new(14);
        let data: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64 * 0.3).sin() * 5.0).collect();

        let mut results = Vec::new();
        for &v in &data {
            if let Some(val) = ind.next(v) {
                results.push(val);
            }
        }
        assert!(!results.is_empty());
        for v in &results {
            assert!(v.is_finite());
        }
    }

    #[test]
    fn test_streaming_cfo_reset() {
        let mut ind = StreamingCfo::new(5);
        for i in 0..10 {
            ind.next(100.0 + i as f64);
        }
        assert!(ind.is_ready());

        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), None);
    }

    #[test]
    fn test_streaming_cfo_meta() {
        let ind = StreamingCfo::new(14);
        assert_eq!(StreamingCfo::name(), "CFO");
        assert_eq!(StreamingCfo::category(), "momentum");
        assert_eq!(ind.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..40)
            .map(|i| 100.0 + (i as f64 * 0.2).sin() * 5.0 + i as f64 * 0.05)
            .collect();
        let period = 14;

        let batch = crate::indicators::momentum_ext::chande_forecast_oscillator(&data, period).unwrap();

        let mut streaming = StreamingCfo::new(period);
        for i in 0..data.len() {
            let result = streaming.next(data[i]);
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
