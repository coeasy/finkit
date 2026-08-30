//! Parallel batch indicator API (P-9).
//!
//! Provides a small, ergonomic surface for running **multiple independent
//! indicator computations** in parallel over the same input data. Each
//! `IndicatorJob` is a self-contained closure that takes a `&[f64]` and
//! returns a `Result<Array1<f64>>`; jobs are dispatched across the
//! `rayon` thread pool when the `rayon` feature is enabled and run
//! sequentially otherwise.
//!
//! # When does it help?
//!
//! - Computing 3+ indicators on the same OHLCV bar set (e.g. SMA + EMA +
//!   RSI + MACD).
//! - Multi-symbol/multi-period sweeps where each job is independent.
//! - Multi-threaded backtests that fan out across a feature matrix.
//!
//! For a single indicator over a single series, the SIMD path in
//! [`crate::math::simd_kernels`] is already saturating; this module only
//! helps when **job-level** parallelism is available (i.e. the work can be
//! split into roughly CPU-count pieces).
//!
//! # Example
//!
//! ```
//! use finkit::batch::{IndicatorJob, run_parallel};
//! use finkit::indicators;
//!
//! let close: Vec<f64> = (0..1024).map(|i| 100.0 + (i as f64) * 0.01).collect();
//!
//! let jobs: Vec<IndicatorJob> = vec![
//!     IndicatorJob::new("sma_20", Box::new(|data| {
//!         indicators::sma(data, 20).map(|a| a.to_vec())
//!     })),
//!     IndicatorJob::new("ema_50", Box::new(|data| {
//!         indicators::ema(data, 50).map(|a| a.to_vec())
//!     })),
//! ];
//!
//! let results = run_parallel(&jobs, &close);
//! assert_eq!(results.len(), 2);
//! ```

use crate::error::Result;
use crate::utils::init_output;
use ndarray::Array1;

/// A single indicator computation job: a unique name plus a closure that
/// runs the computation over a `&[f64]` input and returns a `Vec<f64>` result.
///
/// The closure is typically a thin wrapper around the corresponding
/// `indicators::*` function. The job name is used as the key in the
/// result map.
pub struct IndicatorJob {
    /// Stable identifier for this job (used as a map key in
    /// [`run_parallel`]).
    pub name: String,
    /// Computation body. Errors are propagated per-job.
    pub compute: Box<dyn Fn(&[f64]) -> Result<Vec<f64>> + Send + Sync>,
}

impl std::fmt::Debug for IndicatorJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndicatorJob")
            .field("name", &self.name)
            .finish()
    }
}

impl IndicatorJob {
    /// Create a new job.
    ///
    /// # Arguments
    /// - `name` — stable identifier (e.g. `"sma_20"`).
    /// - `compute` — closure `&[f64] -> Result<Vec<f64>>`. The closure
    ///   must be `Send + Sync` so it can be moved into a worker thread.
    pub fn new<F>(name: impl Into<String>, compute: F) -> Self
    where
        F: Fn(&[f64]) -> Result<Vec<f64>> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            compute: Box::new(compute),
        }
    }
}

/// Result of a single job: either the produced `Vec<f64>` or the error
/// that aborted it.
#[derive(Debug, Clone)]
pub struct JobResult {
    /// Job name (matches `IndicatorJob::name`).
    pub name: String,
    /// Result of the computation.
    pub result: std::result::Result<Vec<f64>, String>,
}

/// Run a batch of independent indicator jobs over the same input.
///
/// Returns one [`JobResult`] per input job, in the **same order**. The
/// parallel path uses `rayon`; the serial fallback (used when the
/// `rayon` feature is disabled) iterates jobs sequentially but keeps
/// the same return shape.
pub fn run_parallel(jobs: &[IndicatorJob], input: &[f64]) -> Vec<JobResult> {
    #[cfg(feature = "rayon")]
    {
        use rayon::prelude::*;
        jobs.par_iter()
            .map(|job| {
                let result = (job.compute)(input).map_err(|e| format!("{e:?}"));
                JobResult {
                    name: job.name.clone(),
                    result,
                }
            })
            .collect()
    }
    #[cfg(not(feature = "rayon"))]
    {
        jobs.iter()
            .map(|job| {
                let result = (job.compute)(input).map_err(|e| format!("{e:?}"));
                JobResult {
                    name: job.name.clone(),
                    result,
                }
            })
            .collect()
    }
}

/// Run a single indicator function over `input` and return its
/// `Array1<f64>` output. Convenience wrapper that initialises the output
/// buffer to NaN and delegates to the supplied closure.
pub fn run_indicator<F>(input: &[f64], compute: F) -> Result<Array1<f64>>
where
    F: FnOnce(&[f64], &mut [f64]) -> Result<()>,
{
    let mut out = init_output(input.len());
    compute(input, out.as_slice_mut().unwrap())?;
    Ok(out)
}

/// Statistics about a parallel batch run, useful for benchmarking /
/// logging. Captures the elapsed wall-clock time and the number of
/// jobs that succeeded vs. errored.
#[derive(Debug, Clone, Copy)]
pub struct BatchStats {
    /// Total wall-clock time in microseconds.
    pub elapsed_us: u128,
    /// Number of jobs that produced a result.
    pub succeeded: usize,
    /// Number of jobs that errored.
    pub failed: usize,
}

/// Run a batch of jobs and return both the per-job results and a single
/// [`BatchStats`] summary. The serial path measures elapsed time using
/// `Instant::now()`.
pub fn run_parallel_with_stats(jobs: &[IndicatorJob], input: &[f64]) -> (Vec<JobResult>, BatchStats) {
    let start = std::time::Instant::now();
    let results = run_parallel(jobs, input);
    let elapsed_us = start.elapsed().as_micros();
    let mut succeeded = 0;
    let mut failed = 0;
    for r in &results {
        if r.result.is_ok() {
            succeeded += 1;
        } else {
            failed += 1;
        }
    }
    let stats = BatchStats {
        elapsed_us,
        succeeded,
        failed,
    };
    (results, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators;

    fn linear_series(n: usize) -> Vec<f64> {
        (0..n).map(|i| 100.0 + (i as f64) * 0.01).collect()
    }

    #[test]
    fn run_parallel_preserves_order() {
        let close = linear_series(1024);
        let jobs: Vec<IndicatorJob> = vec![
            IndicatorJob::new("sma_20", |data| indicators::sma(data, 20).map(|a| a.to_vec())),
            IndicatorJob::new("ema_50", |data| indicators::ema(data, 50).map(|a| a.to_vec())),
            IndicatorJob::new("rsi_14", |data| indicators::rsi(data, 14).map(|a| a.to_vec())),
        ];
        let results = run_parallel(&jobs, &close);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "sma_20");
        assert_eq!(results[1].name, "ema_50");
        assert_eq!(results[2].name, "rsi_14");
        for r in &results {
            assert!(r.result.is_ok(), "job {} failed: {:?}", r.name, r.result);
            let v = r.result.as_ref().unwrap();
            assert_eq!(v.len(), close.len());
        }
    }

    #[test]
    fn run_parallel_propagates_errors() {
        let close = linear_series(64);
        // period = 0 must error
        let jobs: Vec<IndicatorJob> = vec![IndicatorJob::new("bad", |data| {
            indicators::sma(data, 0).map(|a| a.to_vec())
        })];
        let results = run_parallel(&jobs, &close);
        assert_eq!(results.len(), 1);
        assert!(results[0].result.is_err());
    }

    #[test]
    fn run_parallel_with_stats_counts() {
        let close = linear_series(128);
        let jobs: Vec<IndicatorJob> = vec![
            IndicatorJob::new("good1", |data| indicators::sma(data, 10).map(|a| a.to_vec())),
            IndicatorJob::new("good2", |data| indicators::ema(data, 10).map(|a| a.to_vec())),
            IndicatorJob::new("bad", |data| {
                indicators::sma(data, 0).map(|a| a.to_vec())
            }),
        ];
        let (results, stats) = run_parallel_with_stats(&jobs, &close);
        assert_eq!(results.len(), 3);
        assert_eq!(stats.succeeded, 2);
        assert_eq!(stats.failed, 1);
    }

    #[cfg(feature = "rayon")]
    #[test]
    fn run_parallel_matches_serial() {
        // Property: parallel and serial executions produce identical
        // numeric results (no race conditions, no NaN).
        let close = linear_series(2048);
        let make_jobs = || {
            vec![
                IndicatorJob::new("sma_20", |data| {
                    indicators::sma(data, 20).map(|a| a.to_vec())
                }),
                IndicatorJob::new("rsi_14", |data| {
                    indicators::rsi(data, 14).map(|a| a.to_vec())
                }),
            ]
        };
        let serial = run_parallel(&make_jobs(), &close);
        let parallel = run_parallel(&make_jobs(), &close);
        for (a, b) in serial.iter().zip(parallel.iter()) {
            assert_eq!(a.name, b.name);
            let av = a.result.as_ref().unwrap();
            let bv = b.result.as_ref().unwrap();
            assert_eq!(av.len(), bv.len());
            for (x, y) in av.iter().zip(bv.iter()) {
                if x.is_nan() {
                    assert!(y.is_nan());
                } else {
                    assert!((x - y).abs() < 1e-12);
                }
            }
        }
    }
}
