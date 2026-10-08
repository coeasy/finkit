//! AVX2 kernels for the dual-difference / diff-sum primitives.

use super::prelude::*;

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn diff_sum_avx2(a: &[f64], b: &[f64], period: usize) -> f64 {
    use core::arch::x86_64::*;
    let mut acc = _mm256_setzero_pd();
    let chunks = period / 4;
    for c in 0..chunks {
        let off = c * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        acc = _mm256_add_pd(acc, _mm256_sub_pd(va, vb));
    }
    let mut sum = horizontal_sum_avx2(acc);
    for i in (chunks * 4)..period {
        sum += a[i] - b[i];
    }
    sum
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn max_diff_sum_avx2(a: &[f64], b: &[f64], period: usize) -> f64 {
    use core::arch::x86_64::*;
    let zero = _mm256_setzero_pd();
    let mut acc = _mm256_setzero_pd();
    let chunks = period / 4;
    for c in 0..chunks {
        let off = c * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        let d = _mm256_sub_pd(va, vb);
        // max(0, d) = blend with zero where d < 0
        let mask = _mm256_cmp_pd(d, zero, _CMP_GT_OQ);
        let pos = _mm256_and_pd(d, mask);
        acc = _mm256_add_pd(acc, pos);
    }
    let mut sum = horizontal_sum_avx2(acc);
    for i in (chunks * 4)..period {
        let d = a[i] - b[i];
        if d > 0.0 {
            sum += d;
        }
    }
    sum
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn dual_diff_init_avx2(
    high: &[f64],
    open: &[f64],
    low: &[f64],
    period: usize,
) -> (f64, f64) {
    use core::arch::x86_64::*;
    let mut acc_ho = _mm256_setzero_pd();
    let mut acc_ol = _mm256_setzero_pd();
    let chunks = period / 4;
    for c in 0..chunks {
        let off = c * 4;
        let vh = _mm256_loadu_pd(high.as_ptr().add(off));
        let vo = _mm256_loadu_pd(open.as_ptr().add(off));
        let vl = _mm256_loadu_pd(low.as_ptr().add(off));
        acc_ho = _mm256_add_pd(acc_ho, _mm256_sub_pd(vh, vo));
        acc_ol = _mm256_add_pd(acc_ol, _mm256_sub_pd(vo, vl));
    }
    let mut sum_ho = horizontal_sum_avx2(acc_ho);
    let mut sum_ol = horizontal_sum_avx2(acc_ol);
    for i in (chunks * 4)..period {
        sum_ho += high[i] - open[i];
        sum_ol += open[i] - low[i];
    }
    (sum_ho, sum_ol)
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn dual_max_init_avx2(
    high: &[f64],
    close: &[f64],
    low: &[f64],
    period: usize,
) -> (f64, f64) {
    use core::arch::x86_64::*;
    let zero = _mm256_setzero_pd();
    let mut acc_up = _mm256_setzero_pd();
    let mut acc_down = _mm256_setzero_pd();
    // j 范围 1..=period，每批 4 个 j，连续存放
    let chunks = period / 4;
    for c in 0..chunks {
        let j_start = 1 + c * 4;
        // high[j_start..j_start+4] 与 close[j_start-1..j_start+3]
        let vh = _mm256_loadu_pd(high.as_ptr().add(j_start));
        let vc_prev = _mm256_loadu_pd(close.as_ptr().add(j_start - 1));
        let vl = _mm256_loadu_pd(low.as_ptr().add(j_start));
        let d_up = _mm256_sub_pd(vh, vc_prev);
        let d_down = _mm256_sub_pd(vc_prev, vl);
        let mask_up = _mm256_cmp_pd(d_up, zero, _CMP_GT_OQ);
        let mask_down = _mm256_cmp_pd(d_down, zero, _CMP_GT_OQ);
        acc_up = _mm256_add_pd(acc_up, _mm256_and_pd(d_up, mask_up));
        acc_down = _mm256_add_pd(acc_down, _mm256_and_pd(d_down, mask_down));
    }
    let mut sum_up = horizontal_sum_avx2(acc_up);
    let mut sum_down = horizontal_sum_avx2(acc_down);
    let j_start_tail = 1 + chunks * 4;
    for j in j_start_tail..=period {
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
