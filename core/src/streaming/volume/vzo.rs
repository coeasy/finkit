use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming Volume Zone Oscillator (VZO).
///
/// VZO = EMA(VP, period) / EMA(TV, period) × 100
/// Input: `(close, volume)` per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingVzo {
    period: usize,
    alpha: f64,
    ema_vp: f64,
    ema_tv: f64,
    prev_close: f64,
    // SMA accumulation for seed
    vp_sum: f64,
    tv_sum: f64,
    init_count: usize,
    ema_started: bool,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingVzo {
    pub fn new(period: usize) -> Self {
        let alpha = 2.0 / (period as f64 + 1.0);
        Self {
            period,
            alpha,
            ema_vp: 0.0,
            ema_tv: 0.0,
            prev_close: f64::NAN,
            vp_sum: 0.0,
            tv_sum: 0.0,
            init_count: 0,
            ema_started: false,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64), f64> for StreamingVzo {
    #[inline]
    fn next(&mut self, input: (f64, f64)) -> Option<f64> {
        let (close, volume) = input;
        self.count += 1;

        // Compute VP for this bar
        let vp = if self.prev_close.is_nan() {
            0.0 // First bar: no previous close
        } else if close > self.prev_close {
            volume
        } else if close < self.prev_close {
            -volume
        } else {
            0.0
        };
        self.prev_close = close;

        if !self.ema_started {
            self.vp_sum += vp;
            self.tv_sum += volume;
            self.init_count += 1;

            if self.init_count == self.period {
                // Seed EMA with SMA of first period values
                self.ema_vp = self.vp_sum / self.period as f64;
                self.ema_tv = self.tv_sum / self.period as f64;
                self.ema_started = true;

                if self.ema_tv.abs() > 1e-15 {
                    let val = (self.ema_vp / self.ema_tv) * 100.0;
                    self.last_value = Some(val);
                    return Some(val);
                }
            }
            self.last_value = None;
            return None;
        }

        // EMA update
        self.ema_vp = self.alpha * vp + (1.0 - self.alpha) * self.ema_vp;
        self.ema_tv = self.alpha * volume + (1.0 - self.alpha) * self.ema_tv;

        if self.ema_tv.abs() > 1e-15 {
            let val = (self.ema_vp / self.ema_tv) * 100.0;
            self.last_value = Some(val);
            Some(val)
        } else {
            self.last_value = Some(0.0);
            Some(0.0)
        }
    }

    fn reset(&mut self) {
        self.ema_vp = 0.0;
        self.ema_tv = 0.0;
        self.prev_close = f64::NAN;
        self.vp_sum = 0.0;
        self.tv_sum = 0.0;
        self.init_count = 0;
        self.ema_started = false;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.ema_started
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingVzo {
    fn name() -> &'static str {
        "VZO"
    }
    fn category() -> &'static str {
        "volume"
    }
    fn description() -> &'static str {
        "Volume Zone Oscillator: EMA-smoothed volume pressure ratio"
    }
    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_vzo_basic() {
        let mut ind = StreamingVzo::new(14);
        let n = 30;
        let close: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.5).collect();
        let volume: Vec<f64> = (0..n).map(|i| 1000.0 + (i as f64 * 0.7).sin() * 200.0).collect();

        let mut results = Vec::new();
        for i in 0..n {
            if let Some(val) = ind.next((close[i], volume[i])) {
                results.push((i, val));
            }
        }
        assert!(!results.is_empty());
        for (_, v) in &results {
            assert!(v.is_finite());
            assert!(*v > 0.0); // uptrend
        }
    }

    #[test]
    fn test_streaming_vzo_reset() {
        let mut ind = StreamingVzo::new(5);
        for i in 0..15 {
            ind.next((100.0 + i as f64, 1000.0));
        }
        assert!(ind.is_ready());

        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), None);
    }

    #[test]
    fn test_streaming_vzo_meta() {
        let ind = StreamingVzo::new(14);
        assert_eq!(StreamingVzo::name(), "VZO");
        assert_eq!(StreamingVzo::category(), "volume");
        assert_eq!(ind.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 40;
        let close: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64 * 0.2).sin() * 5.0 + i as f64 * 0.1).collect();
        let volume: Vec<f64> = (0..n).map(|i| 1000.0 + (i as f64 * 0.5).cos() * 200.0).collect();
        let period = 14;

        let batch = crate::indicators::volume_ext::vzo(&close, &volume, period).unwrap();

        let mut streaming = StreamingVzo::new(period);
        for i in 0..n {
            let result = streaming.next((close[i], volume[i]));
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
