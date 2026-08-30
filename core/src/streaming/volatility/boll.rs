use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BollOutput {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct StreamingBoll {
    period: usize,
    nb_dev_up: f64,
    nb_dev_dn: f64,
    buffer: Vec<f64>,
    head: usize,
    len: usize,
    sum: f64,
    sum_sq: f64,
    count: usize,
    inv_n: f64,
    inv_n_minus_1: f64,
    last_value: Option<BollOutput>,
    #[cfg_attr(feature = "serde", serde(skip))]
    snapshot: Option<SnapshotState>,
    last_open_time: i64,
}

#[derive(Clone, Copy)]
struct SnapshotState {
    head: usize,
    len: usize,
    sum: f64,
    sum_sq: f64,
    count: usize,
    last_value: Option<BollOutput>,
    last_open_time: i64,
    head_val: f64,
}

impl StreamingBoll {
    pub fn new(period: usize, nb_dev_up: f64, nb_dev_dn: f64) -> Self {
        Self {
            period,
            nb_dev_up,
            nb_dev_dn,
            buffer: vec![0.0; period],
            head: 0,
            len: 0,
            sum: 0.0,
            sum_sq: 0.0,
            count: 0,
            inv_n: 1.0 / period as f64,
            inv_n_minus_1: 1.0 / (period as f64 - 1.0),
            last_value: None,
            snapshot: None,
            last_open_time: 0,
        }
    }

    pub fn compute_bar(&mut self, bar: &dyn Ohlcv) -> Option<BollOutput> {
        let t = bar.open_time();
        if t != 0 && t == self.last_open_time {
            if let Some(snap) = self.snapshot.take() {
                self.head = snap.head;
                self.len = snap.len;
                self.sum = snap.sum;
                self.sum_sq = snap.sum_sq;
                self.count = snap.count;
                self.last_value = snap.last_value;
                self.last_open_time = snap.last_open_time;
                self.buffer[snap.head] = snap.head_val;
            }
        }
        self.snapshot = Some(SnapshotState {
            head: self.head,
            len: self.len,
            sum: self.sum,
            sum_sq: self.sum_sq,
            count: self.count,
            last_value: self.last_value,
            last_open_time: self.last_open_time,
            head_val: self.buffer[self.head],
        });
        self.last_open_time = t;
        self.next(bar.close())
    }
}

impl StreamingIndicator<f64, BollOutput> for StreamingBoll {
    #[inline]
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip(self, input)))]
    fn next(&mut self, input: f64) -> Option<BollOutput> {
        crate::streaming_measure!("bbands", self.count, {
            self.count += 1;
            self.sum += input;
            self.sum_sq += input * input;

            if self.len == self.period {
                let old = self.buffer[self.head];
                self.sum -= old;
                self.sum_sq -= old * old;
            } else {
                self.len += 1;
            }

            self.buffer[self.head] = input;
            self.head += 1;
            if self.head == self.period {
                self.head = 0;
            }

            if self.len < self.period {
                self.last_value = None;
                return None;
            }

            let mean = self.sum * self.inv_n;
            // TA-Lib uses population variance (÷n), not sample variance (÷n-1)
            let variance = (self.sum_sq - self.sum * mean) * self.inv_n;
            let std_dev = variance.max(0.0).sqrt();

            let result = Some(BollOutput {
                middle: mean,
                upper: mean + std_dev * self.nb_dev_up,
                lower: mean - std_dev * self.nb_dev_dn,
            });
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum = 0.0;
        self.sum_sq = 0.0;
        self.count = 0;
        self.last_value = None;
        self.snapshot = None;
        self.last_open_time = 0;
    }

    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

        impl_standard_methods!(output = BollOutput);


}

impl IndicatorMeta for StreamingBoll {
    fn name() -> &'static str {
        "BBANDS"
    }

    fn category() -> &'static str {
        "overlap"
    }

    fn description() -> &'static str {
        "Bollinger Bands"
    }

    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_boll_basic() {
        let mut boll = StreamingBoll::new(5, 2.0, 2.0);
        for i in 1..=5 {
            let out = boll.next(i as f64);
            if i < 5 {
                assert!(out.is_none());
            } else {
                let out = out.unwrap();
                assert!(out.upper > out.middle);
                assert!(out.lower < out.middle);
            }
        }
    }

    #[test]
    fn test_streaming_boll_reset() {
        let mut boll = StreamingBoll::new(3, 2.0, 2.0);
        for i in 1..=5 {
            boll.next(i as f64);
        }
        assert!(boll.is_ready());
        boll.reset();
        assert!(!boll.is_ready());
        assert_eq!(boll.count(), 0);
    }

    #[test]
    fn test_streaming_boll_meta() {
        let boll = StreamingBoll::new(20, 2.0, 2.0);
        assert_eq!(StreamingBoll::name(), "BBANDS");
        assert_eq!(StreamingBoll::category(), "overlap");
        assert_eq!(boll.warm_up_period(), 20);
    }

    #[test]
    fn test_streaming_boll_repaint() {
        use crate::streaming::OhlcvBar;

        let mut boll = StreamingBoll::new(5, 2.0, 2.0);
        boll.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 1.0, 0.0, 1000));
        boll.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 2.0, 0.0, 2000));
        boll.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 3.0, 0.0, 3000));
        boll.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 4.0, 0.0, 4000));
        boll.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 5.0, 0.0, 5000));

        boll.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 10.0, 0.0, 6000));
        boll.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 20.0, 0.0, 6000));
        let result_repaint = boll.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 3.0, 0.0, 6000));

        let mut boll_clean = StreamingBoll::new(5, 2.0, 2.0);
        boll_clean.next(1.0);
        boll_clean.next(2.0);
        boll_clean.next(3.0);
        boll_clean.next(4.0);
        boll_clean.next(5.0);
        let result_clean = boll_clean.next(3.0);

        let result_repaint = result_repaint.unwrap();
        let result_clean = result_clean.unwrap();
        assert!((result_repaint.upper - result_clean.upper).abs() < 1e-10);
        assert!((result_repaint.middle - result_clean.middle).abs() < 1e-10);
        assert!((result_repaint.lower - result_clean.lower).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 14;
        let nb_dev = 2.0;

        let batch_result =
            crate::indicators::overlap::bbands(&data, period, nb_dev, nb_dev).unwrap();

        let mut streaming = StreamingBoll::new(period, nb_dev, nb_dev);
        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if !batch_result.middle[i].is_nan() {
                    assert!(
                        (s.middle - batch_result.middle[i]).abs() < 1e-10,
                        "Middle mismatch at index {i}: streaming={}, batch={}",
                        s.middle,
                        batch_result.middle[i]
                    );
                }
                if !batch_result.upper[i].is_nan() {
                    assert!(
                        (s.upper - batch_result.upper[i]).abs() < 1e-10,
                        "Upper mismatch at index {i}: streaming={}, batch={}",
                        s.upper,
                        batch_result.upper[i]
                    );
                }
                if !batch_result.lower[i].is_nan() {
                    assert!(
                        (s.lower - batch_result.lower[i]).abs() < 1e-10,
                        "Lower mismatch at index {i}: streaming={}, batch={}",
                        s.lower,
                        batch_result.lower[i]
                    );
                }
            }
        }
    }
}
