use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MacdOutput {
    pub macd: f64,
    pub signal: f64,
    pub histogram: f64,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMacd {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    fast_k: f64,
    slow_k: f64,
    signal_k: f64,
    // Pre-seed SMA accumulation (TA-Lib 兼容)
    slow_sum: f64,
    fast_sum: f64,
    fast_ema: f64,
    slow_ema: f64,
    ema_seeded: bool,
    macd_count: usize,
    sig_sum: f64,
    signal_ema: f64,
    signal_seeded: bool,
    count: usize,
    last_value: Option<MacdOutput>,
    #[cfg_attr(feature = "serde", serde(skip))]
    snapshot: Option<SnapshotState>,
    last_open_time: i64,
}

#[derive(Clone, Copy)]
struct SnapshotState {
    slow_sum: f64,
    fast_sum: f64,
    fast_ema: f64,
    slow_ema: f64,
    ema_seeded: bool,
    macd_count: usize,
    sig_sum: f64,
    signal_ema: f64,
    signal_seeded: bool,
    count: usize,
    last_value: Option<MacdOutput>,
    last_open_time: i64,
}

impl StreamingMacd {
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            signal_period,
            fast_k: 2.0 / (fast_period as f64 + 1.0),
            slow_k: 2.0 / (slow_period as f64 + 1.0),
            signal_k: 2.0 / (signal_period as f64 + 1.0),
            slow_sum: 0.0,
            fast_sum: 0.0,
            fast_ema: 0.0,
            slow_ema: 0.0,
            ema_seeded: false,
            macd_count: 0,
            sig_sum: 0.0,
            signal_ema: 0.0,
            signal_seeded: false,
            count: 0,
            last_value: None,
            snapshot: None,
            last_open_time: 0,
        }
    }

    pub fn compute_bar(&mut self, bar: &dyn Ohlcv) -> Option<MacdOutput> {
        let t = bar.open_time();
        if t != 0 && t == self.last_open_time {
            if let Some(snap) = self.snapshot.take() {
                self.slow_sum = snap.slow_sum;
                self.fast_sum = snap.fast_sum;
                self.fast_ema = snap.fast_ema;
                self.slow_ema = snap.slow_ema;
                self.ema_seeded = snap.ema_seeded;
                self.macd_count = snap.macd_count;
                self.sig_sum = snap.sig_sum;
                self.signal_ema = snap.signal_ema;
                self.signal_seeded = snap.signal_seeded;
                self.count = snap.count;
                self.last_value = snap.last_value;
                self.last_open_time = snap.last_open_time;
            }
        }
        self.snapshot = Some(SnapshotState {
            slow_sum: self.slow_sum,
            fast_sum: self.fast_sum,
            fast_ema: self.fast_ema,
            slow_ema: self.slow_ema,
            ema_seeded: self.ema_seeded,
            macd_count: self.macd_count,
            sig_sum: self.sig_sum,
            signal_ema: self.signal_ema,
            signal_seeded: self.signal_seeded,
            count: self.count,
            last_value: self.last_value,
            last_open_time: self.last_open_time,
        });
        self.last_open_time = t;
        self.next(bar.close())
    }
}

impl StreamingIndicator<f64, MacdOutput> for StreamingMacd {
    #[inline]
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip(self, input)))]
    fn next(&mut self, input: f64) -> Option<MacdOutput> {
        crate::streaming_measure!("macd", self.count, {
            self.count += 1;

            let offset = self.slow_period - self.fast_period;

            // TA-Lib 兼容：SMA 种子 + FMA 递推（与 batch macd_inner 完全一致）
            if !self.ema_seeded {
                // 累积 SMA 种子
                if self.count <= offset {
                    self.slow_sum += input;
                } else {
                    self.slow_sum += input;
                    self.fast_sum += input;
                }

                if self.count < self.slow_period {
                    self.last_value = None;
                    return None;
                }

                // 种子点：计算 SMA 种子
                self.slow_ema = self.slow_sum / self.slow_period as f64;
                self.fast_ema = self.fast_sum / self.fast_period as f64;
                self.ema_seeded = true;
            } else {
                // EMA 递推：FMA 精确匹配 TA-Lib 浮点舍入路径
                self.fast_ema = (input - self.fast_ema).mul_add(self.fast_k, self.fast_ema);
                self.slow_ema = (input - self.slow_ema).mul_add(self.slow_k, self.slow_ema);
            }

            let macd = self.fast_ema - self.slow_ema;

            // Signal：SMA 种子取前 signal_period 个 MACD 值
            if !self.signal_seeded {
                self.sig_sum += macd;
                self.macd_count += 1;
                if self.macd_count == self.signal_period {
                    self.signal_ema = self.sig_sum / self.signal_period as f64;
                    self.signal_seeded = true;
                } else {
                    self.last_value = None;
                    return None;
                }
            } else {
                self.signal_ema = (macd - self.signal_ema).mul_add(self.signal_k, self.signal_ema);
            }

            let signal = self.signal_ema;
            let histogram = macd - signal;

            let result = Some(MacdOutput {
                macd,
                signal,
                histogram,
            });
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.slow_sum = 0.0;
        self.fast_sum = 0.0;
        self.fast_ema = 0.0;
        self.slow_ema = 0.0;
        self.ema_seeded = false;
        self.macd_count = 0;
        self.sig_sum = 0.0;
        self.signal_ema = 0.0;
        self.signal_seeded = false;
        self.count = 0;
        self.last_value = None;
        self.snapshot = None;
        self.last_open_time = 0;
    }

    fn is_ready(&self) -> bool {
        self.signal_seeded
    }

        impl_standard_methods!(output = MacdOutput);


}

impl IndicatorMeta for StreamingMacd {
    fn name() -> &'static str {
        "MACD"
    }

    fn category() -> &'static str {
        "momentum"
    }

    fn description() -> &'static str {
        "Moving Average Convergence Divergence"
    }

    fn warm_up_period(&self) -> usize {
        self.slow_period + self.signal_period - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_macd_basic() {
        let mut macd = StreamingMacd::new(3, 5, 3);
        for i in 1..=10 {
            let out = macd.next(i as f64);
            if macd.is_ready() {
                let out = out.unwrap();
                assert!(!out.macd.is_nan());
                assert!(!out.signal.is_nan());
                assert!(!out.histogram.is_nan());
            }
        }
    }

    #[test]
    fn test_streaming_macd_reset() {
        let mut macd = StreamingMacd::new(3, 5, 3);
        for i in 1..=10 {
            macd.next(i as f64);
        }
        assert!(macd.is_ready());
        macd.reset();
        assert!(!macd.is_ready());
        assert_eq!(macd.count(), 0);
    }

    #[test]
    fn test_streaming_macd_meta() {
        let macd = StreamingMacd::new(12, 26, 9);
        assert_eq!(StreamingMacd::name(), "MACD");
        assert_eq!(StreamingMacd::category(), "momentum");
        assert_eq!(macd.warm_up_period(), 34);
    }

    #[test]
    fn test_streaming_macd_nan_output() {
        let mut macd = StreamingMacd::new(12, 26, 9);
        let out = macd.next(100.0);
        assert_eq!(out, None);
    }

    #[test]
    fn test_streaming_macd_repaint() {
        use crate::streaming::OhlcvBar;

        let data = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
        let mut macd = StreamingMacd::new(3, 5, 3);
        for (i, &v) in data.iter().enumerate() {
            macd.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, v, 0.0, (i + 1) as i64 * 1000));
        }

        // Repaint bar 9 three times
        macd.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 100.0, 0.0, 9000));
        macd.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 200.0, 0.0, 9000));
        let result_repaint = macd.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 18.0, 0.0, 9000));

        // Clean path
        let mut macd_clean = StreamingMacd::new(3, 5, 3);
        for &v in &data {
            macd_clean.next(v);
        }
        let result_clean = macd_clean.next(18.0);

        let rp = result_repaint.unwrap();
        let rc = result_clean.unwrap();
        assert!((rp.macd - rc.macd).abs() < 1e-10);
        assert!((rp.signal - rc.signal).abs() < 1e-10);
        assert!((rp.histogram - rc.histogram).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let fast_period = 12;
        let slow_period = 26;
        let signal_period = 9;

        let batch_result =
            crate::indicators::momentum::macd(&data, fast_period, slow_period, signal_period)
                .unwrap();

        let mut streaming = StreamingMacd::new(fast_period, slow_period, signal_period);
        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if !batch_result.macd[i].is_nan() {
                    assert!(
                        (s.macd - batch_result.macd[i]).abs() < 1e-10,
                        "MACD mismatch at index {i}: streaming={}, batch={}",
                        s.macd,
                        batch_result.macd[i]
                    );
                }
            }
        }
    }
}
