//! SweepEngine: multi-parameter Cartesian product sweep framework.
//!
//! Provides parallel parameter scanning for indicators using rayon.

use crate::error::Result;
use super::sweepable::{Sweepable, SweepParams, SweepResult};

/// Configuration for a parameter sweep dimension.
#[derive(Debug, Clone)]
pub struct ParamRange {
    pub start: usize,
    pub end: usize,
    pub step: usize,
}

impl ParamRange {
    pub fn new(start: usize, end: usize, step: usize) -> Self {
        Self { start, end, step }
    }

    /// Generate all values in this range (inclusive of start, exclusive of end).
    pub fn values(&self) -> Vec<usize> {
        let mut v = Vec::new();
        let mut cur = self.start;
        while cur < self.end {
            v.push(cur);
            cur += self.step;
        }
        v
    }
}

/// Result of a full sweep engine run.
#[derive(Debug, Clone)]
pub struct SweepEngineResult {
    pub indicator_name: String,
    pub results: Vec<SweepResult>,
    pub param_count: usize,
}

/// Multi-parameter sweep engine with Cartesian product expansion and parallel execution.
///
/// # Example
///
/// ```
/// use alpha_ta_core::indicators::sweep_engine::{SweepEngine, ParamRange};
/// use alpha_ta_core::indicators::sweepable::SmaSweepable;
///
/// let data: Vec<f64> = (0..200).map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0).collect();
/// let engine = SweepEngine::new();
/// let result = engine.run(
///     &SmaSweepable,
///     &data,
///     &[ParamRange::new(5, 51, 5)],
/// ).unwrap();
/// assert_eq!(result.param_count, 10); // 5,10,15,20,25,30,35,40,45,50
/// ```
pub struct SweepEngine {
    parallel: bool,
}

impl SweepEngine {
    pub fn new() -> Self {
        Self { parallel: true }
    }

    /// Disable parallel execution (useful for debugging or single-threaded contexts).
    pub fn sequential(mut self) -> Self {
        self.parallel = false;
        self
    }

    /// Run a parameter sweep with Cartesian product of all ParamRange dimensions.
    ///
    /// For a single dimension, this sweeps one parameter.
    /// For multiple dimensions, it generates the full Cartesian product.
    pub fn run(
        &self,
        indicator: &dyn Sweepable,
        data: &[f64],
        ranges: &[ParamRange],
    ) -> Result<SweepEngineResult> {
        let params = self.expand_params(ranges);
        let param_count = params.len();

        let results = {
            #[cfg(feature = "rayon")]
            {
                if self.parallel && param_count > 1 {
                    self.run_parallel(indicator, data, &params)?
                } else {
                    indicator.sweep(data, &params)?
                }
            }
            #[cfg(not(feature = "rayon"))]
            {
                indicator.sweep(data, &params)?
            }
        };

        Ok(SweepEngineResult {
            indicator_name: indicator.name().to_string(),
            results,
            param_count,
        })
    }

    /// Expand parameter ranges into Cartesian product of SweepParams.
    fn expand_params(&self, ranges: &[ParamRange]) -> Vec<SweepParams> {
        match ranges.len() {
            0 => vec![],
            1 => ranges[0].values().into_iter().map(SweepParams::Period).collect(),
            2 => {
                let v0 = ranges[0].values();
                let v1 = ranges[1].values();
                let mut params = Vec::with_capacity(v0.len() * v1.len());
                for &a in &v0 {
                    for &b in &v1 {
                        params.push(SweepParams::DualPeriod(a, b));
                    }
                }
                params
            }
            _ => {
                let v0 = ranges[0].values();
                let v1 = ranges[1].values();
                let v2 = if ranges.len() > 2 {
                    ranges[2].values()
                } else {
                    vec![0]
                };
                let mut params = Vec::with_capacity(v0.len() * v1.len() * v2.len());
                for &a in &v0 {
                    for &b in &v1 {
                        for &c in &v2 {
                            params.push(SweepParams::TriplePeriod(a, b, c));
                        }
                    }
                }
                params
            }
        }
    }

    /// Parallel sweep: split params into chunks, run each chunk in parallel via rayon.
    #[cfg(feature = "rayon")]
    fn run_parallel(
        &self,
        indicator: &dyn Sweepable,
        data: &[f64],
        params: &[SweepParams],
    ) -> Result<Vec<SweepResult>> {
        use rayon::prelude::*;

        let chunk_size = (params.len() / rayon::current_num_threads()).max(1);
        let chunks: Vec<&[SweepParams]> = params.chunks(chunk_size).collect();

        let chunk_results: Vec<Result<Vec<SweepResult>>> = chunks
            .par_iter()
            .map(|chunk| indicator.sweep(data, chunk))
            .collect();

        let mut all_results = Vec::with_capacity(params.len());
        for chunk_result in chunk_results {
            all_results.extend(chunk_result?);
        }
        Ok(all_results)
    }
}

impl Default for SweepEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::sweepable::{EmaSweepable, RsiSweepable, SmaSweepable};

    fn gen_data(n: usize) -> Vec<f64> {
        let mut data = Vec::with_capacity(n);
        let mut price = 100.0;
        for i in 0..n {
            price += ((i as f64 * 0.1).sin() * 2.0) + 0.01;
            data.push(price);
        }
        data
    }

    #[test]
    fn test_sweep_engine_sma_single_range() {
        let data = gen_data(200);
        let engine = SweepEngine::new();
        let result = engine.run(&SmaSweepable, &data, &[ParamRange::new(5, 51, 5)]).unwrap();
        assert_eq!(result.param_count, 10);
        assert_eq!(result.results.len(), 10);
        assert_eq!(result.indicator_name, "SMA");
    }

    #[test]
    fn test_sweep_engine_ema_single_range() {
        let data = gen_data(200);
        let engine = SweepEngine::new();
        let result = engine.run(&EmaSweepable, &data, &[ParamRange::new(5, 26, 5)]).unwrap();
        assert_eq!(result.param_count, 5);
        assert_eq!(result.results.len(), 5);
        assert_eq!(result.indicator_name, "EMA");
    }

    #[test]
    fn test_sweep_engine_rsi_single_range() {
        let data = gen_data(200);
        let engine = SweepEngine::new();
        let result = engine.run(&RsiSweepable, &data, &[ParamRange::new(5, 31, 5)]).unwrap();
        assert_eq!(result.param_count, 6);
        assert_eq!(result.results.len(), 6);
    }

    #[test]
    fn test_sweep_engine_cartesian_product_2d() {
        let data = gen_data(200);
        let engine = SweepEngine::new();
        let result = engine.run(
            &SmaSweepable,
            &data,
            &[ParamRange::new(5, 16, 5), ParamRange::new(20, 41, 10)],
        ).unwrap();
        // 5,10,15 x 20,30,40 = 9 combinations
        assert_eq!(result.param_count, 9);
        assert_eq!(result.results.len(), 9);
    }

    #[test]
    fn test_sweep_engine_cartesian_product_3d() {
        let data = gen_data(200);
        let engine = SweepEngine::new();
        let result = engine.run(
            &SmaSweepable,
            &data,
            &[
                ParamRange::new(5, 11, 5),
                ParamRange::new(10, 21, 10),
                ParamRange::new(1, 3, 1),
            ],
        ).unwrap();
        // 2 x 2 x 2 = 8
        assert_eq!(result.param_count, 8);
    }

    #[test]
    fn test_sweep_engine_sequential_mode() {
        let data = gen_data(200);
        let engine = SweepEngine::new().sequential();
        let result = engine.run(&SmaSweepable, &data, &[ParamRange::new(5, 21, 5)]).unwrap();
        assert_eq!(result.param_count, 4);
    }

    #[test]
    fn test_sweep_engine_results_correctness() {
        let data = gen_data(200);
        let engine = SweepEngine::new();
        let result = engine.run(&SmaSweepable, &data, &[ParamRange::new(10, 11, 1)]).unwrap();
        assert_eq!(result.param_count, 1);
        let expected = crate::math::moving_avg::sma(&data, 10).unwrap();
        for i in 0..data.len() {
            if expected[i].is_nan() {
                assert!(result.results[0].values[i].is_nan());
            } else {
                assert!((result.results[0].values[i] - expected[i]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_sweep_engine_empty_range() {
        let data = gen_data(200);
        let engine = SweepEngine::new();
        let result = engine.run(&SmaSweepable, &data, &[]).unwrap();
        assert_eq!(result.param_count, 0);
    }

    #[test]
    fn test_param_range_values() {
        let r = ParamRange::new(5, 21, 5);
        assert_eq!(r.values(), vec![5, 10, 15, 20]);
    }

    #[test]
    fn test_param_range_step_one() {
        let r = ParamRange::new(3, 7, 1);
        assert_eq!(r.values(), vec![3, 4, 5, 6]);
    }

    #[test]
    fn test_sweep_engine_parallel_matches_sequential() {
        let data = gen_data(500);
        let ranges = [ParamRange::new(5, 51, 5)];

        let par_result = SweepEngine::new().run(&SmaSweepable, &data, &ranges).unwrap();
        let seq_result = SweepEngine::new().sequential().run(&SmaSweepable, &data, &ranges).unwrap();

        assert_eq!(par_result.param_count, seq_result.param_count);
        for (pr, sr) in par_result.results.iter().zip(seq_result.results.iter()) {
            for (a, b) in pr.values.iter().zip(sr.values.iter()) {
                if a.is_nan() {
                    assert!(b.is_nan());
                } else {
                    assert!((a - b).abs() < 1e-10);
                }
            }
        }
    }

    #[test]
    fn test_sweep_engine_large_param_space() {
        let data = gen_data(500);
        let engine = SweepEngine::new();
        let result = engine.run(
            &SmaSweepable,
            &data,
            &[ParamRange::new(2, 102, 1)],
        ).unwrap();
        assert_eq!(result.param_count, 100);
        assert_eq!(result.results.len(), 100);
    }
}
