use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming QStick Indicator.
///
/// QStick = SMA(Close - Open, period). Streaming version uses SMA smoothing.
/// Input: `(open, close)` tuple per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingQStick {
    period: usize,
    ring: Vec<f64>,
    ring_idx: usize,
    sum: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingQStick {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            ring: vec![0.0; period],
            ring_idx: 0,
            sum: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64), f64> for StreamingQStick {
    #[inline]
    fn next(&mut self, input: (f64, f64)) -> Option<f64> {
        let (open, close) = input;
        let diff = close - open;

        let old = self.ring[self.ring_idx];
        self.ring[self.ring_idx] = diff;
        self.ring_idx += 1;
        if self.ring_idx == self.period {
            self.ring_idx = 0;
        }

        if self.count < self.period {
            self.sum += diff;
            self.count += 1;
        } else {
            self.sum += diff - old;
        }

        if self.count >= self.period {
            let val = self.sum / self.period as f64;
            self.last_value = Some(val);
            Some(val)
        } else {
            self.last_value = None;
            None
        }
    }

    fn reset(&mut self) {
        self.ring.fill(0.0);
        self.ring_idx = 0;
        self.sum = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingQStick {
    fn name() -> &'static str {
        "QStick"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "QStick: MA of (Close - Open) for trend confirmation"
    }
    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_qstick_basic() {
        let mut ind = StreamingQStick::new(5);
        let open = [100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0];
        let close = [101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0];

        let mut results = Vec::new();
        for i in 0..open.len() {
            if let Some(val) = ind.next((open[i], close[i])) {
                results.push((i, val));
            }
        }
        assert!(!results.is_empty());
        for (_, v) in &results {
            assert!((*v - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_streaming_qstick_reset() {
        let mut ind = StreamingQStick::new(3);
        ind.next((100.0, 101.0));
        ind.next((101.0, 102.0));
        ind.next((102.0, 103.0));
        assert!(ind.is_ready());

        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), None);
    }

    #[test]
    fn test_streaming_qstick_meta() {
        let ind = StreamingQStick::new(10);
        assert_eq!(StreamingQStick::name(), "QStick");
        assert_eq!(StreamingQStick::category(), "momentum");
        assert_eq!(ind.warm_up_period(), 10);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        use crate::indicators::MaType;
        let n = 30;
        let open: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64 * 0.3).sin() * 2.0).collect();
        let close: Vec<f64> = open.iter().map(|o| o + 0.5).collect();
        let period = 10;

        let batch = crate::indicators::momentum_ext::qstick(&open, &close, period, MaType::Sma).unwrap();

        let mut streaming = StreamingQStick::new(period);
        for i in 0..n {
            let result = streaming.next((open[i], close[i]));
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
