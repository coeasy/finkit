use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Squeeze Momentum output.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SqueezeMomentumOutput {
    pub momentum: f64,
    pub squeeze_on: bool,
    pub squeeze_off: bool,
}

/// Streaming Squeeze Momentum Indicator (John Carter version).
///
/// Input: `(high, low, close)` per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingSqueezeMomentum {
    bb_period: usize,
    bb_mult: f64,
    kc_period: usize,
    kc_mult: f64,
    // Welford online for BB SMA + sample stddev
    close_ring: Vec<f64>,
    close_ring_idx: usize,
    close_count: usize,
    bb_mean: f64,
    bb_m2: f64,
    // ATR via Wilder's RMA (matches batch implementation)
    atr_val: f64,
    atr_inv_period: f64,
    atr_ready: bool,
    tr_init_sum: f64,
    tr_init_count: usize,
    prev_close: f64,
    // KC midline (SMA of close over kc_period)
    kc_close_ring: Vec<f64>,
    kc_close_ring_idx: usize,
    kc_close_sum: f64,
    kc_close_count: usize,
    // Delta ring buffer for linear regression
    delta_ring: Vec<f64>,
    delta_ring_idx: usize,
    delta_count: usize,
    // TSF precomputed constants
    sx: f64,
    n_linreg: f64,
    denom: f64,
    // Incremental linreg accumulators
    sum_y: f64,
    sum_xy: f64,

    prev_squeeze: bool,
    count: usize,
    last_value: Option<SqueezeMomentumOutput>,
}

impl StreamingSqueezeMomentum {
    pub fn new(bb_period: usize, bb_mult: f64, kc_period: usize, kc_mult: f64) -> Self {
        let n_linreg = bb_period as f64;
        let sx: f64 = (0..bb_period).map(|j| j as f64).sum();
        let sx2: f64 = (0..bb_period).map(|j| (j as f64) * (j as f64)).sum();
        let denom = n_linreg * sx2 - sx * sx;
        let atr_inv_period = 1.0 / kc_period as f64;

        Self {
            bb_period,
            bb_mult,
            kc_period,
            kc_mult,
            close_ring: vec![0.0; bb_period],
            close_ring_idx: 0,
            close_count: 0,
            bb_mean: 0.0,
            bb_m2: 0.0,
            atr_val: 0.0,
            atr_inv_period,
            atr_ready: false,
            tr_init_sum: 0.0,
            tr_init_count: 0,
            prev_close: f64::NAN,
            kc_close_ring: vec![0.0; kc_period],
            kc_close_ring_idx: 0,
            kc_close_sum: 0.0,
            kc_close_count: 0,
            delta_ring: vec![0.0; bb_period],
            delta_ring_idx: 0,
            delta_count: 0,
            sx,
            n_linreg,
            denom,
            sum_y: 0.0,
            sum_xy: 0.0,
            prev_squeeze: false,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64), SqueezeMomentumOutput> for StreamingSqueezeMomentum {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<SqueezeMomentumOutput> {
        let (high, low, close) = input;
        self.count += 1;

        // Compute True Range
        let tr = if self.prev_close.is_nan() {
            high - low
        } else {
            let hl = high - low;
            let hc = (high - self.prev_close).abs();
            let lc = (low - self.prev_close).abs();
            hl.max(hc).max(lc)
        };

        // Update ATR，严格对齐 batch atr()：
        // 首有效在索引 kc_period，种子 = SMA of TR[1..=kc_period]（不含 TR[0]），之后 Wilder RMA 递推
        if !self.atr_ready {
            // 跳过首根 TR[0]（其 prev_close 为 NaN，TR=high-low），避免种子污染
            if self.count >= 2 {
                self.tr_init_sum += tr;
                self.tr_init_count += 1;
            }
            if self.tr_init_count == self.kc_period {
                self.atr_val = self.tr_init_sum / self.kc_period as f64;
                self.atr_ready = true;
            }
        } else {
            // Wilder's RMA: ATR[i] = ATR[i-1] + (TR[i] - ATR[i-1]) / period
            self.atr_val += (tr - self.atr_val) * self.atr_inv_period;
        }

        // Update BB via Welford online (matching bbands sample variance)
        if self.close_count < self.bb_period {
            // Accumulating initial window
            let n = (self.close_count + 1) as f64;
            let delta = close - self.bb_mean;
            self.bb_mean += delta / n;
            self.bb_m2 += delta * (close - self.bb_mean);
            self.close_ring[self.close_ring_idx] = close;
            self.close_ring_idx += 1;
            if self.close_ring_idx == self.bb_period {
                self.close_ring_idx = 0;
            }
            self.close_count += 1;
        } else {
            // Sliding window update matching bbands inner loop
            let old = self.close_ring[self.close_ring_idx];
            let old_mean = self.bb_mean;
            self.bb_mean += (close - old) / self.bb_period as f64;
            self.bb_m2 += (close - self.bb_mean) * (close - old_mean)
                - (old - self.bb_mean) * (old - old_mean);
            if self.bb_m2 < 0.0 {
                self.bb_m2 = 0.0;
            }
            self.close_ring[self.close_ring_idx] = close;
            self.close_ring_idx += 1;
            if self.close_ring_idx == self.bb_period {
                self.close_ring_idx = 0;
            }
        }

        // Update KC midline (SMA of close over kc_period)
        let old_kc_close = self.kc_close_ring[self.kc_close_ring_idx];
        self.kc_close_ring[self.kc_close_ring_idx] = close;
        self.kc_close_ring_idx += 1;
        if self.kc_close_ring_idx == self.kc_period {
            self.kc_close_ring_idx = 0;
        }
        if self.kc_close_count < self.kc_period {
            self.kc_close_sum += close;
            self.kc_close_count += 1;
        } else {
            self.kc_close_sum += close - old_kc_close;
        }

        self.prev_close = close;

        // Check if we have enough data
        let bb_ready = self.close_count >= self.bb_period;
        let kc_ready = self.kc_close_count >= self.kc_period && self.atr_ready;

        if !bb_ready || !kc_ready {
            self.last_value = None;
            return None;
        }

        // Compute BB bands
        // TA-Lib uses population variance (÷n), not sample variance (÷n-1)
        let inv_p = 1.0 / self.bb_period as f64;
        let stddev = (self.bb_m2 * inv_p).max(0.0).sqrt();
        let bb_upper = self.bb_mean + self.bb_mult * stddev;
        let bb_lower = self.bb_mean - self.bb_mult * stddev;
        let bb_mid = self.bb_mean;

        // Compute KC
        let kc_mid = self.kc_close_sum / self.kc_period as f64;
        let kc_upper = kc_mid + self.kc_mult * self.atr_val;
        let kc_lower = kc_mid - self.kc_mult * self.atr_val;

        // Squeeze state
        let squeeze_on = bb_lower > kc_lower && bb_upper < kc_upper;
        let squeeze_off = self.prev_squeeze && !squeeze_on;
        self.prev_squeeze = squeeze_on;

        let delta_val = close - (bb_mid + kc_mid) / 2.0;
        let n = self.bb_period as f64;
        let old_y = self.delta_ring[self.delta_ring_idx];
        self.sum_xy = self.sum_xy - self.sum_y + old_y + (n - 1.0) * delta_val;
        self.sum_y = self.sum_y - old_y + delta_val;
        self.delta_ring[self.delta_ring_idx] = delta_val;
        self.delta_ring_idx += 1;
        if self.delta_ring_idx == self.bb_period {
            self.delta_ring_idx = 0;
        }
        if self.delta_count < self.bb_period {
            self.delta_count += 1;
        }

        if self.delta_count < self.bb_period || self.denom.abs() < 1e-15 {
            let out = SqueezeMomentumOutput {
                momentum: 0.0,
                squeeze_on,
                squeeze_off,
            };
            self.last_value = Some(out);
            return Some(out);
        }

        let slope = (self.n_linreg * self.sum_xy - self.sx * self.sum_y) / self.denom;
        let intercept = (self.sum_y - slope * self.sx) / self.n_linreg;
        let momentum = intercept + slope * self.n_linreg;

        let out = SqueezeMomentumOutput {
            momentum,
            squeeze_on,
            squeeze_off,
        };
        self.last_value = Some(out);
        Some(out)
    }

    fn reset(&mut self) {
        self.close_ring.fill(0.0);
        self.close_ring_idx = 0;
        self.close_count = 0;
        self.bb_mean = 0.0;
        self.bb_m2 = 0.0;
        self.atr_val = 0.0;
        self.atr_ready = false;
        self.tr_init_sum = 0.0;
        self.tr_init_count = 0;
        self.prev_close = f64::NAN;
        self.kc_close_ring.fill(0.0);
        self.kc_close_ring_idx = 0;
        self.kc_close_sum = 0.0;
        self.kc_close_count = 0;
        self.delta_ring.fill(0.0);
        self.delta_ring_idx = 0;
        self.delta_count = 0;
        self.sum_y = 0.0;
        self.sum_xy = 0.0;
        self.prev_squeeze = false;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.close_count >= self.bb_period
            && self.kc_close_count >= self.kc_period
            && self.atr_ready
    }

    impl_standard_methods!(output = SqueezeMomentumOutput);
}

impl IndicatorMeta for StreamingSqueezeMomentum {
    fn name() -> &'static str {
        "SqueezeMomentum"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Squeeze Momentum (John Carter): BB vs KC squeeze detection with linreg momentum"
    }
    fn warm_up_period(&self) -> usize {
        self.bb_period.max(self.kc_period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_squeeze_momentum_basic() {
        let mut ind = StreamingSqueezeMomentum::new(20, 2.0, 20, 1.5);
        let n = 100;
        let close: Vec<f64> = (0..n)
            .map(|i| 100.0 + (i as f64 * 0.1).sin() * 5.0)
            .collect();
        let high: Vec<f64> = close.iter().map(|c| c + 1.5).collect();
        let low: Vec<f64> = close.iter().map(|c| c - 1.5).collect();

        let mut results = Vec::new();
        for i in 0..n {
            if let Some(out) = ind.next((high[i], low[i], close[i])) {
                results.push((i, out));
            }
        }
        assert!(!results.is_empty());
        for (_, out) in &results {
            assert!(out.momentum.is_finite());
        }
    }

    #[test]
    fn test_streaming_squeeze_momentum_reset() {
        let mut ind = StreamingSqueezeMomentum::new(5, 2.0, 5, 1.5);
        for i in 0..20 {
            let p = 100.0 + i as f64;
            ind.next((p + 1.0, p - 1.0, p));
        }
        assert!(ind.is_ready());

        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), None);
    }

    #[test]
    fn test_streaming_squeeze_momentum_meta() {
        let ind = StreamingSqueezeMomentum::new(20, 2.0, 20, 1.5);
        assert_eq!(StreamingSqueezeMomentum::name(), "SqueezeMomentum");
        assert_eq!(StreamingSqueezeMomentum::category(), "momentum");
        assert_eq!(ind.warm_up_period(), 20);
    }

    #[test]
    fn test_streaming_vs_batch_squeeze_on() {
        let n = 100;
        let close: Vec<f64> = (0..n)
            .map(|i| 100.0 + (i as f64 * 0.15).sin() * 4.0)
            .collect();
        let high: Vec<f64> = close.iter().map(|c| c + 1.2).collect();
        let low: Vec<f64> = close.iter().map(|c| c - 1.2).collect();

        let batch = crate::indicators::momentum_ext::squeeze_momentum(
            &high, &low, &close, 20, 2.0, 20, 1.5,
        )
        .unwrap();

        let mut streaming = StreamingSqueezeMomentum::new(20, 2.0, 20, 1.5);
        for i in 0..n {
            if let Some(out) = streaming.next((high[i], low[i], close[i])) {
                if !batch.squeeze_on[i].is_nan() {
                    let expected_sq = batch.squeeze_on[i] == 1.0;
                    assert_eq!(
                        out.squeeze_on, expected_sq,
                        "squeeze_on mismatch at {i}: streaming={}, batch={}",
                        out.squeeze_on, batch.squeeze_on[i]
                    );
                }
            }
        }
    }
}
