//! Streaming MACD with fixed 12/26 fast/slow periods (MACDFIX).
//!
//! This is the same shape as [`super::macd::StreamingMacd`] except the fast
//! and slow EMA periods are pinned to 12 and 26; only the signal period is
//! user-controlled. Useful as a streaming counterpart to the batch
//! `indicators::momentum::macdfix` entry point.

use crate::streaming::momentum::macd::MacdOutput;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

/// Fast EMA period (canonical TA-Lib MACDFIX value).
pub const MACDFIX_FAST_PERIOD: usize = 12;
/// Slow EMA period (canonical TA-Lib MACDFIX value).
pub const MACDFIX_SLOW_PERIOD: usize = 26;

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMacdFix {
    signal_period: usize,
    fast_k: f64,
    slow_k: f64,
    signal_k: f64,
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

impl StreamingMacdFix {
    pub fn new(signal_period: usize) -> Self {
        Self {
            signal_period,
            fast_k: 2.0 / (MACDFIX_FAST_PERIOD as f64 + 1.0),
            slow_k: 2.0 / (MACDFIX_SLOW_PERIOD as f64 + 1.0),
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

impl StreamingIndicator<f64, MacdOutput> for StreamingMacdFix {
    #[inline]
    fn next(&mut self, input: f64) -> Option<MacdOutput> {
        self.count += 1;

        let offset = MACDFIX_SLOW_PERIOD - MACDFIX_FAST_PERIOD;

        // TA-Lib 兼容：SMA 种子 + FMA 递推（与 batch macd_inner 完全一致）
        if !self.ema_seeded {
            if self.count <= offset {
                self.slow_sum += input;
            } else {
                self.slow_sum += input;
                self.fast_sum += input;
            }

            if self.count < MACDFIX_SLOW_PERIOD {
                self.last_value = None;
                return None;
            }

            self.slow_ema = self.slow_sum / MACDFIX_SLOW_PERIOD as f64;
            self.fast_ema = self.fast_sum / MACDFIX_FAST_PERIOD as f64;
            self.ema_seeded = true;
        } else {
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

impl IndicatorMeta for StreamingMacdFix {
    fn name() -> &'static str {
        "MACDFIX"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "MACD with fixed 12/26 fast/slow periods"
    }
    fn warm_up_period(&self) -> usize {
        MACDFIX_SLOW_PERIOD + self.signal_period - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_macd_fix_basic() {
        let mut m = StreamingMacdFix::new(9);
        for i in 1..=50 {
            let v = m.next(50.0 + (i as f64 * 0.1).sin() * 5.0);
            if m.is_ready() {
                let v = v.unwrap();
                assert!(!v.macd.is_nan());
                assert!(!v.signal.is_nan());
                assert!(!v.histogram.is_nan());
            }
        }
    }

    #[test]
    fn test_streaming_macd_fix_reset() {
        let mut m = StreamingMacdFix::new(9);
        for i in 0..50 {
            m.next(i as f64);
        }
        assert!(m.is_ready());
        m.reset();
        assert!(!m.is_ready());
        assert_eq!(m.count(), 0);
    }

    #[test]
    fn test_streaming_macd_fix_meta() {
        let m = StreamingMacdFix::new(9);
        assert_eq!(StreamingMacdFix::name(), "MACDFIX");
        assert_eq!(StreamingMacdFix::category(), "momentum");
        assert_eq!(m.warm_up_period(), 26 + 9 - 1);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..120)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 5.0)
            .collect();
        let signal_period = 9;

        let batch = crate::indicators::momentum::macdfix(&data).unwrap();
        let mut streaming = StreamingMacdFix::new(signal_period);
        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if !batch.macd[i].is_nan() {
                    assert!(
                        (s.macd - batch.macd[i]).abs() < 1e-9,
                        "MACDFIX macd mismatch at {i}: {} vs {}",
                        s.macd,
                        batch.macd[i]
                    );
                }
            }
        }
    }
}
