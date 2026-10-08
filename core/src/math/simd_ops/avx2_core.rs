//! AVX2 kernels for the basic vector primitives.

use super::prelude::*;

// ============================================================================
// SIMD SMA kernel — rolling sum via AVX2 accumulation
// ============================================================================

/// SIMD-accelerated SMA: uses AVX2 for the initial window sum, then O(1)
/// rolling update per bar. The initial accumulation over `period` elements
/// benefits from 4-wide SIMD addition.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn sma_avx2(input: &[f64], period: usize, output: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = input.len().min(output.len());
    if period == 0 || len < period {
        return;
    }

    let inv_period = 1.0 / period as f64;

    for o in output.iter_mut().take(period - 1) {
        *o = f64::NAN;
    }

    let mut sum = 0.0f64;
    let chunks = period / 4;
    let mut acc = _mm256_setzero_pd();
    for c in 0..chunks {
        let off = c * 4;
        let v = _mm256_loadu_pd(input.as_ptr().add(off));
        acc = _mm256_add_pd(acc, v);
    }
    sum += horizontal_sum_avx2(acc);
    for &val in input.iter().take(period).skip(chunks * 4) {
        sum += val;
    }
    output[period - 1] = sum * inv_period;

    for i in period..len {
        sum += input[i] - input[i - period];
        output[i] = sum * inv_period;
    }
}

// ============================================================================
// SIMD WMA kernel — AVX2-accelerated weighted sum for initial window
// ============================================================================

/// SIMD-accelerated WMA: uses AVX2 for initial weighted accumulation, then
/// O(1) recursive update per bar.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn wma_avx2(input: &[f64], period: usize, output: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = input.len().min(output.len());
    if period == 0 || len < period {
        return;
    }

    let inv_weight_sum = 1.0 / (period * (period + 1) / 2) as f64;
    let p = period as f64;

    for o in output.iter_mut().take(period - 1) {
        *o = f64::NAN;
    }

    let mut window_sum = 0.0f64;
    let mut wsum = 0.0f64;

    let chunks = period / 4;
    let mut ws_acc = _mm256_setzero_pd();
    let mut w_acc = _mm256_setzero_pd();
    for c in 0..chunks {
        let off = c * 4;
        let v = _mm256_loadu_pd(input.as_ptr().add(off));
        ws_acc = _mm256_add_pd(ws_acc, v);
        let weights = _mm256_set_pd(
            (off + 4) as f64,
            (off + 3) as f64,
            (off + 2) as f64,
            (off + 1) as f64,
        );
        w_acc = _mm256_add_pd(w_acc, _mm256_mul_pd(v, weights));
    }
    window_sum += horizontal_sum_avx2(ws_acc);
    wsum += horizontal_sum_avx2(w_acc);
    for j in (chunks * 4)..period {
        window_sum += input[j];
        wsum += (j + 1) as f64 * input[j];
    }
    output[period - 1] = wsum * inv_weight_sum;

    for i in period..len {
        let old = input[i - period];
        let new = input[i];
        wsum += p * new - window_sum;
        window_sum += new - old;
        output[i] = wsum * inv_weight_sum;
    }
}

// AVX2 kernel for prefix_sum / cumsum using block-level parallelism.
// Process 4 f64 at a time: compute local prefix sum within each 4-wide block,
// then broadcast the block's carry-out and add it to every element in the next block.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn prefix_sum_avx2_kernel(data: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if len == 0 {
        return;
    }

    let chunks = len / 4;
    let mut carry = _mm256_setzero_pd();

    for c in 0..chunks {
        let off = c * 4;
        let v = _mm256_loadu_pd(data.as_ptr().add(off));
        // Compute inclusive prefix sum within the 4-lane block:
        // [a, b, c, d] -> [a, a+b, a+b+c, a+b+c+d]
        //
        // Step 1: shift right by 1 within each 128-bit lane and add:
        //   [0, a, 0, c] + [a, b, c, d] = [a, a+b, c, c+d]
        let _shuf1 = _mm256_permute_pd(v, 0b1010);
        // _mm256_permute_pd with imm8: for each 128-bit lane, select from [a,b] pairs
        // imm8 bit 0: lane0 selects a or b for result[0]
        // imm8 bit 1: lane0 selects a or b for result[1]
        // imm8 bit 2: lane1 selects a or b for result[2]
        // imm8 bit 3: lane1 selects a or b for result[3]
        // 0b1010 = bit0=0(a), bit1=1(b), bit2=0(c), bit3=1(d) → [a, b, c, d] (identity)
        // 0b0000 = [a, a, c, c]
        // 0b0101 = [b, b, d, d]
        // We want [0, a, 0, c]: blend [a, a, c, c] with zero, keeping only odd positions
        let dup_even = _mm256_permute_pd(v, 0b0000); // [a, a, c, c]
        let shift1 = _mm256_blend_pd(dup_even, _mm256_setzero_pd(), 0b0101); // [0, a, 0, c]
        let v1 = _mm256_add_pd(v, shift1); // [a, a+b, c, c+d]

        // Step 2: broadcast low lane sum to high lane and add:
        //   [0, 0, a+b, a+b] + [a, a+b, c, c+d] = [a, a+b, a+b+c, a+b+c+d]
        // low_sum = [a, a+b], we need a+b (the sum of the low lane)
        let low_sum = _mm256_castpd256_pd128(v1); // [a, a+b]
        let _low_lane_total = _mm_add_sd(low_sum, _mm_unpackhi_pd(low_sum, low_sum));
        // Actually we want just a+b from position [1] of low_sum
        // Use _mm_shuffle_pd to get element [1] to position [0], then broadcast
        let low_sum_shuffled = _mm_shuffle_pd(low_sum, low_sum, 0b01); // [a+b, a]
        let low_sum_bcast = _mm256_broadcastsd_pd(low_sum_shuffled); // [a+b, a+b, a+b, a+b]
        let shift2 = _mm256_blend_pd(_mm256_setzero_pd(), low_sum_bcast, 0b1100); // [0, 0, a+b, a+b]
        let v2 = _mm256_add_pd(v1, shift2); // [a, a+b, a+b+c, a+b+c+d]

        // Add carry from previous block to all 4 elements
        let scanned = _mm256_add_pd(v2, carry);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), scanned);

        // New carry = last element of scanned block (element [3])
        // Extract high 128-bit lane, then get element [1] of that lane
        let high_lane = _mm256_extractf128_pd(scanned, 1); // [scanned[2], scanned[3]]
        let last_elem = _mm_shuffle_pd(high_lane, high_lane, 0b01); // [scanned[3], scanned[2]]
        carry = _mm256_broadcastsd_pd(last_elem); // [scanned[3], scanned[3], scanned[3], scanned[3]]
    }

    // Handle remaining elements
    let mut acc: f64 = if chunks > 0 {
        let last: [f64; 4] = core::mem::transmute(carry);
        last[0]
    } else {
        0.0
    };
    for i in (chunks * 4)..len {
        acc += data[i];
        result[i] = acc;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn diff_avx2(data: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if len == 0 {
        return;
    }
    result[0] = f64::NAN;
    let body = len - 1;
    let chunks = body / 4;
    for i in 0..chunks {
        let off = i * 4 + 1;
        let va = _mm256_loadu_pd(data.as_ptr().add(off));
        let vb = _mm256_loadu_pd(data.as_ptr().add(off - 1));
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_sub_pd(va, vb));
    }
    for i in (chunks * 4 + 1)..len {
        result[i] = data[i] - data[i - 1];
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn scale_avx2(data: &[f64], factor: f64, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    let chunks = len / 4;
    let vf = _mm256_set1_pd(factor);
    for i in 0..chunks {
        let off = i * 4;
        let v = _mm256_loadu_pd(data.as_ptr().add(off));
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_mul_pd(v, vf));
    }
    for i in (chunks * 4)..len {
        result[i] = data[i] * factor;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn pct_change_avx2(data: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if len == 0 {
        return;
    }
    result[0] = f64::NAN;
    let body = len - 1;
    let chunks = body / 4;
    let hundred = _mm256_set1_pd(100.0);
    let sign_mask = _mm256_castsi256_pd(_mm256_set1_epi64x(i64::MIN));
    let eps = _mm256_set1_pd(crate::utils::NUMERIC_EPSILON);
    let nan = _mm256_set1_pd(f64::NAN);
    for i in 0..chunks {
        let off = i * 4 + 1;
        let curr = _mm256_loadu_pd(data.as_ptr().add(off));
        let prev = _mm256_loadu_pd(data.as_ptr().add(off - 1));
        let diff = _mm256_sub_pd(curr, prev);
        let abs_prev = _mm256_andnot_pd(sign_mask, prev);
        let near_zero = _mm256_cmp_pd(abs_prev, eps, _CMP_LT_OS);
        let ratio = _mm256_div_pd(diff, prev);
        let pct = _mm256_mul_pd(ratio, hundred);
        let blended = _mm256_blendv_pd(pct, nan, near_zero);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), blended);
    }
    for i in (chunks * 4 + 1)..len {
        result[i] = if data[i - 1].abs() < crate::utils::NUMERIC_EPSILON {
            f64::NAN
        } else {
            (data[i] - data[i - 1]) / data[i - 1] * 100.0
        };
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn clamp_avx2(data: &[f64], lo: f64, hi: f64, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    let chunks = len / 4;
    let vlo = _mm256_set1_pd(lo);
    let vhi = _mm256_set1_pd(hi);
    for i in 0..chunks {
        let off = i * 4;
        let v = _mm256_loadu_pd(data.as_ptr().add(off));
        let clamped = _mm256_min_pd(_mm256_max_pd(v, vlo), vhi);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), clamped);
    }
    for i in (chunks * 4)..len {
        result[i] = data[i].max(lo).min(hi);
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn weighted_sum_avx2(data: &[f64], weights: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(weights.len()).min(result.len());
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        let vd = _mm256_loadu_pd(data.as_ptr().add(off));
        let vw = _mm256_loadu_pd(weights.as_ptr().add(off));
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_mul_pd(vd, vw));
    }
    for i in (chunks * 4)..len {
        result[i] = data[i] * weights[i];
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn true_range_avx2(
    high: &[f64],
    low: &[f64],
    prev_close: &[f64],
    result: &mut [f64],
) {
    use core::arch::x86_64::*;
    let len = high
        .len()
        .min(low.len())
        .min(prev_close.len())
        .min(result.len());
    let chunks = len / 4;
    let sign_mask = _mm256_castsi256_pd(_mm256_set1_epi64x(i64::MIN));
    for i in 0..chunks {
        let off = i * 4;
        let vh = _mm256_loadu_pd(high.as_ptr().add(off));
        let vl = _mm256_loadu_pd(low.as_ptr().add(off));
        let vpc = _mm256_loadu_pd(prev_close.as_ptr().add(off));
        let hl = _mm256_sub_pd(vh, vl);
        let hpc = _mm256_andnot_pd(sign_mask, _mm256_sub_pd(vh, vpc));
        let lpc = _mm256_andnot_pd(sign_mask, _mm256_sub_pd(vl, vpc));
        let tr = _mm256_max_pd(hl, _mm256_max_pd(hpc, lpc));
        _mm256_storeu_pd(result.as_mut_ptr().add(off), tr);
    }
    for i in (chunks * 4)..len {
        let hl = high[i] - low[i];
        let hpc = (high[i] - prev_close[i]).abs();
        let lpc = (low[i] - prev_close[i]).abs();
        result[i] = hl.max(hpc).max(lpc);
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn typical_price_avx2(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    result: &mut [f64],
) {
    use core::arch::x86_64::*;
    let len = high.len().min(low.len()).min(close.len()).min(result.len());
    let chunks = len / 4;
    let three = _mm256_set1_pd(3.0);
    for i in 0..chunks {
        let off = i * 4;
        let vh = _mm256_loadu_pd(high.as_ptr().add(off));
        let vl = _mm256_loadu_pd(low.as_ptr().add(off));
        let vc = _mm256_loadu_pd(close.as_ptr().add(off));
        let sum = _mm256_add_pd(_mm256_add_pd(vh, vl), vc);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_div_pd(sum, three));
    }
    for i in (chunks * 4)..len {
        result[i] = (high[i] + low[i] + close[i]) / 3.0;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn median_price_avx2(high: &[f64], low: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = high.len().min(low.len()).min(result.len());
    let chunks = len / 4;
    let two = _mm256_set1_pd(2.0);
    for i in 0..chunks {
        let off = i * 4;
        let vh = _mm256_loadu_pd(high.as_ptr().add(off));
        let vl = _mm256_loadu_pd(low.as_ptr().add(off));
        _mm256_storeu_pd(
            result.as_mut_ptr().add(off),
            _mm256_div_pd(_mm256_add_pd(vh, vl), two),
        );
    }
    for i in (chunks * 4)..len {
        result[i] = (high[i] + low[i]) / 2.0;
    }
}

// AVX2 kernel for log_return: uses AVX2 division for ratio computation,
// then scalar ln() since AVX2 has no hardware log instruction.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn log_return_avx2_kernel(data: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if len == 0 {
        return;
    }
    result[0] = f64::NAN;
    if len < 2 {
        return;
    }

    let body = len - 1;
    let chunks = body / 4;
    let nan = _mm256_set1_pd(f64::NAN);

    // AVX2 pass: compute ratios (curr/prev) for aligned chunks
    for i in 0..chunks {
        let off = i * 4 + 1;
        let curr = _mm256_loadu_pd(data.as_ptr().add(off));
        let prev = _mm256_loadu_pd(data.as_ptr().add(off - 1));
        let zero = _mm256_setzero_pd();
        let prev_pos = _mm256_cmp_pd(prev, zero, _CMP_GT_OS);
        let curr_pos = _mm256_cmp_pd(curr, zero, _CMP_GT_OS);
        let valid = _mm256_and_pd(prev_pos, curr_pos);
        let ratio = _mm256_div_pd(curr, prev);
        let ratio_or_nan = _mm256_blendv_pd(nan, ratio, valid);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), ratio_or_nan);
    }

    // Scalar ln() pass over AVX2-computed ratios
    for i in 1..=(chunks * 4) {
        if i < len && !result[i].is_nan() {
            result[i] = f64_ln(result[i]);
        }
    }

    // Handle tail elements that didn't fit in AVX2 chunks
    for i in (chunks * 4 + 1)..len {
        result[i] = if data[i - 1] > 0.0 && data[i] > 0.0 {
            f64_ln(data[i] / data[i - 1])
        } else {
            f64::NAN
        };
    }
}

// AVX2 kernel for cumsum: identical to prefix_sum (cumulative sum).
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn cumsum_avx2_kernel(data: &[f64], result: &mut [f64]) {
    prefix_sum_avx2_kernel(data, result)
}

// AVX2-accelerated OBV: vectorised delta computation followed by SIMD prefix
// sum. The deltas (sign(close[i]-close[i-1]) * volume[i]) are computed in
// chunks of 4 with branchless blends, then the running total comes from the
// same AVX2 prefix-sum kernel used by AD line.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn obv_core_avx2(close: &[f64], volume: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = close.len().min(volume.len()).min(result.len());
    if len == 0 {
        return;
    }

    // Reuse the caller-owned result as the delta scratch buffer. The prefix
    // scan below is alias-safe because it loads one complete SIMD block before
    // storing that same block, then advances monotonically.
    result[0] = volume[0];
    let result_ptr = result.as_mut_ptr();
    let zero = _mm256_setzero_pd();

    // Process 4 close deltas per iteration. Each lane compares close[i] vs
    // close[i-1] and produces ±volume[i] (or 0 if equal). Because the second
    // lane of every chunk needs close[i-1] from the previous lane, we always
    // load 5 close values for every 4 deltas, then blend based on the signed
    // comparison mask.
    if len >= 5 {
        let chunks = (len - 1) / 4;
        for c in 0..chunks {
            let off = c * 4;
            // Curr = close[off+1 .. off+5]
            let curr = _mm256_loadu_pd(close.as_ptr().add(off + 1));
            // Prev = close[off .. off+4]
            let prev = _mm256_loadu_pd(close.as_ptr().add(off));
            let vol = _mm256_loadu_pd(volume.as_ptr().add(off + 1));
            let diff = _mm256_sub_pd(curr, prev);
            let pos = _mm256_cmp_pd(diff, zero, _CMP_GT_OS);
            let neg = _mm256_cmp_pd(diff, zero, _CMP_LT_OS);
            // +vol where pos, -vol where neg, 0 where equal
            let plus = _mm256_and_pd(vol, pos);
            let minus = _mm256_and_pd(vol, neg);
            let signed = _mm256_sub_pd(plus, minus);
            _mm256_storeu_pd(result_ptr.add(off + 1), signed);
        }
    }
    // Scalar tail for the very last partial chunk.
    for i in ((((len.saturating_sub(1)) / 4) * 4 + 1).max(1))..len {
        let diff = close[i] - close[i - 1];
        result[i] = if diff > 0.0 {
            volume[i]
        } else if diff < 0.0 {
            -volume[i]
        } else {
            0.0
        };
    }

    let deltas = core::slice::from_raw_parts(result.as_ptr(), len);
    prefix_sum_avx2_kernel(deltas, result);
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn roc_avx2(data: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    for r in result.iter_mut().take(period.min(len)) {
        *r = f64::NAN;
    }
    let body = len.saturating_sub(period);
    let chunks = body / 4;
    let hundred = _mm256_set1_pd(100.0);
    let sign_mask = _mm256_castsi256_pd(_mm256_set1_epi64x(i64::MIN));
    let eps = _mm256_set1_pd(crate::utils::NUMERIC_EPSILON);
    let nan = _mm256_set1_pd(f64::NAN);
    for i in 0..chunks {
        let off = i * 4 + period;
        let curr = _mm256_loadu_pd(data.as_ptr().add(off));
        let prev = _mm256_loadu_pd(data.as_ptr().add(off - period));
        let diff = _mm256_sub_pd(curr, prev);
        let abs_prev = _mm256_andnot_pd(sign_mask, prev);
        let near_zero = _mm256_cmp_pd(abs_prev, eps, _CMP_LT_OS);
        let ratio = _mm256_div_pd(diff, prev);
        let pct = _mm256_mul_pd(ratio, hundred);
        let blended = _mm256_blendv_pd(pct, nan, near_zero);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), blended);
    }
    for i in (chunks * 4 + period)..len {
        result[i] = if data[i - period].abs() < crate::utils::NUMERIC_EPSILON {
            f64::NAN
        } else {
            (data[i] - data[i - period]) / data[i - period] * 100.0
        };
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn floor_avx2(data: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let n = data.len().min(result.len());
    let mut i = 0usize;
    while i + 4 <= n {
        let values = _mm256_loadu_pd(data.as_ptr().add(i));
        _mm256_storeu_pd(result.as_mut_ptr().add(i), _mm256_floor_pd(values));
        i += 4;
    }
    while i < n {
        *result.get_unchecked_mut(i) = f64_floor(*data.get_unchecked(i));
        i += 1;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn ceil_avx2(data: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let n = data.len().min(result.len());
    let mut i = 0usize;
    while i + 4 <= n {
        let values = _mm256_loadu_pd(data.as_ptr().add(i));
        _mm256_storeu_pd(result.as_mut_ptr().add(i), _mm256_ceil_pd(values));
        i += 4;
    }
    while i < n {
        *result.get_unchecked_mut(i) = f64_ceil(*data.get_unchecked(i));
        i += 1;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn first_non_finite_avx2(data: &[f64]) -> Option<usize> {
    use core::arch::x86_64::*;

    // `|x| < +inf` is true for every finite value and for nothing else: both
    // `NaN` (all comparisons false) and `±inf` (equal, not less) fail it.
    let infinity = _mm256_set1_pd(f64::INFINITY);
    let abs_mask = _mm256_set1_pd(-0.0);
    let mut index = 0usize;
    while index + 4 <= data.len() {
        let values = _mm256_loadu_pd(data.as_ptr().add(index));
        let absolute = _mm256_andnot_pd(abs_mask, values);
        let finite = _mm256_cmp_pd(absolute, infinity, _CMP_LT_OQ);
        if _mm256_movemask_pd(finite) != 0b1111 {
            for offset in 0..4 {
                if !data.get_unchecked(index + offset).is_finite() {
                    return Some(index + offset);
                }
            }
        }
        index += 4;
    }
    while index < data.len() {
        if !data.get_unchecked(index).is_finite() {
            return Some(index);
        }
        index += 1;
    }
    None
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn count_less_avx2(data: &[f64], cutoff: f64) -> usize {
    use core::arch::x86_64::*;

    // `_CMP_LT_OQ` is false whenever either operand is `NaN`, which is exactly
    // the predicate the scalar form applies: a missing bar is never below the
    // cutoff. No extra NaN test is needed.
    let bound = _mm256_set1_pd(cutoff);
    let mut count = 0usize;
    let mut index = 0usize;
    while index + 4 <= data.len() {
        let values = _mm256_loadu_pd(data.as_ptr().add(index));
        let below = _mm256_cmp_pd(values, bound, _CMP_LT_OQ);
        count += (_mm256_movemask_pd(below) as u32).count_ones() as usize;
        index += 4;
    }
    while index < data.len() {
        if *data.get_unchecked(index) < cutoff {
            count += 1;
        }
        index += 1;
    }
    count
}

// ============================================================================
// SIMD sin/cos for HT_SINE terminal stage
// ============================================================================
//
// `compute_hilbert_components` produces `phase = atan(im/re)`, which is always
// bounded to (-π/2, π/2). We still implement a general, branchless range
// reduction so the primitive is reusable:
//   * reduce |x| to z ∈ [-π/4, π/4] via the nearest multiple of π/2,
//   * evaluate sin/cos with degree-13/12 Taylor polynomials on z²,
//   * select the correct quadrant with branchless blends.
// Absolute error for |x| <= π/2 is <= 1e-11 (well within the 1e-9 SLA).

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn simd_sin_cos_avx2(input: &[f64], sin_out: &mut [f64], cos_out: &mut [f64]) {
    use core::arch::x86_64::*;
    let n = input.len().min(sin_out.len()).min(cos_out.len());
    if n == 0 {
        return;
    }

    let two_over_pi = 2.0 / core::f64::consts::PI;
    let two_over_pi_v = _mm256_set1_pd(two_over_pi);
    let half_pi_v = _mm256_set1_pd(core::f64::consts::FRAC_PI_2);
    let sign_mask = _mm256_castsi256_pd(_mm256_set1_epi64x(i64::MIN));
    let one = _mm256_set1_pd(1.0);
    let neg_one = _mm256_set1_pd(-1.0);
    let quarter = _mm256_set1_pd(0.25);
    let four = _mm256_set1_pd(4.0);

    // sin polynomial coefficients (Horner on z2): 1 - z2/6 + z2^2/120 - ... + z2^6/13!
    let s0 = _mm256_set1_pd(1.0);
    let s1 = _mm256_set1_pd(-1.0 / 6.0);
    let s2 = _mm256_set1_pd(1.0 / 120.0);
    let s3 = _mm256_set1_pd(-1.0 / 5040.0);
    let s4 = _mm256_set1_pd(1.0 / 362880.0);
    let s5 = _mm256_set1_pd(-1.0 / 39916800.0);
    let s6 = _mm256_set1_pd(1.0 / 6227020800.0);

    // cos polynomial coefficients: 1 - z2/2 + z2^2/24 - ... + z2^6/12!
    let c0 = _mm256_set1_pd(1.0);
    let c1 = _mm256_set1_pd(-0.5);
    let c2 = _mm256_set1_pd(1.0 / 24.0);
    let c3 = _mm256_set1_pd(-1.0 / 720.0);
    let c4 = _mm256_set1_pd(1.0 / 40320.0);
    let c5 = _mm256_set1_pd(-1.0 / 3628800.0);
    let c6 = _mm256_set1_pd(1.0 / 479001600.0);

    let mut i = 0;
    let chunks = n / 4;
    for _ in 0..chunks {
        let x = _mm256_loadu_pd(input.as_ptr().add(i));

        // |x| and the sign of x (for sin, which is odd)
        let xabs = _mm256_andnot_pd(sign_mask, x);
        let sin_sign = _mm256_or_pd(one, _mm256_and_pd(sign_mask, x));

        // k = nearest multiple of π/2; z = |x| - k·(π/2) ∈ [-π/4, π/4]
        let y = _mm256_mul_pd(xabs, two_over_pi_v);
        let k_f = _mm256_round_pd(y, _MM_FROUND_TO_NEAREST_INT | _MM_FROUND_NO_EXC);
        let z = _mm256_sub_pd(xabs, _mm256_mul_pd(k_f, half_pi_v));

        let z2 = _mm256_mul_pd(z, z);

        // sin(z)
        let mut poly_s = s6;
        poly_s = _mm256_mul_pd(poly_s, z2);
        poly_s = _mm256_add_pd(poly_s, s5);
        poly_s = _mm256_mul_pd(poly_s, z2);
        poly_s = _mm256_add_pd(poly_s, s4);
        poly_s = _mm256_mul_pd(poly_s, z2);
        poly_s = _mm256_add_pd(poly_s, s3);
        poly_s = _mm256_mul_pd(poly_s, z2);
        poly_s = _mm256_add_pd(poly_s, s2);
        poly_s = _mm256_mul_pd(poly_s, z2);
        poly_s = _mm256_add_pd(poly_s, s1);
        poly_s = _mm256_mul_pd(poly_s, z2);
        poly_s = _mm256_add_pd(poly_s, s0);
        let sinz = _mm256_mul_pd(z, poly_s);

        // cos(z)
        let mut poly_c = c6;
        poly_c = _mm256_mul_pd(poly_c, z2);
        poly_c = _mm256_add_pd(poly_c, c5);
        poly_c = _mm256_mul_pd(poly_c, z2);
        poly_c = _mm256_add_pd(poly_c, c4);
        poly_c = _mm256_mul_pd(poly_c, z2);
        poly_c = _mm256_add_pd(poly_c, c3);
        poly_c = _mm256_mul_pd(poly_c, z2);
        poly_c = _mm256_add_pd(poly_c, c2);
        poly_c = _mm256_mul_pd(poly_c, z2);
        poly_c = _mm256_add_pd(poly_c, c1);
        poly_c = _mm256_mul_pd(poly_c, z2);
        poly_c = _mm256_add_pd(poly_c, c0);
        let cosz = poly_c;

        // km4 = k mod 4 ∈ {0,1,2,3}
        let kf_div4 = _mm256_mul_pd(k_f, quarter);
        let kf_floor = _mm256_round_pd(kf_div4, _MM_FROUND_FLOOR | _MM_FROUND_NO_EXC);
        let km4 = _mm256_sub_pd(k_f, _mm256_mul_pd(kf_floor, four));

        // Quadrant masks
        let m1 = _mm256_cmp_pd(km4, _mm256_set1_pd(1.0), _CMP_EQ_OQ);
        let m2 = _mm256_cmp_pd(km4, _mm256_set1_pd(2.0), _CMP_EQ_OQ);
        let m3 = _mm256_cmp_pd(km4, _mm256_set1_pd(3.0), _CMP_EQ_OQ);

        // Sine: value = (m1||m3) ? cosz : sinz ; sign = (m2||m3) ? -1 : +1
        let use_cos = _mm256_or_pd(m1, m3);
        let sin_base = _mm256_blendv_pd(sinz, cosz, use_cos);
        let sin_neg = _mm256_blendv_pd(one, neg_one, _mm256_or_pd(m2, m3));
        let sin_abs = _mm256_mul_pd(sin_base, sin_neg);
        let sin_result = _mm256_mul_pd(sin_abs, sin_sign);

        // Cosine: value = (m1||m3) ? sinz : cosz ; sign = (m1||m2) ? -1 : +1
        let cos_base = _mm256_blendv_pd(cosz, sinz, use_cos);
        let cos_neg = _mm256_blendv_pd(one, neg_one, _mm256_or_pd(m1, m2));
        let cos_result = _mm256_mul_pd(cos_base, cos_neg);

        _mm256_storeu_pd(sin_out.as_mut_ptr().add(i), sin_result);
        _mm256_storeu_pd(cos_out.as_mut_ptr().add(i), cos_result);
        i += 4;
    }

    // Scalar tail for the remaining < 4 elements
    for j in i..n {
        let (s, c) = f64_sin_cos(input[j]);
        sin_out[j] = s;
        cos_out[j] = c;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn simd_sqrt_avx2(input: &[f64], output: &mut [f64]) {
    use core::arch::x86_64::*;
    let n = input.len().min(output.len());
    let input_ptr = input.as_ptr();
    let output_ptr = output.as_mut_ptr();
    let mut i = 0usize;
    while i + 4 <= n {
        let values = _mm256_loadu_pd(input_ptr.add(i));
        let roots = _mm256_sqrt_pd(values);
        _mm256_storeu_pd(output_ptr.add(i), roots);
        i += 4;
    }
    while i < n {
        *output_ptr.add(i) = f64_sqrt(*input_ptr.add(i));
        i += 1;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn simd_sqrt_avx2_checked(input: &[f64], output: &mut [f64]) -> Option<usize> {
    use core::arch::x86_64::*;

    let n = input.len().min(output.len());
    let input_ptr = input.as_ptr();
    let output_ptr = output.as_mut_ptr();
    let zero = _mm256_setzero_pd();
    let exponent_mask = _mm256_set1_epi64x(0x7ff0_0000_0000_0000u64 as i64);
    let mut i = 0usize;
    while i + 4 <= n {
        let values = _mm256_loadu_pd(input_ptr.add(i));
        let bits = _mm256_castpd_si256(values);
        let negative = _mm256_castpd_si256(_mm256_cmp_pd(values, zero, _CMP_LT_OQ));
        let non_finite = _mm256_cmpeq_epi64(_mm256_and_si256(bits, exponent_mask), exponent_mask);
        let invalid = _mm256_or_si256(negative, non_finite);
        let invalid_mask = _mm256_movemask_pd(_mm256_castsi256_pd(invalid));
        if invalid_mask != 0 {
            for lane in 0..4 {
                let value = *input_ptr.add(i + lane);
                if !value.is_finite() || value < 0.0 {
                    return Some(i + lane);
                }
            }
        }
        _mm256_storeu_pd(output_ptr.add(i), _mm256_sqrt_pd(values));
        i += 4;
    }
    while i < n {
        let value = *input_ptr.add(i);
        if !value.is_finite() || value < 0.0 {
            return Some(i);
        }
        *output_ptr.add(i) = value.sqrt();
        i += 1;
    }
    None
}

// ============================================================================
// SIMD Ultimate Oscillator raw series (bp / tr pre-pass)
// ============================================================================
//
//   bp[i] = close[i] - min(low[i], close[i-1])
//   tr[i] = max(high[i], close[i-1]) - min(low[i], close[i-1])   (i >= 1)
//   bp[0] = tr[0] = 0
//
// Elementwise given prev_close, so the AVX2 path is bit-identical to the
// scalar form. The only cross-element dependency is the `close[i-1]` shift,
// which is handled by loading the close lane shifted by one element.

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn simd_bp_tr_avx2(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    bp: &mut [f64],
    tr: &mut [f64],
    len: usize,
) {
    use core::arch::x86_64::*;
    if len == 0 {
        return;
    }
    bp[0] = 0.0;
    tr[0] = 0.0;

    let high_p = high.as_ptr();
    let low_p = low.as_ptr();
    let close_p = close.as_ptr();
    let bp_p = bp.as_mut_ptr();
    let tr_p = tr.as_mut_ptr();

    let mut i = 1usize;
    let last_block_start = len.saturating_sub(4);
    while i <= last_block_start {
        let low_v = _mm256_loadu_pd(low_p.add(i));
        let high_v = _mm256_loadu_pd(high_p.add(i));
        let close_v = _mm256_loadu_pd(close_p.add(i));
        // prev_close for lane k = close[i - 1 + k] = close[elem - 1]
        let prev_v = _mm256_loadu_pd(close_p.add(i - 1));

        let min_lp = _mm256_min_pd(low_v, prev_v);
        let bp_v = _mm256_sub_pd(close_v, min_lp);
        let max_hp = _mm256_max_pd(high_v, prev_v);
        let tr_v = _mm256_sub_pd(max_hp, min_lp);

        _mm256_storeu_pd(bp_p.add(i), bp_v);
        _mm256_storeu_pd(tr_p.add(i), tr_v);
        i += 4;
    }

    for j in i..len {
        let prev_close = *close_p.add(j - 1);
        let tl = (*low_p.add(j)).min(prev_close);
        *bp_p.add(j) = *close_p.add(j) - tl;
        *tr_p.add(j) = (*high_p.add(j)).max(prev_close) - tl;
    }
}
