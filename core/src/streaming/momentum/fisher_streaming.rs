use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FisherOutput {
    pub fisher: f64,
    pub signal: f64,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingFisher {
    period: usize,
    highest: RollingMax,
    lowest: RollingMin,
    value_prev: f64,
    fisher_prev: f64,
    count: usize,
    last_value: Option<FisherOutput>,
}

impl StreamingFisher {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            highest: RollingMax::new(),
            lowest: RollingMin::new(),
            value_prev: 0.0,
            fisher_prev: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64), FisherOutput> for StreamingFisher {
    #[inline]
    fn next(&mut self, input: (f64, f64)) -> Option<FisherOutput> {
        let (high, low) = input;
        self.count += 1;
        let idx = self.count - 1;

        self.highest.push(idx, high);
        self.lowest.push(idx, low);

        if idx >= self.period {
            let expired = idx - self.period;
            self.highest.pop(expired);
            self.lowest.pop(expired);
        }

        if self.count < self.period {
            self.last_value = None;
            return None;
        }

        let highest = self.highest.current().unwrap();
        let lowest = self.lowest.current().unwrap();

        let mid = (high + low) / 2.0;
        let range = highest - lowest;

        let normalized = if range.abs() > 1e-15 {
            2.0 * ((mid - lowest) / range - 0.5)
        } else {
            0.0
        };

        let mut value = 0.33 * normalized + 0.67 * self.value_prev;
        value = value.clamp(-0.999, 0.999);
        self.value_prev = value;

        let signal = self.fisher_prev;
        let fisher_val = 0.5 * ((1.0 + value) / (1.0 - value)).ln() + 0.5 * self.fisher_prev;
        self.fisher_prev = fisher_val;

        let result = Some(FisherOutput {
            fisher: fisher_val,
            signal,
        });
        self.last_value = result;
        result
    }

    #[inline]
    fn reset(&mut self) {
        self.highest.reset();
        self.lowest.reset();
        self.value_prev = 0.0;
        self.fisher_prev = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    #[inline]
    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!(output = FisherOutput);
}

impl IndicatorMeta for StreamingFisher {
    fn name() -> &'static str { "Fisher" }
    fn category() -> &'static str { "momentum" }
    fn description() -> &'static str { "Ehlers Fisher Transform" }
    fn warm_up_period(&self) -> usize { self.period }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_streaming_fisher_basic() {
        let mut fisher = StreamingFisher::new(3);
        assert_eq!(fisher.next((11.0, 9.0)), None);
        assert_eq!(fisher.next((12.0, 10.0)), None);
        let out = fisher.next((13.0, 11.0)).unwrap();
        assert!(!out.fisher.is_nan());
        assert!(out.fisher.abs() <= 10.0);
    }

    #[test]
    fn test_streaming_fisher_meta() {
        let fisher = StreamingFisher::new(10);
        assert_eq!(StreamingFisher::name(), "Fisher");
        assert_eq!(StreamingFisher::category(), "momentum");
        assert_eq!(fisher.warm_up_period(), 10);
    }

    #[test]
    fn test_streaming_fisher_reset() {
        let mut fisher = StreamingFisher::new(3);
        for i in 0..5 {
            fisher.next((10.0 + i as f64, 8.0 + i as f64));
        }
        assert!(fisher.is_ready());
        fisher.reset();
        assert!(!fisher.is_ready());
        assert_eq!(fisher.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 80;
        let high: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.15).sin() * 10.0 + 5.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();
        let period = 10;

        let batch = crate::indicators::momentum_ext::fisher(&high, &low, period).unwrap();
        let mut streaming = StreamingFisher::new(period);

        for i in 0..n {
            if let Some(s) = streaming.next((high[i], low[i])) {
                if !batch.fisher[i].is_nan() {
                    assert_relative_eq!(s.fisher, batch.fisher[i], epsilon = 1e-10);
                }
                if !batch.signal[i].is_nan() {
                    assert_relative_eq!(s.signal, batch.signal[i], epsilon = 1e-10);
                }
            }
        }
    }
}
