use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;
use crate::streaming::Ohlcv;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KvoOutput {
    pub kvo: f64,
    pub signal: f64,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingKvo {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    fast_ema: StreamingEma,
    slow_ema: StreamingEma,
    signal_ema: StreamingEma,
    prev_hlc_sum: f64,
    prev_dm: f64,
    prev_trend: i32,
    cm: f64,
    has_prev_bar: bool,
    count: usize,
    last_value: Option<KvoOutput>,
}

impl StreamingKvo {
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            signal_period,
            fast_ema: StreamingEma::new(fast_period),
            slow_ema: StreamingEma::new(slow_period),
            signal_ema: StreamingEma::new(signal_period),
            prev_hlc_sum: f64::NAN,
            prev_dm: f64::NAN,
            prev_trend: 1,
            cm: f64::NAN,
            has_prev_bar: false,
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn volume_force(
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        prev_hlc_sum: f64,
        prev_dm: f64,
        prev_trend: i32,
        cm: f64,
        has_prev_bar: bool,
    ) -> (f64, f64, i32, f64, f64) {
        let dm = if high.is_nan() || low.is_nan() {
            f64::NAN
        } else {
            high - low
        };

        let (trend, new_cm) = if !has_prev_bar {
            let t = if close.is_nan() { prev_trend } else { 1 };
            let c = if dm.is_nan() { f64::NAN } else { dm };
            (t, c)
        } else if close.is_nan()
            || prev_hlc_sum.is_nan()
            || high.is_nan()
            || low.is_nan()
        {
            (prev_trend, f64::NAN)
        } else {
            let today = high + low + close;
            let trend = if today > prev_hlc_sum { 1 } else { -1 };
            let new_cm = if dm.is_nan() || prev_dm.is_nan() {
                f64::NAN
            } else if trend == prev_trend {
                cm + dm
            } else {
                prev_dm + dm
            };
            (trend, new_cm)
        };

        let vf = if volume.is_nan() || dm.is_nan() || new_cm.is_nan() || new_cm.abs() <= 1e-15 {
            0.0
        } else {
            volume * (2.0 * dm / new_cm - 1.0).abs() * trend as f64
        };

        let hlc_sum = if high.is_nan() || low.is_nan() || close.is_nan() {
            f64::NAN
        } else {
            high + low + close
        };

        (vf, hlc_sum, trend, new_cm, dm)
    }
}

impl StreamingIndicator<&dyn Ohlcv, KvoOutput> for StreamingKvo {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<KvoOutput> {
        self.count += 1;

        let (vf, hlc_sum, trend, new_cm, dm) = Self::volume_force(
            bar.high(),
            bar.low(),
            bar.close(),
            bar.volume(),
            self.prev_hlc_sum,
            self.prev_dm,
            self.prev_trend,
            self.cm,
            self.has_prev_bar,
        );

        self.prev_hlc_sum = hlc_sum;
        self.prev_dm = dm;
        self.prev_trend = trend;
        self.cm = new_cm;
        self.has_prev_bar = true;

        let fast = self.fast_ema.next(vf);
        let slow = self.slow_ema.next(vf);
        let (Some(fast_val), Some(slow_val)) = (fast, slow) else {
            self.last_value = None;
            return None;
        };

        let kvo = fast_val - slow_val;
        let Some(signal) = self.signal_ema.next(kvo) else {
            self.last_value = None;
            return None;
        };

        let result = Some(KvoOutput { kvo, signal });
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.fast_ema.reset();
        self.slow_ema.reset();
        self.signal_ema.reset();
        self.prev_hlc_sum = f64::NAN;
        self.prev_dm = f64::NAN;
        self.prev_trend = 1;
        self.cm = f64::NAN;
        self.has_prev_bar = false;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.fast_ema.is_ready() && self.slow_ema.is_ready() && self.signal_ema.is_ready()
    }

        impl_standard_methods!(output = KvoOutput);


}

impl IndicatorMeta for StreamingKvo {
    fn name() -> &'static str { "KVO" }
    fn category() -> &'static str { "volume" }
    fn description() -> &'static str { "Klinger Volume Oscillator" }
    fn warm_up_period(&self) -> usize {
        self.slow_period.max(self.fast_period) + self.signal_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::OhlcvBar;
    use approx::assert_relative_eq;

    #[test]
    fn test_streaming_kvo_basic() {
        let mut kvo = StreamingKvo::new(3, 5, 2);
        let bars: Vec<OhlcvBar> = (0..20)
            .map(|i| {
                let h = 12.0 + i as f64;
                let l = 10.0 + i as f64;
                OhlcvBar::new(h - 1.0, h, l, (h + l) / 2.0, 100.0 + i as f64 * 10.0)
            })
            .collect();

        let mut last = None;
        for bar in &bars {
            last = kvo.next(bar as &dyn Ohlcv);
        }
        assert!(last.is_some());
        assert!(kvo.is_ready());
    }

    #[test]
    fn test_streaming_kvo_meta() {
        let kvo = StreamingKvo::new(34, 55, 13);
        assert_eq!(StreamingKvo::name(), "KVO");
        assert_eq!(StreamingKvo::category(), "volume");
        assert_eq!(kvo.warm_up_period(), 68);
    }

    #[test]
    fn test_streaming_kvo_reset() {
        let mut kvo = StreamingKvo::new(3, 5, 2);
        let bars: Vec<OhlcvBar> = (0..30)
            .map(|i| {
                let h = 12.0 + i as f64;
                let l = 10.0 + i as f64;
                OhlcvBar::new(h - 1.0, h, l, (h + l) / 2.0, 100.0)
            })
            .collect();
        for bar in &bars {
            kvo.next(bar as &dyn Ohlcv);
        }
        assert!(kvo.is_ready());
        kvo.reset();
        assert!(!kvo.is_ready());
        assert_eq!(kvo.count(), 0);
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
        let high: Vec<f64> = bars.iter().map(|b| b.high()).collect();
        let low: Vec<f64> = bars.iter().map(|b| b.low()).collect();
        let close: Vec<f64> = bars.iter().map(|b| b.close()).collect();
        let volume: Vec<f64> = bars.iter().map(|b| b.volume()).collect();
        let fast = 13;
        let slow = 34;
        let signal = 9;

        let batch = crate::indicators::volume_ext::kvo(
            &high, &low, &close, &volume, fast, slow, signal,
        )
        .unwrap();
        let mut streaming = StreamingKvo::new(fast, slow, signal);

        for (i, bar) in bars.iter().enumerate() {
            if let Some(out) = streaming.next(bar as &dyn Ohlcv) {
                if !batch.kvo[i].is_nan() {
                    assert_relative_eq!(out.kvo, batch.kvo[i], epsilon = 1e-10);
                }
                if !batch.signal[i].is_nan() {
                    assert_relative_eq!(out.signal, batch.signal[i], epsilon = 1e-10);
                }
            }
        }
    }
}
