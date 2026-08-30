use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming PSY (psychology line).
///
/// PSY = (Number of up days in last N periods) / N * 100
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingPsy {
    period: usize,
    buffer: Vec<u8>,
    head: usize,
    len: usize,
    up_count: usize,
    prev_close: Option<f64>,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingPsy {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![0; period],
            head: 0,
            len: 0,
            up_count: 0,
            prev_close: None,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingPsy {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        if let Some(prev) = self.prev_close {
            let is_up = u8::from(input > prev);

            if self.len == self.period {
                self.up_count -= self.buffer[self.head] as usize;
            } else {
                self.len += 1;
            }

            self.buffer[self.head] = is_up;
            self.up_count += is_up as usize;
            self.head += 1;
            if self.head == self.period {
                self.head = 0;
            }
        }

        self.prev_close = Some(input);

        let result = if self.len < self.period {
            None
        } else {
            Some(self.up_count as f64 / self.period as f64 * 100.0)
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.up_count = 0;
        self.prev_close = None;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingPsy {
    fn name() -> &'static str {
        "PSY"
    }

    fn category() -> &'static str {
        "sentiment"
    }

    fn description() -> &'static str {
        "Psychological Line (心理�?"
    }

    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::StreamingIndicator;

    #[test]
    fn test_streaming_psy_basic() {
        let mut psy = StreamingPsy::new(3);
        // Need period+1 bars: first bar sets prev_close, then 3 comparisons
        assert_eq!(psy.next(10.0), None);
        assert_eq!(psy.next(12.0), None);
        assert_eq!(psy.next(11.0), None);
        // closes: 10,12,11,13 -> ups at 12>10, 13>11 = 2/3*100
        let v = psy.next(13.0).unwrap();
        assert!((v - 200.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_psy_reset() {
        let mut psy = StreamingPsy::new(5);
        for i in 0..10 {
            psy.next(50.0 + (i as f64 * 0.5).sin());
        }
        assert!(psy.is_ready());
        psy.reset();
        assert!(!psy.is_ready());
        assert_eq!(psy.count(), 0);
    }

    #[test]
    fn test_streaming_psy_meta() {
        let psy = StreamingPsy::new(12);
        assert_eq!(StreamingPsy::name(), "PSY");
        assert_eq!(StreamingPsy::category(), "sentiment");
        assert_eq!(psy.warm_up_period(), 13);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.15).sin() * 10.0)
            .collect();
        let period = 12;

        let batch = crate::indicators::china::psy(&data, period).unwrap();

        let mut streaming = StreamingPsy::new(period);
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
