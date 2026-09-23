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
//! - [`directional`] — single-output +DI/-DI Wilder kernels (requires `std` feature)
//! - [`mfi`] — fused Money Flow Index kernel without a full typical-price scratch array (requires `std` feature)
//! - [`moving_avg`] — canonical moving-average namespace with Architecture v3 hot-kernel overrides (requires `std` feature)
//! - [`ohlc_family_state`] — shared TR/ATR/DM/DI/DX/ADX Wilder state (requires `std` feature)
//! - [`statistics`] — Rolling variance, standard deviation, min, max, correlation (requires `std` feature)
//! - [`rolling_stats`] — TA-Lib 0.7.1-compatible rolling statistics (requires `std` feature)
//! - [`sar`] — TA-Lib 0.7.1-compatible Parabolic SAR kernel (requires `std` feature)
//! - [`trange`] — single-write TA-Lib-compatible True Range kernel (requires `std` feature)
//! - [`linear`] — Linear regression and related functions (requires `std` feature)
//! - [`reduction`] — allocation-free typed scalar reductions for f32/f64 (requires `std` feature)
//! - [`typed_moving_avg`] — native f32 SMA/EMA caller-owned kernels (requires `std` feature)
//! - [`volume_kernels`] — caller-owned OBV/VWAP output kernels (requires `std` feature)

/// Length of the leading non-finite (warm-up) run of a series, i.e. the index
/// of the first finite value. Returns `input.len()` when every value is
/// non-finite.
///
/// Every rolling indicator in this crate marks its warm-up region with a leading
/// run of `NaN`. That run is structural, not data, so composing two indicators
/// (`EMA(EMA(x, 5), 9)`) must start the second kernel *after* it rather than
/// treating it as bad input — otherwise the accumulator is seeded from `NaN`
/// and the whole output is `NaN` (`NaN - NaN` is still `NaN`).
#[inline]
pub(crate) fn leading_warmup(input: &[f64]) -> usize {
    input
        .iter()
        .position(|value| value.is_finite())
        .unwrap_or(input.len())
}

/// Safety factor applied to the floating-point noise floor in
/// [`degenerate_variance`].
///
/// The floor is an order-of-magnitude estimate, not a bound, so the test needs
/// headroom above it. 64 is ~75x the observed residue for a constant 60-bar
/// window at a price scale of 100, and still eight orders of magnitude below
/// the relative variance of any window carrying real signal.
pub(crate) const DEGENERATE_VARIANCE_SAFETY: f64 = 64.0;

/// Is a rolling variance indistinguishable from zero?
///
/// Several kernels compute a window's variance in the cheap, incrementally
/// updatable form `sum_sq - sum * sum / n`. That form subtracts two large,
/// nearly equal quantities, so for a window that is *exactly* constant it does
/// not return `0.0` — it returns a residue of order `n * eps * mean(x^2)`.
///
/// That residue is why an absolute threshold cannot work. Measured on the
/// Alpha158 reference market (a constant 20-bar window at a price of ~105.79),
/// the residue is `2.9e-11`; at 60 bars it is `7.0e-10`. Both are twelve orders
/// of magnitude above the `1e-15` that `rolling_correlation_into` used to test
/// against, so its degeneracy guard never fired at realistic price levels and
/// `CORREL` returned a correlation computed from a window with no variance at
/// all. The bug is invisible at `O(1)` inputs, which is presumably why it
/// survived: at unit scale the residue really is below `1e-15`.
///
/// The test is therefore expressed against the noise floor itself, which scales
/// with both the magnitude of the data and the window length, so it holds at
/// any price scale and any window size:
///
/// ```text
/// variance <= 64 * n * eps * mean(x^2)   =>  degenerate
/// ```
///
/// `variance` and `sum_sq` must come from the same window, and `size` is the
/// window length. A window carrying real signal has a relative variance many
/// orders of magnitude above this floor, so the predicate is not a "small
/// variance" filter — it only fires when the variance is numerically absent.
#[inline]
pub(crate) fn degenerate_variance(variance: f64, sum_sq: f64, size: f64) -> bool {
    // A non-finite variance means the window was not fully finite; report it as
    // degenerate so the caller leaves the output as `NaN`.
    if !variance.is_finite() {
        return true;
    }
    let mean_square = (sum_sq / size).abs();
    variance.abs() <= DEGENERATE_VARIANCE_SAFETY * size * f64::EPSILON * mean_square
}

/// `SIGN(X)` for a single value: `-1.0`, `0.0` or `1.0`.
///
/// Deliberately not [`f64::signum`], which maps both `0.0` and `-0.0` to `1.0`
/// and `-1.0` respectively. The three-way form is what the dialect contracts
/// describe and what the `WorldQuant` alphas need: `Alpha7` computes
/// `... * sign(close - ref(close, 7))` to re-attach the direction of a 7-bar
/// move, and on a flat 7-bar stretch that difference is exactly `0.0`. Under
/// `signum` the factor would acquire a direction that the data does not have —
/// and because the wrong branch is `+1`, not `NaN`, nothing downstream notices.
///
/// `NaN` and both signed zeros are passed through unchanged, so a missing value
/// stays missing rather than becoming a direction.
///
/// This lives here rather than in `formula::functions` because the formula
/// table is not the only implementation of `SIGN`: the stateful streaming path
/// (`formula::stateful::apply_stateful_unary_function`) evaluates the same
/// operator one value at a time. Both call this function, so the two cannot
/// drift apart again — they did once, and a differential test could not see it
/// because the probe data contained no exact zero.
#[inline]
pub(crate) fn three_way_sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

#[cfg(feature = "std")]
pub mod cci;
#[cfg(feature = "std")]
pub mod directional;
#[cfg(feature = "std")]
pub mod fast_moving_avg;
#[cfg(feature = "std")]
pub mod information;
#[cfg(feature = "std")]
pub mod linear;
#[cfg(feature = "std")]
pub mod mfi;
#[cfg(feature = "std")]
#[path = "moving_avg.rs"]
#[allow(dead_code)]
mod moving_avg_legacy;
/// Canonical moving-average API.
///
/// The complete established implementation remains the source for the broad
/// moving-average surface. Architecture v3 overrides only the proven hot
/// entry points, so callers keep the same module path and signatures.
#[cfg(feature = "std")]
pub mod moving_avg {
    pub use super::fast_moving_avg::{ema_into, kama, kama_into, wma_into};
    pub use super::moving_avg_legacy::*;
}
// B1: `libm_shim` is the `no_std`-portable home for the float primitives used
// by the isolated numeric helpers. It is compiled in both `std` and `no_std`
// builds (its `FloatExt`/`f64_*` helpers route to `core`/`libm` accordingly).
/// Canonical Architecture V4 execution kernels.
pub mod kernels;
pub mod libm_shim;
#[cfg(feature = "std")]
pub mod ohlc_family_state;
#[cfg(feature = "std")]
pub mod quantile;
#[cfg(feature = "std")]
pub mod rank;
#[cfg(feature = "std")]
pub mod reduction;
#[cfg(feature = "std")]
pub mod regression;
#[cfg(feature = "std")]
pub mod rolling_stats;
#[cfg(feature = "std")]
pub mod sar;
#[cfg(feature = "std")]
pub mod segmented;
pub mod simd_kernels;
pub mod simd_ops;
#[cfg(feature = "std")]
pub mod simd_ops_avx512;
pub mod simd_ops_wasm;
#[cfg(feature = "std")]
pub mod statistics;
#[cfg(feature = "std")]
pub mod trange;
#[cfg(feature = "std")]
pub mod typed_moving_avg;
#[cfg(feature = "std")]
pub mod volume_kernels;
