use crate::streaming::traits::{IndicatorMeta, Ohlcv};

/// Streaming Money Flow Index with O(1) amortized per-update complexity.
///
/// The MFI is the ratio of positive to negative money flow over the last
/// `period` *deltas* (i.e. between consecutive bars). To make streaming
/// O(1) per update, we keep a ring buffer of length `period` storing the
/// sign and money-flow of each delta. When a new bar arrives, the oldest
/// delta is subtracted from the running sum and the new delta is added.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMfi {
    period: usize,
    /// Sign of each delta in the ring window: +1 (TP rose), -1 (TP fell), 0 (unchanged).
    sign_buf: Vec<i8>,
    /// Money flow (= tp * volume) of the second bar in each delta pair.
    mf_buf: Vec<f64>,
    head: usize,
    len: usize,
    sum_pos: f64,
    sum_neg: f64,
    count: usize,
    prev_tp: f64,
    last_value: Option<f64>,
}

impl StreamingMfi {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "StreamingMfi: period must be > 0");
        Self {
            period,
            sign_buf: vec![0i8; period],
            mf_buf: vec![0.0; period],
            head: 0,
            len: 0,
            sum_pos: 0.0,
            sum_neg: 0.0,
            count: 0,
            prev_tp: f64::NAN,
            last_value: None,
        }
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, bar))
    )]
    pub fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        crate::streaming_measure!("mfi", self.count, {
            self.count += 1;
            let tp = (bar.high() + bar.low() + bar.close()) / 3.0;
            let mf = tp * bar.volume();

            // Determine the new delta's sign by comparing to the previous bar's TP.
            // The first bar has no previous, so its delta is neutral.
            let new_sign: i8 = if self.count == 1 {
                0
            } else if tp > self.prev_tp {
                1
            } else if tp < self.prev_tp {
                -1
            } else {
                0
            };

            if self.len < self.period {
                // Still filling the window: append at the tail.
                let idx = (self.head + self.len) % self.period;
                self.mf_buf[idx] = mf;
                self.sign_buf[idx] = new_sign;
                self.len += 1;
            } else {
                // Window full: subtract the oldest delta's contribution, then
                // overwrite its slot with the new delta.
                let oldest_sign = self.sign_buf[self.head];
                let oldest_mf = self.mf_buf[self.head];
                match oldest_sign {
                    1 => self.sum_pos -= oldest_mf,
                    -1 => self.sum_neg -= oldest_mf,
                    _ => {}
                }
                self.mf_buf[self.head] = mf;
                self.sign_buf[self.head] = new_sign;
                self.head = (self.head + 1) % self.period;
            }

            // Add the new delta's contribution.
            match new_sign {
                1 => self.sum_pos += mf,
                -1 => self.sum_neg += mf,
                _ => {}
            }

            self.prev_tp = tp;

            // We need period+1 bars to have period deltas.
            if self.count < self.period + 1 {
                self.last_value = None;
                return None;
            }

            let result = Some(if self.sum_neg.abs() > 1e-15 {
                let ratio = self.sum_pos / self.sum_neg;
                100.0 - 100.0 / (1.0 + ratio)
            } else {
                100.0
            });
            self.last_value = result;
            result
        })
    }

    pub fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum_pos = 0.0;
        self.sum_neg = 0.0;
        self.count = 0;
        self.prev_tp = f64::NAN;
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool {
        self.count > self.period
    }
    pub fn count(&self) -> usize {
        self.count
    }

    pub fn value(&self) -> Option<f64> {
        self.last_value
    }
}

impl IndicatorMeta for StreamingMfi {
    fn name() -> &'static str {
        "MFI"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Money Flow Index"
    }
    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_mfi_basic() {
        let mut mfi = StreamingMfi::new(3);
        let bars = [
            OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0),
            OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 150.0),
            OhlcvBar::new(12.0, 14.0, 11.0, 13.0, 200.0),
            OhlcvBar::new(13.0, 15.0, 12.0, 14.0, 180.0),
        ];
        for bar in &bars[..3] {
            assert_eq!(mfi.next(bar), None);
        }
        let v = mfi.next(&bars[3]).unwrap();
        assert!((0.0..=100.0).contains(&v));
    }

    #[test]
    fn test_streaming_mfi_meta() {
        assert_eq!(StreamingMfi::name(), "MFI");
    }

    #[test]
    fn test_streaming_mfi_reset() {
        let mut mfi = StreamingMfi::new(3);
        for i in 0..10 {
            mfi.next(&OhlcvBar::new(
                i as f64,
                i as f64 + 2.0,
                i as f64 - 1.0,
                i as f64 + 1.0,
                100.0,
            ));
        }
        assert!(mfi.is_ready());
        mfi.reset();
        assert!(!mfi.is_ready());
    }

    /// 与标准 MFI 公式（last `period` deltas）交叉验证。
    #[test]
    fn test_streaming_mfi_parity_with_linear_scan() {
        let period = 14;
        let mut mfi = StreamingMfi::new(period);
        let bars: Vec<OhlcvBar> = (0..500)
            .map(|i| {
                let h = 100.0 + (i as f64 * 0.21).sin() * 6.0;
                let l = h - 2.0 - (i as f64 * 0.13).cos().abs() * 1.5;
                let c = (h + l) / 2.0;
                let v = 1000.0 + (i as f64 * 0.07).sin().abs() * 200.0;
                OhlcvBar::new(c, h, l, c, v)
            })
            .collect();

        // Linear reference: track last (period+1) bars, sum pos/neg of
        // the period deltas between consecutive bars in the window.
        let mut tps: Vec<f64> = Vec::new();
        let mut mfs: Vec<f64> = Vec::new();
        for bar in &bars {
            let tp = (bar.high() + bar.low() + bar.close()) / 3.0;
            let mf = tp * bar.volume();
            tps.push(tp);
            mfs.push(mf);
            let opt = mfi.next(bar);

            if tps.len() <= period {
                assert!(opt.is_none());
                continue;
            }
            // Trim to (period+1) bars; the streaming code's window is also
            // (period+1) bars (period deltas).
            while tps.len() > period + 1 {
                tps.remove(0);
                mfs.remove(0);
            }
            // Now tps.len() == period+1; compare j in 1..tps.len()
            // (period comparisons between consecutive bars).
            let mut pos = 0.0;
            let mut neg = 0.0;
            for j in 1..tps.len() {
                if tps[j] > tps[j - 1] {
                    pos += mfs[j];
                } else if tps[j] < tps[j - 1] {
                    neg += mfs[j];
                }
            }
            let expected = if neg.abs() > 1e-15 {
                let ratio = pos / neg;
                100.0 - 100.0 / (1.0 + ratio)
            } else {
                100.0
            };
            let actual = opt.unwrap();
            assert!(
                (actual - expected).abs() < 1e-9,
                "MFI mismatch: actual={actual} expected={expected}"
            );
        }
    }
}
