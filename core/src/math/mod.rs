//! Mathematical utility functions.
//!
//! Provides moving average implementations, statistical functions, and
//! linear algebra helpers used by the indicator modules.

#[cfg(feature = "std")]
pub mod information;
pub mod libm_shim;
#[cfg(feature = "std")]
pub mod linear;
#[cfg(feature = "std")]
pub mod moving_avg;
#[cfg(feature = "std")]
pub mod quantile;
#[cfg(feature = "std")]
pub mod rank;
#[cfg(feature = "std")]
pub mod regression;
#[cfg(feature = "std")]
pub mod segmented;
pub mod simd_kernels;
pub mod simd_ops;
#[cfg(feature = "std")]
pub mod simd_ops_avx512;
pub mod simd_ops_wasm;
#[cfg(feature = "std")]
pub mod statistics;

/// Architecture V4 canonical execution kernels.
pub mod kernels;