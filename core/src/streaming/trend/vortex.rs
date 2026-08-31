use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VortexOutput {
    pub vi_plus: f64,
    pub vi_minus: f64,
}

/// Streaming Vortex Indicator using ring buffer for VM+/VM-/TR sums.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingVortex {
    period: usize,
    vm_plus_ring: Vec<f64>,
    vm_minus_ring: Vec<f64>,
    tr_ring: Vec<f64>,
    vm_plus_sum: f64,
    vm_minus_sum: f64,
    tr_sum: f64,
    ring_idx: usize,
    prev_high: f64,
    prev_low: f64,
    prev_close: f64,
    count: usize,
    last_value: Option<VortexOutput>,
}

impl StreamingVortex {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            vm_plus_ring: vec![0.0; period],
            vm_minus_ring: vec![0.0; period],
            tr_ring: vec![0.0; period],
            vm_plus_sum: 0.0,
            vm_minus_sum: 0.0,
            tr_sum: 0.0,
            ring_idx: 0,
            prev_high: f64::NAN,
            prev_low: f64::NAN,
            prev_close: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64), VortexOutput> for StreamingVortex {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<VortexOutput> {
        crate::streaming_measure!("vortex", self.count, {
            let (high, low, close) = input;
            self.count += 1;

            if self.count == 1 {
                self.prev_high = high;
                self.prev_low = low;
                self.prev_close = close;
                self.last_value = None;
                return None;
            }

            let vm_p = (high - self.prev_low).abs();
            let vm_m = (low - self.prev_high).abs();
            let hl = high - low;
            let hc = (high - self.prev_close).abs();
            let lc = (low - self.prev_close).abs();
            let tr = hl.max(hc).max(lc);

            self.vm_plus_sum += vm_p - self.vm_plus_ring[self.ring_idx];
            self.vm_minus_sum += vm_m - self.vm_minus_ring[self.ring_idx];
            self.tr_sum += tr - self.tr_ring[self.ring_idx];
            self.vm_plus_ring[self.ring_idx] = vm_p;
            self.vm_minus_ring[self.ring_idx] = vm_m;
            self.tr_ring[self.ring_idx] = tr;
            self.ring_idx += 1;
            if self.ring_idx == self.period {
                self.ring_idx = 0;
            }

            self.prev_high = high;
            self.prev_low = low;
            self.prev_close = close;

            if self.count > self.period {
                let result = if self.tr_sum > 1e-15 {
                    Some(VortexOutput {
                        vi_plus: self.vm_plus_sum / self.tr_sum,
                        vi_minus: self.vm_minus_sum / self.tr_sum,
                    })
                } else {
                    Some(VortexOutput {
                        vi_plus: 0.0,
                        vi_minus: 0.0,
                    })
                };
                self.last_value = result;
                result
            } else {
                self.last_value = None;
                None
            }
        })
    }

    fn reset(&mut self) {
        self.vm_plus_ring.fill(0.0);
        self.vm_minus_ring.fill(0.0);
        self.tr_ring.fill(0.0);
        self.vm_plus_sum = 0.0;
        self.vm_minus_sum = 0.0;
        self.tr_sum = 0.0;
        self.ring_idx = 0;
        self.prev_high = f64::NAN;
        self.prev_low = f64::NAN;
        self.prev_close = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count > self.period
    }

    impl_standard_methods!(output = VortexOutput);
}

impl IndicatorMeta for StreamingVortex {
    fn name() -> &'static str {
        "Vortex"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Vortex Indicator measuring positive and negative trend strength"
    }
    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_vortex_basic() {
        let mut vortex = StreamingVortex::new(5);
        let data: Vec<(f64, f64, f64)> = vec![
            (45.0, 43.0, 44.0),
            (45.5, 43.5, 44.5),
            (46.0, 44.0, 45.0),
            (45.5, 43.0, 44.0),
            (46.5, 44.0, 45.0),
            (46.0, 43.5, 44.5),
            (45.5, 43.0, 44.0),
            (45.0, 42.5, 43.5),
            (45.5, 43.0, 44.0),
            (46.0, 43.5, 44.5),
        ];

        let mut results = Vec::new();
        for bar in &data {
            if let Some(out) = vortex.next(*bar) {
                results.push(out);
            }
        }
        assert!(!results.is_empty());
        for r in &results {
            assert!(r.vi_plus > 0.0);
            assert!(r.vi_minus > 0.0);
        }
    }

    #[test]
    fn test_streaming_vortex_reset() {
        let mut vortex = StreamingVortex::new(3);
        vortex.next((10.0, 8.0, 9.0));
        vortex.next((11.0, 9.0, 10.0));
        vortex.next((12.0, 10.0, 11.0));
        vortex.next((13.0, 11.0, 12.0));
        assert!(vortex.is_ready());

        vortex.reset();
        assert!(!vortex.is_ready());
        assert_eq!(vortex.value(), None);
    }

    #[test]
    fn test_streaming_vortex_meta() {
        let vortex = StreamingVortex::new(14);
        assert_eq!(StreamingVortex::name(), "Vortex");
        assert_eq!(StreamingVortex::category(), "momentum");
        assert_eq!(vortex.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 50;
        let high: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.3).sin() * 5.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 2.0).collect();
        let close: Vec<f64> = high
            .iter()
            .zip(low.iter())
            .map(|(h, l)| (h + l) / 2.0)
            .collect();
        let period = 14;

        let batch = crate::indicators::momentum_ext::vortex(&high, &low, &close, period).unwrap();

        let mut streaming = StreamingVortex::new(period);
        for i in 0..n {
            if let Some(out) = streaming.next((high[i], low[i], close[i])) {
                if !batch.vi_plus[i].is_nan() {
                    assert!(
                        (out.vi_plus - batch.vi_plus[i]).abs() < 1e-10,
                        "VI+ mismatch at {i}: streaming={}, batch={}",
                        out.vi_plus,
                        batch.vi_plus[i]
                    );
                }
                if !batch.vi_minus[i].is_nan() {
                    assert!(
                        (out.vi_minus - batch.vi_minus[i]).abs() < 1e-10,
                        "VI- mismatch at {i}: streaming={}, batch={}",
                        out.vi_minus,
                        batch.vi_minus[i]
                    );
                }
            }
        }
    }
}
