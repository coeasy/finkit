use crate::streaming::traits::StreamingIndicator;
use crate::{impl_indicator_meta, impl_standard_methods};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingKama {
    period: usize,
    fast_sc: f64,
    slow_sc: f64,
    kama: f64,
    buf: Vec<f64>,
    head: usize,
    len: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingKama {
    pub fn new(period: usize) -> Self {
        Self::with_scales(period, 2, 30)
    }

    pub fn with_scales(period: usize, fast_period: usize, slow_period: usize) -> Self {
        Self {
            period,
            fast_sc: 2.0 / (fast_period as f64 + 1.0),
            slow_sc: 2.0 / (slow_period as f64 + 1.0),
            kama: f64::NAN,
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
    fn ring_get(&self, i: usize) -> f64 {
        self.buf[(self.head + i) % self.cap()]
    }
}

impl StreamingIndicator for StreamingKama {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
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
            if self.len == self.period {
                self.kama = input;
                let result = Some(self.kama);
                self.last_value = result;
                return result;
            }
            self.last_value = None;
            return None;
        }

        let direction = (input - self.ring_get(0)).abs();
        let mut volatility = 0.0;
        for i in 1..self.len {
            volatility += (self.ring_get(i) - self.ring_get(i - 1)).abs();
        }

        let er = if volatility.abs() > 1e-15 {
            direction / volatility
        } else {
            0.0
        };
        let sc = (er * (self.fast_sc - self.slow_sc) + self.slow_sc).powi(2);
        self.kama += sc * (input - self.kama);
        let result = Some(self.kama);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.kama = f64::NAN;
        self.head = 0;
        self.len = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len >= self.period
    }
    impl_standard_methods!();
}

impl_indicator_meta!(
    StreamingKama,
    "KAMA",
    "overlap",
    "Kaufman Adaptive Moving Average"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_kama_basic() {
        let mut kama = StreamingKama::new(10);
        for i in 0..15 {
            kama.next(i as f64 + 1.0);
        }
        assert!(kama.is_ready());
    }

    #[test]
    fn test_streaming_kama_meta() {
        assert_eq!(StreamingKama::name(), "KAMA");
    }

    #[test]
    fn test_streaming_kama_reset() {
        let mut kama = StreamingKama::new(5);
        for i in 0..10 {
            kama.next(i as f64);
        }
        assert!(kama.is_ready());
        kama.reset();
        assert!(!kama.is_ready());
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 10;
        let batch = crate::math::moving_avg::kama(&data, period, 2, 30).unwrap();
        let mut streaming = StreamingKama::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-10,
                    "Mismatch at {i}: s={s}, b={}",
                    batch[i]
                );
            }
        }
    }
}
