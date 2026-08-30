use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RviOutput {
    pub rvi: f64,
    pub signal: f64,
}

#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct OhlcSnapshot {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingRvi {
    period: usize,
    bar_buf: Vec<OhlcSnapshot>,
    bar_head: usize,
    bar_len: usize,
    num_sma: StreamingSma,
    denom_sma: StreamingSma,
    rvi_ring: [f64; 4],
    rvi_pos: usize,
    rvi_count: usize,
    count: usize,
    last_value: Option<RviOutput>,
}

impl StreamingRvi {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            bar_buf: vec![
                OhlcSnapshot {
                    open: 0.0,
                    high: 0.0,
                    low: 0.0,
                    close: 0.0,
                };
                4
            ],
            bar_head: 0,
            bar_len: 0,
            num_sma: StreamingSma::new(period),
            denom_sma: StreamingSma::new(period),
            rvi_ring: [0.0; 4],
            rvi_pos: 0,
            rvi_count: 0,
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    fn ring_bar(&self, i: usize) -> OhlcSnapshot {
        self.bar_buf[(self.bar_head + i) % 4]
    }

    #[inline]
    fn push_bar(&mut self, snap: OhlcSnapshot) {
        if self.bar_len < 4 {
            self.bar_buf[(self.bar_head + self.bar_len) % 4] = snap;
            self.bar_len += 1;
        } else {
            self.bar_buf[self.bar_head] = snap;
            self.bar_head = (self.bar_head + 1) % 4;
        }
    }

    #[inline]
    fn weighted_avg_co(&self) -> f64 {
        let b0 = self.ring_bar(self.bar_len - 1);
        let b1 = self.ring_bar(self.bar_len - 2);
        let b2 = self.ring_bar(self.bar_len - 3);
        let b3 = self.ring_bar(self.bar_len - 4);
        let d0 = b0.close - b0.open;
        let d1 = b1.close - b1.open;
        let d2 = b2.close - b2.open;
        let d3 = b3.close - b3.open;
        (d0 + 2.0 * d1 + 2.0 * d2 + d3) / 6.0
    }

    #[inline]
    fn weighted_avg_hl(&self) -> f64 {
        let b0 = self.ring_bar(self.bar_len - 1);
        let b1 = self.ring_bar(self.bar_len - 2);
        let b2 = self.ring_bar(self.bar_len - 3);
        let b3 = self.ring_bar(self.bar_len - 4);
        let d0 = b0.high - b0.low;
        let d1 = b1.high - b1.low;
        let d2 = b2.high - b2.low;
        let d3 = b3.high - b3.low;
        (d0 + 2.0 * d1 + 2.0 * d2 + d3) / 6.0
    }

    #[inline]
    fn push_rvi(&mut self, rvi: f64) {
        self.rvi_ring[self.rvi_pos] = rvi;
        self.rvi_pos = (self.rvi_pos + 1) % 4;
        if self.rvi_count < 4 {
            self.rvi_count += 1;
        }
    }

    #[inline]
    fn rvi_signal(&self) -> Option<f64> {
        if self.rvi_count < 4 {
            return None;
        }
        let newest = (self.rvi_pos + 3) % 4;
        let n1 = (self.rvi_pos + 2) % 4;
        let n2 = (self.rvi_pos + 1) % 4;
        let oldest = self.rvi_pos;
        Some(
            (self.rvi_ring[newest] + 2.0 * self.rvi_ring[n1] + 2.0 * self.rvi_ring[n2]
                + self.rvi_ring[oldest])
                / 6.0,
        )
    }
}

impl StreamingIndicator<&dyn Ohlcv, RviOutput> for StreamingRvi {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<RviOutput> {
        self.count += 1;
        self.push_bar(OhlcSnapshot {
            open: bar.open(),
            high: bar.high(),
            low: bar.low(),
            close: bar.close(),
        });

        if self.bar_len < 4 {
            self.last_value = None;
            return None;
        }

        let numerator = self.weighted_avg_co();
        let denominator = self.weighted_avg_hl();

        let sma_num = self.num_sma.next(numerator);
        let sma_denom = self.denom_sma.next(denominator);

        let (Some(sma_num), Some(sma_denom)) = (sma_num, sma_denom) else {
            self.last_value = None;
            return None;
        };

        if sma_denom.abs() <= 1e-15 {
            self.last_value = None;
            return None;
        }

        let rvi = sma_num / sma_denom;
        self.push_rvi(rvi);

        let signal = self.rvi_signal().unwrap_or(f64::NAN);
        let result = Some(RviOutput { rvi, signal });
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.bar_head = 0;
        self.bar_len = 0;
        self.num_sma.reset();
        self.denom_sma.reset();
        self.rvi_pos = 0;
        self.rvi_count = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.num_sma.is_ready() && self.denom_sma.is_ready()
    }

        impl_standard_methods!(output = RviOutput);


}

impl IndicatorMeta for StreamingRvi {
    fn name() -> &'static str {
        "RVI"
    }

    fn category() -> &'static str {
        "volatility"
    }

    fn description() -> &'static str {
        "Relative Vigor Index"
    }

    fn warm_up_period(&self) -> usize {
        self.period + 6
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::OhlcvBar;

    #[test]
    fn test_streaming_rvi_basic() {
        let mut rvi = StreamingRvi::new(10);
        let bars: Vec<OhlcvBar> = (0..40)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.2).sin() * 10.0;
                let l = h - 3.0;
                let c = (h + l) / 2.0;
                let v = 1000.0 + (i as f64 * 0.5).cos() * 500.0;
                OhlcvBar::new(c - 0.5, h, l, c, v)
            })
            .collect();
        let mut last = None;
        for bar in &bars {
            last = rvi.next(bar);
        }
        let out = last.unwrap();
        assert!(!out.rvi.is_nan());
    }

    #[test]
    fn test_streaming_rvi_meta() {
        let rvi = StreamingRvi::new(10);
        assert_eq!(StreamingRvi::name(), "RVI");
        assert_eq!(StreamingRvi::category(), "volatility");
        assert_eq!(rvi.warm_up_period(), 16);
    }

    #[test]
    fn test_streaming_rvi_reset() {
        let mut rvi = StreamingRvi::new(5);
        let bars: Vec<OhlcvBar> = (0..30)
            .map(|i| OhlcvBar::new(i as f64, i as f64 + 2.0, i as f64, i as f64 + 1.0, 100.0))
            .collect();
        for bar in &bars {
            rvi.next(bar);
        }
        assert!(rvi.is_ready());
        rvi.reset();
        assert!(!rvi.is_ready());
        assert_eq!(rvi.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let bars: Vec<OhlcvBar> = (0..n)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.2).sin() * 10.0;
                let l = h - 3.0;
                let c = (h + l) / 2.0;
                let v = 1000.0 + (i as f64 * 0.5).cos() * 500.0;
                OhlcvBar::new(c - 0.5, h, l, c, v)
            })
            .collect();
        let open: Vec<f64> = bars.iter().map(|b| b.open()).collect();
        let high: Vec<f64> = bars.iter().map(|b| b.high()).collect();
        let low: Vec<f64> = bars.iter().map(|b| b.low()).collect();
        let close: Vec<f64> = bars.iter().map(|b| b.close()).collect();
        let period = 10;

        let batch =
            crate::indicators::rvi(&open, &high, &low, &close, period).unwrap();
        let mut streaming = StreamingRvi::new(period);

        for (i, bar) in bars.iter().enumerate() {
            if let Some(s) = streaming.next(bar) {
                if !batch.rvi[i].is_nan() {
                    assert!(
                        (s.rvi - batch.rvi[i]).abs() < 1e-10,
                        "RVI mismatch at {i}: streaming={}, batch={}",
                        s.rvi,
                        batch.rvi[i]
                    );
                }
                if !batch.signal[i].is_nan() {
                    assert!(
                        (s.signal - batch.signal[i]).abs() < 1e-10,
                        "Signal mismatch at {i}: streaming={}, batch={}",
                        s.signal,
                        batch.signal[i]
                    );
                }
            }
        }
    }
}
