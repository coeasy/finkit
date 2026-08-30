use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;
use crate::indicators::volatility_ext::AdrMode;

/// Streaming Average Day Range (ADR).
///
/// Input: `(high, low, close)` per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAdr {
    period: usize,
    mode: AdrMode,
    ring: Vec<f64>,
    ring_idx: usize,
    sum: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAdr {
    pub fn new(period: usize, mode: AdrMode) -> Self {
        Self {
            period,
            mode,
            ring: vec![0.0; period],
            ring_idx: 0,
            sum: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64), f64> for StreamingAdr {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<f64> {
        let (high, low, close) = input;

        let range_val = match self.mode {
            AdrMode::Absolute => high - low,
            AdrMode::Percent => {
                if close.abs() > 1e-15 {
                    (high - low) / close * 100.0
                } else {
                    0.0
                }
            }
        };

        let old = self.ring[self.ring_idx];
        self.ring[self.ring_idx] = range_val;
        self.ring_idx += 1;
        if self.ring_idx == self.period {
            self.ring_idx = 0;
        }

        if self.count < self.period {
            self.sum += range_val;
            self.count += 1;
        } else {
            self.sum += range_val - old;
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

impl IndicatorMeta for StreamingAdr {
    fn name() -> &'static str {
        "ADR"
    }
    fn category() -> &'static str {
        "volatility"
    }
    fn description() -> &'static str {
        "Average Day Range: mean of High-Low range over a period"
    }
    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_adr_basic() {
        let mut ind = StreamingAdr::new(5, AdrMode::Absolute);
        let high = [12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let low = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
        let close = [11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];

        let mut results = Vec::new();
        for i in 0..high.len() {
            if let Some(val) = ind.next((high[i], low[i], close[i])) {
                results.push((i, val));
            }
        }
        assert!(!results.is_empty());
        for (_, v) in &results {
            assert!((*v - 2.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_streaming_adr_reset() {
        let mut ind = StreamingAdr::new(3, AdrMode::Absolute);
        ind.next((12.0, 10.0, 11.0));
        ind.next((13.0, 11.0, 12.0));
        ind.next((14.0, 12.0, 13.0));
        assert!(ind.is_ready());

        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), None);
    }

    #[test]
    fn test_streaming_adr_meta() {
        let ind = StreamingAdr::new(10, AdrMode::Absolute);
        assert_eq!(StreamingAdr::name(), "ADR");
        assert_eq!(StreamingAdr::category(), "volatility");
        assert_eq!(ind.warm_up_period(), 10);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 30;
        let high: Vec<f64> = (0..n).map(|i| 110.0 + (i as f64 * 0.3).sin() * 5.0).collect();
        let low: Vec<f64> = (0..n).map(|i| 90.0 + (i as f64 * 0.3).sin() * 5.0).collect();
        let close: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64 * 0.3).sin() * 5.0).collect();
        let period = 10;

        let batch = crate::indicators::volatility_ext::adr(
            &high, &low, &close, period, AdrMode::Absolute,
        )
        .unwrap();

        let mut streaming = StreamingAdr::new(period, AdrMode::Absolute);
        for i in 0..n {
            let result = streaming.next((high[i], low[i], close[i]));
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
