use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Twiggs Money Flow.
///
/// TMF = EMA(Vol × AD_ratio) / EMA(Vol) where AD_ratio uses True Range.
/// Input: `(high, low, close, volume)` per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingTwiggsMf {
    period: usize,
    alpha: f64,
    ema_ad: f64,
    ema_vol: f64,
    prev_close: f64,
    initialized: bool,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingTwiggsMf {
    pub fn new(period: usize) -> Self {
        let alpha = 2.0 / (period as f64 + 1.0);
        Self {
            period,
            alpha,
            ema_ad: 0.0,
            ema_vol: 0.0,
            prev_close: f64::NAN,
            initialized: false,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64, f64), f64> for StreamingTwiggsMf {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64, f64)) -> Option<f64> {
        let (high, low, close, volume) = input;
        self.count += 1;

        if self.count == 1 {
            self.prev_close = close;
            self.last_value = None;
            return None;
        }

        let true_high = high.max(self.prev_close);
        let true_low = low.min(self.prev_close);
        let tr = true_high - true_low;

        let ad_val = if tr > 1e-15 {
            volume * (2.0 * close - true_high - true_low) / tr
        } else {
            0.0
        };

        if !self.initialized {
            self.ema_ad = ad_val;
            self.ema_vol = volume;
            self.initialized = true;
        } else {
            self.ema_ad = self.alpha * ad_val + (1.0 - self.alpha) * self.ema_ad;
            self.ema_vol = self.alpha * volume + (1.0 - self.alpha) * self.ema_vol;
        }

        self.prev_close = close;

        if self.count > self.period && self.ema_vol.abs() > 1e-15 {
            let tmf = self.ema_ad / self.ema_vol;
            self.last_value = Some(tmf);
            Some(tmf)
        } else {
            self.last_value = None;
            None
        }
    }

    fn reset(&mut self) {
        self.ema_ad = 0.0;
        self.ema_vol = 0.0;
        self.prev_close = f64::NAN;
        self.initialized = false;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count > self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingTwiggsMf {
    fn name() -> &'static str {
        "TwiggsMF"
    }
    fn category() -> &'static str {
        "volume"
    }
    fn description() -> &'static str {
        "Twiggs Money Flow: EMA-smoothed AD with True Range normalization"
    }
    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_twiggs_basic() {
        let mut ind = StreamingTwiggsMf::new(14);
        let n = 30;
        let high: Vec<f64> = (0..n).map(|i| 105.0 + i as f64 * 0.5).collect();
        let low: Vec<f64> = (0..n).map(|i| 95.0 + i as f64 * 0.5).collect();
        let close: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.5).collect();
        let volume: Vec<f64> = (0..n)
            .map(|i| 1000.0 + (i as f64 * 0.7).sin() * 200.0)
            .collect();

        let mut results = Vec::new();
        for i in 0..n {
            if let Some(val) = ind.next((high[i], low[i], close[i], volume[i])) {
                results.push((i, val));
            }
        }
        assert!(!results.is_empty());
        for (_, v) in &results {
            assert!(v.is_finite());
        }
    }

    #[test]
    fn test_streaming_twiggs_reset() {
        let mut ind = StreamingTwiggsMf::new(5);
        for i in 0..15 {
            let p = 100.0 + i as f64;
            ind.next((p + 2.0, p - 2.0, p, 1000.0));
        }
        assert!(ind.is_ready());

        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), None);
    }

    #[test]
    fn test_streaming_twiggs_meta() {
        let ind = StreamingTwiggsMf::new(14);
        assert_eq!(StreamingTwiggsMf::name(), "TwiggsMF");
        assert_eq!(StreamingTwiggsMf::category(), "volume");
        assert_eq!(ind.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 40;
        let high: Vec<f64> = (0..n)
            .map(|i| 105.0 + (i as f64 * 0.2).sin() * 3.0)
            .collect();
        let low: Vec<f64> = (0..n)
            .map(|i| 95.0 + (i as f64 * 0.2).sin() * 3.0)
            .collect();
        let close: Vec<f64> = (0..n)
            .map(|i| 100.0 + (i as f64 * 0.2).sin() * 3.0)
            .collect();
        let volume: Vec<f64> = (0..n)
            .map(|i| 1000.0 + (i as f64 * 0.5).cos() * 200.0)
            .collect();
        let period = 14;

        let batch =
            crate::indicators::volume_ext::twiggs_money_flow(&high, &low, &close, &volume, period)
                .unwrap();

        let mut streaming = StreamingTwiggsMf::new(period);
        for i in 0..n {
            let result = streaming.next((high[i], low[i], close[i], volume[i]));
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
