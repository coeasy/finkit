//! Mathematical utility functions.
//!
//! Provides moving average implementations, statistical functions, and
//! linear algebra helpers used by the indicator modules.
//!
//! # Submodules
//!
//! - [`simd_kernels`] — SIMD-accelerated batch indicator kernels (SMA, EMA, RSI, MACD)
//! - [`simd_ops`] — SIMD primitives (prefix sum, diff, scale, etc.)
//!
//! ## std-only submodules
//!
//! - [`cci`] — TA-Lib 0.7.1-compatible Commodity Channel Index kernel (requires `std` feature)
//! - [`mfi`] — fused Money Flow Index kernel without a full typical-price scratch array (requires `std` feature)
//! - [`moving_avg`] — SMA, EMA, WMA, DEMA, TEMA, KAMA, T3, TRIMA, HMA, ALMA, MAVP (requires `std` feature)
//! - [`statistics`] — Rolling variance, standard deviation, min, max, correlation (requires `std` feature)
//! - [`rolling_stats`] — TA-Lib 0.7.1-compatible rolling statistics (requires `std` feature)
//! - [`sar`] — TA-Lib 0.7.1-compatible Parabolic SAR kernel (requires `std` feature)
//! - [`linear`] — Linear regression and related functions (requires `std` feature)
//! - [`reduction`] — allocation-free typed scalar reductions for f32/f64 (requires `std` feature)
//! - [`typed_moving_avg`] — native f32 SMA/EMA caller-owned kernels (requires `std` feature)
//! - [`volume_kernels`] — caller-owned OBV/VWAP output kernels (requires `std` feature)

#[cfg(feature = "std")]
pub mod cci;
#[cfg(feature = "std")]
pub mod linear;
#[cfg(feature = "std")]
pub mod mfi;
#[cfg(feature = "std")]
pub mod moving_avg;
// B1: `libm_shim` is the `no_std`-portable home for the float primitives used
// by the isolated numeric helpers. It is compiled in both `std` and `no_std`
// builds (its `FloatExt`/`f64_*` helpers route to `core`/`libm` accordingly).
pub mod libm_shim;
#[cfg(feature = "std")]
pub mod reduction;
#[cfg(feature = "std")]
pub mod rolling_stats;
#[cfg(feature = "std")]
pub mod sar;
pub mod simd_kernels;
pub mod simd_ops;
#[cfg(feature = "std")]
pub mod simd_ops_avx512;
pub mod simd_ops_wasm;
#[cfg(feature = "std")]
pub mod statistics;
#[cfg(feature = "std")]
pub mod typed_moving_avg;
#[cfg(feature = "std")]
pub mod volume_kernels;
