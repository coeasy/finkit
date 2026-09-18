use crate::impl_standard_methods;
use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming DPO (Detrended Price Oscillator 去趋势价格振荡器).
///
/// DPO = Close shifted back by (period / 2 + 1) bars - current SMA(Close, period).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingDpo {
    period: usize,
    shift: usize,
    sma: StreamingSma,
    price_buf: Vec<f64>,
    price_head: usize,
    price_len: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingDpo {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            shift: period / 2 + 1,
            sma: StreamingSma::new(period),
            price_buf: vec![0.0; period / 2 + 1],
            price_head: 0,
            price_len: 0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingDpo {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        let displaced = if self.price_len == self.shift {
            Some(self.price_buf[self.price_head])
        } else {
            None
        };

        if self.shift > 0 {
            self.price_buf[self.price_head] = input;
            self.price_head = (self.price_head + 1) % self.shift;
            if self.price_len < self.shift {
                self.price_len += 1;
            } else {
                self.price_len = self.shift;
            }
        }

        let result = self
            .sma
            .next(input)
            .zip(displaced)
            .map(|(sma, price)| price - sma);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.sma.reset();
        self.price_head = 0;
        self.price_len = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.price_len >= self.shift && self.sma.is_ready()
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
        (self.period - 1).max(self.shift)
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
        assert_eq!(dpo.warm_up_period(), 19);
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
