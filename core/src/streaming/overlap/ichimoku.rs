use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::streaming::traits::{IndicatorMeta, Ohlcv};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IchimokuOutput {
    pub tenkan: f64,
    pub kijun: f64,
    pub senkou_a: f64,
    pub senkou_b: f64,
    pub chikou: f64,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingIchimoku {
    tenkan_period: usize,
    kijun_period: usize,
    senkou_b_period: usize,
    tenkan_high: RollingMax,
    tenkan_low: RollingMin,
    kijun_high: RollingMax,
    kijun_low: RollingMin,
    senkou_high: RollingMax,
    senkou_low: RollingMin,
    count: usize,
    last_value: Option<IchimokuOutput>,
}

impl StreamingIchimoku {
    pub fn new(tenkan: usize, kijun: usize, senkou_b: usize) -> Self {
        Self {
            tenkan_period: tenkan,
            kijun_period: kijun,
            senkou_b_period: senkou_b,
            tenkan_high: RollingMax::new(),
            tenkan_low: RollingMin::new(),
            kijun_high: RollingMax::new(),
            kijun_low: RollingMin::new(),
            senkou_high: RollingMax::new(),
            senkou_low: RollingMin::new(),
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    pub fn next(&mut self, bar: &dyn Ohlcv) -> Option<IchimokuOutput> {
        self.count += 1;
        let idx = self.count - 1;
        let close = bar.close();
        let high = bar.high();
        let low = bar.low();

        self.tenkan_high.push(idx, high);
        self.tenkan_low.push(idx, low);
        self.kijun_high.push(idx, high);
        self.kijun_low.push(idx, low);
        self.senkou_high.push(idx, high);
        self.senkou_low.push(idx, low);

        if idx >= self.tenkan_period {
            let expired = idx - self.tenkan_period;
            self.tenkan_high.pop(expired);
            self.tenkan_low.pop(expired);
        }
        if idx >= self.kijun_period {
            let expired = idx - self.kijun_period;
            self.kijun_high.pop(expired);
            self.kijun_low.pop(expired);
        }
        if idx >= self.senkou_b_period {
            let expired = idx - self.senkou_b_period;
            self.senkou_high.pop(expired);
            self.senkou_low.pop(expired);
        }

        if self.count < self.senkou_b_period {
            self.last_value = None;
            return None;
        }

        let th = self.tenkan_high.current().unwrap();
        let tl = self.tenkan_low.current().unwrap();
        let kh = self.kijun_high.current().unwrap();
        let kl = self.kijun_low.current().unwrap();
        let sh = self.senkou_high.current().unwrap();
        let sl = self.senkou_low.current().unwrap();

        let tenkan = (th + tl) * 0.5;
        let kijun = (kh + kl) * 0.5;
        let result = Some(IchimokuOutput {
            tenkan,
            kijun,
            senkou_a: (tenkan + kijun) * 0.5,
            senkou_b: (sh + sl) * 0.5,
            chikou: close,
        });
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.tenkan_high.reset();
        self.tenkan_low.reset();
        self.kijun_high.reset();
        self.kijun_low.reset();
        self.senkou_high.reset();
        self.senkou_low.reset();
        self.count = 0;
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool { self.count >= self.senkou_b_period }
    pub fn count(&self) -> usize { self.count }

    pub fn value(&self) -> Option<IchimokuOutput> {
        self.last_value
    }
}

impl IndicatorMeta for StreamingIchimoku {
    fn name() -> &'static str { "Ichimoku" }
    fn category() -> &'static str { "overlap" }
    fn description() -> &'static str { "Ichimoku Kinko Hyo" }
    fn warm_up_period(&self) -> usize { self.senkou_b_period }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::types::OhlcvBar;
    use crate::indicators;

    #[test]
    fn test_streaming_ichimoku_basic() {
        let mut ich = StreamingIchimoku::new(9, 26, 52);
        for i in 0..60 {
            let bar = OhlcvBar::new(10.0 + i as f64, 12.0 + i as f64, 9.0 + i as f64, 11.0 + i as f64, 100.0);
            let out = ich.next(&bar);
            if let Some(out) = out {
                assert!(!out.tenkan.is_nan());
                assert!(!out.kijun.is_nan());
                assert!(!out.senkou_a.is_nan());
                assert!(!out.senkou_b.is_nan());
                assert!(!out.chikou.is_nan());
            }
        }
    }

    #[test]
    fn test_streaming_ichimoku_matches_batch_tenkan_kijun() {
        let len = 120;
        let high: Vec<f64> = (0..len).map(|i| 100.0 + i as f64 + 2.0).collect();
        let low: Vec<f64> = (0..len).map(|i| 100.0 + i as f64 - 2.0).collect();
        let close: Vec<f64> = (0..len).map(|i| 100.0 + i as f64).collect();

        let batch = indicators::ichimoku(&high, &low, &close, 9, 26, 52, 26).unwrap();
        let mut stream = StreamingIchimoku::new(9, 26, 52);

        for i in 0..len {
            let bar = OhlcvBar::new(close[i], high[i], low[i], close[i], 100.0);
            if let Some(out) = stream.next(&bar) {
                assert!((out.tenkan - batch.tenkan_sen[i]).abs() < 1e-10, "tenkan at {i}");
                assert!((out.kijun - batch.kijun_sen[i]).abs() < 1e-10, "kijun at {i}");
                let unshifted_senkou_a = (batch.tenkan_sen[i] + batch.kijun_sen[i]) / 2.0;
                assert!((out.senkou_a - unshifted_senkou_a).abs() < 1e-10, "senkou_a at {i}");
            }
        }
    }

    #[test]
    fn test_streaming_ichimoku_meta() {
        assert_eq!(StreamingIchimoku::name(), "Ichimoku");
    }

    #[test]
    fn test_streaming_ichimoku_reset() {
        let mut ich = StreamingIchimoku::new(3, 5, 10);
        for i in 0..15 {
            ich.next(&OhlcvBar::new(i as f64, i as f64 + 2.0, i as f64 - 1.0, i as f64 + 1.0, 100.0));
        }
        assert!(ich.is_ready());
        ich.reset();
        assert!(!ich.is_ready());
    }
}
