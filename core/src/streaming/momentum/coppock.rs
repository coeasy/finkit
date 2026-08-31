use crate::impl_standard_methods;
use crate::streaming::momentum::roc::StreamingRoc;
use crate::streaming::overlap::wma::StreamingWma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Coppock Curve.
///
/// Coppock = WMA(ROC(long) + ROC(short), wma_period)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingCoppock {
    wma_period: usize,
    long_roc_period: usize,
    short_roc_period: usize,
    long_roc: StreamingRoc,
    short_roc: StreamingRoc,
    wma: StreamingWma,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingCoppock {
    pub fn new(wma_period: usize, long_roc: usize, short_roc: usize) -> Self {
        Self {
            wma_period,
            long_roc_period: long_roc,
            short_roc_period: short_roc,
            long_roc: StreamingRoc::new(long_roc),
            short_roc: StreamingRoc::new(short_roc),
            wma: StreamingWma::new(wma_period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingCoppock {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let lr = self.long_roc.next(input);
        let sr = self.short_roc.next(input);
        match (lr, sr) {
            (Some(l), Some(s)) => {
                let combined = if l.is_nan() { 0.0 } else { l } + if s.is_nan() { 0.0 } else { s };
                let result = self.wma.next(combined);
                self.last_value = result;
                result
            }
            _ => {
                self.last_value = None;
                None
            }
        }
    }

    fn reset(&mut self) {
        self.long_roc.reset();
        self.short_roc.reset();
        self.wma.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.wma.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingCoppock {
    fn name() -> &'static str {
        "Coppock"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Coppock Curve"
    }
    fn warm_up_period(&self) -> usize {
        self.long_roc_period.max(self.short_roc_period) + self.wma_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_coppock_basic() {
        let mut coppock = StreamingCoppock::new(10, 14, 11);
        let data: Vec<f64> = (0..50)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let mut last = None;
        for &v in &data {
            last = coppock.next(v);
        }
        assert!(last.is_some());
        assert!(coppock.is_ready());
    }

    #[test]
    fn test_streaming_coppock_meta() {
        let c = StreamingCoppock::new(10, 14, 11);
        assert_eq!(StreamingCoppock::name(), "Coppock");
        assert_eq!(StreamingCoppock::category(), "momentum");
        assert_eq!(c.warm_up_period(), 24);
    }

    #[test]
    fn test_streaming_coppock_reset() {
        let mut c = StreamingCoppock::new(10, 14, 11);
        for i in 0..50 {
            c.next(i as f64 + 1.0);
        }
        assert!(c.is_ready());
        c.reset();
        assert!(!c.is_ready());
        assert_eq!(c.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.15).sin() * 20.0)
            .collect();
        let wma_period = 10;
        let long_roc = 14;
        let short_roc = 11;

        let batch =
            crate::indicators::momentum_ext::coppock(&data, wma_period, long_roc, short_roc)
                .unwrap();

        let mut streaming = StreamingCoppock::new(wma_period, long_roc, short_roc);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-10,
                    "Mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
