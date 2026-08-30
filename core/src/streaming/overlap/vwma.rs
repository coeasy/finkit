use crate::streaming::traits::{Ohlcv, StreamingIndicator};
use crate::{impl_indicator_meta, impl_standard_methods};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingVwma {
    period: usize,
    pv_buf: Vec<f64>,
    vol_buf: Vec<f64>,
    head: usize,
    len: usize,
    sum_pv: f64,
    sum_vol: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingVwma {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            pv_buf: vec![0.0; period],
            vol_buf: vec![0.0; period],
            head: 0,
            len: 0,
            sum_pv: 0.0,
            sum_vol: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingVwma {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;
        let close = bar.close();
        let volume = bar.volume();
        let pv = close * volume;
        let cap = self.period;

        self.sum_pv += pv;
        self.sum_vol += volume;

        if self.len < cap {
            let idx = (self.head + self.len) % cap;
            self.pv_buf[idx] = pv;
            self.vol_buf[idx] = volume;
            self.len += 1;
        } else {
            let old_pv = self.pv_buf[self.head];
            let old_vol = self.vol_buf[self.head];
            self.sum_pv -= old_pv;
            self.sum_vol -= old_vol;
            self.pv_buf[self.head] = pv;
            self.vol_buf[self.head] = volume;
            self.head = (self.head + 1) % cap;
        }

        let result = if self.len == self.period && self.sum_vol.abs() > 1e-15 {
            Some(self.sum_pv / self.sum_vol)
        } else {
            None
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum_pv = 0.0;
        self.sum_vol = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingVwma, "VWMA", "overlap", "Volume Weighted Moving Average");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;
    use crate::streaming::OhlcvBar;

    #[test]
    fn test_streaming_vwma_basic() {
        let mut vwma = StreamingVwma::new(5);
        let bars: Vec<OhlcvBar> = (0..20)
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
            last = vwma.next(bar);
        }
        assert!(last.is_some());
    }

    #[test]
    fn test_streaming_vwma_meta() {
        let vwma = StreamingVwma::new(20);
        assert_eq!(StreamingVwma::name(), "VWMA");
        assert_eq!(StreamingVwma::category(), "overlap");
        assert_eq!(vwma.warm_up_period(), 20);
    }

    #[test]
    fn test_streaming_vwma_reset() {
        let mut vwma = StreamingVwma::new(5);
        let bars: Vec<OhlcvBar> = (0..15)
            .map(|i| OhlcvBar::new(i as f64, i as f64 + 2.0, i as f64, i as f64 + 1.0, 100.0))
            .collect();
        for bar in &bars {
            vwma.next(bar);
        }
        assert!(vwma.is_ready());
        vwma.reset();
        assert!(!vwma.is_ready());
        assert_eq!(vwma.count(), 0);
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
        let close: Vec<f64> = bars.iter().map(|b| b.close()).collect();
        let volume: Vec<f64> = bars.iter().map(|b| b.volume()).collect();
        let period = 14;

        let batch = crate::math::moving_avg::vwma(&close, &volume, period).unwrap();
        let mut streaming = StreamingVwma::new(period);

        for (i, bar) in bars.iter().enumerate() {
            if let Some(s) = streaming.next(bar) {
                if !batch[i].is_nan() {
                    assert!(
                        (s - batch[i]).abs() < 1e-10,
                        "Mismatch at {i}: streaming={s}, batch={}",
                        batch[i]
                    );
                }
            }
        }
    }
}
