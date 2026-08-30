//! Sweepable trait for parameter-sweep-compatible indicators.

use crate::error::Result;

/// A parameter combination for sweeping.
/// Each variant stores the constructor parameters for different indicator types.
#[derive(Debug, Clone)]
pub enum SweepParams {
    /// Single period (SMA, EMA, RSI, etc.)
    Period(usize),
    /// Two periods (APO, PPO, ADOSC, etc.)
    DualPeriod(usize, usize),
    /// Three periods (MACD, Stoch, etc.)
    TriplePeriod(usize, usize, usize),
}

/// Result of a single sweep run.
#[derive(Debug, Clone)]
pub struct SweepResult {
    pub params: SweepParams,
    pub values: Vec<f64>,
}

/// Trait for indicators that support batch parameter sweeping.
///
/// Implementors provide a `sweep` method that computes the indicator
/// for all parameter combinations in one optimized pass.
///
/// # Example
///
/// ```
/// use finkit::indicators::sweepable::{Sweepable, SweepParams};
///
/// struct SmaSweepable;
/// // See SweepEngine for usage with Cartesian product scans.
/// ```
pub trait Sweepable: Sync + Send {
    /// Compute the indicator for all given parameter sets over the input data.
    /// Returns one result per parameter combination.
    fn sweep(&self, data: &[f64], params: &[SweepParams]) -> Result<Vec<SweepResult>>;

    /// Name of the indicator (for reporting).
    fn name(&self) -> &'static str;
}

/// Built-in SMA sweepable implementation.
pub struct SmaSweepable;

impl Sweepable for SmaSweepable {
    fn sweep(&self, data: &[f64], params: &[SweepParams]) -> Result<Vec<SweepResult>> {
        let periods: Vec<usize> = params
            .iter()
            .map(|p| match p {
                SweepParams::Period(n) => *n,
                SweepParams::DualPeriod(n, _) => *n,
                SweepParams::TriplePeriod(n, _, _) => *n,
            })
            .collect();
        let results = super::sweep::sma_sweep(data, &periods)?;
        Ok(results
            .into_iter()
            .zip(params.iter())
            .map(|(values, p)| SweepResult {
                params: p.clone(),
                values,
            })
            .collect())
    }

    fn name(&self) -> &'static str {
        "SMA"
    }
}

/// Built-in EMA sweepable implementation.
pub struct EmaSweepable;

impl Sweepable for EmaSweepable {
    fn sweep(&self, data: &[f64], params: &[SweepParams]) -> Result<Vec<SweepResult>> {
        let periods: Vec<usize> = params
            .iter()
            .map(|p| match p {
                SweepParams::Period(n) => *n,
                SweepParams::DualPeriod(n, _) => *n,
                SweepParams::TriplePeriod(n, _, _) => *n,
            })
            .collect();
        let results = super::sweep::ema_sweep(data, &periods)?;
        Ok(results
            .into_iter()
            .zip(params.iter())
            .map(|(values, p)| SweepResult {
                params: p.clone(),
                values,
            })
            .collect())
    }

    fn name(&self) -> &'static str {
        "EMA"
    }
}

/// Built-in RSI sweepable implementation.
pub struct RsiSweepable;

impl Sweepable for RsiSweepable {
    fn sweep(&self, data: &[f64], params: &[SweepParams]) -> Result<Vec<SweepResult>> {
        let periods: Vec<usize> = params
            .iter()
            .map(|p| match p {
                SweepParams::Period(n) => *n,
                SweepParams::DualPeriod(n, _) => *n,
                SweepParams::TriplePeriod(n, _, _) => *n,
            })
            .collect();
        let results = super::sweep::rsi_sweep(data, &periods)?;
        Ok(results
            .into_iter()
            .zip(params.iter())
            .map(|(values, p)| SweepResult {
                params: p.clone(),
                values,
            })
            .collect())
    }

    fn name(&self) -> &'static str {
        "RSI"
    }
}
