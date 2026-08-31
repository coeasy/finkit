use crate::streaming::traits::{Ohlcv, StreamingIndicator};
use crate::utils::true_range;
use crate::{impl_indicator_meta, impl_standard_methods};

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct StreamingAtr {
    period: usize,
    /// Wilder's RMA accumulator
    atr_val: f64,
    /// Sum of TR during warm-up phase (first `period` bars)
    tr_sum: f64,
    prev_close: f64,
    count: usize,
    last_value: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip))]
    snapshot: Option<SnapshotState>,
    last_open_time: i64,
}

#[derive(Clone, Copy)]
struct SnapshotState {
    atr_val: f64,
    tr_sum: f64,
    prev_close: f64,
    count: usize,
    last_value: Option<f64>,
    last_open_time: i64,
}

impl StreamingAtr {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            atr_val: 0.0,
            tr_sum: 0.0,
            prev_close: f64::NAN,
            count: 0,
            last_value: None,
            snapshot: None,
            last_open_time: 0,
        }
    }

    pub fn compute_bar(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        let t = bar.open_time();
        if t != 0 && t == self.last_open_time {
            if let Some(snap) = self.snapshot.take() {
                self.atr_val = snap.atr_val;
                self.tr_sum = snap.tr_sum;
                self.prev_close = snap.prev_close;
                self.count = snap.count;
                self.last_value = snap.last_value;
                self.last_open_time = snap.last_open_time;
            }
        }
        self.snapshot = Some(SnapshotState {
            atr_val: self.atr_val,
            tr_sum: self.tr_sum,
            prev_close: self.prev_close,
            count: self.count,
            last_value: self.last_value,
            last_open_time: self.last_open_time,
        });
        self.last_open_time = t;
        self.next((bar.high(), bar.low(), bar.close()))
    }
}

impl StreamingIndicator<(f64, f64, f64)> for StreamingAtr {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<f64> {
        crate::streaming_measure!("atr", self.count, {
            let (high, low, close) = input;
            self.count += 1;

            let tr = if self.count == 1 {
                high - low
            } else {
                true_range(high, low, self.prev_close)
            };
            self.prev_close = close;

            let inv_period = 1.0 / self.period as f64;

            if self.count < self.period {
                // Warm-up: accumulate TR, no output yet
                self.tr_sum += tr;
                self.last_value = None;
                None
            } else if self.count == self.period {
                // First ATR = SMA of first `period` TR values
                self.tr_sum += tr;
                self.atr_val = self.tr_sum * inv_period;
                self.last_value = Some(self.atr_val);
                Some(self.atr_val)
            } else {
                // Wilder's RMA: ATR[i] = ATR[i-1] + (TR[i] - ATR[i-1]) / period
                self.atr_val += (tr - self.atr_val) * inv_period;
                self.last_value = Some(self.atr_val);
                Some(self.atr_val)
            }
        })
    }

    fn reset(&mut self) {
        self.atr_val = 0.0;
        self.tr_sum = 0.0;
        self.prev_close = f64::NAN;
        self.count = 0;
        self.last_value = None;
        self.snapshot = None;
        self.last_open_time = 0;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingAtr, "ATR", "volatility", "Average True Range");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_atr_basic() {
        let mut atr = StreamingAtr::new(3);
        assert_eq!(atr.next((12.0, 10.0, 11.0)), None);
        assert_eq!(atr.next((13.0, 11.0, 12.0)), None);
        let val = atr.next((14.0, 12.0, 13.0)).unwrap();
        assert!(val > 0.0);
    }

    #[test]
    fn test_streaming_atr_reset() {
        let mut atr = StreamingAtr::new(3);
        for i in 0..5 {
            atr.next((10.0 + i as f64, 8.0 + i as f64, 9.0 + i as f64));
        }
        assert!(atr.is_ready());
        atr.reset();
        assert!(!atr.is_ready());
        assert_eq!(atr.count(), 0);
    }

    #[test]
    fn test_streaming_atr_meta() {
        let atr = StreamingAtr::new(14);
        assert_eq!(StreamingAtr::name(), "ATR");
        assert_eq!(StreamingAtr::category(), "volatility");
        assert_eq!(atr.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_atr_repaint() {
        use crate::streaming::OhlcvBar;

        let bars: Vec<(f64, f64, f64)> = vec![
            (12.0, 10.0, 11.0),
            (13.0, 11.0, 12.0),
            (14.0, 12.0, 13.0),
            (15.0, 13.0, 14.0),
        ];

        let mut atr = StreamingAtr::new(3);
        for (i, &(h, l, c)) in bars.iter().enumerate() {
            atr.compute_bar(&OhlcvBar::new_with_time(
                0.0,
                h,
                l,
                c,
                0.0,
                (i + 1) as i64 * 1000,
            ));
        }

        // Repaint bar 5 three times
        atr.compute_bar(&OhlcvBar::new_with_time(0.0, 50.0, 10.0, 30.0, 0.0, 5000));
        atr.compute_bar(&OhlcvBar::new_with_time(0.0, 60.0, 5.0, 25.0, 0.0, 5000));
        let result_repaint =
            atr.compute_bar(&OhlcvBar::new_with_time(0.0, 16.0, 14.0, 15.0, 0.0, 5000));

        // Clean path
        let mut atr_clean = StreamingAtr::new(3);
        for &(h, l, c) in &bars {
            atr_clean.next((h, l, c));
        }
        let result_clean = atr_clean.next((16.0, 14.0, 15.0));

        assert!((result_repaint.unwrap() - result_clean.unwrap()).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let high: Vec<f64> = (0..n)
            .map(|i| 55.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 2.0).collect();
        let close: Vec<f64> = high
            .iter()
            .zip(low.iter())
            .map(|(h, l)| (h + l) / 2.0)
            .collect();
        let period = 14;

        let batch_result = crate::indicators::volatility::atr(&high, &low, &close, period).unwrap();

        let mut streaming = StreamingAtr::new(period);
        for i in 0..n {
            if let Some(s) = streaming.next((high[i], low[i], close[i])) {
                if !batch_result[i].is_nan() {
                    assert!(
                        (s - batch_result[i]).abs() < 1e-10,
                        "Mismatch at index {i}: streaming={s}, batch={}",
                        batch_result[i]
                    );
                }
            }
        }
    }
}
