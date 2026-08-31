use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::streaming::trend::adx::StreamingAdx;

/// Streaming Average Directional Movement Index Rating (ADXR).
///
/// ADXR = (ADX_today + ADX_n_periods_ago) / 2
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAdxr {
    period: usize,
    adx: StreamingAdx,
    buffer: Vec<f64>,
    head: usize,
    len: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAdxr {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            adx: StreamingAdx::new(period),
            buffer: vec![f64::NAN; period],
            head: 0,
            len: 0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64)> for StreamingAdxr {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<f64> {
        self.count += 1;

        let adx_val = self.adx.next(input);

        let Some(adx_now) = adx_val else {
            self.last_value = None;
            return None;
        };

        let old_idx = self.head;

        if self.len < self.period {
            let idx = (self.head + self.len) % self.period;
            self.buffer[idx] = adx_now;
            self.len += 1;
        } else {
            self.buffer[self.head] = adx_now;
            self.head = (self.head + 1) % self.period;
        }

        if self.len < self.period {
            self.last_value = None;
            return None;
        }

        let adx_ago = self.buffer[old_idx];
        if adx_ago.is_nan() {
            self.last_value = None;
            return None;
        }

        let result = (adx_now + adx_ago) / 2.0;
        self.last_value = Some(result);
        Some(result)
    }

    fn reset(&mut self) {
        self.adx.reset();
        for v in &mut self.buffer {
            *v = f64::NAN;
        }
        self.head = 0;
        self.len = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.last_value.is_some()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAdxr {
    fn name() -> &'static str {
        "ADXR"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Average Directional Movement Index Rating"
    }
    fn warm_up_period(&self) -> usize {
        self.period * 3 + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_data(n: usize) -> Vec<(f64, f64, f64)> {
        (0..n)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.3).sin() * 10.0;
                (h, h - 4.0, h - 2.0)
            })
            .collect()
    }

    #[test]
    fn test_streaming_adxr_basic() {
        let mut adxr = StreamingAdxr::new(14);
        let data = gen_data(100);
        let mut last = None;
        for &d in &data {
            if let Some(v) = adxr.next(d) {
                last = Some(v);
            }
        }
        let v = last.unwrap();
        assert!((0.0..=100.0).contains(&v), "ADXR should be 0-100, got {v}");
    }

    #[test]
    fn test_streaming_adxr_reset() {
        let mut adxr = StreamingAdxr::new(14);
        for &d in &gen_data(100) {
            adxr.next(d);
        }
        assert!(adxr.is_ready());
        adxr.reset();
        assert!(!adxr.is_ready());
        assert_eq!(adxr.count(), 0);
    }

    #[test]
    fn test_streaming_adxr_meta() {
        let _adxr = StreamingAdxr::new(14);
        assert_eq!(StreamingAdxr::name(), "ADXR");
        assert_eq!(StreamingAdxr::category(), "momentum");
    }
}
