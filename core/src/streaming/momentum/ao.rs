use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;
use crate::streaming::Ohlcv;

/// Streaming Awesome Oscillator (AO) —Bill Williams.
///
/// AO = SMA(median_price, fast) - SMA(median_price, slow)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAo {
    fast_period: usize,
    slow_period: usize,
    fast_sma: StreamingSma,
    slow_sma: StreamingSma,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAo {
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            fast_sma: StreamingSma::new(fast_period),
            slow_sma: StreamingSma::new(slow_period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv, f64> for StreamingAo {
    #[inline]
    fn next(&mut self, input: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;
        let median = input.median_price();
        let fast = self.fast_sma.next(median);
        let slow = self.slow_sma.next(median);
        let result = match (fast, slow) {
            (Some(f), Some(s)) => Some(f - s),
            _ => None,
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.fast_sma.reset();
        self.slow_sma.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.fast_sma.is_ready() && self.slow_sma.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAo {
    fn name() -> &'static str { "AO" }
    fn category() -> &'static str { "momentum" }
    fn description() -> &'static str { "Awesome Oscillator" }
    fn warm_up_period(&self) -> usize { self.fast_period.max(self.slow_period) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::OhlcvBar;

    #[test]
    fn test_streaming_ao_basic() {
        let mut ao = StreamingAo::new(5, 34);
        let bars: Vec<OhlcvBar> = (0..50)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.2).sin() * 10.0;
                let l = h - 3.0;
                OhlcvBar::new(h - 1.0, h, l, (h + l) / 2.0, 1000.0)
            })
            .collect();

        let mut last = None;
        for bar in &bars {
            last = ao.next(bar as &dyn Ohlcv);
        }
        assert!(last.is_some());
        assert!(ao.is_ready());
    }

    #[test]
    fn test_streaming_ao_meta() {
        let ao = StreamingAo::new(5, 34);
        assert_eq!(StreamingAo::name(), "AO");
        assert_eq!(StreamingAo::category(), "momentum");
        assert_eq!(ao.warm_up_period(), 34);
    }

    #[test]
    fn test_streaming_ao_reset() {
        let mut ao = StreamingAo::new(5, 34);
        let bars: Vec<OhlcvBar> = (0..50)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.2).sin() * 10.0;
                let l = h - 3.0;
                OhlcvBar::new(h - 1.0, h, l, (h + l) / 2.0, 1000.0)
            })
            .collect();
        for bar in &bars {
            ao.next(bar as &dyn Ohlcv);
        }
        assert!(ao.is_ready());
        ao.reset();
        assert!(!ao.is_ready());
        assert_eq!(ao.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let high: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();
        let bars: Vec<OhlcvBar> = (0..n)
            .map(|i| OhlcvBar::new(high[i] - 1.0, high[i], low[i], (high[i] + low[i]) / 2.0, 1000.0))
            .collect();

        let fast = 5;
        let slow = 34;
        let batch = crate::indicators::momentum_ext::ao(&high, &low, fast, slow).unwrap();

        let mut streaming = StreamingAo::new(fast, slow);
        for (i, bar) in bars.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(bar as &dyn Ohlcv), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-10,
                    "Mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
