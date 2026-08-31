use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct StreamingRsi {
    period: usize,
    inv_period: f64,
    decay: f64,
    avg_gain: f64,
    avg_loss: f64,
    sum_gain: f64,
    sum_loss: f64,
    prev_input: f64,
    count: usize,
    last_value: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip))]
    snapshot: Option<SnapshotState>,
    last_open_time: i64,
}

#[derive(Clone, Copy)]
struct SnapshotState {
    avg_gain: f64,
    avg_loss: f64,
    sum_gain: f64,
    sum_loss: f64,
    prev_input: f64,
    count: usize,
    last_value: Option<f64>,
    last_open_time: i64,
}

impl StreamingRsi {
    pub fn new(period: usize) -> Self {
        let inv_period = 1.0 / period as f64;
        Self {
            period,
            inv_period,
            decay: (period as f64 - 1.0) * inv_period,
            avg_gain: 0.0,
            avg_loss: 0.0,
            sum_gain: 0.0,
            sum_loss: 0.0,
            prev_input: f64::NAN,
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
                self.avg_gain = snap.avg_gain;
                self.avg_loss = snap.avg_loss;
                self.sum_gain = snap.sum_gain;
                self.sum_loss = snap.sum_loss;
                self.prev_input = snap.prev_input;
                self.count = snap.count;
                self.last_value = snap.last_value;
                self.last_open_time = snap.last_open_time;
            }
        }
        self.snapshot = Some(SnapshotState {
            avg_gain: self.avg_gain,
            avg_loss: self.avg_loss,
            sum_gain: self.sum_gain,
            sum_loss: self.sum_loss,
            prev_input: self.prev_input,
            count: self.count,
            last_value: self.last_value,
            last_open_time: self.last_open_time,
        });
        self.last_open_time = t;
        self.next(bar.close())
    }
}

impl StreamingIndicator for StreamingRsi {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: f64) -> Option<f64> {
        crate::streaming_measure!("rsi", self.count, {
            self.count += 1;

            if self.count == 1 {
                self.prev_input = input;
                self.last_value = None;
                return None;
            }

            let change = input - self.prev_input;
            self.prev_input = input;
            let gain = change.max(0.0);
            let loss = (-change).max(0.0);

            if self.count <= self.period + 1 {
                self.sum_gain += gain;
                self.sum_loss += loss;

                if self.count == self.period + 1 {
                    self.avg_gain = self.sum_gain * self.inv_period;
                    self.avg_loss = self.sum_loss * self.inv_period;
                } else {
                    self.last_value = None;
                    return None;
                }
            } else {
                self.avg_gain = self.avg_gain * self.decay + gain * self.inv_period;
                self.avg_loss = self.avg_loss * self.decay + loss * self.inv_period;
            }

            let result = if self.avg_loss.abs() < 1e-15 {
                Some(100.0)
            } else {
                let rs = self.avg_gain / self.avg_loss;
                Some(100.0 - (100.0 / (1.0 + rs)))
            };
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.avg_gain = 0.0;
        self.avg_loss = 0.0;
        self.sum_gain = 0.0;
        self.sum_loss = 0.0;
        self.prev_input = f64::NAN;
        self.count = 0;
        self.last_value = None;
        self.snapshot = None;
        self.last_open_time = 0;
    }

    fn is_ready(&self) -> bool {
        self.count > self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingRsi {
    fn name() -> &'static str {
        "RSI"
    }

    fn category() -> &'static str {
        "momentum"
    }

    fn description() -> &'static str {
        "Relative Strength Index"
    }

    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_rsi_all_gains() {
        let mut rsi = StreamingRsi::new(5);
        for i in 0..6 {
            rsi.next(10.0 + i as f64);
        }
        let val = rsi.next(20.0).unwrap();
        assert!(val > 50.0);
        assert!(val <= 100.0);
    }

    #[test]
    fn test_streaming_rsi_range() {
        let mut rsi = StreamingRsi::new(14);
        let data = [
            44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 45.89, 46.03,
            45.61, 46.28, 46.28, 46.00, 46.03, 46.41, 46.22, 45.64,
        ];
        let mut last = None;
        for &d in &data {
            last = rsi.next(d);
        }
        let last = last.unwrap();
        assert!((0.0..=100.0).contains(&last));
    }

    #[test]
    fn test_streaming_rsi_meta() {
        let rsi = StreamingRsi::new(14);
        assert_eq!(StreamingRsi::name(), "RSI");
        assert_eq!(StreamingRsi::category(), "momentum");
        assert_eq!(rsi.warm_up_period(), 15);
    }

    #[test]
    fn test_streaming_rsi_reset() {
        let mut rsi = StreamingRsi::new(5);
        for i in 0..10 {
            rsi.next(i as f64);
        }
        assert!(rsi.is_ready());
        rsi.reset();
        assert!(!rsi.is_ready());
        assert_eq!(rsi.count(), 0);
    }

    #[test]
    fn test_streaming_rsi_repaint() {
        use crate::streaming::OhlcvBar;

        let mut rsi = StreamingRsi::new(5);
        rsi.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 1.0, 0.0, 1000));
        rsi.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 2.0, 0.0, 2000));
        rsi.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 3.0, 0.0, 3000));
        rsi.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 4.0, 0.0, 4000));
        rsi.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 5.0, 0.0, 5000));
        rsi.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 6.0, 0.0, 6000));

        rsi.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 10.0, 0.0, 7000));
        rsi.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 20.0, 0.0, 7000));
        let result_repaint =
            rsi.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 3.0, 0.0, 7000));

        let mut rsi_clean = StreamingRsi::new(5);
        rsi_clean.next(1.0);
        rsi_clean.next(2.0);
        rsi_clean.next(3.0);
        rsi_clean.next(4.0);
        rsi_clean.next(5.0);
        rsi_clean.next(6.0);
        let result_clean = rsi_clean.next(3.0);

        assert_eq!(result_repaint, result_clean);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 14;

        let batch_result = crate::indicators::momentum::rsi(&data, period).unwrap();

        let mut streaming = StreamingRsi::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch_result[i].is_nan()) {
                assert!(
                    (s - batch_result[i]).abs() < 1e-10,
                    "Mismatch at index {i}: streaming={s}, batch={}",
                    batch_result[i]
                );
            }
        }
    }
}
