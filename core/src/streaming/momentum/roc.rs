use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingRoc {
    period: usize,
    buf: Vec<f64>,
    head: usize,
    len: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingRoc {
    pub fn new(period: usize) -> Self {
        Self {
            period,
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
    fn oldest(&self) -> f64 {
        self.buf[self.head]
    }
}

impl StreamingIndicator for StreamingRoc {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: f64) -> Option<f64> {
        crate::streaming_measure!("roc", self.count, {
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
                self.last_value = None;
                return None;
            }

            let prev = self.oldest();
            let result = if prev.abs() > 1e-15 {
                Some((input - prev) / prev * 100.0)
            } else {
                Some(0.0)
            };
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len > self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingRoc {
    fn name() -> &'static str {
        "ROC"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Rate of Change"
    }
    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

// ---------------------------------------------------------------------------
// Streaming ROCP (Rate of Change Percentage): (close - close_n) / close_n
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingRocp {
    period: usize,
    buf: Vec<f64>,
    head: usize,
    len: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingRocp {
    pub fn new(period: usize) -> Self {
        Self {
            period,
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
    fn oldest(&self) -> f64 {
        self.buf[self.head]
    }
}

impl StreamingIndicator for StreamingRocp {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: f64) -> Option<f64> {
        crate::streaming_measure!("rocp", self.count, {
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
                self.last_value = None;
                return None;
            }

            let prev = self.oldest();
            let result = if prev.abs() > 1e-15 {
                Some((input - prev) / prev)
            } else {
                Some(0.0)
            };
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len > self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingRocp {
    fn name() -> &'static str {
        "ROCP"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Rate of Change Percentage"
    }
    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

// ---------------------------------------------------------------------------
// Streaming ROCR (Rate of Change Ratio): close / close_n
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingRocr {
    period: usize,
    buf: Vec<f64>,
    head: usize,
    len: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingRocr {
    pub fn new(period: usize) -> Self {
        Self {
            period,
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
    fn oldest(&self) -> f64 {
        self.buf[self.head]
    }
}

impl StreamingIndicator for StreamingRocr {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: f64) -> Option<f64> {
        crate::streaming_measure!("rocr", self.count, {
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
                self.last_value = None;
                return None;
            }

            let prev = self.oldest();
            let result = if prev.abs() > 1e-15 {
                Some(input / prev)
            } else {
                Some(0.0)
            };
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len > self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingRocr {
    fn name() -> &'static str {
        "ROCR"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Rate of Change Ratio"
    }
    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

// ---------------------------------------------------------------------------
// Streaming ROCR100 (Rate of Change Ratio scaled to 100): (close / close_n) * 100
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingRocr100 {
    period: usize,
    buf: Vec<f64>,
    head: usize,
    len: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingRocr100 {
    pub fn new(period: usize) -> Self {
        Self {
            period,
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
    fn oldest(&self) -> f64 {
        self.buf[self.head]
    }
}

impl StreamingIndicator for StreamingRocr100 {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: f64) -> Option<f64> {
        crate::streaming_measure!("rocr100", self.count, {
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
                self.last_value = None;
                return None;
            }

            let prev = self.oldest();
            let result = if prev.abs() > 1e-15 {
                Some((input / prev) * 100.0)
            } else {
                Some(0.0)
            };
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len > self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingRocr100 {
    fn name() -> &'static str {
        "ROCR100"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Rate of Change Ratio scaled to 100"
    }
    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_roc_basic() {
        let mut roc = StreamingRoc::new(1);
        assert_eq!(roc.next(10.0), None);
        let v = roc.next(12.0).unwrap();
        assert!((v - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_roc_meta() {
        assert_eq!(StreamingRoc::name(), "ROC");
    }

    #[test]
    fn test_streaming_roc_reset() {
        let mut roc = StreamingRoc::new(3);
        for i in 0..10 {
            roc.next(i as f64 + 1.0);
        }
        assert!(roc.is_ready());
        roc.reset();
        assert!(!roc.is_ready());
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..50)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let period = 5;
        let batch = crate::indicators::momentum::roc(&data, period).unwrap();
        let mut streaming = StreamingRoc::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!((s - batch[i]).abs() < 1e-10, "Mismatch at {i}");
            }
        }
    }

    // ---------------- ROCP ----------------

    #[test]
    fn test_streaming_rocp_basic() {
        let mut rocp = StreamingRocp::new(2);
        assert_eq!(rocp.next(10.0), None);
        assert_eq!(rocp.next(11.0), None);
        let v = rocp.next(12.0).unwrap();
        // (12 - 10) / 10 = 0.2
        assert!((v - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_rocp_meta() {
        assert_eq!(StreamingRocp::name(), "ROCP");
        assert_eq!(StreamingRocp::category(), "momentum");
    }

    #[test]
    fn test_streaming_vs_batch_rocp() {
        let data: Vec<f64> = (0..60)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let period = 5;
        let batch = crate::indicators::momentum::rocp(&data, period).unwrap();
        let mut streaming = StreamingRocp::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!((s - batch[i]).abs() < 1e-10, "ROCP mismatch at {i}");
            }
        }
    }

    // ---------------- ROCR ----------------

    #[test]
    fn test_streaming_rocr_basic() {
        let mut rocr = StreamingRocr::new(2);
        assert_eq!(rocr.next(10.0), None);
        assert_eq!(rocr.next(11.0), None);
        let v = rocr.next(12.0).unwrap();
        // 12 / 10 = 1.2
        assert!((v - 1.2).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_rocr_meta() {
        assert_eq!(StreamingRocr::name(), "ROCR");
    }

    #[test]
    fn test_streaming_vs_batch_rocr() {
        let data: Vec<f64> = (0..60)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let period = 5;
        let batch = crate::indicators::momentum::rocr(&data, period).unwrap();
        let mut streaming = StreamingRocr::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!((s - batch[i]).abs() < 1e-10, "ROCR mismatch at {i}");
            }
        }
    }

    // ---------------- ROCR100 ----------------

    #[test]
    fn test_streaming_rocr100_basic() {
        let mut r = StreamingRocr100::new(2);
        assert_eq!(r.next(10.0), None);
        assert_eq!(r.next(11.0), None);
        let v = r.next(12.0).unwrap();
        // (12 / 10) * 100 = 120
        assert!((v - 120.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_rocr100_meta() {
        assert_eq!(StreamingRocr100::name(), "ROCR100");
    }

    #[test]
    fn test_streaming_vs_batch_rocr100() {
        let data: Vec<f64> = (0..60)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let period = 5;
        let batch = crate::indicators::momentum::rocr100(&data, period).unwrap();
        let mut streaming = StreamingRocr100::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!((s - batch[i]).abs() < 1e-10, "ROCR100 mismatch at {i}");
            }
        }
    }
}
