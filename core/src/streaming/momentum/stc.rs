use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Schaff Trend Cycle (STC).
///
/// Applies MACD (fast EMA - slow EMA), then double stochastic smoothing
/// with recursive factor 0.5.
/// Uses O(1) amortized monotonic deques for rolling max/min.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingStc {
    fast_period: usize,
    slow_period: usize,
    cycle: usize,
    fast_ema: StreamingEma,
    slow_ema: StreamingEma,
    // First stochastic: rolling max/min of MACD over `cycle`
    macd_buf: Vec<f64>,
    macd_head: usize,
    macd_len: usize,
    macd_rolling_max: RollingMax,
    macd_rolling_min: RollingMin,
    // Recursive smooth of first stoch %K
    smooth1: f64,
    smooth1_init: bool,
    // Second stochastic: rolling max/min of smooth1 over `cycle`
    s1_buf: Vec<f64>,
    s1_head: usize,
    s1_len: usize,
    s1_rolling_max: RollingMax,
    s1_rolling_min: RollingMin,
    // Recursive smooth of second stoch %K
    smooth2: f64,
    smooth2_init: bool,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingStc {
    pub fn new(fast_period: usize, slow_period: usize, cycle: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            cycle,
            fast_ema: StreamingEma::new(fast_period),
            slow_ema: StreamingEma::new(slow_period),
            macd_buf: vec![0.0; cycle],
            macd_head: 0,
            macd_len: 0,
            macd_rolling_max: RollingMax::new(),
            macd_rolling_min: RollingMin::new(),
            smooth1: f64::NAN,
            smooth1_init: false,
            s1_buf: vec![0.0; cycle],
            s1_head: 0,
            s1_len: 0,
            s1_rolling_max: RollingMax::new(),
            s1_rolling_min: RollingMin::new(),
            smooth2: f64::NAN,
            smooth2_init: false,
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    fn ring_push(buf: &mut [f64], head: &mut usize, len: &mut usize, val: f64) {
        let cap = buf.len();
        if *len < cap {
            buf[(*head + *len) % cap] = val;
            *len += 1;
        } else {
            buf[*head] = val;
            *head = (*head + 1) % cap;
        }
    }

    #[inline]
    fn stoch_k(value: f64, lowest: f64, highest: f64) -> f64 {
        let range = highest - lowest;
        if range.abs() > 1e-15 {
            (value - lowest) / range * 100.0
        } else {
            50.0
        }
    }
}

impl StreamingIndicator for StreamingStc {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        let fe = self.fast_ema.next(input);
        let se = self.slow_ema.next(input);

        let macd = match (fe, se) {
            (Some(f), Some(s)) => f - s,
            _ => {
                self.last_value = None;
                return None;
            }
        };

        // Push MACD to rolling deques for O(1) amortized max/min
        self.macd_rolling_max.push(self.count, macd);
        self.macd_rolling_min.push(self.count, macd);
        if self.count > self.cycle {
            self.macd_rolling_max.pop(self.count - self.cycle);
            self.macd_rolling_min.pop(self.count - self.cycle);
        }

        Self::ring_push(
            &mut self.macd_buf,
            &mut self.macd_head,
            &mut self.macd_len,
            macd,
        );

        if self.macd_len < self.cycle {
            self.last_value = None;
            return None;
        }

        let macd_lo = self.macd_rolling_min.current().unwrap_or(f64::INFINITY);
        let macd_hi = self.macd_rolling_max.current().unwrap_or(f64::NEG_INFINITY);
        let k1 = Self::stoch_k(macd, macd_lo, macd_hi);

        let s1 = if !self.smooth1_init {
            self.smooth1 = k1;
            self.smooth1_init = true;
            k1
        } else {
            self.smooth1 = 0.5 * k1 + 0.5 * self.smooth1;
            self.smooth1
        };

        // Push s1 to rolling deques for O(1) amortized max/min
        self.s1_rolling_max.push(self.count, s1);
        self.s1_rolling_min.push(self.count, s1);
        if self.count > self.cycle {
            self.s1_rolling_max.pop(self.count - self.cycle);
            self.s1_rolling_min.pop(self.count - self.cycle);
        }

        Self::ring_push(&mut self.s1_buf, &mut self.s1_head, &mut self.s1_len, s1);

        if self.s1_len < self.cycle {
            self.last_value = None;
            return None;
        }

        let s1_lo = self.s1_rolling_min.current().unwrap_or(f64::INFINITY);
        let s1_hi = self.s1_rolling_max.current().unwrap_or(f64::NEG_INFINITY);
        let k2 = Self::stoch_k(s1, s1_lo, s1_hi);

        let s2 = if !self.smooth2_init {
            self.smooth2 = k2;
            self.smooth2_init = true;
            k2
        } else {
            self.smooth2 = 0.5 * k2 + 0.5 * self.smooth2;
            self.smooth2
        };

        self.last_value = Some(s2);
        Some(s2)
    }

    fn reset(&mut self) {
        self.fast_ema.reset();
        self.slow_ema.reset();
        self.macd_head = 0;
        self.macd_len = 0;
        self.macd_rolling_max.reset();
        self.macd_rolling_min.reset();
        self.smooth1 = f64::NAN;
        self.smooth1_init = false;
        self.s1_head = 0;
        self.s1_len = 0;
        self.s1_rolling_max.reset();
        self.s1_rolling_min.reset();
        self.smooth2 = f64::NAN;
        self.smooth2_init = false;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.smooth2_init
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingStc {
    fn name() -> &'static str {
        "STC"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Schaff Trend Cycle"
    }
    fn warm_up_period(&self) -> usize {
        self.slow_period + 2 * self.cycle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_stc_basic() {
        let mut stc = StreamingStc::new(23, 50, 10);
        let data: Vec<f64> = (0..150)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 20.0)
            .collect();
        let mut last = None;
        for &v in &data {
            last = stc.next(v);
        }
        assert!(last.is_some());
        assert!(stc.is_ready());
        let v = last.unwrap();
        assert!(
            (-1.0..=101.0).contains(&v),
            "STC value {v} out of expected range"
        );
    }

    #[test]
    fn test_streaming_stc_meta() {
        let stc = StreamingStc::new(23, 50, 10);
        assert_eq!(StreamingStc::name(), "STC");
        assert_eq!(StreamingStc::category(), "momentum");
        assert_eq!(stc.warm_up_period(), 70);
    }

    #[test]
    fn test_streaming_stc_reset() {
        let mut stc = StreamingStc::new(23, 50, 10);
        for i in 0..150 {
            stc.next(i as f64 + 1.0);
        }
        assert!(stc.is_ready());
        stc.reset();
        assert!(!stc.is_ready());
        assert_eq!(stc.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..300)
            .map(|i| 50.0 + (i as f64 * 0.05).sin() * 20.0 + (i as f64 * 0.13).cos() * 5.0)
            .collect();

        let batch = crate::indicators::momentum_ext::stc(&data, 23, 50, 10).unwrap();

        let mut streaming = StreamingStc::new(23, 50, 10);
        let mut match_count = 0;
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                if (s - batch[i]).abs() < 1.0 {
                    match_count += 1;
                }
            }
        }
        assert!(
            match_count > 100,
            "Expected convergence, got only {match_count} matches"
        );
    }
}
