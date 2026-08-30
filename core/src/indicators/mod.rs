//! Technical analysis indicators.
//!
//! This module provides 150+ batch-mode indicator functions organized
//! by category: overlap studies, momentum, volume, volatility, cycle,
//! price transforms, statistics, candlestick patterns, and chart patterns.
//!
//! All indicator functions accept `&[f64]` input slices and return
//! `Result<Array1<f64>>` or structured result types.
//!
//! # Example
//!
//! ```
//! use alpha_ta_core::indicators;
//!
//! let close = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
//! let sma = indicators::sma(&close, 3).unwrap();
//! assert_eq!(sma.len(), 10);
//! ```

/// Fast indexing macro for hot-path indicator loops.
///
/// When the `unchecked-indexing` feature is enabled, uses
/// `get_unchecked` to skip bounds checks. Otherwise falls back to
/// normal indexing. Callers must guarantee that `$i` is in bounds
/// when the feature is active.
#[macro_export]
macro_rules! idx {
    ($slice:expr, $i:expr) => {
        if cfg!(feature = "unchecked-indexing") {
            unsafe { *$slice.get_unchecked($i) }
        } else {
            $slice[$i]
        }
    };
}

/// Mutable version of [`idx!`] for writing to slices.
#[macro_export]
macro_rules! idx_mut {
    ($slice:expr, $i:expr) => {
        if cfg!(feature = "unchecked-indexing") {
            unsafe { *$slice.get_unchecked_mut($i) }
        } else {
            $slice[$i]
        }
    };
}

// ── Indicator modules, gated by opt-in category features ──────────────────
// Every category is enabled by default (see `indicators-all` in Cargo.toml),
// so the public API is unchanged unless a consumer explicitly turns a
// category off. This is the incremental scaffold for tree-shaking the core
// crate: individual categories can be disabled once their cross-module
// dependencies are verified.
#[cfg(feature = "indicators-market")]
pub mod breadth;
#[cfg(feature = "indicators-market")]
pub mod astock;
#[cfg(feature = "indicators-patterns")]
pub mod chart;
#[cfg(feature = "indicators-patterns")]
pub mod classic_patterns;
#[cfg(feature = "indicators-patterns")]
pub mod classic_tools;
#[cfg(feature = "indicators-market")]
pub mod china;
#[cfg(feature = "indicators-market")]
pub mod consolidation;
#[cfg(feature = "indicators-cycle")]
pub mod cycle;
#[cfg(feature = "indicators-market")]
pub mod donchian;
#[cfg(feature = "indicators-market")]
pub mod fibonacci;
#[cfg(feature = "indicators-market")]
pub mod ichimoku;
#[cfg(feature = "indicators-price-transform")]
pub mod math_operators;
#[cfg(feature = "indicators-price-transform")]
pub mod math_transform;
#[cfg(feature = "indicators-momentum")]
pub mod momentum;
#[cfg(feature = "indicators-momentum")]
pub mod momentum_ext;
#[cfg(feature = "indicators-overlap")]
pub mod overlap;
#[cfg(feature = "indicators-market")]
pub mod parallel;
#[cfg(feature = "indicators-market")]
pub mod pivot;
#[cfg(feature = "indicators-price-transform")]
pub mod price_transform;
#[cfg(feature = "indicators-market")]
pub mod relative_strength;
#[cfg(feature = "indicators-market")]
pub mod sentiment;
#[cfg(feature = "indicators-market")]
pub mod short_term;
#[cfg(feature = "indicators-market")]
pub mod smc;
#[cfg(feature = "indicators-statistics")]
pub mod statistics;
#[cfg(all(feature = "indicators-market", feature = "indicators-all"))]
pub mod supertrend;
#[cfg(feature = "indicators-market")]
pub mod top_bottom;
#[cfg(feature = "indicators-volatility")]
pub mod volatility;
#[cfg(all(feature = "indicators-volatility", feature = "indicators-all"))]
pub mod volatility_ext;
#[cfg(feature = "indicators-market")]
pub mod sweep;
#[cfg(feature = "indicators-market")]
pub mod sweep_engine;
#[cfg(feature = "indicators-market")]
pub mod sweepable;
#[cfg(feature = "indicators-volume")]
pub mod volume;
#[cfg(feature = "indicators-volume")]
pub mod volume_ext;
#[cfg(feature = "indicators-volume")]
pub mod volume_profile;

#[cfg(feature = "indicators-market")]
pub use astock::*;
#[cfg(feature = "indicators-market")]
pub use breadth::*;
#[cfg(feature = "indicators-patterns")]
pub use chart::*;
#[cfg(feature = "indicators-market")]
pub use consolidation::*;
#[cfg(feature = "indicators-market")]
pub use relative_strength::*;
#[cfg(feature = "indicators-market")]
pub use short_term::*;
#[cfg(feature = "indicators-patterns")]
pub use classic_patterns::*;
#[cfg(feature = "indicators-patterns")]
pub use classic_tools::*;
#[cfg(feature = "indicators-market")]
pub use china::*;
#[cfg(feature = "indicators-cycle")]
pub use cycle::*;
#[cfg(feature = "indicators-market")]
pub use donchian::*;
#[cfg(feature = "indicators-market")]
pub use fibonacci::*;
#[cfg(feature = "indicators-market")]
pub use ichimoku::*;
#[cfg(feature = "indicators-price-transform")]
pub use math_operators::*;
#[cfg(feature = "indicators-price-transform")]
pub use math_transform::*;
#[cfg(feature = "indicators-momentum")]
pub use momentum::*;
#[cfg(feature = "indicators-momentum")]
pub use momentum_ext::*;
#[cfg(feature = "indicators-overlap")]
pub use overlap::*;
#[cfg(feature = "indicators-market")]
pub use pivot::*;
#[cfg(feature = "indicators-price-transform")]
pub use price_transform::*;
#[cfg(feature = "indicators-market")]
pub use sentiment::*;
#[cfg(feature = "indicators-market")]
pub use smc::*;
#[cfg(feature = "indicators-statistics")]
pub use statistics::*;
#[cfg(all(feature = "indicators-market", feature = "indicators-all"))]
pub use supertrend::*;
#[cfg(feature = "indicators-volatility")]
pub use volatility::*;
#[cfg(all(feature = "indicators-volatility", feature = "indicators-all"))]
pub use volatility_ext::*;
#[cfg(feature = "indicators-market")]
pub use sweep::*;
#[cfg(feature = "indicators-market")]
pub use sweep_engine::*;
#[cfg(feature = "indicators-market")]
pub use sweepable::*;
#[cfg(feature = "indicators-market")]
pub use top_bottom::*;
#[cfg(feature = "indicators-volume")]
pub use volume::*;
#[cfg(feature = "indicators-volume")]
pub use volume_ext::*;
#[cfg(feature = "indicators-volume")]
pub use volume_profile::*;

use crate::error::Result;

/// Zero-allocation output trait for indicators that write into a caller-owned
/// `&mut [f64]` buffer instead of returning an `Array1<f64>`.
///
/// Implementors compute the indicator in a single pass and write the result
/// directly into `output`. Warm-up positions are written as `NaN`.
///
/// # Contract
///
/// - `input.len() == output.len()` must hold; implementations return
///   `TaError::InvalidParameter` otherwise.
/// - The first `warmup` elements of `output` are set to `f64::NAN`.
pub trait SliceOutput {
    /// Compute the indicator from `input` and write results into `output`.
    fn compute_into(&self, input: &[f64], output: &mut [f64]) -> Result<()>;
}

/// SMA adapter for [`SliceOutput`].
///
/// # Examples
///
/// ```
/// use alpha_ta_core::indicators::{SliceOutput, SmaSlice};
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let mut out = vec![0.0; 5];
/// SmaSlice(3).compute_into(&data, &mut out).unwrap();
/// assert!(out[1].is_nan());
/// assert!((out[2] - 2.0).abs() < 1e-10);
/// ```
pub struct SmaSlice(pub usize);

impl SliceOutput for SmaSlice {
    fn compute_into(&self, input: &[f64], output: &mut [f64]) -> Result<()> {
        crate::math::moving_avg::sma_into(input, self.0, output)
    }
}

/// EMA adapter for [`SliceOutput`].
///
/// # Examples
///
/// ```
/// use alpha_ta_core::indicators::{SliceOutput, EmaSlice};
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let mut out = vec![0.0; 5];
/// EmaSlice(3).compute_into(&data, &mut out).unwrap();
/// assert!(out[0].is_nan());
/// assert!((out[2] - 2.0).abs() < 1e-10);
/// ```
pub struct EmaSlice(pub usize);

impl SliceOutput for EmaSlice {
    fn compute_into(&self, input: &[f64], output: &mut [f64]) -> Result<()> {
        crate::math::moving_avg::ema_into(input, self.0, output)
    }
}

/// Generate a [`SliceOutput`] adapter for a single-input batch indicator.
///
/// The adapter stores the indicator's extra parameters as public fields and
/// calls the canonical batch function inside [`SliceOutput::compute_into`],
/// writing the result into the caller-provided buffer (zero per-call
/// allocation). See TASK-301.
macro_rules! impl_slice_output {
    ($Name:ident, $batch:path, ($($field:ident: $t:ty),* $(,)?)) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $Name { $(pub $field: $t),* }
        impl SliceOutput for $Name {
            fn compute_into(&self, input: &[f64], output: &mut [f64]) -> Result<()> {
                let result = $batch(input, $(self.$field),*)?;
                if result.len() != output.len() {
                    return Err(crate::error::TaError::InvalidParameter {
                        name: "output".to_string(),
                        constraint: "must have the same length as input".to_string(),
                    });
                }
                output.copy_from_slice(result.as_slice().unwrap());
                Ok(())
            }
        }
    };
}

impl_slice_output!(RsiSlice, crate::indicators::momentum::rsi, (period: usize));
impl_slice_output!(CmoSlice, crate::indicators::momentum::cmo, (period: usize));
impl_slice_output!(TrixSlice, crate::indicators::momentum::trix, (period: usize));
impl_slice_output!(ApoSlice, crate::indicators::momentum::apo, (fast_period: usize, slow_period: usize));
impl_slice_output!(PpoSlice, crate::indicators::momentum::ppo, (fast_period: usize, slow_period: usize));
impl_slice_output!(RocpSlice, crate::indicators::momentum::rocp, (period: usize));
impl_slice_output!(RocrSlice, crate::indicators::momentum::rocr, (period: usize));
impl_slice_output!(Rocr100Slice, crate::indicators::momentum::rocr100, (period: usize));
impl_slice_output!(TrimaSlice, crate::math::moving_avg::trima, (period: usize));
impl_slice_output!(MomSlice, crate::indicators::momentum::mom, (period: usize));
impl_slice_output!(RocSlice, crate::indicators::momentum::roc, (period: usize));

#[cfg(test)]
mod slice_output_tests {
    use super::*;
    #[test]
    fn test_rsi_slice_adapter_matches_batch() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 3.0, 4.0];
        let expected = crate::indicators::momentum::rsi(&data, 5).unwrap();
        let mut out = vec![0.0; data.len()];
        RsiSlice { period: 5 }.compute_into(&data, &mut out).unwrap();
        for i in 0..data.len() {
            if expected[i].is_nan() {
                assert!(out[i].is_nan(), "nan mismatch at {i}");
            } else {
                assert!((expected[i] - out[i]).abs() < 1e-12, "mismatch at {i}");
            }
        }
    }
    #[test]
    fn test_kama_slice_adapter_matches_batch() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let expected = crate::math::moving_avg::trima(&data, 5).unwrap();
        let mut out = vec![0.0; data.len()];
        TrimaSlice { period: 5 }.compute_into(&data, &mut out).unwrap();
        for i in 0..data.len() {
            if expected[i].is_nan() {
                assert!(out[i].is_nan(), "nan mismatch at {i}");
            } else {
                assert!((expected[i] - out[i]).abs() < 1e-12, "mismatch at {i}");
            }
        }
    }
}

/// Convenience function: compute SMA directly into a pre-allocated slice.
///
/// # Examples
///
/// ```
/// use alpha_ta_core::indicators;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let mut out = vec![0.0; 5];
/// indicators::sma_into_slice(&data, 3, &mut out).unwrap();
/// assert!((out[2] - 2.0).abs() < 1e-10);
/// ```
pub fn sma_into_slice(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    crate::math::moving_avg::sma_into(input, period, output)
}

/// Convenience function: compute EMA directly into a pre-allocated slice.
///
/// # Examples
///
/// ```
/// use alpha_ta_core::indicators;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let mut out = vec![0.0; 5];
/// indicators::ema_into_slice(&data, 3, &mut out).unwrap();
/// assert!((out[2] - 2.0).abs() < 1e-10);
/// ```
pub fn ema_into_slice(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    crate::math::moving_avg::ema_into(input, period, output)
}
