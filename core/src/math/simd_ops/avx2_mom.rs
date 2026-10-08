//! AVX2 kernels for momentum, BOP, average price and CMO.

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn ema_next_avx2(prev: f64, sample: f64, k: f64) -> f64 {
    // 单步版本：单条 fmadd 与标量等价，但保留 `_mm256_fmadd_pd` 路径，
    // 便于上层批量化循环进一步展开。
    use core::arch::x86_64::*;
    let prev_v = _mm256_set1_pd(prev);
    let sample_v = _mm256_set1_pd(sample);
    let k_v = _mm256_set1_pd(k);
    let diff = _mm256_sub_pd(sample_v, prev_v);
    let out = _mm256_fmadd_pd(k_v, diff, prev_v);
    _mm256_cvtsd_f64(out)
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn cmo_avx2(src: &[f64], period: usize, out: &mut [f64], len: usize) {
    use core::arch::x86_64::*;
    for i in period..len {
        let mut up_v = _mm256_setzero_pd();
        let mut down_v = _mm256_setzero_pd();
        let start = i - period + 1;
        let end = i;
        let mut k = start;
        // 处理 4 元素对齐块
        while k + 4 <= end + 1 {
            let curr = _mm256_loadu_pd(src.as_ptr().add(k));
            let prev = _mm256_loadu_pd(src.as_ptr().add(k - 1));
            let diff = _mm256_sub_pd(curr, prev);
            let zero = _mm256_setzero_pd();
            // up_mask: diff > 0 → diff; else 0
            let pos = _mm256_max_pd(diff, zero);
            let neg = _mm256_sub_pd(zero, _mm256_min_pd(diff, zero));
            up_v = _mm256_add_pd(up_v, pos);
            down_v = _mm256_add_pd(down_v, neg);
            k += 4;
        }
        // 标量收尾
        let mut up = 0.0f64;
        let mut down = 0.0f64;
        while k <= end {
            let diff = src[k] - src[k - 1];
            if diff > 0.0 {
                up += diff;
            } else {
                down += -diff;
            }
            k += 1;
        }
        // 横向求和
        let mut up_arr = [0.0f64; 4];
        let mut down_arr = [0.0f64; 4];
        _mm256_storeu_pd(up_arr.as_mut_ptr(), up_v);
        _mm256_storeu_pd(down_arr.as_mut_ptr(), down_v);
        up += up_arr.iter().sum::<f64>();
        down += down_arr.iter().sum::<f64>();
        let denom = up + down;
        out[i] = if denom > crate::utils::NUMERIC_EPSILON {
            (up - down) / denom * 100.0
        } else {
            0.0
        };
    }
}

/// Fixed-period MOM kernel for the public default (10 bars).
///
/// The general AVX2 kernel must subtract a runtime period for every vector.
/// MOM10 is the hot path used by the Python release gate, so keeping the
/// offset constant lets LLVM fold those address calculations into the load
/// addressing mode while retaining the same output and warm-up semantics.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn mom10_avx2(input: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    const PERIOD: usize = 10;
    let len = input.len().min(result.len());
    if len <= PERIOD {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    for r in result.iter_mut().take(PERIOD) {
        *r = f64::NAN;
    }

    let mut current_ptr = input.as_ptr().add(PERIOD);
    let mut previous_ptr = input.as_ptr();
    let mut out_ptr = result.as_mut_ptr().add(PERIOD);
    let mut remaining = len - PERIOD;
    while remaining >= 32 {
        let v0 = _mm256_sub_pd(_mm256_loadu_pd(current_ptr), _mm256_loadu_pd(previous_ptr));
        let v1 = _mm256_sub_pd(
            _mm256_loadu_pd(current_ptr.add(4)),
            _mm256_loadu_pd(previous_ptr.add(4)),
        );
        let v2 = _mm256_sub_pd(
            _mm256_loadu_pd(current_ptr.add(8)),
            _mm256_loadu_pd(previous_ptr.add(8)),
        );
        let v3 = _mm256_sub_pd(
            _mm256_loadu_pd(current_ptr.add(12)),
            _mm256_loadu_pd(previous_ptr.add(12)),
        );
        let v4 = _mm256_sub_pd(
            _mm256_loadu_pd(current_ptr.add(16)),
            _mm256_loadu_pd(previous_ptr.add(16)),
        );
        let v5 = _mm256_sub_pd(
            _mm256_loadu_pd(current_ptr.add(20)),
            _mm256_loadu_pd(previous_ptr.add(20)),
        );
        let v6 = _mm256_sub_pd(
            _mm256_loadu_pd(current_ptr.add(24)),
            _mm256_loadu_pd(previous_ptr.add(24)),
        );
        let v7 = _mm256_sub_pd(
            _mm256_loadu_pd(current_ptr.add(28)),
            _mm256_loadu_pd(previous_ptr.add(28)),
        );
        _mm256_storeu_pd(out_ptr, v0);
        _mm256_storeu_pd(out_ptr.add(4), v1);
        _mm256_storeu_pd(out_ptr.add(8), v2);
        _mm256_storeu_pd(out_ptr.add(12), v3);
        _mm256_storeu_pd(out_ptr.add(16), v4);
        _mm256_storeu_pd(out_ptr.add(20), v5);
        _mm256_storeu_pd(out_ptr.add(24), v6);
        _mm256_storeu_pd(out_ptr.add(28), v7);
        current_ptr = current_ptr.add(32);
        previous_ptr = previous_ptr.add(32);
        out_ptr = out_ptr.add(32);
        remaining -= 32;
    }
    while remaining >= 4 {
        let current = _mm256_loadu_pd(current_ptr);
        let previous = _mm256_loadu_pd(previous_ptr);
        _mm256_storeu_pd(out_ptr, _mm256_sub_pd(current, previous));
        current_ptr = current_ptr.add(4);
        previous_ptr = previous_ptr.add(4);
        out_ptr = out_ptr.add(4);
        remaining -= 4;
    }
    while remaining != 0 {
        *out_ptr = *current_ptr - *previous_ptr;
        current_ptr = current_ptr.add(1);
        previous_ptr = previous_ptr.add(1);
        out_ptr = out_ptr.add(1);
        remaining -= 1;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn mom_avx2(input: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = input.len().min(result.len());
    if period == 0 || len <= period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    // Fill NaN for warmup period
    for r in result.iter_mut().take(period) {
        *r = f64::NAN;
    }

    let ptr = input.as_ptr();
    let out_ptr = result.as_mut_ptr();

    // Process eight AVX2 vectors per iteration. The wider unroll keeps the
    // tiny bandwidth-bound kernel out of the loop-control bottleneck on the
    // installed-wheel benchmark sizes.
    let chunks = (len - period) / 4;
    let unrolled_end = period + (chunks / 8) * 32;
    let mut i = period;
    while i < unrolled_end {
        let v0 = _mm256_sub_pd(
            _mm256_loadu_pd(ptr.add(i)),
            _mm256_loadu_pd(ptr.add(i - period)),
        );
        let v1 = _mm256_sub_pd(
            _mm256_loadu_pd(ptr.add(i + 4)),
            _mm256_loadu_pd(ptr.add(i + 4 - period)),
        );
        let v2 = _mm256_sub_pd(
            _mm256_loadu_pd(ptr.add(i + 8)),
            _mm256_loadu_pd(ptr.add(i + 8 - period)),
        );
        let v3 = _mm256_sub_pd(
            _mm256_loadu_pd(ptr.add(i + 12)),
            _mm256_loadu_pd(ptr.add(i + 12 - period)),
        );
        let v4 = _mm256_sub_pd(
            _mm256_loadu_pd(ptr.add(i + 16)),
            _mm256_loadu_pd(ptr.add(i + 16 - period)),
        );
        let v5 = _mm256_sub_pd(
            _mm256_loadu_pd(ptr.add(i + 20)),
            _mm256_loadu_pd(ptr.add(i + 20 - period)),
        );
        let v6 = _mm256_sub_pd(
            _mm256_loadu_pd(ptr.add(i + 24)),
            _mm256_loadu_pd(ptr.add(i + 24 - period)),
        );
        let v7 = _mm256_sub_pd(
            _mm256_loadu_pd(ptr.add(i + 28)),
            _mm256_loadu_pd(ptr.add(i + 28 - period)),
        );
        _mm256_storeu_pd(out_ptr.add(i), v0);
        _mm256_storeu_pd(out_ptr.add(i + 4), v1);
        _mm256_storeu_pd(out_ptr.add(i + 8), v2);
        _mm256_storeu_pd(out_ptr.add(i + 12), v3);
        _mm256_storeu_pd(out_ptr.add(i + 16), v4);
        _mm256_storeu_pd(out_ptr.add(i + 20), v5);
        _mm256_storeu_pd(out_ptr.add(i + 24), v6);
        _mm256_storeu_pd(out_ptr.add(i + 28), v7);
        i += 32;
    }
    let vector_end = period + chunks * 4;
    while i < vector_end {
        let v_curr = _mm256_loadu_pd(ptr.add(i));
        let v_prev = _mm256_loadu_pd(ptr.add(i - period));
        _mm256_storeu_pd(out_ptr.add(i), _mm256_sub_pd(v_curr, v_prev));
        i += 4;
    }

    // Handle remaining elements
    while i < len {
        result[i] = input[i] - input[i - period];
        i += 1;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn bop_avx2(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    result: &mut [f64],
) {
    use core::arch::x86_64::*;
    let len = open
        .len()
        .min(high.len())
        .min(low.len())
        .min(close.len())
        .min(result.len());
    let _zero = _mm256_setzero_pd();
    let epsilon = _mm256_set1_pd(1e-15);

    let o_ptr = open.as_ptr();
    let h_ptr = high.as_ptr();
    let l_ptr = low.as_ptr();
    let c_ptr = close.as_ptr();
    let out_ptr = result.as_mut_ptr();

    let chunks = len / 4;
    for c in 0..chunks {
        let i = c * 4;
        let vo = _mm256_loadu_pd(o_ptr.add(i));
        let vh = _mm256_loadu_pd(h_ptr.add(i));
        let vl = _mm256_loadu_pd(l_ptr.add(i));
        let vc = _mm256_loadu_pd(c_ptr.add(i));

        let range = _mm256_sub_pd(vh, vl);
        let range_abs = _mm256_andnot_pd(_mm256_set1_pd(-0.0), range);
        let mask = _mm256_cmp_pd(range_abs, epsilon, _CMP_GT_OQ);

        let numerator = _mm256_sub_pd(vc, vo);
        let division = _mm256_div_pd(numerator, range);
        let masked_result = _mm256_and_pd(division, mask);

        _mm256_storeu_pd(out_ptr.add(i), masked_result);
    }

    // Handle remaining elements
    for i in (chunks * 4)..len {
        let range = high[i] - low[i];
        if range.abs() > crate::utils::NUMERIC_EPSILON {
            result[i] = (close[i] - open[i]) / range;
        } else {
            result[i] = 0.0;
        }
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn avgprice_avx2(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    result: &mut [f64],
) {
    unsafe {
        use core::arch::x86_64::*;
        let len = open
            .len()
            .min(high.len())
            .min(low.len())
            .min(close.len())
            .min(result.len());
        let quarter = _mm256_set1_pd(0.25);

        let o_ptr = open.as_ptr();
        let h_ptr = high.as_ptr();
        let l_ptr = low.as_ptr();
        let c_ptr = close.as_ptr();
        let out_ptr = result.as_mut_ptr();

        let chunks = len / 4;
        for c in 0..chunks {
            let i = c * 4;
            let vo = _mm256_loadu_pd(o_ptr.add(i));
            let vh = _mm256_loadu_pd(h_ptr.add(i));
            let vl = _mm256_loadu_pd(l_ptr.add(i));
            let vc = _mm256_loadu_pd(c_ptr.add(i));

            let sum = _mm256_add_pd(_mm256_add_pd(vo, vh), _mm256_add_pd(vl, vc));
            let avg = _mm256_mul_pd(sum, quarter);

            _mm256_storeu_pd(out_ptr.add(i), avg);
        }

        // Handle remaining elements
        for i in (chunks * 4)..len {
            result[i] = (open[i] + high[i] + low[i] + close[i]) * 0.25;
        }
    }
}
