use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming DPO (Detrended Price Oscillator 去趋势价格振荡器).
///
/// DPO = Close - SMA(Close, period) shifted back by (period / 2 + 1) bars.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingDpo {
    period: usize,
    shift: usize,
    sma: StreamingSma,
    sma_buf: Vec<f64>,
    sma_head: usize,
    sma_len: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingDpo {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            shift: period / 2 + 1,
            sma: StreamingSma::new(period),
            sma_buf: vec![0.0; period / 2 + 1],
            sma_head: 0,
            sma_len: 0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingDpo {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        let lagged = if self.sma_len == self.shift {
            Some(self.sma_buf[self.sma_head])
        } else {
            None
        };

        if let Some(sma_val) = self.sma.next(input) {
            if self.sma_len == self.shift {
                self.sma_head = (self.sma_head + 1) % self.shift;
            } else {
                self.sma_len += 1;
            }
            let idx = (self.sma_head + self.sma_len - 1) % self.shift;
            self.sma_buf[idx] = sma_val;
        }

        let result = lagged.map(|s| input - s);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.sma.reset();
        self.sma_head = 0;
        self.sma_len = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.sma_len >= self.shift && self.sma.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingDpo {
    fn name() -> &'static str {
        "DPO"
    }

    fn category() -> &'static str {
        "momentum"
    }

    fn description() -> &'static str {
        "Detrended Price Oscillator (去趋势价格振荡器)"
    }

    fn warm_up_period(&self) -> usize {
        self.period + self.period / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_dpo_basic() {
        let mut dpo = StreamingDpo::new(5);
        for i in 1..=10 {
            dpo.next(i as f64 * 10.0);
        }
        assert!(dpo.is_ready());
        let v = dpo.value().unwrap();
        assert!(v.is_finite());
    }

    #[test]
    fn test_streaming_dpo_reset() {
        let mut dpo = StreamingDpo::new(5);
        for i in 0..20 {
            dpo.next(i as f64 + 1.0);
        }
        assert!(dpo.is_ready());
        dpo.reset();
        assert!(!dpo.is_ready());
        assert_eq!(dpo.count(), 0);
    }

    #[test]
    fn test_streaming_dpo_meta() {
        let dpo = StreamingDpo::new(20);
        assert_eq!(StreamingDpo::name(), "DPO");
        assert_eq!(StreamingDpo::category(), "momentum");
        assert_eq!(dpo.warm_up_period(), 30);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 20;

        let batch = crate::indicators::china::dpo(&data, period).unwrap();

        let mut streaming = StreamingDpo::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-10,
                    "Mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
