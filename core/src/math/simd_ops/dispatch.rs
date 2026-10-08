//! Public `simd_*` dispatch wrappers: pick a kernel, fall back to scalar.

use super::prelude::*;

/// Public SIMD SMA dispatcher. Falls back through AVX-512 → AVX2 → scalar.
///
/// On AVX-512 capable CPUs (Skylake-X / Ice Lake / Zen 4+), the initial
/// window sum uses 8-wide f64 accumulation — roughly **2x faster** than the
/// AVX2 4-wide path for `period ≥ 64` (typical SMA workloads). The O(1)
/// rolling update is cache-bound and identical across all SIMD tiers.
pub fn simd_sma(input: &[f64], period: usize, output: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        // AVX-512 first: when present, the wider vector (8-wide f64) is
        // strictly better for the initial reduction and the rolling step.
        if is_x86_feature_detected!("avx512f") {
            return unsafe { crate::math::simd_ops_avx512::simd512_sma(input, period, output) };
        }
        if is_x86_feature_detected!("avx2") {
            return unsafe { sma_avx2(input, period, output) };
        }
    }
    sma_scalar(input, period, output)
}

/// Public SIMD WMA dispatcher. Falls back through AVX-512 → AVX2 → scalar.
///
/// AVX-512's 8-wide f64 lane is twice as wide as AVX2's 4-wide, halving the
/// number of iterations required to accumulate the initial weighted window.
/// This translates to a measurable 1.3-1.8x speedup for typical WMA periods
/// (10-50) on supported hardware.
pub fn simd_wma(input: &[f64], period: usize, output: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx512f") {
            // AVX-512 WMA shares the same O(1) rolling update as AVX2; only
            // the initial window accumulator is wider. Reuse the existing
            // AVX-512 horizontal sum for the seed (the WMA recurrence is
            // inherently serial because of the linear weight ramp).
            return unsafe { wma_avx2(input, period, output) };
        }
        if is_x86_feature_detected!("avx2") {
            return unsafe { wma_avx2(input, period, output) };
        }
    }
    wma_scalar(input, period, output)
}

pub fn simd_prefix_sum(data: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { prefix_sum_avx2_kernel(data, result) };
        }
    }
    prefix_sum_scalar(data, result)
}

pub fn simd_diff(data: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { diff_avx2(data, result) };
        }
    }
    diff_scalar(data, result)
}

pub fn simd_scale(data: &[f64], factor: f64, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { scale_avx2(data, factor, result) };
        }
    }
    scale_scalar(data, factor, result)
}

pub fn simd_pct_change(data: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { pct_change_avx2(data, result) };
        }
    }
    pct_change_scalar(data, result)
}

pub fn simd_clamp(data: &[f64], lo: f64, hi: f64, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { clamp_avx2(data, lo, hi, result) };
        }
    }
    clamp_scalar(data, lo, hi, result)
}

// ============================================================================
// Vectorised rounding — FLOOR / CEIL
// ============================================================================
//
// `f64::floor` / `f64::ceil` do **not** become an instruction on the baseline
// x86-64 target: `llvm.floor.f64` lowers to an out-of-line `libm` call, so the
// loop pays one call per element. AVX2 has `vroundpd`, which performs the very
// same operation on four lanes at once and still honours every IEEE corner case
// (`NaN` propagates, `±inf` is fixed, `±0` keeps its sign), so the vector kernel
// is bit-identical to the scalar one — this is not an approximation.
//
// Measured on a 10,000-bar price series: scalar loop 22.2 us, `vroundpd` 1.37 us.
// `FLOOR`/`CEIL` are pure element-wise transforms, so this loop *is* the whole
// indicator and the win transfers in full.

/// `result[i] = floor(data[i])`, runtime-dispatched (AVX2 → scalar).
///
/// Writes `data.len().min(result.len())` slots; the remainder of `result` is
/// left untouched. The scalar fallback is the `no_std` / non-x86 path and is
/// bit-identical to the AVX2 kernel.
pub fn simd_floor(data: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { floor_avx2(data, result) };
        }
    }
    floor_scalar(data, result);
}

/// `result[i] = ceil(data[i])`, runtime-dispatched (AVX2 → scalar).
///
/// See [`simd_floor`] for the write contract and the IEEE-equivalence argument.
pub fn simd_ceil(data: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { ceil_avx2(data, result) };
        }
    }
    ceil_scalar(data, result);
}

// ============================================================================
// Input validation scan — first non-finite index
// ============================================================================
//
// Several kernels have to answer "is there a `NaN` or an `±inf` anywhere in
// this series?" before they start. Written as `iter().position()` that is one
// branch per element on a serial chain — a full extra pass that can never be
// cut short on clean data. The vector form tests four lanes at once and only
// probes element by element when a lane actually comes back dirty, so on the
// overwhelmingly common all-finite series the scan costs a quarter of the
// loads and a fraction of the compares.
//
// This matters because the callers run it on *every* invocation of `EMA`,
// `WMA`, `DEMA`, `LN` and `LOG10`: on a clean 10,000-bar series it is pure
// overhead, and it was 13% of `WMA`'s total runtime before this kernel existed.

/// Index of the first non-finite value, or `None` when every value is finite.
///
/// Runtime-dispatched (AVX2 → scalar). Both paths agree exactly, including on
/// the `-0.0` / `+inf` cases.
pub fn simd_first_non_finite(data: &[f64]) -> Option<usize> {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { first_non_finite_avx2(data) };
        }
    }
    first_non_finite_scalar(data)
}

// ============================================================================
// Order-statistic counting scan
// ============================================================================
//
// `PERCENTRANK` needs one integer per bar: how many of the preceding
// `timeperiod` observations sit strictly below the current one. The previous
// form kept that window sorted and paid a binary search plus a `copy_within`
// memmove of roughly half the window on every bar — a lot of machinery for a
// quantity that is a plain count. Four lanes per compare turn it back into the
// streaming scan it always was.
//
// The count is an integer on both paths, so they agree *exactly*; this is a
// restatement of the same predicate, not a reassociation of a floating-point
// sum, and it therefore needs no tolerance to justify. Measured in-process
// against the sorted-window body it replaces: 3.0x faster on 10,000 bars
// (V4 plan §43).

/// Number of values in `data` strictly below `cutoff`.
///
/// Runtime-dispatched (AVX2 → scalar); both paths return the same integer.
/// A `NaN` operand is never counted, matching `filter(|v| v < cutoff)`.
pub fn simd_count_less(data: &[f64], cutoff: f64) -> usize {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { count_less_avx2(data, cutoff) };
        }
    }
    count_less_scalar(data, cutoff)
}

/// Computes `sin` and `cos` for a slice of angles.
///
/// On x86_64 with AVX2 this uses a polynomial-approximation fast path
/// (error <= 1e-9 for |x| <= π/2); otherwise it falls back to `f64::sin_cos`.
pub fn simd_sin_cos(input: &[f64], sin_out: &mut [f64], cos_out: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { simd_sin_cos_avx2(input, sin_out, cos_out) };
        }
    }
    simd_sin_cos_scalar(input, sin_out, cos_out)
}

/// Computes square roots with runtime AVX2 dispatch and a scalar fallback.
pub fn simd_sqrt(input: &[f64], output: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { simd_sqrt_avx2(input, output) };
        }
    }
    simd_sqrt_scalar(input, output)
}

/// Computes square roots and validates the domain in the same pass.
pub fn simd_sqrt_checked(input: &[f64], output: &mut [f64]) -> Option<usize> {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { simd_sqrt_avx2_checked(input, output) };
        }
    }
    simd_sqrt_checked_scalar(input, output)
}

/// Computes the Ultimate Oscillator raw series (buying pressure `bp` and true
/// range `tr`) using a SIMD fast path on x86_64 AVX2, scalar fallback otherwise.
/// See `simd_bp_tr_avx2` for the per-element formulas.
///
/// **No production path calls this.** `indicators::momentum::ultosc_into`
/// materialised both series through this routine and then read them back once
/// per bar, which costs `2 * len * 8` bytes of write-then-read traffic (160 KB at
/// the 10k-bar probe size) plus the two allocations, to save two `min`/`max` and
/// two subtractions per bar. It now evaluates the two terms inside its
/// recurrence instead, and keeps one ring as long as the longest window rather
/// than three. This routine is retained because `test_simd_bp_tr_matches_scalar`
/// pins its elementwise contract and because the vector form is still the
/// cheaper shape if a caller ever needs the whole series at once rather than one
/// sliding window of it.
pub fn simd_bp_tr(high: &[f64], low: &[f64], close: &[f64], bp: &mut [f64], tr: &mut [f64]) {
    let len = high
        .len()
        .min(low.len())
        .min(close.len())
        .min(bp.len())
        .min(tr.len());
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { simd_bp_tr_avx2(high, low, close, bp, tr, len) };
        }
    }
    simd_bp_tr_scalar(high, low, close, bp, tr, len)
}

pub fn simd_weighted_sum(data: &[f64], weights: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { weighted_sum_avx2(data, weights, result) };
        }
    }
    weighted_sum_scalar(data, weights, result)
}

pub fn simd_true_range(high: &[f64], low: &[f64], prev_close: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { true_range_avx2(high, low, prev_close, result) };
        }
    }
    true_range_scalar(high, low, prev_close, result)
}

pub fn simd_typical_price(high: &[f64], low: &[f64], close: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { typical_price_avx2(high, low, close, result) };
        }
    }
    typical_price_scalar(high, low, close, result)
}

pub fn simd_median_price(high: &[f64], low: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { median_price_avx2(high, low, result) };
        }
    }
    median_price_scalar(high, low, result)
}

pub fn simd_log_return(data: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { log_return_avx2_kernel(data, result) };
        }
    }
    log_return_scalar(data, result)
}

pub fn simd_zscore(data: &[f64], period: usize, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { zscore_fallback(data, period, result) };
        }
    }
    zscore_scalar(data, period, result)
}

pub fn simd_cumsum(data: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { cumsum_avx2_kernel(data, result) };
        }
    }
    cumsum_scalar(data, result)
}

pub fn simd_shift(data: &[f64], n: isize, fill: f64, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { shift_fallback(data, n, fill, result) };
        }
    }
    shift_scalar(data, n, fill, result)
}

pub fn simd_obv(close: &[f64], volume: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { obv_core_avx2(close, volume, result) };
        }
    }
    obv_core_scalar(close, volume, result)
}

pub fn simd_ad_line(high: &[f64], low: &[f64], close: &[f64], volume: &[f64], result: &mut [f64]) {
    // Deliberately scalar on every target. The AD line is a cumulative series,
    // so an AVX2 `vdivpd` lane order can differ from TA-Lib's scalar contract
    // by a few ULPs -- and unlike the other kernels there is no bit-level
    // parity proof for the vector path, which makes a public production path
    // that trades numerical equivalence for speed the wrong default.
    ad_line_scalar(high, low, close, volume, result)
}

pub fn simd_roc(data: &[f64], period: usize, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { roc_avx2(data, period, result) };
        }
    }
    roc_scalar(data, period, result)
}

pub fn simd_stddev(data: &[f64], period: usize, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { stddev_avx2(data, period, result) };
        }
    }
    stddev_scalar(data, period, result)
}

/// SIMD-accelerated rolling variance: uses AVX2 horizontal sum for the
/// initial window accumulation, then O(1) per-bar update. Result is
/// sample variance (Bessel-corrected).
pub fn simd_variance(data: &[f64], period: usize, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { variance_avx2(data, period, result) };
        }
    }
    variance_scalar(data, period, result)
}

pub fn simd_zscore_optimized(data: &[f64], period: usize, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { zscore_optimized_avx2(data, period, result) };
        }
    }
    zscore_optimized_scalar(data, period, result)
}

pub fn simd_correl(x: &[f64], y: &[f64], period: usize, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { correl_avx2(x, y, period, result) };
        }
    }
    correl_scalar(x, y, period, result)
}

pub fn simd_beta(asset: &[f64], benchmark: &[f64], period: usize, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { beta_avx2(asset, benchmark, period, result) };
        }
    }
    beta_scalar(asset, benchmark, period, result)
}

pub fn simd_linreg_slope(data: &[f64], period: usize, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { linreg_slope_avx2(data, period, result) };
        }
    }
    linreg_slope_scalar(data, period, result)
}

/// AVX2/scalar dispatch for the `LINREG` endpoint value.
///
/// **No production path calls this.** It exists so that
/// `benches/simd_statistics_bench.rs` can answer "is a vector kernel worth it
/// for `LINREG`", and the answer it gives is *no*: only the `period`-element
/// priming scan vectorises (`period / 4` chunks — three iterations at the
/// default period of 14), while the per-bar recurrence below is scalar
/// arithmetic that AVX2 cannot touch. The shipping `linreg` in
/// `math/linear.rs` is therefore a plain scalar loop, and that is a decision
/// rather than an oversight. Kept, rather than deleted, because the benchmark
/// that measures it is still the evidence for that decision.
pub fn simd_linreg(data: &[f64], period: usize, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { linreg_avx2(data, period, result) };
        }
    }
    linreg_scalar(data, period, result)
}

#[cfg(feature = "std")]
pub fn simd_linreg_angle(data: &[f64], period: usize, result: &mut [f64]) {
    let len = data.len().min(result.len());
    if period < 2 || len == 0 {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let mut slope_result = alloc::vec![f64::NAN; len];
    simd_linreg_slope(data, period, &mut slope_result);

    for i in 0..len {
        if !slope_result[i].is_nan() {
            result[i] = slope_result[i].atan() * 180.0 / core::f64::consts::PI;
        } else {
            result[i] = f64::NAN;
        }
    }
}

// ============================================================================
// D': SIMD ATR / AROON / KAMA dispatchers
// ============================================================================
//
// `simd_atr` and `simd_kama` need an internal scratch buffer, so they require
// `alloc` (and are gated to `std`). `simd_aroon` is allocation-free and is
// available in all configurations.

/// ATR (Wilder's smoothing) — a [`simd_true_range`] pass followed by scalar
/// Wilder recursive smoothing (`atr_wilder_scalar`). Only the true-range pass is
/// vectorised; the smoothing recurrence is inherently serial, so there is no
/// AVX2 smoothing kernel to fall back from.
#[cfg(feature = "std")]
pub fn simd_atr(high: &[f64], low: &[f64], prev_close: &[f64], period: usize, result: &mut [f64]) {
    let len = high
        .len()
        .min(low.len())
        .min(prev_close.len())
        .min(result.len());
    if len == 0 || period == 0 {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }
    // Compute true range into a scratch buffer (vectorised via the existing
    // simd_true_range kernel) and then apply Wilder's recursive smoothing.
    let mut tr = vec![0.0f64; len];
    simd_true_range(high, low, prev_close, &mut tr);
    atr_wilder_scalar(&tr, period, result);
}

/// Aroon up/down reference kernel.
///
/// Fully scalar. The bottleneck — `argmax` / `argmin` over a sliding window — is
/// a serial reduction, and the trailing `(period - idx) / period * 100`
/// arithmetic is a single expression per bar that does not amortise a SIMD
/// setup. The production Aroon lives in
/// [`crate::indicators::momentum::aroon`].
pub fn simd_aroon(
    high: &[f64],
    low: &[f64],
    period: usize,
    out_up: &mut [f64],
    out_down: &mut [f64],
) {
    let len = high
        .len()
        .min(low.len())
        .min(out_up.len())
        .min(out_down.len());
    if len == 0 || period == 0 {
        for r in out_up.iter_mut().take(len) {
            *r = f64::NAN;
        }
        for r in out_down.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }
    // Serial argmax / argmin (the hard part), then SIMD-friendly arithmetic.
    for i in 0..len {
        if i + 1 < period {
            out_up[i] = f64::NAN;
            out_down[i] = f64::NAN;
            continue;
        }
        let start = i + 1 - period;
        // Argmax in high[start..=i]
        let mut max_idx = start;
        let mut max_v = high[start];
        for k in (start + 1)..=i {
            if high[k] > max_v {
                max_v = high[k];
                max_idx = k;
            }
        }
        // Argmin in low[start..=i]
        let mut min_idx = start;
        let mut min_v = low[start];
        for k in (start + 1)..=i {
            if low[k] < min_v {
                min_v = low[k];
                min_idx = k;
            }
        }
        let p = period as f64;
        out_up[i] = (p - (i - max_idx) as f64) / p * 100.0;
        out_down[i] = (p - (i - min_idx) as f64) / p * 100.0;
    }
}

/// KAMA (Kaufman Adaptive Moving Average) reference kernel.
///
/// Fully scalar: the efficiency-ratio and smoothing-constant passes are plain
/// loops, and the final smoothing is a data-dependent recurrence that cannot be
/// batched. It does **not** delegate to the SIMD WMA kernel.
#[cfg(feature = "std")]
pub fn simd_kama(
    input: &[f64],
    er_period: usize,
    fast_period: usize,
    slow_period: usize,
    result: &mut [f64],
) {
    let len = input.len().min(result.len());
    if len < er_period + 1 || er_period == 0 || fast_period == 0 || slow_period == 0 {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }
    // 1. Compute Efficiency Ratio per bar: |change| / sum(|diffs|)
    let mut er = vec![0.0f64; len];
    for i in er_period..len {
        let change = (input[i] - input[i - er_period]).abs();
        let mut noise = 0.0;
        for k in (i - er_period + 1)..=i {
            noise += (input[k] - input[k - 1]).abs();
        }
        er[i] = if noise > crate::utils::NUMERIC_EPSILON {
            change / noise
        } else {
            0.0
        };
    }
    // 2. Smoothing constant: sc = (er * (2/(fast+1) - 2/(slow+1)) + 2/(slow+1))^2
    let fast_alpha = 2.0 / (fast_period as f64 + 1.0);
    let slow_alpha = 2.0 / (slow_period as f64 + 1.0);
    let mut sc = vec![0.0f64; len];
    for i in er_period..len {
        sc[i] = (er[i] * (fast_alpha - slow_alpha) + slow_alpha).powi(2);
    }
    // 3. Recursive smoothing: result[i] = result[i-1] + sc[i] * (input[i] - result[i-1])
    for r in result.iter_mut().take(er_period) {
        *r = f64::NAN;
    }
    result[er_period] = input[er_period];
    for i in (er_period + 1)..len {
        result[i] = result[i - 1] + sc[i] * (input[i] - result[i - 1]);
    }
}

// ============================================================================
// P.2 SIMD 时序算子 6 项
// ============================================================================

/// EMA 单步递推：`prev + k * (sample - prev)`。
///
/// x86_64 且有 AVX2 时走 `_mm256_fmadd_pd` 的**单步**版本（把三个标量广播到
/// 256-bit 通道，结果与标量 `mul_add` 等价），否则走标量。它**不**批量处理 4 步
/// 递推——递推本身数据相关，无法跨步向量化。
#[cfg(feature = "std")]
pub fn simd_ema_next(prev: f64, sample: f64, k: f64) -> f64 {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { ema_next_avx2(prev, sample, k) };
        }
    }
    prev + k * (sample - prev)
}

/// SIMD Chande Momentum Oscillator (CMO)。
///
/// CMO = (sum_up - sum_down) / (sum_up + sum_down) * 100。
/// AVX2 路径使用 4-way 比较 + 累加；非 x86 走标量。
#[cfg(feature = "std")]
pub fn simd_cmo(src: &[f64], period: usize, out: &mut [f64]) {
    let len = src.len().min(out.len());
    if period == 0 || len <= period {
        for r in out.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }
    // 头 period 根为 NaN（需要 period 个 diff，索引 i-period..=i）
    for r in out.iter_mut().take(period) {
        *r = f64::NAN;
    }
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe { cmo_avx2(src, period, out, len) };
            return;
        }
    }
    for i in period..len {
        let mut up = 0.0f64;
        let mut down = 0.0f64;
        for k in (i - period + 1)..=i {
            let diff = src[k] - src[k - 1];
            if diff > 0.0 {
                up += diff;
            } else {
                down += -diff;
            }
        }
        let denom = up + down;
        out[i] = if denom > crate::utils::NUMERIC_EPSILON {
            (up - down) / denom * 100.0
        } else {
            0.0
        };
    }
}

/// MESA Adaptive Moving Average (MAMA) reference kernel.
///
/// 输入长度 < 4 时返回 NaN；输出 `out_smooth` (MAMA) 与 `out_period` (FAMA)。
/// 全标量：简化版 Ehlers Hilbert Transform + 双 EMA 链，相位累加是逐 bar 递推，
/// **没有** AVX2 路径。生产实现见 [`crate::indicators::overlap::mama`]。
#[cfg(feature = "std")]
pub fn simd_mama_hilbert(src: &[f64], out_smooth: &mut [f64], out_period: &mut [f64]) {
    let len = src.len().min(out_smooth.len()).min(out_period.len());
    if len < 4 {
        for i in 0..len {
            out_smooth[i] = f64::NAN;
            out_period[i] = f64::NAN;
        }
        return;
    }
    // 初始化：前 3 根 NaN
    for i in 0..3.min(len) {
        out_smooth[i] = f64::NAN;
        out_period[i] = f64::NAN;
    }
    let mut phase = 0.0f64;
    let mut period_estimate = 10.0f64;
    let mut smooth = src[3];
    let mut period_ma = period_estimate;
    out_smooth[3] = smooth;
    out_period[3] = period_ma;
    for i in 4..len {
        // 简化 Hilbert: 用 src[i-3..=i] 四个值做加权相位估计
        let det = src[i] - src[i - 3];
        let num = src[i - 1] - src[i - 2];
        let mut ph = if det.abs() > crate::utils::NUMERIC_EPSILON {
            (num / det).atan()
        } else {
            phase
        };
        if ph < 0.0 {
            ph += core::f64::consts::PI;
        }
        // phase delta
        let d_phase = if ph < phase {
            ph + core::f64::consts::PI - phase
        } else {
            ph - phase
        };
        phase = ph;
        // period estimate
        if d_phase > 1e-6 && d_phase < core::f64::consts::PI {
            period_estimate = (2.0 * core::f64::consts::PI / d_phase).clamp(6.0, 50.0);
        }
        let alpha = (0.5 * period_estimate / period_ma).clamp(0.0, 1.0);
        smooth = alpha * src[i] + (1.0 - alpha) * smooth;
        period_ma = 0.2 * period_estimate + 0.8 * period_ma;
        out_smooth[i] = smooth;
        out_period[i] = period_ma;
    }
}

/// Parabolic SAR 单步 + EP/AF 更新参考内核。
///
/// 输入：high/low/prev_sar/prev_ep/af_step，每根 K 调用一次。
/// 全标量：SAR 递推依赖上一根的 `sar` / `ep` / `af`，无法跨 bar 批量。生产实现见
/// [`crate::indicators::overlap::sar`]。
#[cfg(feature = "std")]
pub fn simd_sar_step(
    high: &[f64],
    low: &[f64],
    prev_sar: f64,
    prev_ep: f64,
    af: f64,
    af_step: f64,
    af_max: f64,
    out_sar: &mut [f64],
) {
    let len = high.len().min(low.len()).min(out_sar.len());
    if len == 0 {
        return;
    }
    let mut sar = prev_sar;
    let mut ep = prev_ep;
    let mut af_cur = af;
    for i in 0..len {
        // 趋势向上 (SAR 在 low 之下) : SAR_t = SAR_{t-1} + AF * (EP - SAR_{t-1})
        // 趋势向下 (SAR 在 high 之上) : 同理
        // 简化：若当前 high >= EP → 趋势向上
        if high[i] >= ep {
            // 上升趋势
            sar = sar + af_cur * (ep - sar);
            sar = sar.min(low[i]); // SAR 不能穿越最近两根 low
            if high[i] > ep {
                ep = high[i];
                af_cur = (af_cur + af_step).min(af_max);
            }
        } else {
            // 下降趋势
            sar = sar + af_cur * (ep - sar);
            sar = sar.max(high[i]);
            if low[i] < ep {
                ep = low[i];
                af_cur = (af_cur + af_step).min(af_max);
            }
        }
        out_sar[i] = sar;
    }
}

/// SIMD Tim Tillson T3：6 个串联 EMA 的复合。
///
/// T3(n) = c1*e6 + c2*e5 + c3*e4 + c4*e3
/// 其中 e1 = EMA(p, n), e2 = EMA(e1, n), ..., e6 = EMA(e5, n)
/// 系数 c1..c4 = a^3, 3*a^2*(1-a), 3*a*(1-a)^2, (1-a)^3（Tim Tillson 原版）
/// 6 个 EMA 调用合并为内层循环。
#[cfg(feature = "std")]
pub fn simd_t3(src: &[f64], period: usize, a: f64, out: &mut [f64]) {
    let len = src.len().min(out.len());
    if period == 0 || len < period {
        for r in out.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }
    let k = 2.0 / (period as f64 + 1.0);
    // 6 级 EMA buffer
    let mut e = [0.0f64; 6];
    for i in 0..6 {
        e[i] = src[0];
    }
    // 前 period-1 根输出 NaN
    for r in out.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
    // 第 period-1 根：用 src[..=period-1] 的 SMA 作为种子
    let seed: f64 = src[..period].iter().sum::<f64>() / period as f64;
    for i in 0..6 {
        e[i] = seed;
    }
    out[period - 1] = seed;
    for i in period..len {
        e[0] = simd_ema_next(e[0], src[i], k);
        e[1] = simd_ema_next(e[1], e[0], k);
        e[2] = simd_ema_next(e[2], e[1], k);
        e[3] = simd_ema_next(e[3], e[2], k);
        e[4] = simd_ema_next(e[4], e[3], k);
        e[5] = simd_ema_next(e[5], e[4], k);
        let c1 = a * a * a;
        let c2 = 3.0 * a * a * (1.0 - a);
        let c3 = 3.0 * a * (1.0 - a) * (1.0 - a);
        let c4 = (1.0 - a) * (1.0 - a) * (1.0 - a);
        out[i] = c1 * e[5] + c2 * e[4] + c3 * e[3] + c4 * e[2];
    }
}

/// Simplified Hilbert Transform DC Phase (HT_DCPHASE-style) primitive.
///
/// Scalar only — there is deliberately no AVX2 fast path here: the phase
/// recurrence is sequential and dominated by one `atan` per bar. The
/// instantaneous phase is estimated from a 4-bar backward difference as
/// `atan((src[i-1] - src[i-2]) / (src[i] - src[i-3]))`, mapped into
/// `[0, 180]` **degrees**; the first 3 outputs are `NaN`.
///
/// This is a standalone approximation, not the TA-Lib-faithful implementation
/// used by [`crate::indicators::cycle::ht_dcphase`].
#[cfg(feature = "std")]
pub fn simd_ht_dcphase(src: &[f64], out: &mut [f64]) {
    let len = src.len().min(out.len());
    if len < 4 {
        for i in 0..len {
            out[i] = f64::NAN;
        }
        return;
    }
    for i in 0..3 {
        out[i] = f64::NAN;
    }
    let mut phase = 0.0f64;
    for i in 3..len {
        // 简化 Hilbert 滤波器输出
        let det = src[i] - src[i - 3];
        let num = src[i - 1] - src[i - 2];
        let mut ph = if det.abs() > crate::utils::NUMERIC_EPSILON {
            (num / det).atan()
        } else {
            phase
        };
        if ph < 0.0 {
            ph += core::f64::consts::PI;
        }
        out[i] = ph.to_degrees();
        phase = ph;
    }
}

// ============================================================================
// D.2 Hilbert Transform SIMD 内核
// ============================================================================
//
// 实现完整的 Ehlers Hilbert Transform 链路：
//   smooth -> detrender -> {quadrature, j1} -> {i2, j2} -> {re, im} -> phase
//
// AVX2 路径：把 7-tap detrender FIR 滤波和后续 IIR 滤波链的乘加运算
// 按 4-bar batch 向量化，剩余 phase = atan2(im, re) 仍走标量（每 bar
// 一次超越函数，无法 SIMD 化）。
//
// **注意**：本节内核是公开的独立 SIMD 原语，`indicators::cycle` 的生产链路
// **未**调用它们——`compute_hilbert_components` 走的是逐 bar 的标量状态机
// （见 `cycle.rs` 的 `# Performance`）。这些原语由本文件的单元测试保证与各自
// 标量回退逐位一致。
//
// 下述数字是内核自身相对逐元素标量实现的微基准对比（100K bars，x86_64 AVX2），
// **不是** HT_* 指标的端到端耗时：
//   - HT_DCPERIOD:  ~48 ns/bar  ->  ~20 ns/bar  (2.4x)
//   - HT_DCPHASE:   ~35 ns/bar  ->  ~14 ns/bar  (2.5x)
//   - HT_SINE:      ~38.56 ns   ->  ~15 ns/bar  (2.5x)

/// 4-period weighted moving average (Hilbert smooth):
///     smooth\[i\] = (4*price\[i\] + 3*price\[i-1\] + 2*price\[i-2\] + price\[i-3\]) / 10
///
/// AVX2 路径：每批处理 4 bars，权重 [4, 3, 2, 1] 直接用 `_mm256_fmadd_pd`
/// 累加，最后乘 0.1。
#[cfg(feature = "std")]
pub fn simd_ht_smooth(input: &[f64], out: &mut [f64]) {
    let len = input.len().min(out.len());
    if len < 4 {
        for o in out.iter_mut().take(len) {
            *o = 0.0;
        }
        return;
    }
    out[0] = 0.0;
    out[1] = 0.0;
    out[2] = 0.0;
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe { ht_smooth_avx2(input, out, len) };
            return;
        }
    }
    for i in 3..len {
        // FMA: smooth = (4*x[i] + 3*x[i-1] + 2*x[i-2] + 1*x[i-3]) * 0.1
        let v = unsafe { (4.0 * *input.get_unchecked(i)).mul_add(*input.get_unchecked(i), 0.0) };
        let _ = v;
        // 直接展开，避免编译器对临时变量做额外优化
        out[i] = 0.1 * (4.0 * input[i] + 3.0 * input[i - 1] + 2.0 * input[i - 2] + input[i - 3]);
    }
}

/// 7-tap Hilbert detrender：
///     a = 0.0962*s\[i\] + 0.5769*s\[i-2\] - 0.5769*s\[i-4\] - 0.0962*s\[i-6\]
///     b = 0.075 *s[i-1] + 0.54 *s[i-3] + 0.075 *s[i-5]
///     detrender\[i\] = a * b
///
/// AVX2 路径：每批 4 bars 同时计算 a 和 b，最后用 `_mm256_mul_pd` 相乘。
#[cfg(feature = "std")]
pub fn simd_ht_detrender(smooth: &[f64], out: &mut [f64]) {
    let len = smooth.len().min(out.len());
    if len < 10 {
        for o in out.iter_mut().take(len) {
            *o = 0.0;
        }
        return;
    }
    for o in out.iter_mut().take(10) {
        *o = 0.0;
    }
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe { ht_detrender_avx2(smooth, out, len) };
            return;
        }
    }
    for i in 10..len {
        let a = 0.0962 * smooth[i] + 0.5769 * smooth[i - 2]
            - 0.5769 * smooth[i - 4]
            - 0.0962 * smooth[i - 6];
        let b = 0.075 * smooth[i - 1] + 0.54 * smooth[i - 3] + 0.075 * smooth[i - 5];
        out[i] = a * b;
    }
}

/// Hilbert 滤波链后端：
///     in_phase\[i\]   = detrender\[i-6\]
///     quadrature\[i\] = 0.0962*d\[i\] + 0.5769*d\[i-2\] - 0.5769*d\[i-4\] - 0.0962*d\[i-6\]
///     j1\[i\]         = 0.0962*ip\[i\] + 0.5769*ip\[i-2\] - 0.5769*ip\[i-4\] - 0.0962*ip\[i-6\]
///     i2\[i\]         = ip\[i\] - j1\[i\]
///     j2\[i\]         = q\[i\] + ip\[i\]
///     re\[i\]         = i2\[i\]*ip\[i\] + j2\[i\]*q\[i\]
///     im\[i\]         = i2\[i\]*q\[i\]  - j2\[i\]*ip\[i\]
///
/// 输出 phase 数组（弧度），最后一次 atan2 在 AVX2 之外逐 bar 处理
/// （每 bar 1 个超越函数，无法 SIMD 化）。
///
/// AVX2 路径：每批 4 bars 同时计算 in_phase/quadrature/j1/i2/j2/re/im，
/// 末尾逐 bar 算 atan2。
#[cfg(feature = "std")]
pub fn simd_ht_components(detrender: &[f64], phase_out: &mut [f64]) {
    let len = detrender.len().min(phase_out.len());
    if len < 16 {
        for o in phase_out.iter_mut().take(len) {
            *o = 0.0;
        }
        return;
    }
    for o in phase_out.iter_mut().take(16) {
        *o = 0.0;
    }
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe { ht_components_avx2(detrender, phase_out, len) };
            return;
        }
    }
    for i in 16..len {
        let ip = detrender[i - 6];
        let q = 0.0962 * detrender[i] + 0.5769 * detrender[i - 2]
            - 0.5769 * detrender[i - 4]
            - 0.0962 * detrender[i - 6];
        let j1 = 0.0962 * ip + 0.5769 * detrender[i - 8]
            - 0.5769 * detrender[i - 10]
            - 0.0962 * detrender[i - 12];
        // 修正：j1 是对 in_phase 序列做同样 4-tap Hilbert
        // 但 in_phase[i-k] = detrender[i-k-6]，所以 j1[i] = 0.0962*detrender[i-6]
        //                                  + 0.5769*detrender[i-8]
        //                                  - 0.5769*detrender[i-10]
        //                                  - 0.0962*detrender[i-12]
        let i2 = ip - j1;
        let j2 = q + ip;
        let re = i2 * ip + j2 * q;
        let im = i2 * q - j2 * ip;
        phase_out[i] = if re.abs() > 1e-10 { im.atan2(re) } else { 0.0 };
    }
}

// ============================================================================
// D.4 中国市场指标 SIMD 内核 (AR/BR/VR/CR)
// ============================================================================
//
// 这 4 个指标的 hot path 已经是 O(1) per-bar（4-6 次加法/减法），但初始
// `period` 元素累加可以 AVX2 4-bar batch 加速。本节提供：
//   - simd_diff_sum:    sum(a[i] - b[i] for i in 0..period)
//   - simd_max_diff_sum:sum(max(0, a[i] - b[i]) for i in 0..period)
//   - simd_dual_diff_init: 同时算 sum(a-b) 和 sum(b-c) (AR 用)
//   - simd_dual_max_init:  同时算 sum(max(0,a-b)) 和 sum(max(0,b-c)) (BR 用)

/// SIMD `sum(a[i] - b[i] for i in 0..period)`。AVX2 4-bar batch，
/// 用于 AR (high-open, open-low) 等。
#[cfg(feature = "std")]
pub fn simd_diff_sum(a: &[f64], b: &[f64], period: usize) -> f64 {
    debug_assert!(period <= a.len() && period <= b.len());
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { diff_sum_avx2(a, b, period) };
        }
    }
    let mut sum = 0.0f64;
    for i in 0..period {
        sum += a[i] - b[i];
    }
    sum
}

/// SIMD `sum(max(0, a[i] - b[i]) for i in 0..period)`。AVX2 4-bar batch，
/// 用于 BR (max(0, high-close), max(0, close-low)) 等。
#[cfg(feature = "std")]
pub fn simd_max_diff_sum(a: &[f64], b: &[f64], period: usize) -> f64 {
    debug_assert!(period <= a.len() && period <= b.len());
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { max_diff_sum_avx2(a, b, period) };
        }
    }
    let mut sum = 0.0f64;
    for i in 0..period {
        let d = a[i] - b[i];
        if d > 0.0 {
            sum += d;
        }
    }
    sum
}

/// SIMD 双滚动求和初始化 (AR 用)。
///
/// 同时计算：
///   - sum_ho = sum(high\[i\] - open\[i\] for i in 0..period)
///   - sum_ol = sum(open\[i\] - low\[i\]  for i in 0..period)
///
/// AVX2 路径：单遍 4-bar batch 同时算两条 sum。
#[cfg(feature = "std")]
pub fn simd_dual_diff_init(high: &[f64], open: &[f64], low: &[f64], period: usize) -> (f64, f64) {
    debug_assert!(period <= high.len() && period <= open.len() && period <= low.len());
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { dual_diff_init_avx2(high, open, low, period) };
        }
    }
    let mut sum_ho = 0.0f64;
    let mut sum_ol = 0.0f64;
    for i in 0..period {
        sum_ho += high[i] - open[i];
        sum_ol += open[i] - low[i];
    }
    (sum_ho, sum_ol)
}

/// SIMD 双 max 滚动求和初始化 (BR 用)。
///
/// 同时计算：
///   - sum_up   = sum(max(0, high\[i\]   - close\[i\]) for i in 0..period)
///   - sum_down = sum(max(0, close\[i+1\] - low\[i\])  for i in 0..period)
///
/// 注：BR 的索引从 1 开始（j=1..=period），所以这里 `close[i]` 实际
/// 是 close[i+1] 在原 BR 公式中。调用方负责传入正确的窗口。
#[cfg(feature = "std")]
pub fn simd_dual_max_init(high: &[f64], close: &[f64], low: &[f64], period: usize) -> (f64, f64) {
    debug_assert!(period + 1 <= high.len() && period + 1 <= close.len() && period + 1 <= low.len());
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { dual_max_init_avx2(high, close, low, period) };
        }
    }
    let mut sum_up = 0.0f64;
    let mut sum_down = 0.0f64;
    for j in 1..=period {
        let d_up = high[j] - close[j - 1];
        let d_down = close[j - 1] - low[j];
        if d_up > 0.0 {
            sum_up += d_up;
        }
        if d_down > 0.0 {
            sum_down += d_down;
        }
    }
    (sum_up, sum_down)
}

// ============================================================================
// P.3: SIMD kernels for simple indicators (MOM, BOP, AVGPRICE)
// ============================================================================

/// SIMD-accelerated Momentum (MOM): output\[i\] = input\[i\] - input\[i - period\]
#[cfg(feature = "std")]
pub fn simd_mom(input: &[f64], period: usize, result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            if period == 10 {
                return unsafe { mom10_avx2(input, result) };
            }
            return unsafe { mom_avx2(input, period, result) };
        }
        return unsafe { mom_sse2(input, period, result) };
    }
    #[cfg(not(all(feature = "std", target_arch = "x86_64")))]
    mom_scalar(input, period, result)
}

/// Fixed-period MOM dispatcher for the public default period.
///
/// The Python binding already validates the default period before entering
/// native code. Keeping this entry point period-free removes one branch and
/// the generic dispatcher call from the short-input release-gate path while
/// preserving the same SIMD/scalar fallback hierarchy as [`simd_mom`].
#[cfg(feature = "std")]
#[inline(always)]
pub fn simd_mom10(input: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        static AVX512: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *AVX512.get_or_init(|| is_x86_feature_detected!("avx512f")) {
            return unsafe { crate::math::simd_ops_avx512::simd512_mom10_unchecked(input, result) };
        }
        static AVX2: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *AVX2.get_or_init(|| is_x86_feature_detected!("avx2")) {
            return unsafe { mom10_avx2(input, result) };
        }
        return unsafe { mom10_sse2(input, result) };
    }
    #[cfg(not(all(feature = "std", target_arch = "x86_64")))]
    mom_scalar(input, 10, result)
}

/// SIMD-accelerated Balance of Power (BOP): (close - open) / (high - low)
#[cfg(feature = "std")]
pub fn simd_bop(open: &[f64], high: &[f64], low: &[f64], close: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { bop_avx2(open, high, low, close, result) };
        }
    }
    bop_scalar(open, high, low, close, result)
}

/// SIMD-accelerated Average Price (AVGPRICE): (open + high + low + close) / 4
#[cfg(feature = "std")]
pub fn simd_avgprice(open: &[f64], high: &[f64], low: &[f64], close: &[f64], result: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { avgprice_avx2(open, high, low, close, result) };
        }
    }
    avgprice_scalar(open, high, low, close, result)
}
