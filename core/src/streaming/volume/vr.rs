use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

/// Volume direction for VR classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VolDir {
    Down = 0,
    Flat = 1,
    Up = 2,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingVr {
    period: usize,
    buffer: Vec<(f64, f64)>,
    prev_close_buf: Vec<f64>,
    head: usize,
    len: usize,
    prev_close: f64,
    up_vol: f64,
    down_vol: f64,
    flat_vol: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingVr {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![(0.0, 0.0); period],
            prev_close_buf: vec![0.0; period],
            head: 0,
            len: 0,
            prev_close: f64::NAN,
            up_vol: 0.0,
            down_vol: 0.0,
            flat_vol: 0.0,
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    fn classify(prev: f64, close: f64) -> VolDir {
        if close > prev {
            VolDir::Up
        } else if close < prev {
            VolDir::Down
        } else {
            VolDir::Flat
        }
    }

    #[inline]
    fn add_vol(&mut self, dir: VolDir, volume: f64) {
        match dir {
            VolDir::Up => self.up_vol += volume,
            VolDir::Down => self.down_vol += volume,
            VolDir::Flat => self.flat_vol += volume,
        }
    }

    #[inline]
    fn sub_vol(&mut self, dir: VolDir, volume: f64) {
        match dir {
            VolDir::Up => self.up_vol -= volume,
            VolDir::Down => self.down_vol -= volume,
            VolDir::Flat => self.flat_vol -= volume,
        }
    }

    #[inline]
    fn compute(&self) -> Option<f64> {
        let denom = self.down_vol + 0.5 * self.flat_vol;
        if denom.abs() <= 1e-15 {
            None
        } else {
            Some((self.up_vol + 0.5 * self.flat_vol) / denom * 100.0)
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingVr {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;
        let close = bar.close();
        let volume = bar.volume();

        if self.count == 1 {
            self.prev_close = close;
            self.last_value = None;
            return None;
        }

        let dir = Self::classify(self.prev_close, close);
        self.add_vol(dir, volume);

        if self.len == self.period {
            let prev = self.prev_close_buf[self.head];
            let (leave_close, leave_vol) = self.buffer[self.head];
            let leave_dir = Self::classify(prev, leave_close);
            self.sub_vol(leave_dir, leave_vol);
            self.buffer[self.head] = (close, volume);
            self.prev_close_buf[self.head] = self.prev_close;
            self.head = (self.head + 1) % self.period;
        } else {
            let idx = (self.head + self.len) % self.period;
            self.buffer[idx] = (close, volume);
            self.prev_close_buf[idx] = self.prev_close;
            self.len += 1;
        }

        self.prev_close = close;

        let result = if self.count > self.period {
            self.compute()
        } else {
            None
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.prev_close = f64::NAN;
        self.up_vol = 0.0;
        self.down_vol = 0.0;
        self.flat_vol = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count > self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingVr {
    fn name() -> &'static str {
        "VR"
    }

    fn category() -> &'static str {
        "volume"
    }

    fn description() -> &'static str {
        "Volume Ratio"
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
    fn test_streaming_vr_basic() {
        let mut vr = StreamingVr::new(3);
        let bars = [
            OhlcvBar::new(10.0, 12.0, 9.0, 10.0, 100.0),
            OhlcvBar::new(10.0, 13.0, 10.0, 11.0, 150.0),
            OhlcvBar::new(11.0, 14.0, 11.0, 10.0, 200.0),
            OhlcvBar::new(10.0, 15.0, 9.0, 12.0, 180.0),
        ];
        for bar in &bars[..3] {
            assert_eq!(vr.next(bar), None);
        }
        let v = vr.next(&bars[3]).unwrap();
        assert!(v > 0.0);
    }

    #[test]
    fn test_streaming_vr_reset() {
        let mut vr = StreamingVr::new(3);
        for i in 0..10 {
            vr.next(&OhlcvBar::new(
                i as f64,
                i as f64 + 2.0,
                i as f64 - 1.0,
                i as f64 + 1.0,
                100.0,
            ));
        }
        assert!(vr.is_ready());
        vr.reset();
        assert!(!vr.is_ready());
        assert_eq!(vr.count(), 0);
    }

    #[test]
    fn test_streaming_vr_meta() {
        let vr = StreamingVr::new(26);
        assert_eq!(StreamingVr::name(), "VR");
        assert_eq!(StreamingVr::category(), "volume");
        assert_eq!(vr.warm_up_period(), 27);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let close: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let volume: Vec<f64> = (0..100).map(|i| 1000.0 + i as f64 * 10.0).collect();
        let period = 26;

        let batch = crate::indicators::china::vr(&close, &volume, period).unwrap();

        let mut streaming = StreamingVr::new(period);
        for (i, (&c, &v)) in close.iter().zip(volume.iter()).enumerate() {
            let bar = OhlcvBar::new(c, c + 1.0, c - 1.0, c, v);
            if let (Some(s), false) = (streaming.next(&bar), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-10,
                    "Mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
