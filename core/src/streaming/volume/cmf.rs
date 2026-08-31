use crate::impl_indicator_meta;
use crate::impl_standard_methods;
use crate::streaming::traits::StreamingIndicator;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingCmf {
    period: usize,
    mfv_buf: Vec<f64>,
    vol_buf: Vec<f64>,
    head: usize,
    len: usize,
    sum_mfv: f64,
    sum_vol: f64,
    valid_mfv: usize,
    valid_vol: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingCmf {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            mfv_buf: vec![f64::NAN; period],
            vol_buf: vec![f64::NAN; period],
            head: 0,
            len: 0,
            sum_mfv: 0.0,
            sum_vol: 0.0,
            valid_mfv: 0,
            valid_vol: 0,
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    fn money_flow_multiplier(high: f64, low: f64, close: f64) -> f64 {
        let range = high - low;
        if range.abs() <= 1e-15 || high.is_nan() || low.is_nan() || close.is_nan() {
            f64::NAN
        } else {
            ((close - low) - (high - close)) / range
        }
    }
}

impl StreamingIndicator<(f64, f64, f64, f64)> for StreamingCmf {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64, f64)) -> Option<f64> {
        let (high, low, close, volume) = input;
        self.count += 1;

        let mfm = Self::money_flow_multiplier(high, low, close);
        let mfv = if mfm.is_nan() || volume.is_nan() {
            f64::NAN
        } else {
            mfm * volume
        };

        let cap = self.period;

        if self.len < cap {
            let idx = (self.head + self.len) % cap;
            self.mfv_buf[idx] = mfv;
            self.vol_buf[idx] = volume;
            self.len += 1;

            if !mfv.is_nan() {
                self.sum_mfv += mfv;
                self.valid_mfv += 1;
            }
            if !volume.is_nan() {
                self.sum_vol += volume;
                self.valid_vol += 1;
            }
        } else {
            let old_mfv = self.mfv_buf[self.head];
            let old_vol = self.vol_buf[self.head];

            if !old_mfv.is_nan() {
                self.sum_mfv -= old_mfv;
                self.valid_mfv -= 1;
            }
            if !old_vol.is_nan() {
                self.sum_vol -= old_vol;
                self.valid_vol -= 1;
            }

            self.mfv_buf[self.head] = mfv;
            self.vol_buf[self.head] = volume;
            self.head = (self.head + 1) % cap;

            if !mfv.is_nan() {
                self.sum_mfv += mfv;
                self.valid_mfv += 1;
            }
            if !volume.is_nan() {
                self.sum_vol += volume;
                self.valid_vol += 1;
            }
        }

        if self.len < self.period {
            self.last_value = None;
            return None;
        }

        let result = if self.valid_mfv == self.period
            && self.valid_vol == self.period
            && self.sum_vol.abs() > 1e-15
        {
            Some(self.sum_mfv / self.sum_vol)
        } else {
            None
        };
        self.last_value = result;
        result
    }

    #[inline]
    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum_mfv = 0.0;
        self.sum_vol = 0.0;
        self.valid_mfv = 0;
        self.valid_vol = 0;
        self.count = 0;
        self.last_value = None;
        for v in &mut self.mfv_buf {
            *v = f64::NAN;
        }
        for v in &mut self.vol_buf {
            *v = f64::NAN;
        }
    }

    #[inline]
    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingCmf, "CMF", "volume", "Chaikin Money Flow");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;
    use approx::assert_relative_eq;

    #[test]
    fn test_streaming_cmf_basic() {
        let mut cmf = StreamingCmf::new(3);
        assert_eq!(cmf.next((12.0, 10.0, 11.0, 100.0)), None);
        assert_eq!(cmf.next((13.0, 11.0, 12.0, 150.0)), None);
        let val = cmf.next((14.0, 12.0, 13.0, 200.0)).unwrap();
        assert!(!val.is_nan());
        assert!((-1.0..=1.0).contains(&val));
    }

    #[test]
    fn test_streaming_cmf_meta() {
        let cmf = StreamingCmf::new(20);
        assert_eq!(StreamingCmf::name(), "CMF");
        assert_eq!(StreamingCmf::category(), "volume");
        assert_eq!(cmf.warm_up_period(), 20);
    }

    #[test]
    fn test_streaming_cmf_reset() {
        let mut cmf = StreamingCmf::new(3);
        for i in 0..5 {
            cmf.next((10.0 + i as f64, 8.0 + i as f64, 9.0 + i as f64, 100.0));
        }
        assert!(cmf.is_ready());
        cmf.reset();
        assert!(!cmf.is_ready());
        assert_eq!(cmf.count(), 0);
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
        let volume: Vec<f64> = (0..n).map(|i| 1000.0 + i as f64 * 10.0).collect();
        let period = 20;

        let batch =
            crate::indicators::volume_ext::cmf(&high, &low, &close, &volume, period).unwrap();
        let mut streaming = StreamingCmf::new(period);

        for i in 0..n {
            if let Some(s) = streaming.next((high[i], low[i], close[i], volume[i])) {
                if !batch[i].is_nan() {
                    assert_relative_eq!(s, batch[i], epsilon = 1e-10);
                }
            }
        }
    }
}
