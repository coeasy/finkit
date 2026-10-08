//! SIMD kernels and their dispatch layer.
//!
//! Split into `dispatch` (public `simd_*` wrappers), `avx2_*` (`x86_64`
//! kernels) and `scalar_*` (portable fallbacks). The module path and the
//! public surface are unchanged; the shared imports and `no_std` shims
//! stay here and reach the kernels through `prelude`.

#![allow(unused_unsafe)]
#![allow(unsafe_op_in_unsafe_fn)]

//! Batch indicator primitives.
//!
//! Most `pub fn simd_*` functions provide a runtime-dispatched fast path
//! (AVX-512 → AVX2 → scalar on x86_64, `simd128` on wasm32). Some are
//! scalar-only reference kernels — where that is the case the function's own
//! doc comment says so explicitly; the `simd_` prefix is this module's naming
//! convention, not a guarantee about every function. Functions operate on
//! `&[f64]` slices and write results into a caller-provided `&mut [f64]` buffer.
//!
//! ## no_std support
//!
//! In `no_std` mode, only scalar fallback functions are available.

#[cfg(not(feature = "std"))]
use libm::{ceil as libm_ceil, cos, floor as libm_floor, log, sin, sqrt};

#[cfg(not(feature = "std"))]
#[inline]
fn f64_sqrt(x: f64) -> f64 {
    sqrt(x)
}

#[cfg(not(feature = "std"))]
#[inline]
fn f64_ln(x: f64) -> f64 {
    log(x)
}

// `f64::floor` / `f64::ceil` are `std`-only inherent methods; the scalar tails of
// the rounding kernels need a `no_std` equivalent so the fallback compiles
// without `std`. `libm` implements the same IEEE operation.
#[cfg(not(feature = "std"))]
#[inline]
fn f64_floor(x: f64) -> f64 {
    libm_floor(x)
}

#[cfg(not(feature = "std"))]
#[inline]
fn f64_ceil(x: f64) -> f64 {
    libm_ceil(x)
}

#[cfg(feature = "std")]
#[inline]
fn f64_floor(x: f64) -> f64 {
    x.floor()
}

#[cfg(feature = "std")]
#[inline]
fn f64_ceil(x: f64) -> f64 {
    x.ceil()
}

// B1: `sin_cos` is a `std`-only `f64` method; provide a `no_std` equivalent
// via `libm` so the scalar fallback compiles without `std`.
#[cfg(not(feature = "std"))]
#[inline]
fn f64_sin_cos(x: f64) -> (f64, f64) {
    (sin(x), cos(x))
}

#[cfg(feature = "std")]
#[inline]
fn f64_sin_cos(x: f64) -> (f64, f64) {
    x.sin_cos()
}

#[cfg(feature = "std")]
#[inline]
fn f64_sqrt(x: f64) -> f64 {
    x.sqrt()
}

#[cfg(feature = "std")]
#[inline]
fn f64_ln(x: f64) -> f64 {
    x.ln()
}

mod avx2_core;
mod avx2_diff;
mod avx2_hilbert;
mod avx2_mom;
mod avx2_stats;
mod capability;
mod dispatch;
pub(crate) mod prelude;
mod scalar_core;
mod scalar_mom;
mod scalar_stats;
#[cfg(all(feature = "std", test))]
mod tests;

// Crate-internal facade: every moved item is at least `pub(crate)`,
// so a glob re-export keeps sibling buckets resolving exactly as they
// did when they shared one file.
pub(crate) use avx2_core::*;
pub(crate) use avx2_diff::*;
pub(crate) use avx2_hilbert::*;
pub(crate) use avx2_mom::*;
pub(crate) use avx2_stats::*;
pub(crate) use scalar_core::*;
pub(crate) use scalar_mom::*;
pub(crate) use scalar_stats::*;

// The public surface is re-exported explicitly rather than by glob, so
// splitting the file cannot accidentally publish (or hide) a name.
pub use capability::has_avx2;
pub use capability::has_fma;
pub use capability::has_sse41;
pub use capability::simd_capability_flags;
pub use dispatch::simd_ad_line;
pub use dispatch::simd_aroon;
pub use dispatch::simd_atr;
pub use dispatch::simd_avgprice;
pub use dispatch::simd_beta;
pub use dispatch::simd_bop;
pub use dispatch::simd_bp_tr;
pub use dispatch::simd_ceil;
pub use dispatch::simd_clamp;
pub use dispatch::simd_cmo;
pub use dispatch::simd_correl;
pub use dispatch::simd_count_less;
pub use dispatch::simd_cumsum;
pub use dispatch::simd_diff;
pub use dispatch::simd_diff_sum;
pub use dispatch::simd_dual_diff_init;
pub use dispatch::simd_dual_max_init;
pub use dispatch::simd_ema_next;
pub use dispatch::simd_first_non_finite;
pub use dispatch::simd_floor;
pub use dispatch::simd_ht_components;
pub use dispatch::simd_ht_dcphase;
pub use dispatch::simd_ht_detrender;
pub use dispatch::simd_ht_smooth;
pub use dispatch::simd_kama;
pub use dispatch::simd_linreg;
pub use dispatch::simd_linreg_angle;
pub use dispatch::simd_linreg_slope;
pub use dispatch::simd_log_return;
pub use dispatch::simd_mama_hilbert;
pub use dispatch::simd_max_diff_sum;
pub use dispatch::simd_median_price;
pub use dispatch::simd_mom;
pub use dispatch::simd_mom10;
pub use dispatch::simd_obv;
pub use dispatch::simd_pct_change;
pub use dispatch::simd_prefix_sum;
pub use dispatch::simd_roc;
pub use dispatch::simd_sar_step;
pub use dispatch::simd_scale;
pub use dispatch::simd_shift;
pub use dispatch::simd_sin_cos;
pub use dispatch::simd_sma;
pub use dispatch::simd_sqrt;
pub use dispatch::simd_sqrt_checked;
pub use dispatch::simd_stddev;
pub use dispatch::simd_t3;
pub use dispatch::simd_true_range;
pub use dispatch::simd_typical_price;
pub use dispatch::simd_variance;
pub use dispatch::simd_weighted_sum;
pub use dispatch::simd_wma;
pub use dispatch::simd_zscore;
pub use dispatch::simd_zscore_optimized;
