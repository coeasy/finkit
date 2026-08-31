use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::streaming::traits::{IndicatorMeta, Ohlcv};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DonchianOutput {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingDonchian {
    period: usize,
    count: usize,
    rolling_max: RollingMax,
    rolling_min: RollingMin,
    last_value: Option<DonchianOutput>,
}

impl StreamingDonchian {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            count: 0,
            rolling_max: RollingMax::new(),
            rolling_min: RollingMin::new(),
            last_value: None,
        }
    }

    #[inline]
    pub fn next(&mut self, bar: &dyn Ohlcv) -> Option<DonchianOutput> {
        self.count += 1;
        let idx = self.count - 1;

        self.rolling_max.push(idx, bar.high());
        self.rolling_min.push(idx, bar.low());

        if self.count > self.period {
            let expire_idx = self.count - self.period - 1;
            self.rolling_max.pop(expire_idx);
            self.rolling_min.pop(expire_idx);
        }

        if self.count < self.period {
            self.last_value = None;
            return None;
        }

        let upper = self.rolling_max.current().unwrap();
        let lower = self.rolling_min.current().unwrap();
        let result = Some(DonchianOutput {
            upper,
            middle: (upper + lower) / 2.0,
            lower,
        });
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.count = 0;
        self.rolling_max.reset();
        self.rolling_min.reset();
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }
    pub fn count(&self) -> usize {
        self.count
    }

    pub fn value(&self) -> Option<DonchianOutput> {
        self.last_value
    }
}

impl IndicatorMeta for StreamingDonchian {
    fn name() -> &'static str {
        "Donchian"
    }
    fn category() -> &'static str {
        "volatility"
    }
    fn description() -> &'static str {
        "Donchian Channels"
    }
    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_donchian_basic() {
        let mut d = StreamingDonchian::new(3);
        let out = d.next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0));
        assert!(out.is_none());
        d.next(&OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 100.0));
        let out = d
            .next(&OhlcvBar::new(12.0, 14.0, 11.0, 13.0, 100.0))
            .unwrap();
        assert!((out.upper - 14.0).abs() < 1e-10);
        assert!((out.lower - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_donchian_meta() {
        assert_eq!(StreamingDonchian::name(), "Donchian");
    }

    #[test]
    fn test_streaming_donchian_reset() {
        let mut d = StreamingDonchian::new(3);
        for i in 0..5 {
            d.next(&OhlcvBar::new(
                i as f64,
                i as f64 + 2.0,
                i as f64 - 1.0,
                i as f64 + 1.0,
                100.0,
            ));
        }
        assert!(d.is_ready());
        d.reset();
        assert!(!d.is_ready());
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let highs = vec![12.0, 13.0, 14.0, 13.5, 14.5, 15.0, 14.0, 15.5, 16.0, 15.0];
        let lows = vec![10.0, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 13.5, 14.0, 13.0];
        let closes = vec![11.0, 12.0, 13.0, 12.0, 14.0, 14.5, 13.0, 15.0, 15.5, 14.0];
        let period = 3;
        let batch = crate::indicators::donchian::donchian(&highs, &lows, period).unwrap();
        let mut streaming = StreamingDonchian::new(period);
        for i in 0..closes.len() {
            let bar = OhlcvBar::new(closes[i] - 1.0, highs[i], lows[i], closes[i], 100.0);
            if let Some(out) = streaming.next(&bar) {
                if !batch.upper[i].is_nan() {
                    assert!(
                        (out.upper - batch.upper[i]).abs() < 1e-10,
                        "upper mismatch at {i}"
                    );
                    assert!(
                        (out.lower - batch.lower[i]).abs() < 1e-10,
                        "lower mismatch at {i}"
                    );
                }
            }
        }
    }
}
