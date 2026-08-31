use crate::streaming::traits::{Ohlcv, StreamingIndicator};
use crate::{impl_indicator_meta, impl_standard_methods};

/// Re-exported so callers can pick an EMA seeding convention without a second
/// import path (see [`crate::math::moving_avg::EmaSeed`] for the contract).
pub use crate::math::moving_avg::EmaSeed;

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct StreamingEma {
    period: usize,
    seed: EmaSeed,
    multiplier: f64,
    decay: f64,
    inv_period: f64,
    value: f64,
    count: usize,
    sum: f64,
    last_value: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip))]
    snapshot: Option<SnapshotState>,
    last_open_time: i64,
}

#[derive(Clone, Copy)]
struct SnapshotState {
    value: f64,
    count: usize,
    sum: f64,
    last_value: Option<f64>,
    last_open_time: i64,
}

impl StreamingEma {
    /// Create a streaming EMA seeded with the SMA of the first `period` inputs
    /// (warm-up `NaN` for `0..period-1`). This preserves the historical default
    /// and matches the standalone batch [`crate::math::moving_avg::ema`].
    pub fn new(period: usize) -> Self {
        Self::with_seed(period, EmaSeed::Sma)
    }

    /// Create a streaming EMA with an explicit [`EmaSeed`] convention.
    ///
    /// Use [`EmaSeed::FirstValue`] when you need the recursion seeded with the
    /// first observed value (valid immediately, no warm-up) — e.g. to converge
    /// with `macd` / `macdfix`, which internally use `FirstValue`. Note that
    /// [`EmaSeed::FirstValue`] makes `is_ready()` true after the very first
    /// `next()`, whereas [`EmaSeed::Sma`] requires `period` observations.
    pub fn with_seed(period: usize, seed: EmaSeed) -> Self {
        let multiplier = 2.0 / (period as f64 + 1.0);
        Self {
            period,
            seed,
            multiplier,
            decay: 1.0 - multiplier,
            inv_period: 1.0 / period as f64,
            value: f64::NAN,
            count: 0,
            sum: 0.0,
            last_value: None,
            snapshot: None,
            last_open_time: 0,
        }
    }

    pub fn compute_bar(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        let t = bar.open_time();
        if t != 0 && t == self.last_open_time {
            if let Some(snap) = self.snapshot.take() {
                self.value = snap.value;
                self.count = snap.count;
                self.sum = snap.sum;
                self.last_value = snap.last_value;
                self.last_open_time = snap.last_open_time;
            }
        }
        self.snapshot = Some(SnapshotState {
            value: self.value,
            count: self.count,
            sum: self.sum,
            last_value: self.last_value,
            last_open_time: self.last_open_time,
        });
        self.last_open_time = t;
        self.next(bar.close())
    }
}

impl StreamingIndicator for StreamingEma {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: f64) -> Option<f64> {
        #[cfg(feature = "metrics")]
        let __start = std::time::Instant::now();
        self.count += 1;

        // FirstValue seed: the recursion starts at input[0] and is valid
        // immediately (no NaN warm-up). Used by macd / macdfix-style pipelines.
        if self.seed == EmaSeed::FirstValue {
            if self.count == 1 {
                self.value = input;
            } else {
                self.value = input * self.multiplier + self.value * self.decay;
            }
            let result = Some(self.value);
            self.last_value = result;
            #[cfg(feature = "metrics")]
            {
                crate::metrics::streaming_next("ema", true);
                crate::metrics::record_indicator_duration(
                    "ema_streaming",
                    __start.elapsed().as_secs_f64(),
                );
            }
            return result;
        }

        if self.count < self.period {
            self.sum += input;
            self.last_value = None;
            #[cfg(feature = "metrics")]
            {
                crate::metrics::streaming_next("ema", false);
                crate::metrics::record_indicator_duration(
                    "ema_streaming",
                    __start.elapsed().as_secs_f64(),
                );
            }
            return None;
        }

        if self.count == self.period {
            self.sum += input;
            self.value = self.sum * self.inv_period;
        } else {
            self.value = input * self.multiplier + self.value * self.decay;
        }

        let result = Some(self.value);
        self.last_value = result;
        #[cfg(feature = "metrics")]
        {
            crate::metrics::streaming_next("ema", true);
            crate::metrics::record_indicator_duration(
                "ema_streaming",
                __start.elapsed().as_secs_f64(),
            );
        }
        result
    }

    fn next_with_time(&mut self, input: f64, open_time: i64) -> Option<f64> {
        if open_time != 0 && open_time == self.last_open_time {
            if let Some(snap) = self.snapshot.take() {
                self.value = snap.value;
                self.count = snap.count;
                self.sum = snap.sum;
                self.last_value = snap.last_value;
                self.last_open_time = snap.last_open_time;
            }
        }
        self.snapshot = Some(SnapshotState {
            value: self.value,
            count: self.count,
            sum: self.sum,
            last_value: self.last_value,
            last_open_time: self.last_open_time,
        });
        self.last_open_time = open_time;
        self.next(input)
    }

    fn reset(&mut self) {
        self.value = f64::NAN;
        self.count = 0;
        self.sum = 0.0;
        self.last_value = None;
        self.snapshot = None;
        self.last_open_time = 0;
    }

    fn is_ready(&self) -> bool {
        match self.seed {
            EmaSeed::Sma => self.count >= self.period,
            // FirstValue seed produces a valid value from the first sample.
            EmaSeed::FirstValue => self.count >= 1,
        }
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingEma, "EMA", "overlap", "Exponential Moving Average");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_ema_basic() {
        let mut ema = StreamingEma::new(3);
        assert_eq!(ema.next(2.0), None);
        assert_eq!(ema.next(4.0), None);
        let v3 = ema.next(6.0).unwrap();
        assert!((v3 - 4.0).abs() < 1e-10);
        let v4 = ema.next(8.0).unwrap();
        assert!((v4 - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_ema_reset() {
        let mut ema = StreamingEma::new(2);
        ema.next(10.0);
        ema.next(20.0);
        assert!(ema.is_ready());
        ema.reset();
        assert!(!ema.is_ready());
        assert_eq!(ema.count(), 0);
    }

    #[test]
    fn test_streaming_ema_meta() {
        assert_eq!(StreamingEma::name(), "EMA");
        assert_eq!(StreamingEma::category(), "overlap");
    }

    #[test]
    fn test_streaming_ema_repaint() {
        use crate::streaming::OhlcvBar;

        let mut ema = StreamingEma::new(3);
        ema.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 1.0, 0.0, 1000));
        ema.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 2.0, 0.0, 2000));
        ema.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 3.0, 0.0, 3000));

        ema.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 10.0, 0.0, 4000));
        ema.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 20.0, 0.0, 4000));
        let result_repaint =
            ema.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 3.0, 0.0, 4000));

        let mut ema_clean = StreamingEma::new(3);
        ema_clean.next(1.0);
        ema_clean.next(2.0);
        ema_clean.next(3.0);
        let result_clean = ema_clean.next(3.0);

        assert_eq!(result_repaint, result_clean);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 14;

        let batch_result = crate::math::moving_avg::ema(&data, period).unwrap();

        let mut streaming = StreamingEma::new(period);
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

    #[test]
    fn test_streaming_ema_first_value_seed_single_step() {
        // FirstValue seed: valid from the first sample, starts at input[0].
        let mut ema = StreamingEma::with_seed(3, EmaSeed::FirstValue);
        assert_eq!(ema.next(2.0), Some(2.0)); // seed = input[0]
        let v2 = ema.next(4.0).unwrap();
        let k = 2.0 / 4.0;
        let expected = 4.0 * k + 2.0 * (1.0 - k);
        assert!((v2 - expected).abs() < 1e-10);
        assert!(ema.is_ready()); // ready after first sample
    }

    #[test]
    fn test_streaming_vs_batch_first_value_seed() {
        // A FirstValue-seeded streaming EMA must converge with the batch
        // FirstValue variant (the seeding convention is the only difference).
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 14;

        let batch =
            crate::math::moving_avg::ema_with_seed(&data, period, EmaSeed::FirstValue).unwrap();

        let mut streaming = StreamingEma::with_seed(period, EmaSeed::FirstValue);
        for (i, &val) in data.iter().enumerate() {
            let s = streaming.next(val).expect("FirstValue EMA is always ready");
            assert!(
                (s - batch[i]).abs() < 1e-9,
                "FirstValue mismatch at index {i}: streaming={s}, batch={}",
                batch[i]
            );
        }
    }
}
