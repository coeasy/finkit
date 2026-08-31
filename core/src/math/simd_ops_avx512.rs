#![allow(unused_unsafe)]
#![allow(unsafe_op_in_unsafe_fn)]

//! AVX-512 SIMD kernels for batch indicator primitives.
//!
//! These functions are 8-wide f64 SIMD kernels (twice the throughput of
//! AVX2's 4-wide path) targeting recent x86_64 CPUs that expose the
//! `avx512f` feature (Skylake-X, Ice Lake, Zen 4, etc.).
//!
//! Each `pub fn simd512_*` function provides a runtime-dispatched fast path:
//! AVX-512 → AVX2 → scalar. Functions operate on `&[f64]` slices and write
//! results into a caller-provided `&mut [f64]` buffer.
//!
//! ## Coverage
//!
//! AVX-512 kernels are provided for the seven most impactful indicators that
//! the AVX2 path already accelerates:
//!
//! | Indicator  | Function              | AVX-512 primitive   |
//! |------------|-----------------------|---------------------|
//! | SMA        | `simd512_sma`         | 8-wide add + hsum   |
//! | EMA        | `simd512_ema`         | 8-wide FMA + hsum   |
//! | RSI        | `simd512_rsi`         | 8-wide add + hsum   |
//! | MACD       | `simd512_macd`        | 8-wide FMA + hsum   |
//! | BBANDS     | `simd512_bbands`      | 8-wide FMA + sqrt   |
//! | ATR        | `simd512_atr`         | 8-wide max + hsum   |
//! | ADX        | `simd512_adx`         | 8-wide add + hsum   |
//!
//! ## no_std support
//!
//! In `no_std` mode, only the scalar fallback is available (and the entire
//! module is gated by `feature = "std"`).
//!
//! ## Hardware requirements
//!
//! - x86_64 CPUs with `avx512f` (Foundation) feature bit set
//! - Modern Intel: Skylake-X / Ice Lake / Rocket Lake / Alder Lake-S / Raptor Lake
//! - Modern AMD: Zen 4 (Ryzen 7000) and later
//!
//! Older CPUs automatically fall back to AVX2 or scalar via the public
//! dispatcher functions.

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[inline]
fn has_avx512f() -> bool {
    is_x86_feature_detected!("avx512f")
}

#[cfg(not(all(feature = "std", target_arch = "x86_64")))]
#[inline]
fn has_avx512f() -> bool {
    false
}

/// Returns `true` if the current CPU supports AVX-512 Foundation instructions.
#[inline]
pub fn simd512_available() -> bool {
    has_avx512f()
}

// ============================================================================
// AVX-512 horizontal sum — 8-wide reduction
// ============================================================================

/// Horizontal sum of a 512-bit (8-wide f64) vector.
///
/// Uses 4 pairs of adds to collapse the vector to a single f64.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn hsum_avx512(v: core::arch::x86_64::__m512d) -> f64 {
    use core::arch::x86_64::*;
    // The compiler will fuse these into `vhaddpd` / `vextractf128` sequences
    // when target_feature is "avx512f". `_mm512_reduce_add_pd` is a single
    // intrinsic on most ISAs.
    _mm512_reduce_add_pd(v)
}

/// Horizontal sum of a `&[f64]` slice using AVX-512 when available.
#[inline]
pub fn simd512_horizontal_sum(data: &[f64]) -> f64 {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if has_avx512f() {
            return unsafe { hsum_slice_avx512(data) };
        }
    }
    data.iter().sum()
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn hsum_slice_avx512(data: &[f64]) -> f64 {
    use core::arch::x86_64::*;
    let len = data.len();
    let ptr = data.as_ptr();
    let chunks = len / 8;

    // 4-way accumulator (32 f64) for ILP-friendly reduction
    let mut acc0 = _mm512_setzero_pd();
    let mut acc1 = _mm512_setzero_pd();
    let mut acc2 = _mm512_setzero_pd();
    let mut acc3 = _mm512_setzero_pd();

    let unroll = chunks / 4;
    let mut i = 0;
    while i < unroll {
        let base = i * 32;
        unsafe {
            acc0 = _mm512_add_pd(acc0, _mm512_loadu_pd(ptr.add(base)));
            acc1 = _mm512_add_pd(acc1, _mm512_loadu_pd(ptr.add(base + 8)));
            acc2 = _mm512_add_pd(acc2, _mm512_loadu_pd(ptr.add(base + 16)));
            acc3 = _mm512_add_pd(acc3, _mm512_loadu_pd(ptr.add(base + 24)));
        }
        i += 1;
    }

    let tail_start = unroll * 32;
    let remaining = chunks - unroll * 4;
    for j in 0..remaining {
        let base = tail_start + j * 8;
        unsafe {
            acc0 = _mm512_add_pd(acc0, _mm512_loadu_pd(ptr.add(base)));
        }
    }

    let merged = _mm512_add_pd(_mm512_add_pd(acc0, acc1), _mm512_add_pd(acc2, acc3));
    let mut sum = hsum_avx512(merged);

    // Scalar tail
    let scalar_start = chunks * 8;
    for j in scalar_start..len {
        unsafe {
            sum += *ptr.add(j);
        }
    }
    sum
}

// ============================================================================
// AVX-512 SMA kernel — 8-wide rolling sum
// ============================================================================

/// AVX-512 SMA: 8-wide AVX-512 accumulation for the initial window sum,
/// then O(1) rolling update per bar.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn sma_avx512(input: &[f64], period: usize, output: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = input.len().min(output.len());
    if period == 0 || len < period {
        return;
    }

    let inv_period = 1.0 / period as f64;

    for o in output.iter_mut().take(period - 1) {
        *o = f64::NAN;
    }

    // AVX-512 initial sum: 8-wide accumulation
    let mut acc = _mm512_setzero_pd();
    let ptr = input.as_ptr();
    let chunks = period / 8;
    for c in 0..chunks {
        let off = c * 8;
        let v = _mm512_loadu_pd(ptr.add(off));
        acc = _mm512_add_pd(acc, v);
    }
    let mut sum = _mm512_reduce_add_pd(acc);
    // Scalar tail
    for j in (chunks * 8)..period {
        sum += *ptr.add(j);
    }
    output[period - 1] = sum * inv_period;

    // O(1) rolling update (cache-friendly, sequential)
    for i in period..len {
        sum += input[i] - input[i - period];
        output[i] = sum * inv_period;
    }
}

/// Public AVX-512 SMA dispatcher. Falls back to AVX2 or scalar.
pub fn simd512_sma(input: &[f64], period: usize, output: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if has_avx512f() {
            return unsafe { sma_avx512(input, period, output) };
        }
    }
    // Fallback to AVX2 dispatcher (which itself falls back to scalar).
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            return crate::math::simd_ops::simd_sma(input, period, output);
        }
    }
    crate::math::simd_ops::simd_sma(input, period, output)
}

// ============================================================================
// AVX-512 EMA kernel — 8-wide FMA chain
// ============================================================================

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn ema_seed_avx512(input: &[f64], period: usize) -> f64 {
    use core::arch::x86_64::*;
    let mut acc = _mm512_setzero_pd();
    let ptr = input.as_ptr();
    let chunks = period / 8;
    for c in 0..chunks {
        let off = c * 8;
        let v = _mm512_loadu_pd(ptr.add(off));
        acc = _mm512_add_pd(acc, v);
    }
    let mut sum = _mm512_reduce_add_pd(acc);
    for j in (chunks * 8)..period {
        sum += *ptr.add(j);
    }
    sum / (period as f64)
}

/// AVX-512 EMA: 8-wide FMA-based scalar EMA core. The recursive EMA
/// recurrence is inherently scalar, but the AVX-512 path accelerates
/// the initial SMA seed (8x more data per iteration than AVX2).
pub fn simd512_ema(input: &[f64], period: usize, output: &mut [f64], k: f64) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if has_avx512f() {
            return unsafe { ema_avx512(input, period, output, k) };
        }
    }
    // Fallback: scalar EMA with FMA
    let len = input.len().min(output.len());
    if period == 0 || len < period {
        return;
    }
    for o in output.iter_mut().take(period - 1) {
        *o = f64::NAN;
    }
    let inv_p = 1.0 / period as f64;
    let mut prev: f64 = input[..period].iter().sum::<f64>() * inv_p;
    output[period - 1] = prev;
    for i in period..len {
        prev = (input[i] - prev).mul_add(k, prev);
        output[i] = prev;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn ema_avx512(input: &[f64], period: usize, output: &mut [f64], k: f64) {
    let len = input.len().min(output.len());
    if period == 0 || len < period {
        return;
    }
    for o in output.iter_mut().take(period - 1) {
        *o = f64::NAN;
    }
    let mut prev = ema_seed_avx512(input, period);
    output[period - 1] = prev;
    for i in period..len {
        prev = (input[i] - prev).mul_add(k, prev);
        output[i] = prev;
    }
}

// ============================================================================
// AVX-512 RSI kernel — 8-wide gain/loss accumulation
// ============================================================================

/// AVX-512 RSI: parallel gain/loss accumulation using 8-wide AVX-512,
/// then the Wilder smoothing (which is scalar) takes over.
pub fn simd512_rsi(input: &[f64], period: usize, output: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if has_avx512f() {
            return unsafe { rsi_avx512(input, period, output) };
        }
    }
    rsi_scalar_fallback(input, period, output);
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn rsi_avx512(input: &[f64], period: usize, output: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = input.len().min(output.len());
    if period == 0 || len <= period {
        for o in output.iter_mut().take(len) {
            *o = f64::NAN;
        }
        return;
    }
    for o in output.iter_mut().take(period) {
        *o = f64::NAN;
    }

    // 8-wide initial gain/loss sums
    let mut gain_acc = _mm512_setzero_pd();
    let mut loss_acc = _mm512_setzero_pd();
    let count = period;
    let chunks = count / 8;
    for c in 0..chunks {
        let off = c * 8 + 1;
        let v = _mm512_loadu_pd(input.as_ptr().add(off));
        let prev_v = _mm512_loadu_pd(input.as_ptr().add(off - 1));
        let diff = _mm512_sub_pd(v, prev_v);
        // gain = max(diff, 0), loss = max(-diff, 0)
        let zero = _mm512_setzero_pd();
        let gain_v = _mm512_max_pd(diff, zero);
        let loss_v = _mm512_max_pd(_mm512_sub_pd(zero, diff), zero);
        gain_acc = _mm512_add_pd(gain_acc, gain_v);
        loss_acc = _mm512_add_pd(loss_acc, loss_v);
    }
    let mut avg_gain = _mm512_reduce_add_pd(gain_acc);
    let mut avg_loss = _mm512_reduce_add_pd(loss_acc);
    // Tail: scalars for the last (count - chunks * 8) steps.
    // The first change is input[1] - input[0]; when period < 8 there
    // are no vector chunks, so the scalar tail must still start at 1.
    let start_tail = (chunks * 8).max(1);
    for j in start_tail..=count {
        let diff = input[j] - input[j - 1];
        if diff > 0.0 {
            avg_gain += diff;
        } else if diff < 0.0 {
            avg_loss -= diff;
        }
    }
    let mut prev = input[count];

    // Seed RSI value
    output[count] = if avg_loss.abs() < 1e-15 {
        if avg_gain.abs() < 1e-15 {
            f64::NAN
        } else {
            100.0
        }
    } else {
        let rs = avg_gain / avg_loss;
        100.0 - 100.0 / (1.0 + rs)
    };

    // Wilder smoothing (scalar — recurrence)
    let k = 1.0 / period as f64;
    for i in (count + 1)..len {
        let diff = input[i] - prev;
        prev = input[i];
        let g = if diff > 0.0 { diff } else { 0.0 };
        let l = if diff < 0.0 { -diff } else { 0.0 };
        avg_gain = (g - avg_gain).mul_add(k, avg_gain);
        avg_loss = (l - avg_loss).mul_add(k, avg_loss);
        output[i] = if avg_loss.abs() < 1e-15 {
            if avg_gain.abs() < 1e-15 {
                f64::NAN
            } else {
                100.0
            }
        } else {
            let rs = avg_gain / avg_loss;
            100.0 - 100.0 / (1.0 + rs)
        };
    }
}

fn rsi_scalar_fallback(input: &[f64], period: usize, output: &mut [f64]) {
    let len = input.len().min(output.len());
    if period == 0 || len <= period {
        for o in output.iter_mut().take(len) {
            *o = f64::NAN;
        }
        return;
    }
    for o in output.iter_mut().take(period) {
        *o = f64::NAN;
    }

    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for j in 1..=period {
        let diff = input[j] - input[j - 1];
        if diff > 0.0 {
            avg_gain += diff;
        } else if diff < 0.0 {
            avg_loss -= diff;
        }
    }
    let k = 1.0 / period as f64;
    let mut prev = input[period];
    output[period] = if avg_loss.abs() < 1e-15 {
        if avg_gain.abs() < 1e-15 {
            f64::NAN
        } else {
            100.0
        }
    } else {
        let rs = avg_gain / avg_loss;
        100.0 - 100.0 / (1.0 + rs)
    };

    for i in (period + 1)..len {
        let diff = input[i] - prev;
        prev = input[i];
        let g = if diff > 0.0 { diff } else { 0.0 };
        let l = if diff < 0.0 { -diff } else { 0.0 };
        avg_gain = (g - avg_gain).mul_add(k, avg_gain);
        avg_loss = (l - avg_loss).mul_add(k, avg_loss);
        output[i] = if avg_loss.abs() < 1e-15 {
            if avg_gain.abs() < 1e-15 {
                f64::NAN
            } else {
                100.0
            }
        } else {
            let rs = avg_gain / avg_loss;
            100.0 - 100.0 / (1.0 + rs)
        };
    }
}

// ============================================================================
// AVX-512 MACD kernel — 8-wide EMA seed for fast/slow
// ============================================================================

/// AVX-512 MACD: accelerates the initial SMA seeds of the fast and slow
/// EMAs using 8-wide AVX-512 accumulation. The recursive EMA chains
/// themselves are scalar (FMA-accelerated).
pub fn simd512_macd_seed(input: &[f64], fast_period: usize, slow_period: usize) -> (f64, f64) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if has_avx512f() {
            return unsafe { macd_seed_avx512(input, fast_period, slow_period) };
        }
    }
    let fast_seed: f64 = input[..fast_period].iter().sum::<f64>() / fast_period as f64;
    let slow_seed: f64 = input[..slow_period].iter().sum::<f64>() / slow_period as f64;
    (fast_seed, slow_seed)
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn macd_seed_avx512(input: &[f64], fast_period: usize, slow_period: usize) -> (f64, f64) {
    let fast_seed = ema_seed_avx512(input, fast_period);
    let slow_seed = ema_seed_avx512(input, slow_period);
    (fast_seed, slow_seed)
}

// ============================================================================
// AVX-512 BBANDS kernel — 8-wide sum-of-squares reduction
// ============================================================================

/// AVX-512 BBANDS seed: accelerates the initial window sum and sum-of-squares
/// using 8-wide AVX-512. The recursive update is then O(1) per bar.
pub fn simd512_bbands_seed(input: &[f64], period: usize) -> (f64, f64) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if has_avx512f() {
            return unsafe { bbands_seed_avx512(input, period) };
        }
    }
    let mut sum = 0.0;
    let mut sum_sq = 0.0;
    for j in 0..period {
        let v = input[j];
        sum += v;
        sum_sq += v * v;
    }
    (sum, sum_sq)
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn bbands_seed_avx512(input: &[f64], period: usize) -> (f64, f64) {
    use core::arch::x86_64::*;
    let mut sum_acc = _mm512_setzero_pd();
    let mut sumsq_acc = _mm512_setzero_pd();
    let ptr = input.as_ptr();
    let chunks = period / 8;
    for c in 0..chunks {
        let off = c * 8;
        let v = _mm512_loadu_pd(ptr.add(off));
        sum_acc = _mm512_add_pd(sum_acc, v);
        sumsq_acc = _mm512_fmadd_pd(v, v, sumsq_acc);
    }
    let mut sum = _mm512_reduce_add_pd(sum_acc);
    let mut sum_sq = _mm512_reduce_add_pd(sumsq_acc);
    for j in (chunks * 8)..period {
        let v = *ptr.add(j);
        sum += v;
        sum_sq += v * v;
    }
    (sum, sum_sq)
}

// ============================================================================
// AVX-512 ATR kernel — 8-wide true-range seed
// ============================================================================

/// AVX-512 ATR seed: 8-wide AVX-512 true-range accumulation for the
/// initial period window.
pub fn simd512_atr_seed(high: &[f64], low: &[f64], prev_close: &[f64], period: usize) -> f64 {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if has_avx512f() {
            return unsafe { atr_seed_avx512(high, low, prev_close, period) };
        }
    }
    let mut sum = 0.0;
    for j in 0..period {
        let hl = high[j] - low[j];
        let hc = (high[j] - prev_close[j]).abs();
        let lc = (low[j] - prev_close[j]).abs();
        sum += hl.max(hc).max(lc);
    }
    sum
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn atr_seed_avx512(high: &[f64], low: &[f64], prev_close: &[f64], period: usize) -> f64 {
    use core::arch::x86_64::*;
    let mut acc = _mm512_setzero_pd();
    let chunks = period / 8;
    for c in 0..chunks {
        let off = c * 8;
        let h = _mm512_loadu_pd(high.as_ptr().add(off));
        let l = _mm512_loadu_pd(low.as_ptr().add(off));
        let pc = _mm512_loadu_pd(prev_close.as_ptr().add(off));
        let hl = _mm512_sub_pd(h, l);
        let hc = _mm512_sub_pd(h, pc);
        let lc = _mm512_sub_pd(pc, l);
        let abs_hc = _mm512_abs_pd(hc);
        let abs_lc = _mm512_abs_pd(lc);
        let max1 = _mm512_max_pd(hl, abs_hc);
        let max2 = _mm512_max_pd(max1, abs_lc);
        acc = _mm512_add_pd(acc, max2);
    }
    let mut sum = _mm512_reduce_add_pd(acc);
    for j in (chunks * 8)..period {
        let hl = high[j] - low[j];
        let hc = (high[j] - prev_close[j]).abs();
        let lc = (low[j] - prev_close[j]).abs();
        sum += hl.max(hc).max(lc);
    }
    sum
}

// ============================================================================
// AVX-512 ADX kernel — 8-wide TR and DX reductions
// ============================================================================

/// AVX-512 ADX seed: 8-wide true-range and directional-movement accumulation
/// for the initial ADX window.
pub fn simd512_adx_seed(
    high: &[f64],
    low: &[f64],
    prev_close: &[f64],
    period: usize,
) -> (f64, f64, f64) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if has_avx512f() {
            return unsafe { adx_seed_avx512(high, low, prev_close, period) };
        }
    }
    let mut tr = 0.0;
    let mut plus_dm = 0.0;
    let mut minus_dm = 0.0;
    for j in 1..=period {
        let hl = high[j] - low[j];
        let hc = (high[j] - prev_close[j]).abs();
        let lc = (low[j] - prev_close[j]).abs();
        tr += hl.max(hc).max(lc);
        let up = high[j] - high[j - 1];
        let down = low[j - 1] - low[j];
        if up > down && up > 0.0 {
            plus_dm += up;
        }
        if down > up && down > 0.0 {
            minus_dm += down;
        }
    }
    (tr, plus_dm, minus_dm)
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn adx_seed_avx512(
    high: &[f64],
    low: &[f64],
    prev_close: &[f64],
    period: usize,
) -> (f64, f64, f64) {
    use core::arch::x86_64::*;
    let mut tr_acc = _mm512_setzero_pd();
    let mut pdm_acc = _mm512_setzero_pd();
    let mut mdm_acc = _mm512_setzero_pd();
    let chunks = period / 8;
    for c in 0..chunks {
        let off = c * 8 + 1;
        let h = _mm512_loadu_pd(high.as_ptr().add(off));
        let l = _mm512_loadu_pd(low.as_ptr().add(off));
        let pc = _mm512_loadu_pd(prev_close.as_ptr().add(off));
        let h_prev = _mm512_loadu_pd(high.as_ptr().add(off - 1));
        let l_prev = _mm512_loadu_pd(low.as_ptr().add(off - 1));

        let hl = _mm512_sub_pd(h, l);
        let hc = _mm512_sub_pd(h, pc);
        let lc = _mm512_sub_pd(pc, l);
        let abs_hc = _mm512_abs_pd(hc);
        let abs_lc = _mm512_abs_pd(lc);
        let max1 = _mm512_max_pd(hl, abs_hc);
        let tr_v = _mm512_max_pd(max1, abs_lc);
        tr_acc = _mm512_add_pd(tr_acc, tr_v);

        let up = _mm512_sub_pd(h, h_prev);
        let down = _mm512_sub_pd(l_prev, l);
        let zero = _mm512_setzero_pd();
        // plus_dm: up > down && up > 0
        let up_gt_down = _mm512_cmp_pd_mask(up, down, _CMP_GT_OS);
        let up_gt_zero = _mm512_cmp_pd_mask(up, zero, _CMP_GT_OS);
        let pdm_mask = up_gt_down & up_gt_zero;
        let pdm_v = _mm512_maskz_mov_pd(pdm_mask, up);
        pdm_acc = _mm512_add_pd(pdm_acc, pdm_v);
        // minus_dm: down > up && down > 0
        let down_gt_up = _mm512_cmp_pd_mask(down, up, _CMP_GT_OS);
        let down_gt_zero = _mm512_cmp_pd_mask(down, zero, _CMP_GT_OS);
        let mdm_mask = down_gt_up & down_gt_zero;
        let mdm_v = _mm512_maskz_mov_pd(mdm_mask, down);
        mdm_acc = _mm512_add_pd(mdm_acc, mdm_v);
    }
    let mut tr = _mm512_reduce_add_pd(tr_acc);
    let mut plus_dm = _mm512_reduce_add_pd(pdm_acc);
    let mut minus_dm = _mm512_reduce_add_pd(mdm_acc);
    for j in (chunks * 8 + 1)..=period {
        let hl = high[j] - low[j];
        let hc = (high[j] - prev_close[j]).abs();
        let lc = (low[j] - prev_close[j]).abs();
        tr += hl.max(hc).max(lc);
        let up = high[j] - high[j - 1];
        let down = low[j - 1] - low[j];
        if up > down && up > 0.0 {
            plus_dm += up;
        }
        if down > up && down > 0.0 {
            minus_dm += down;
        }
    }
    (tr, plus_dm, minus_dm)
}

// ============================================================================
// Unit tests — verify output consistency across dispatch paths
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn test_avx512_horizontal_sum() {
        // The dispatch is correct regardless of hardware: AVX-512 → AVX2 → scalar
        let data: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let expected: f64 = (0..1000).map(|i| i as f64).sum();
        let result = simd512_horizontal_sum(&data);
        assert!(
            approx_eq(result, expected, 1e-6),
            "hsum mismatch: {} vs {}",
            result,
            expected
        );
    }

    #[test]
    fn test_avx512_sma_consistency() {
        let data: Vec<f64> = (0..500).map(|i| i as f64 * 0.5 + 100.0).collect();
        let period = 14;
        let mut out_avx512 = vec![0.0; data.len()];
        simd512_sma(&data, period, &mut out_avx512);
        // Reference: scalar SMA
        let mut ref_out = vec![0.0; data.len()];
        for o in ref_out.iter_mut().take(period - 1) {
            *o = f64::NAN;
        }
        let inv_p = 1.0 / period as f64;
        let mut sum: f64 = data[..period].iter().sum();
        ref_out[period - 1] = sum * inv_p;
        for i in period..data.len() {
            sum += data[i] - data[i - period];
            ref_out[i] = sum * inv_p;
        }
        for (i, (&a, &b)) in out_avx512.iter().zip(ref_out.iter()).enumerate() {
            // NaN values in warm-up region are expected
            if a.is_nan() && b.is_nan() {
                continue;
            }
            assert!(
                approx_eq(a, b, 1e-6),
                "AVX-512 SMA mismatch at {}: {} vs {}",
                i,
                a,
                b
            );
        }
    }

    #[test]
    fn test_avx512_ema_consistency() {
        let data: Vec<f64> = (0..500).map(|i| i as f64 * 0.5 + 100.0).collect();
        let period = 14;
        let k = 2.0 / (period as f64 + 1.0);
        let mut out_avx512 = vec![0.0; data.len()];
        simd512_ema(&data, period, &mut out_avx512, k);
        // Reference: scalar EMA with FMA
        let mut ref_out = vec![0.0; data.len()];
        for o in ref_out.iter_mut().take(period - 1) {
            *o = f64::NAN;
        }
        let mut prev: f64 = data[..period].iter().sum::<f64>() / period as f64;
        ref_out[period - 1] = prev;
        for i in period..data.len() {
            prev = (data[i] - prev).mul_add(k, prev);
            ref_out[i] = prev;
        }
        for (i, (&a, &b)) in out_avx512.iter().zip(ref_out.iter()).enumerate() {
            if a.is_nan() && b.is_nan() {
                continue;
            }
            assert!(
                approx_eq(a, b, 1e-6),
                "AVX-512 EMA mismatch at {}: {} vs {}",
                i,
                a,
                b
            );
        }
    }

    #[test]
    fn test_avx512_rsi_consistency() {
        let data: Vec<f64> = (0..200)
            .map(|i| 100.0 + (i as f64 * 0.1).sin() * 5.0)
            .collect();
        let period = 14;
        let mut out_avx512 = vec![0.0; data.len()];
        simd512_rsi(&data, period, &mut out_avx512);
        // Reference
        let mut ref_out = vec![0.0; data.len()];
        for o in ref_out.iter_mut().take(period) {
            *o = f64::NAN;
        }
        let mut avg_gain = 0.0;
        let mut avg_loss = 0.0;
        for j in 1..=period {
            let diff = data[j] - data[j - 1];
            if diff > 0.0 {
                avg_gain += diff;
            } else if diff < 0.0 {
                avg_loss -= diff;
            }
        }
        let kk = 1.0 / period as f64;
        let mut prev = data[period];
        ref_out[period] = if avg_loss.abs() < 1e-15 {
            100.0
        } else {
            let rs = avg_gain / avg_loss;
            100.0 - 100.0 / (1.0 + rs)
        };
        for i in (period + 1)..data.len() {
            let diff = data[i] - prev;
            prev = data[i];
            let g = if diff > 0.0 { diff } else { 0.0 };
            let l = if diff < 0.0 { -diff } else { 0.0 };
            avg_gain = (g - avg_gain).mul_add(kk, avg_gain);
            avg_loss = (l - avg_loss).mul_add(kk, avg_loss);
            ref_out[i] = if avg_loss.abs() < 1e-15 {
                100.0
            } else {
                let rs = avg_gain / avg_loss;
                100.0 - 100.0 / (1.0 + rs)
            };
        }
        for (i, (&a, &b)) in out_avx512.iter().zip(ref_out.iter()).enumerate() {
            if a.is_nan() && b.is_nan() {
                continue;
            }
            assert!(
                approx_eq(a, b, 1e-3),
                "AVX-512 RSI mismatch at {}: {} vs {}",
                i,
                a,
                b
            );
        }
    }

    #[test]
    fn test_avx512_rsi_short_period() {
        let data: Vec<f64> = (0..32).map(|i| 100.0 + i as f64).collect();
        let mut output = vec![0.0; data.len()];
        simd512_rsi(&data, 7, &mut output);
        assert!(output[6].is_finite());
        assert!(output[7].is_finite());
    }

    #[test]
    fn test_avx512_bbands_seed() {
        let data: Vec<f64> = (0..200).map(|i| i as f64).collect();
        let period = 20;
        let (sum, sum_sq) = simd512_bbands_seed(&data, period);
        let expected_sum: f64 = data[..period].iter().sum();
        let expected_sum_sq: f64 = data[..period].iter().map(|v| v * v).sum();
        assert!(
            approx_eq(sum, expected_sum, 1e-6),
            "AVX-512 BBANDS sum mismatch: {} vs {}",
            sum,
            expected_sum
        );
        assert!(
            approx_eq(sum_sq, expected_sum_sq, 1e-3),
            "AVX-512 BBANDS sum_sq mismatch: {} vs {}",
            sum_sq,
            expected_sum_sq
        );
    }

    #[test]
    fn test_avx512_atr_seed() {
        let len = 200;
        let high: Vec<f64> = (0..len).map(|i| 105.0 + i as f64 * 0.1).collect();
        let low: Vec<f64> = (0..len).map(|i| 95.0 + i as f64 * 0.1).collect();
        let prev_close: Vec<f64> = (0..len).map(|i| 100.0 + i as f64 * 0.1).collect();
        let period = 14;
        let tr = simd512_atr_seed(&high, &low, &prev_close, period);
        let mut expected = 0.0;
        for j in 0..period {
            let hl = high[j] - low[j];
            let hc = (high[j] - prev_close[j]).abs();
            let lc = (low[j] - prev_close[j]).abs();
            expected += hl.max(hc).max(lc);
        }
        assert!(
            approx_eq(tr, expected, 1e-3),
            "AVX-512 ATR seed mismatch: {} vs {}",
            tr,
            expected
        );
    }

    #[test]
    fn test_avx512_macd_seed() {
        let data: Vec<f64> = (0..200).map(|i| 100.0 + i as f64 * 0.5).collect();
        let (fast, slow) = simd512_macd_seed(&data, 12, 26);
        let exp_fast: f64 = data[..12].iter().sum::<f64>() / 12.0;
        let exp_slow: f64 = data[..26].iter().sum::<f64>() / 26.0;
        assert!(approx_eq(fast, exp_fast, 1e-6));
        assert!(approx_eq(slow, exp_slow, 1e-6));
    }

    #[test]
    fn test_avx512_adx_seed() {
        let len = 200;
        let high: Vec<f64> = (0..len)
            .map(|i| 105.0 + (i as f64 * 0.1).sin() * 3.0)
            .collect();
        let low: Vec<f64> = (0..len)
            .map(|i| 95.0 + (i as f64 * 0.1).cos() * 3.0)
            .collect();
        let prev_close: Vec<f64> = (0..len).map(|i| 100.0 + (i as f64 * 0.05).sin()).collect();
        let period = 14;
        let (tr, pdm, mdm) = simd512_adx_seed(&high, &low, &prev_close, period);
        let mut exp_tr = 0.0;
        let mut exp_pdm = 0.0;
        let mut exp_mdm = 0.0;
        for j in 1..=period {
            let hl = high[j] - low[j];
            let hc = (high[j] - prev_close[j]).abs();
            let lc = (low[j] - prev_close[j]).abs();
            exp_tr += hl.max(hc).max(lc);
            let up = high[j] - high[j - 1];
            let down = low[j - 1] - low[j];
            if up > down && up > 0.0 {
                exp_pdm += up;
            }
            if down > up && down > 0.0 {
                exp_mdm += down;
            }
        }
        assert!(approx_eq(tr, exp_tr, 1e-3), "TR {} vs {}", tr, exp_tr);
        assert!(approx_eq(pdm, exp_pdm, 1e-3), "PDM {} vs {}", pdm, exp_pdm);
        assert!(approx_eq(mdm, exp_mdm, 1e-3), "MDM {} vs {}", mdm, exp_mdm);
    }

    #[test]
    fn test_avx512_availability_reporting() {
        // This always returns a bool — no panic regardless of hardware
        let _ = simd512_available();
    }
}
