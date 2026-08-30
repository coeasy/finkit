use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::momentum::stoch::StochOutput;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming Stochastic Fast (STOCHF).
///
/// %K is unsmoothed (raw), %D is SMA of %K.
/// Uses O(1) amortized monotonic deques for rolling max/min.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingStochF {
    fastk_period: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    hl_head: usize,
    hl_len: usize,
    rolling_max: RollingMax,
    rolling_min: RollingMin,
    d_sma: StreamingSma,
    count: usize,
    last_value: Option<StochOutput>,
}

impl StreamingStochF {
    pub fn new(fastk_period: usize, fastd_period: usize) -> Self {
        Self {
            fastk_period,
            highs: vec![0.0; fastk_period],
            lows: vec![0.0; fastk_period],
            hl_head: 0,
            hl_len: 0,
            rolling_max: RollingMax::new(),
            rolling_min: RollingMin::new(),
            d_sma: StreamingSma::new(fastd_period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64), StochOutput> for StreamingStochF {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<StochOutput> {
        let (high, low, close) = input;
        self.count += 1;

        // Push to rolling deques for O(1) amortized max/min
        self.rolling_max.push(self.count, high);
        self.rolling_min.push(self.count, low);
        if self.count > self.fastk_period {
            self.rolling_max.pop(self.count - self.fastk_period);
            self.rolling_min.pop(self.count - self.fastk_period);
        }

        // Ring buffer management for highs/lows (kept for window membership)
        let cap = self.fastk_period;
        if self.hl_len < cap {
            let idx = (self.hl_head + self.hl_len) % cap;
            self.highs[idx] = high;
            self.lows[idx] = low;
            self.hl_len += 1;
        } else {
            self.highs[self.hl_head] = high;
            self.lows[self.hl_head] = low;
            self.hl_head = (self.hl_head + 1) % cap;
        }

        if self.hl_len < self.fastk_period {
            self.last_value = None;
            return None;
        }

        let highest = self.rolling_max.current().unwrap_or(f64::NEG_INFINITY);
        let lowest = self.rolling_min.current().unwrap_or(f64::INFINITY);

        let range = highest - lowest;
        let k = if range.abs() > 1e-15 {
            ((close - lowest) / range) * 100.0
        } else {
            50.0
        };

        let d = self.d_sma.next(k);

        let result = StochOutput {
            k,
            d: d.unwrap_or(f64::NAN),
        };
        self.last_value = Some(result);
        Some(result)
    }

    fn reset(&mut self) {
        self.hl_head = 0;
        self.hl_len = 0;
        self.rolling_max.reset();
        self.rolling_min.reset();
        self.d_sma.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.hl_len >= self.fastk_period
    }

        impl_standard_methods!(output = StochOutput);


}

impl IndicatorMeta for StreamingStochF {
    fn name() -> &'static str { "STOCHF" }
    fn category() -> &'static str { "momentum" }
    fn description() -> &'static str { "Stochastic Fast" }
    fn warm_up_period(&self) -> usize { self.fastk_period }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_stoch_f_basic() {
        let mut sf = StreamingStochF::new(5, 3);
        let data: Vec<(f64, f64, f64)> = (0..15)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.5).sin() * 10.0;
                (h, h - 3.0, h - 1.5)
            })
            .collect();
        let mut last = None;
        for &d in &data {
            last = sf.next(d);
        }
        let v = last.unwrap();
        assert!((0.0..=100.0).contains(&v.k), "StochF K should be 0-100, got {}", v.k);
    }

    #[test]
    fn test_streaming_stoch_f_reset() {
        let mut sf = StreamingStochF::new(5, 3);
        for i in 0..15 {
            sf.next((50.0 + i as f64, 45.0 + i as f64, 48.0 + i as f64));
        }
        assert!(sf.is_ready());
        sf.reset();
        assert!(!sf.is_ready());
        assert_eq!(sf.count(), 0);
    }

    #[test]
    fn test_streaming_stoch_f_meta() {
        let sf = StreamingStochF::new(5, 3);
        assert_eq!(StreamingStochF::name(), "STOCHF");
        assert_eq!(sf.warm_up_period(), 5);
    }

    /// 与线性扫描实现交叉验证：单调队列 O(1) 与 O(period) 输出一致。
    #[test]
    fn test_streaming_stoch_f_parity_with_linear_scan() {
        let fk = 14;
        let fd = 3;
        let mut sf = StreamingStochF::new(fk, fd);
        // 线性参考实现
        let mut highs: Vec<f64> = Vec::new();
        let mut lows: Vec<f64> = Vec::new();
        let mut ks: Vec<f64> = Vec::new();
        let bars: Vec<(f64, f64, f64)> = (0..300)
            .map(|i| {
                let h = 100.0 + (i as f64 * 0.21).sin() * 8.0 + i as f64 * 0.03;
                let l = h - 2.5 - (i as f64 * 0.13).cos().abs() * 1.5;
                let c = (h + l) / 2.0;
                (h, l, c)
            })
            .collect();
        for &(h, l, c) in &bars {
            // 线性参考 max/min
            highs.push(h);
            lows.push(l);
            if highs.len() > fk { highs.remove(0); }
            if lows.len() > fk { lows.remove(0); }
            let lin_opt = {
                if highs.len() < fk { None } else {
                    let highest = highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    let lowest = lows.iter().cloned().fold(f64::INFINITY, f64::min);
                    let range = highest - lowest;
                    let k = if range.abs() > 1e-15 {
                        (c - lowest) / range * 100.0
                    } else { 50.0 };
                    ks.push(k);
                    if ks.len() > fd { ks.remove(0); }
                    let d = if ks.len() < fd { f64::NAN } else { ks.iter().sum::<f64>() / fd as f64 };
                    Some((k, d))
                }
            };
            let opt = sf.next((h, l, c));
            match (lin_opt, opt) {
                (None, None) => continue,
                (Some((lk, ld)), Some(v)) => {
                    assert!((v.k - lk).abs() < 1e-9, "STOCHF K mismatch: {} vs {}", v.k, lk);
                    if ld.is_finite() {
                        assert!((v.d - ld).abs() < 1e-9, "STOCHF D mismatch: {} vs {}", v.d, ld);
                    }
                }
                (Some(_), None) => panic!("linear ready, streaming not ready"),
                (None, Some(_)) => panic!("streaming ready, linear not ready"),
            }
        }
    }
}
