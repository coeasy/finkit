use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming Inertia Indicator.
///
/// Combines RVI ratio (4-bar weighted average of close-open / high-low) with
/// SMA smoothing and TSF (linear regression forecast) to produce a momentum
/// oscillator that measures trend persistence.
///
/// Input: `(open, high, low, close)` tuple per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingInertia {
    rvi_period: usize,
    tsf_period: usize,
    // Ring buffer for the last 4 bars (for wavg4 calculation)
    bar_ring: [(f64, f64, f64, f64); 4], // (open, high, low, close)
    bar_count: usize,
    // Ring buffer for RVI ratio -> SMA
    ratio_ring: Vec<f64>,
    ratio_ring_idx: usize,
    ratio_sum: f64,
    ratio_count: usize,
    // Ring buffer for smoothed RVI values -> TSF
    rvi_ring: Vec<f64>,
    rvi_ring_idx: usize,
    rvi_count: usize,
    rvi_sum_y: f64,
    rvi_sum_xy: f64,
    // TSF precomputed constants
    sx: f64,
    sx2: f64,
    n: f64,
    denom: f64,

    count: usize,
    last_value: Option<f64>,
}

impl StreamingInertia {
    pub fn new(rvi_period: usize, tsf_period: usize) -> Self {
        let n = tsf_period as f64;
        let sx: f64 = (0..tsf_period).map(|i| i as f64).sum();
        let sx2: f64 = (0..tsf_period).map(|i| (i as f64) * (i as f64)).sum();
        let denom = n * sx2 - sx * sx;

        Self {
            rvi_period,
            tsf_period,
            bar_ring: [(0.0, 0.0, 0.0, 0.0); 4],
            bar_count: 0,
            ratio_ring: vec![0.0; rvi_period],
            ratio_ring_idx: 0,
            ratio_sum: 0.0,
            ratio_count: 0,
            rvi_ring: vec![0.0; tsf_period],
            rvi_ring_idx: 0,
            rvi_count: 0,
            rvi_sum_y: 0.0,
            rvi_sum_xy: 0.0,
            sx,
            sx2,
            n,
            denom,
            count: 0,
            last_value: None,
        }
    }
}

#[inline]
fn wavg4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    (a + 2.0 * b + 2.0 * c + d) / 6.0
}

impl StreamingIndicator<(f64, f64, f64, f64), f64> for StreamingInertia {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64, f64)) -> Option<f64> {
        let (open, high, low, close) = input;
        self.count += 1;

        // Store current bar in the 4-bar ring
        let bar_idx = (self.bar_count) % 4;
        self.bar_ring[bar_idx] = (open, high, low, close);
        self.bar_count += 1;

        // Need at least 4 bars to compute RVI ratio
        if self.bar_count < 4 {
            self.last_value = None;
            return None;
        }

        // Compute RVI ratio using 4-bar weighted average
        let i0 = (self.bar_count - 1) % 4;
        let i1 = (self.bar_count - 2) % 4;
        let i2 = (self.bar_count - 3) % 4;
        let i3 = (self.bar_count - 4) % 4;

        let (o0, h0, l0, c0) = self.bar_ring[i0];
        let (o1, h1, l1, c1) = self.bar_ring[i1];
        let (o2, h2, l2, c2) = self.bar_ring[i2];
        let (o3, h3, l3, c3) = self.bar_ring[i3];

        let num = wavg4(c0 - o0, c1 - o1, c2 - o2, c3 - o3);
        let den = wavg4(h0 - l0, h1 - l1, h2 - l2, h3 - l3);
        let ratio = if den.abs() > 1e-15 { num / den } else { 0.0 };

        // Update SMA ring buffer for ratio
        let old_ratio = self.ratio_ring[self.ratio_ring_idx];
        self.ratio_ring[self.ratio_ring_idx] = ratio;
        self.ratio_ring_idx += 1;
        if self.ratio_ring_idx == self.rvi_period {
            self.ratio_ring_idx = 0;
        }

        if self.ratio_count < self.rvi_period {
            self.ratio_sum += ratio;
            self.ratio_count += 1;
        } else {
            self.ratio_sum += ratio - old_ratio;
        }

        // Need enough ratio values for SMA
        if self.ratio_count < self.rvi_period {
            self.last_value = None;
            return None;
        }

        let rvi_val = self.ratio_sum / self.rvi_period as f64;

        // Update TSF ring buffer with the smoothed RVI value
        let n = self.tsf_period as f64;
        if self.rvi_count == self.tsf_period {
            let old_y = self.rvi_ring[self.rvi_ring_idx];
            self.rvi_sum_xy = self.rvi_sum_xy - self.rvi_sum_y + old_y + (n - 1.0) * rvi_val;
            self.rvi_sum_y = self.rvi_sum_y - old_y + rvi_val;
        } else {
            self.rvi_sum_y += rvi_val;
            self.rvi_sum_xy += (self.rvi_count as f64) * rvi_val;
        }

        self.rvi_ring[self.rvi_ring_idx] = rvi_val;
        self.rvi_ring_idx += 1;
        if self.rvi_ring_idx == self.tsf_period {
            self.rvi_ring_idx = 0;
        }

        if self.rvi_count < self.tsf_period {
            self.rvi_count += 1;
        }

        if self.rvi_count < self.tsf_period {
            self.last_value = None;
            return None;
        }

        if self.denom.abs() < 1e-15 {
            self.last_value = Some(rvi_val);
            return self.last_value;
        }

        let slope = (self.n * self.rvi_sum_xy - self.sx * self.rvi_sum_y) / self.denom;
        let intercept = (self.rvi_sum_y - slope * self.sx) / self.n;
        let result = intercept + slope * self.n;

        self.last_value = Some(result);
        self.last_value
    }

    fn reset(&mut self) {
        self.bar_ring = [(0.0, 0.0, 0.0, 0.0); 4];
        self.bar_count = 0;
        self.ratio_ring.fill(0.0);
        self.ratio_ring_idx = 0;
        self.ratio_sum = 0.0;
        self.ratio_count = 0;
        self.rvi_ring.fill(0.0);
        self.rvi_ring_idx = 0;
        self.rvi_count = 0;
        self.rvi_sum_y = 0.0;
        self.rvi_sum_xy = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.rvi_count >= self.tsf_period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingInertia {
    fn name() -> &'static str {
        "Inertia"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Inertia Indicator: TSF-smoothed Relative Volatility Index for trend persistence"
    }
    fn warm_up_period(&self) -> usize {
        3 + self.rvi_period + self.tsf_period - 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_inertia_basic() {
        let mut ind = StreamingInertia::new(10, 14);
        let n = 50;
        let open: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let close: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) * 0.5 + 0.3).collect();
        let high: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) * 0.5 + 1.0).collect();
        let low: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) * 0.5 - 0.5).collect();

        let mut results = Vec::new();
        for i in 0..n {
            if let Some(val) = ind.next((open[i], high[i], low[i], close[i])) {
                results.push((i, val));
            }
        }
        assert!(!results.is_empty());
        for (_, v) in &results {
            assert!(v.is_finite());
        }
    }

    #[test]
    fn test_streaming_inertia_reset() {
        let mut ind = StreamingInertia::new(5, 5);
        for i in 0..20 {
            let p = 100.0 + i as f64;
            ind.next((p, p + 1.0, p - 0.5, p + 0.3));
        }
        assert!(ind.is_ready());

        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), None);
    }

    #[test]
    fn test_streaming_inertia_meta() {
        let ind = StreamingInertia::new(10, 14);
        assert_eq!(StreamingInertia::name(), "Inertia");
        assert_eq!(StreamingInertia::category(), "momentum");
        assert_eq!(ind.warm_up_period(), 3 + 10 + 14 - 2);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 60;
        let open: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64 * 0.2).sin() * 3.0).collect();
        let high: Vec<f64> = open.iter().map(|o| o + 1.5).collect();
        let low: Vec<f64> = open.iter().map(|o| o - 1.0).collect();
        let close: Vec<f64> = open.iter().map(|o| o + 0.3).collect();
        let rvi_period = 10;
        let tsf_period = 14;

        let batch = crate::indicators::momentum_ext::inertia(
            &open, &high, &low, &close, rvi_period, tsf_period,
        )
        .unwrap();

        let mut streaming = StreamingInertia::new(rvi_period, tsf_period);
        for i in 0..n {
            let result = streaming.next((open[i], high[i], low[i], close[i]));
            if let Some(val) = result {
                if !batch[i].is_nan() {
                    assert!(
                        (val - batch[i]).abs() < 1e-10,
                        "Mismatch at {i}: streaming={val}, batch={}",
                        batch[i]
                    );
                }
            }
        }
    }
}
