use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Percentage Price Oscillator (PPO).
///
/// PPO = (fast_EMA - slow_EMA) / slow_EMA * 100
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingPpo {
    fast_ema: StreamingEma,
    slow_ema: StreamingEma,
    fast_period: usize,
    slow_period: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingPpo {
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self {
            fast_ema: StreamingEma::new(fast_period),
            slow_ema: StreamingEma::new(slow_period),
            fast_period,
            slow_period,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingPpo {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let fast = self.fast_ema.next(input);
        let slow = self.slow_ema.next(input);

        let (Some(f), Some(s)) = (fast, slow) else {
            self.last_value = None;
            return None;
        };

        if s.abs() < 1e-15 {
            self.last_value = Some(0.0);
            return Some(0.0);
        }

        let result = ((f - s) / s) * 100.0;
        self.last_value = Some(result);
        Some(result)
    }

    fn reset(&mut self) {
        self.fast_ema.reset();
        self.slow_ema.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.slow_ema.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingPpo {
    fn name() -> &'static str {
        "PPO"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Percentage Price Oscillator"
    }
    fn warm_up_period(&self) -> usize {
        self.slow_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_ppo_basic() {
        let mut ppo = StreamingPpo::new(12, 26);
        let data: Vec<f64> = (0..50)
            .map(|i| 100.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let mut last = None;
        for &d in &data {
            last = ppo.next(d);
        }
        assert!(last.is_some());
    }

    #[test]
    fn test_streaming_ppo_constant() {
        let mut ppo = StreamingPpo::new(5, 10);
        for _ in 0..20 {
            ppo.next(100.0);
        }
        let v = ppo.value().unwrap();
        assert!(
            v.abs() < 1e-10,
            "PPO of constant input should be ~0, got {v}"
        );
    }

    #[test]
    fn test_streaming_ppo_reset() {
        let mut ppo = StreamingPpo::new(12, 26);
        for i in 0..50 {
            ppo.next(100.0 + i as f64);
        }
        assert!(ppo.is_ready());
        ppo.reset();
        assert!(!ppo.is_ready());
        assert_eq!(ppo.count(), 0);
    }

    #[test]
    fn test_streaming_ppo_meta() {
        let ppo = StreamingPpo::new(12, 26);
        assert_eq!(StreamingPpo::name(), "PPO");
        assert_eq!(ppo.warm_up_period(), 26);
    }
}
