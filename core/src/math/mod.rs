//! Mathematical utility functions.
//!
//! Provides moving average implementations, statistical functions, and
//! linear algebra helpers used by the indicator modules.
//!
//! # Submodules
//!
//! - [`simd_kernels`](crate::math::simd_kernels) — SIMD-accelerated batch indicator kernels (SMA, EMA, RSI, MACD)
//! - [`simd_ops`](crate::math::simd_ops) — SIMD primitives (prefix sum, diff, scale, etc.)
//!
//! ## std-only submodules
//!
//! - [`cci`](crate::math::cci) — TA-Lib 0.7.1-compatible Commodity Channel Index kernel (requires `std` feature)
//! - [`directional`](crate::math::directional) — single-output +DI/-DI Wilder kernels (requires `std` feature)
//! - [`mfi`](crate::math::mfi) — fused Money Flow Index kernel without a full typical-price scratch array (requires `std` feature)
//! - [`moving_avg`](crate::math::moving_avg) — canonical moving-average namespace with Architecture v3 hot-kernel overrides (requires `std` feature)
//! - [`ohlc_family_state`](crate::math::ohlc_family_state) — shared TR/ATR/DM/DI/DX/ADX Wilder state (requires `std` feature)
//! - [`statistics`](crate::math::statistics) — Rolling variance, standard deviation, min, max, correlation (requires `std` feature)
//! - [`rolling_stats`](crate::math::rolling_stats) — TA-Lib 0.7.1-compatible rolling statistics (requires `std` feature)
//! - [`sar`](crate::math::sar) — TA-Lib 0.7.1-compatible Parabolic SAR kernel (requires `std` feature)
//! - [`trange`](crate::math::trange) — single-write TA-Lib-compatible True Range kernel (requires `std` feature)
//! - [`linear`](crate::math::linear) — Linear regression and related functions (requires `std` feature)
//! - [`reduction`](crate::math::reduction) — allocation-free typed scalar reductions for f32/f64 (requires `std` feature)
//! - [`typed_moving_avg`](crate::math::typed_moving_avg) — native f32 SMA/EMA caller-owned kernels (requires `std` feature)
//! - [`volume_kernels`](crate::math::volume_kernels) — caller-owned OBV/VWAP output kernels (requires `std` feature)

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

/// `A % B` for the formula `%` operator: the **floor-based** remainder.
///
/// Rust's `%` truncates toward zero, so `-7.0 % 3.0` is `-1.0`. The dialect
/// contract is the floor form — `-7 % 3` is `2.0` — because the remainder is
/// defined as `a - floor(a / b) * b`. The two disagree in sign whenever the
/// operands have opposite signs, and a truncating implementation returns a
/// plausible number rather than a visible gap, so nothing downstream notices.
///
/// A divisor near zero has no usable remainder, so the result is `NaN`; that
/// also keeps the operation total for the streaming path, which has no
/// per-row error channel.
///
/// Every implementation of the operator must call this — the tree scalar and
/// array kernels, the bytecode VM, the JIT runtime and its bytecode folder, the
/// plan's `BINARY:Mod` kernel, the SIMD fallbacks, and the AST constant folder.
/// Four of those had drifted to Rust's truncating `%`, and the constant folder
/// was the worst of them: folding a literal *changed the value*, so `-7 % 3`
/// answered `-1` while the equivalent `CLOSE % 3` answered the floor form on the
/// very same bar. The differential harness could not see it because its data is
/// strictly positive and it never probed `%` with a negative operand.
///
/// `MOD(A, B)` the *function* is deliberately the truncating remainder and must
/// NOT call this; see `unified_dispatch`'s `MOD`-vs-`%` note.
#[inline]
pub(crate) fn floor_remainder(dividend: f64, divisor: f64) -> f64 {
    if divisor.abs() < 1e-15 {
        f64::NAN
    } else {
        dividend - (dividend / divisor).floor() * divisor
    }
}

/// The tolerance behind the formula `==` and `!=` operators: `1e-10`.
///
/// The comparison is deliberately not exact. Operands are the result of chained
/// floating-point arithmetic, so an exact comparison makes two expressions that
/// are mathematically equal — but reached by different routes — compare
/// unequal. `1e-10` is the established value; it is stated once here so the
/// kernels cannot drift.
pub(crate) const EQUALITY_TOLERANCE: f64 = 1e-10;

/// `A == B` for the formula `==` operator: `|A - B| < `[`EQUALITY_TOLERANCE`].
///
/// # `NaN`
///
/// A `NaN` operand is neither nearly equal nor nearly unequal: `|NaN - x|` is
/// `NaN`, and both `<` and `>=` are false against it. So `Eq` and `Neq` both
/// answer `0.0` when either side is missing. That is the contract every path
/// already implements, and it is preserved here rather than "fixed" — making
/// `Neq` the negation of `Eq` would change every path's answer for a missing
/// value, which is a semantic decision, not a consistency cleanup.
#[inline]
pub(crate) fn nearly_equal(lhs: f64, rhs: f64) -> bool {
    (lhs - rhs).abs() < EQUALITY_TOLERANCE
}

/// `A != B` for the formula `!=` operator: `|A - B| >= `[`EQUALITY_TOLERANCE`].
///
/// Not `!`[`nearly_equal`] — see the `NaN` note there.
#[inline]
pub(crate) fn nearly_not_equal(lhs: f64, rhs: f64) -> bool {
    (lhs - rhs).abs() >= EQUALITY_TOLERANCE
}

/// Centred second moments of two equal-length series: the sum of squared
/// deviations of each series and the sum of their cross-products.
///
/// Returns `(sum_dx_dy, sum_dx_dx, sum_dy_dy)` with `dx = x - mean(x)`.
///
/// # Why two passes
///
/// The textbook one-pass form `sum(x*y) - sum(x)*sum(y)/n` subtracts two large,
/// nearly equal totals. When the mean is large compared with the spread — a
/// price series at `1e9`, a feature that is a large constant plus a small
/// signal — the subtraction loses every significant digit. Measured: a series
/// and its exact affine image `2x + 3` are perfectly correlated, yet the
/// one-pass form reported `r = 0.667` at a `1e9` baseline (and `1.0` at a small
/// baseline, which is why the ordinary-data tests never caught it).
///
/// Subtracting the mean *first* and accumulating the deviations keeps every
/// addend at the scale of the spread, so the relative error stays at the level
/// of the individual subtraction rather than of the baseline. Every caller that
/// needs a variance, covariance, correlation or z-score must come through here,
/// so the crate has one moment formula instead of one per spelling.
///
/// # Panics
///
/// Panics in debug builds if the two series differ in length; callers validate
/// that up front because they also decide the error to report.
#[inline]
pub(crate) fn centred_moments(x: &[f64], y: &[f64]) -> (f64, f64, f64) {
    debug_assert_eq!(
        x.len(),
        y.len(),
        "centred_moments requires equal-length series"
    );
    if x.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let inv_n = 1.0 / x.len() as f64;
    let mean_x = x.iter().sum::<f64>() * inv_n;
    let mean_y = y.iter().sum::<f64>() * inv_n;
    let mut cross = 0.0;
    let mut sum_dx_dx = 0.0;
    let mut sum_dy_dy = 0.0;
    for (xi, yi) in x.iter().zip(y.iter()) {
        let dx = xi - mean_x;
        let dy = yi - mean_y;
        cross += dx * dy;
        sum_dx_dx += dx * dx;
        sum_dy_dy += dy * dy;
    }
    (cross, sum_dx_dx, sum_dy_dy)
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
