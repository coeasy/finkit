use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMom {
    period: usize,
    buf: Vec<f64>,
    head: usize,
    len: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMom {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buf: vec![0.0; period + 1],
            head: 0,
            len: 0,
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    fn cap(&self) -> usize {
        self.period + 1
    }

    #[inline]
    fn oldest(&self) -> f64 {
        self.buf[self.head]
    }
}

impl StreamingIndicator for StreamingMom {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: f64) -> Option<f64> {
        crate::streaming_measure!("mom", self.count, {
            self.count += 1;
            let cap = self.cap();

            if self.len < cap {
                self.buf[(self.head + self.len) % cap] = input;
                self.len += 1;
            } else {
                self.buf[self.head] = input;
                self.head = (self.head + 1) % cap;
            }

            if self.len <= self.period {
                self.last_value = None;
                return None;
            }

            let result = Some(input - self.oldest());
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len > self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingMom {
    fn name() -> &'static str {
        "MOM"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Momentum"
    }
    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_mom_basic() {
        let mut mom = StreamingMom::new(2);
        assert_eq!(mom.next(1.0), None);
        assert_eq!(mom.next(2.0), None);
        let v = mom.next(4.0).unwrap();
        assert!((v - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_mom_meta() {
        assert_eq!(StreamingMom::name(), "MOM");
    }

    #[test]
    fn test_streaming_mom_reset() {
        let mut mom = StreamingMom::new(3);
        for i in 0..10 {
            mom.next(i as f64);
        }
        assert!(mom.is_ready());
        mom.reset();
        assert!(!mom.is_ready());
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..50)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let period = 5;
        let batch = crate::indicators::momentum::mom(&data, period).unwrap();
        let mut streaming = StreamingMom::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!((s - batch[i]).abs() < 1e-10, "Mismatch at {i}");
            }
        }
    }
}
