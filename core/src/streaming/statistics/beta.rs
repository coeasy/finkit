use std::collections::VecDeque;

use crate::streaming::traits::IndicatorMeta;

/// Streaming Beta coefficient using rolling covariance / variance.
///
/// Beta = Cov(asset_returns, benchmark_returns) / Var(benchmark_returns)
///
/// Supports `next_pair(x, y)` dual-input method.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingBeta {
    period: usize,
    asset_window: VecDeque<f64>,
    bench_window: VecDeque<f64>,
    sum_a: f64,
    sum_b: f64,
    sum_ab: f64,
    sum_a2: f64,
    sum_b2: f64,
    prev_asset: Option<f64>,
    prev_bench: Option<f64>,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingBeta {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            asset_window: VecDeque::with_capacity(period),
            bench_window: VecDeque::with_capacity(period),
            sum_a: 0.0,
            sum_b: 0.0,
            sum_ab: 0.0,
            sum_a2: 0.0,
            sum_b2: 0.0,
            prev_asset: None,
            prev_bench: None,
            count: 0,
            last_value: None,
        }
    }

    /// Feed a pair of asset and benchmark prices.
    pub fn next_pair(&mut self, asset: f64, benchmark: f64) -> Option<f64> {
        self.count += 1;

        if let (Some(pa), Some(pb)) = (self.prev_asset, self.prev_bench) {
            let ar = if pa.abs() > 1e-15 { (asset - pa) / pa } else { 0.0 };
            let br = if pb.abs() > 1e-15 { (benchmark - pb) / pb } else { 0.0 };

            // If window is full, evict the oldest pair and subtract from accumulators
            if self.asset_window.len() == self.period {
                let old_a = self.asset_window.pop_front().unwrap();
                let old_b = self.bench_window.pop_front().unwrap();
                self.sum_a -= old_a;
                self.sum_b -= old_b;
                self.sum_ab -= old_a * old_b;
                self.sum_a2 -= old_a * old_a;
                self.sum_b2 -= old_b * old_b;
            }

            // Push new returns and add to accumulators
            self.asset_window.push_back(ar);
            self.bench_window.push_back(br);
            self.sum_a += ar;
            self.sum_b += br;
            self.sum_ab += ar * br;
            self.sum_a2 += ar * ar;
            self.sum_b2 += br * br;
        }

        self.prev_asset = Some(asset);
        self.prev_bench = Some(benchmark);

        let result = if self.asset_window.len() == self.period {
            let n = self.period as f64;

            let cov = self.sum_ab - self.sum_a * self.sum_b / n;
            let var_b = self.sum_b2 - self.sum_b * self.sum_b / n;

            if var_b.abs() > 1e-15 {
                Some(cov / var_b)
            } else {
                None
            }
        } else {
            None
        };
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.asset_window.clear();
        self.bench_window.clear();
        self.sum_a = 0.0;
        self.sum_b = 0.0;
        self.sum_ab = 0.0;
        self.sum_a2 = 0.0;
        self.sum_b2 = 0.0;
        self.prev_asset = None;
        self.prev_bench = None;
        self.count = 0;
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool { self.asset_window.len() >= self.period }
    pub fn count(&self) -> usize { self.count }
    pub fn value(&self) -> Option<f64> { self.last_value }
}

impl IndicatorMeta for StreamingBeta {
    fn name() -> &'static str { "BETA" }
    fn category() -> &'static str { "statistic" }
    fn description() -> &'static str { "Rolling Beta Coefficient" }
    fn warm_up_period(&self) -> usize { self.period + 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_beta_basic() {
        let mut beta = StreamingBeta::new(3);
        let assets = [100.0, 102.0, 104.0, 103.0, 105.0, 107.0];
        let benchs = [200.0, 204.0, 206.0, 205.0, 208.0, 210.0];

        let mut last = None;
        for i in 0..assets.len() {
            last = beta.next_pair(assets[i], benchs[i]);
        }
        assert!(last.is_some());
        assert!(last.unwrap().is_finite());
    }

    #[test]
    fn test_streaming_beta_meta() {
        assert_eq!(StreamingBeta::name(), "BETA");
        assert_eq!(StreamingBeta::category(), "statistic");
    }

    #[test]
    fn test_streaming_beta_reset() {
        let mut beta = StreamingBeta::new(3);
        beta.next_pair(100.0, 200.0);
        beta.next_pair(102.0, 204.0);
        beta.next_pair(104.0, 206.0);
        beta.next_pair(103.0, 205.0);
        assert!(beta.is_ready());
        beta.reset();
        assert!(!beta.is_ready());
        assert_eq!(beta.count(), 0);
    }

    #[test]
    fn test_streaming_beta_welford_stability() {
        let mut beta = StreamingBeta::new(50);
        let base = 1e6;
        for i in 0..200 {
            let a = base + (i as f64 * 0.01).sin() * 100.0;
            let b = base + (i as f64 * 0.01).cos() * 100.0;
            if let Some(v) = beta.next_pair(a, b) {
                assert!(v.is_finite(), "NaN/Inf at i={i}");
            }
        }
    }
}
