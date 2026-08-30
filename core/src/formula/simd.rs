// Allow implicit unsafe inside `unsafe fn` — entire functions are already marked unsafe
// (e.g. `#[target_feature(enable = "avx2")] unsafe fn ...`) and per-call `unsafe {}` blocks
// would just add noise. The `unsafe_op_in_unsafe_fn` lint is warn-by-default on recent
// rustc (≥ 1.82) even in 2021 edition.
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(dead_code)]

use std::sync::OnceLock;

use ndarray::Array1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SimdLevel {
    Scalar,
    #[cfg(target_arch = "aarch64")]
    Neon,
    #[cfg(target_arch = "x86_64")]
    Avx2,
    #[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
    Avx512,
}

static SIMD_LEVEL: OnceLock<SimdLevel> = OnceLock::new();

#[inline]
fn simd_level() -> SimdLevel {
    *SIMD_LEVEL.get_or_init(|| {
        #[cfg(target_arch = "x86_64")]
        {
            #[cfg(feature = "nightly-avx512")]
            {
                if is_x86_feature_detected!("avx512f") {
                    return SimdLevel::Avx512;
                }
            }
            if is_x86_feature_detected!("avx2") {
                return SimdLevel::Avx2;
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            return SimdLevel::Neon;
        }
        SimdLevel::Scalar
    })
}

/// Scalar (non-SIMD) element-wise operations used as fallback paths.
pub mod scalar {
    #[inline]
    pub fn element_mod(a: f64, b: f64) -> f64 {
        if b.abs() < 1e-15 {
            f64::NAN
        } else {
            a - (a / b).floor() * b
        }
    }

    #[inline]
    pub fn element_pow(a: f64, b: f64) -> f64 {
        a.powf(b)
    }

    pub fn mod_slice(a: &[f64], b: &[f64], result: &mut [f64]) {
        let len = a.len().min(b.len()).min(result.len());
        for i in 0..len {
            result[i] = element_mod(a[i], b[i]);
        }
    }

    pub fn pow_slice(a: &[f64], b: &[f64], result: &mut [f64]) {
        let len = a.len().min(b.len()).min(result.len());
        for i in 0..len {
            result[i] = element_pow(a[i], b[i]);
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn add_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_add_pd(va, vb));
    }
    for i in (chunks * 4)..len {
        result[i] = a[i] + b[i];
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn sub_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_sub_pd(va, vb));
    }
    for i in (chunks * 4)..len {
        result[i] = a[i] - b[i];
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn mul_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_mul_pd(va, vb));
    }
    for i in (chunks * 4)..len {
        result[i] = a[i] * b[i];
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn div_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    let sign_mask = _mm256_castsi256_pd(_mm256_set1_epi64x(i64::MIN));
    let eps = _mm256_set1_pd(1e-15);
    let nan = _mm256_set1_pd(f64::NAN);
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        let abs_b = _mm256_andnot_pd(sign_mask, vb);
        let near_zero = _mm256_cmp_pd(abs_b, eps, _CMP_LT_OS);
        let div_result = _mm256_div_pd(va, vb);
        let blended = _mm256_blendv_pd(div_result, nan, near_zero);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), blended);
    }
    for i in (chunks * 4)..len {
        result[i] = if b[i].abs() < 1e-15 {
            f64::NAN
        } else {
            a[i] / b[i]
        };
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn gt_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    let ones = _mm256_set1_pd(1.0);
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        let cmp = _mm256_cmp_pd(va, vb, _CMP_GT_OS);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_and_pd(cmp, ones));
    }
    for i in (chunks * 4)..len {
        result[i] = if a[i] > b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn lt_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    let ones = _mm256_set1_pd(1.0);
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        let cmp = _mm256_cmp_pd(va, vb, _CMP_LT_OS);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_and_pd(cmp, ones));
    }
    for i in (chunks * 4)..len {
        result[i] = if a[i] < b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn gte_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    let ones = _mm256_set1_pd(1.0);
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        let cmp = _mm256_cmp_pd(va, vb, _CMP_GE_OS);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_and_pd(cmp, ones));
    }
    for i in (chunks * 4)..len {
        result[i] = if a[i] >= b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn lte_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    let ones = _mm256_set1_pd(1.0);
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        let cmp = _mm256_cmp_pd(va, vb, _CMP_LE_OS);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_and_pd(cmp, ones));
    }
    for i in (chunks * 4)..len {
        result[i] = if a[i] <= b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn eq_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    let ones = _mm256_set1_pd(1.0);
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        let cmp = _mm256_cmp_pd(va, vb, _CMP_EQ_OQ);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_and_pd(cmp, ones));
    }
    for i in (chunks * 4)..len {
        result[i] = if a[i] == b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn neq_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    let ones = _mm256_set1_pd(1.0);
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        let cmp = _mm256_cmp_pd(va, vb, _CMP_NEQ_UQ);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_and_pd(cmp, ones));
    }
    for i in (chunks * 4)..len {
        result[i] = if a[i] != b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn abs_avx2(data: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    let chunks = len / 4;
    let sign_mask = _mm256_castsi256_pd(_mm256_set1_epi64x(i64::MIN));
    for i in 0..chunks {
        let off = i * 4;
        let v = _mm256_loadu_pd(data.as_ptr().add(off));
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_andnot_pd(sign_mask, v));
    }
    for i in (chunks * 4)..len {
        result[i] = data[i].abs();
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn max_elementwise_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_max_pd(va, vb));
    }
    for i in (chunks * 4)..len {
        result[i] = a[i].max(b[i]);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn min_elementwise_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        _mm256_storeu_pd(result.as_mut_ptr().add(off), _mm256_min_pd(va, vb));
    }
    for i in (chunks * 4)..len {
        result[i] = a[i].min(b[i]);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[allow(clippy::needless_range_loop)]
unsafe fn hhv_avx2(data: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if period == 0 || len == 0 {
        return;
    }
    for i in 0..len {
        if i + 1 < period {
            result[i] = f64::NAN;
        } else {
            let start = i + 1 - period;
            let wlen = period;
            let chunks = wlen / 4;
            if chunks > 0 {
                let mut vmax = _mm256_loadu_pd(data.as_ptr().add(start));
                for j in 1..chunks {
                    let v = _mm256_loadu_pd(data.as_ptr().add(start + j * 4));
                    vmax = _mm256_max_pd(vmax, v);
                }
                let mut buf = [0.0f64; 4];
                _mm256_storeu_pd(buf.as_mut_ptr(), vmax);
                let mut m = buf[0].max(buf[1]).max(buf[2]).max(buf[3]);
                for j in (chunks * 4)..wlen {
                    m = m.max(data[start + j]);
                }
                result[i] = m;
            } else {
                let mut m = data[start];
                for j in 1..wlen {
                    m = m.max(data[start + j]);
                }
                result[i] = m;
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[allow(clippy::needless_range_loop)]
unsafe fn llv_avx2(data: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if period == 0 || len == 0 {
        return;
    }
    for i in 0..len {
        if i + 1 < period {
            result[i] = f64::NAN;
        } else {
            let start = i + 1 - period;
            let wlen = period;
            let chunks = wlen / 4;
            if chunks > 0 {
                let mut vmin = _mm256_loadu_pd(data.as_ptr().add(start));
                for j in 1..chunks {
                    let v = _mm256_loadu_pd(data.as_ptr().add(start + j * 4));
                    vmin = _mm256_min_pd(vmin, v);
                }
                let mut buf = [0.0f64; 4];
                _mm256_storeu_pd(buf.as_mut_ptr(), vmin);
                let mut m = buf[0].min(buf[1]).min(buf[2]).min(buf[3]);
                for j in (chunks * 4)..wlen {
                    m = m.min(data[start + j]);
                }
                result[i] = m;
            } else {
                let mut m = data[start];
                for j in 1..wlen {
                    m = m.min(data[start + j]);
                }
                result[i] = m;
            }
        }
    }
}

// ───────────────────── AVX-512 (x86_64, nightly) ─────────────────────

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn add_avx512(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        let va = _mm512_loadu_pd(a.as_ptr().add(off));
        let vb = _mm512_loadu_pd(b.as_ptr().add(off));
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_add_pd(va, vb));
    }
    for i in (chunks * 8)..len {
        result[i] = a[i] + b[i];
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn sub_avx512(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        let va = _mm512_loadu_pd(a.as_ptr().add(off));
        let vb = _mm512_loadu_pd(b.as_ptr().add(off));
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_sub_pd(va, vb));
    }
    for i in (chunks * 8)..len {
        result[i] = a[i] - b[i];
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn mul_avx512(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        let va = _mm512_loadu_pd(a.as_ptr().add(off));
        let vb = _mm512_loadu_pd(b.as_ptr().add(off));
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_mul_pd(va, vb));
    }
    for i in (chunks * 8)..len {
        result[i] = a[i] * b[i];
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn div_avx512(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 8;
    let eps = _mm512_set1_pd(1e-15);
    let nan = _mm512_set1_pd(f64::NAN);
    for i in 0..chunks {
        let off = i * 8;
        let va = _mm512_loadu_pd(a.as_ptr().add(off));
        let vb = _mm512_loadu_pd(b.as_ptr().add(off));
        let abs_b = _mm512_abs_pd(vb);
        let near_zero = _mm512_cmp_pd_mask(abs_b, eps, _CMP_LT_OS);
        let div_result = _mm512_div_pd(va, vb);
        let blended = _mm512_mask_blend_pd(near_zero, div_result, nan);
        _mm512_storeu_pd(result.as_mut_ptr().add(off), blended);
    }
    for i in (chunks * 8)..len {
        result[i] = if b[i].abs() < 1e-15 {
            f64::NAN
        } else {
            a[i] / b[i]
        };
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn abs_avx512(data: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        let v = _mm512_loadu_pd(data.as_ptr().add(off));
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_abs_pd(v));
    }
    for i in (chunks * 8)..len {
        result[i] = data[i].abs();
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn max_elementwise_avx512(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        let va = _mm512_loadu_pd(a.as_ptr().add(off));
        let vb = _mm512_loadu_pd(b.as_ptr().add(off));
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_max_pd(va, vb));
    }
    for i in (chunks * 8)..len {
        result[i] = a[i].max(b[i]);
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn min_elementwise_avx512(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        let va = _mm512_loadu_pd(a.as_ptr().add(off));
        let vb = _mm512_loadu_pd(b.as_ptr().add(off));
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_min_pd(va, vb));
    }
    for i in (chunks * 8)..len {
        result[i] = a[i].min(b[i]);
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn gt_avx512(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 8;
    let ones = _mm512_set1_pd(1.0);
    let zeros = _mm512_setzero_pd();
    for i in 0..chunks {
        let off = i * 8;
        let va = _mm512_loadu_pd(a.as_ptr().add(off));
        let vb = _mm512_loadu_pd(b.as_ptr().add(off));
        let mask = _mm512_cmp_pd_mask(va, vb, _CMP_GT_OS);
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_mask_blend_pd(mask, zeros, ones));
    }
    for i in (chunks * 8)..len {
        result[i] = if a[i] > b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn lt_avx512(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 8;
    let ones = _mm512_set1_pd(1.0);
    let zeros = _mm512_setzero_pd();
    for i in 0..chunks {
        let off = i * 8;
        let va = _mm512_loadu_pd(a.as_ptr().add(off));
        let vb = _mm512_loadu_pd(b.as_ptr().add(off));
        let mask = _mm512_cmp_pd_mask(va, vb, _CMP_LT_OS);
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_mask_blend_pd(mask, zeros, ones));
    }
    for i in (chunks * 8)..len {
        result[i] = if a[i] < b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn gte_avx512(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 8;
    let ones = _mm512_set1_pd(1.0);
    let zeros = _mm512_setzero_pd();
    for i in 0..chunks {
        let off = i * 8;
        let va = _mm512_loadu_pd(a.as_ptr().add(off));
        let vb = _mm512_loadu_pd(b.as_ptr().add(off));
        let mask = _mm512_cmp_pd_mask(va, vb, _CMP_GE_OS);
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_mask_blend_pd(mask, zeros, ones));
    }
    for i in (chunks * 8)..len {
        result[i] = if a[i] >= b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn lte_avx512(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 8;
    let ones = _mm512_set1_pd(1.0);
    let zeros = _mm512_setzero_pd();
    for i in 0..chunks {
        let off = i * 8;
        let va = _mm512_loadu_pd(a.as_ptr().add(off));
        let vb = _mm512_loadu_pd(b.as_ptr().add(off));
        let mask = _mm512_cmp_pd_mask(va, vb, _CMP_LE_OS);
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_mask_blend_pd(mask, zeros, ones));
    }
    for i in (chunks * 8)..len {
        result[i] = if a[i] <= b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn eq_avx512(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 8;
    let ones = _mm512_set1_pd(1.0);
    let zeros = _mm512_setzero_pd();
    for i in 0..chunks {
        let off = i * 8;
        let va = _mm512_loadu_pd(a.as_ptr().add(off));
        let vb = _mm512_loadu_pd(b.as_ptr().add(off));
        let mask = _mm512_cmp_pd_mask(va, vb, _CMP_EQ_OQ);
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_mask_blend_pd(mask, zeros, ones));
    }
    for i in (chunks * 8)..len {
        result[i] = if a[i] == b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn neq_avx512(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 8;
    let ones = _mm512_set1_pd(1.0);
    let zeros = _mm512_setzero_pd();
    for i in 0..chunks {
        let off = i * 8;
        let va = _mm512_loadu_pd(a.as_ptr().add(off));
        let vb = _mm512_loadu_pd(b.as_ptr().add(off));
        let mask = _mm512_cmp_pd_mask(va, vb, _CMP_NEQ_UQ);
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_mask_blend_pd(mask, zeros, ones));
    }
    for i in (chunks * 8)..len {
        result[i] = if a[i] != b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn select_avx512(
    condition: &[f64],
    then_val: &[f64],
    else_val: &[f64],
    result: &mut [f64],
    len: usize,
) {
    use core::arch::x86_64::*;
    let zero = _mm512_setzero_pd();
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        let cond = _mm512_loadu_pd(condition.as_ptr().add(off));
        let mask = _mm512_cmp_pd_mask(cond, zero, _CMP_NEQ_UQ);
        let tv = _mm512_loadu_pd(then_val.as_ptr().add(off));
        let ev = _mm512_loadu_pd(else_val.as_ptr().add(off));
        _mm512_storeu_pd(result.as_mut_ptr().add(off), _mm512_mask_blend_pd(mask, ev, tv));
    }
    for i in (chunks * 8)..len {
        result[i] = if condition[i] != 0.0 {
            then_val[i]
        } else {
            else_val[i]
        };
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
#[allow(clippy::needless_range_loop)]
unsafe fn hhv_avx512(data: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if period == 0 || len == 0 {
        return;
    }
    for i in 0..len {
        if i + 1 < period {
            result[i] = f64::NAN;
        } else {
            let start = i + 1 - period;
            let wlen = period;
            let chunks = wlen / 8;
            if chunks > 0 {
                let mut vmax = _mm512_loadu_pd(data.as_ptr().add(start));
                for j in 1..chunks {
                    let v = _mm512_loadu_pd(data.as_ptr().add(start + j * 8));
                    vmax = _mm512_max_pd(vmax, v);
                }
                let m256_lo = _mm512_castpd512_pd256(vmax);
                let m256_hi = _mm512_extractf64x4_pd(vmax, 1);
                let m256 = _mm256_max_pd(m256_lo, m256_hi);
                let mut buf = [0.0f64; 4];
                _mm256_storeu_pd(buf.as_mut_ptr(), m256);
                let mut m = buf[0].max(buf[1]).max(buf[2]).max(buf[3]);
                for j in (chunks * 8)..wlen {
                    m = m.max(data[start + j]);
                }
                result[i] = m;
            } else {
                let mut m = data[start];
                for j in 1..wlen {
                    m = m.max(data[start + j]);
                }
                result[i] = m;
            }
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
#[allow(clippy::needless_range_loop)]
unsafe fn llv_avx512(data: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if period == 0 || len == 0 {
        return;
    }
    for i in 0..len {
        if i + 1 < period {
            result[i] = f64::NAN;
        } else {
            let start = i + 1 - period;
            let wlen = period;
            let chunks = wlen / 8;
            if chunks > 0 {
                let mut vmin = _mm512_loadu_pd(data.as_ptr().add(start));
                for j in 1..chunks {
                    let v = _mm512_loadu_pd(data.as_ptr().add(start + j * 8));
                    vmin = _mm512_min_pd(vmin, v);
                }
                let m256_lo = _mm512_castpd512_pd256(vmin);
                let m256_hi = _mm512_extractf64x4_pd(vmin, 1);
                let m256 = _mm256_min_pd(m256_lo, m256_hi);
                let mut buf = [0.0f64; 4];
                _mm256_storeu_pd(buf.as_mut_ptr(), m256);
                let mut m = buf[0].min(buf[1]).min(buf[2]).min(buf[3]);
                for j in (chunks * 8)..wlen {
                    m = m.min(data[start + j]);
                }
                result[i] = m;
            } else {
                let mut m = data[start];
                for j in 1..wlen {
                    m = m.min(data[start + j]);
                }
                result[i] = m;
            }
        }
    }
}

// ───────────────────── ARM NEON (aarch64) ─────────────────────

#[cfg(target_arch = "aarch64")]
unsafe fn add_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        vst1q_f64(result.as_mut_ptr().add(off), vaddq_f64(va, vb));
    }
    for i in (chunks * 2)..len {
        result[i] = a[i] + b[i];
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn sub_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        vst1q_f64(result.as_mut_ptr().add(off), vsubq_f64(va, vb));
    }
    for i in (chunks * 2)..len {
        result[i] = a[i] - b[i];
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn mul_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        vst1q_f64(result.as_mut_ptr().add(off), vmulq_f64(va, vb));
    }
    for i in (chunks * 2)..len {
        result[i] = a[i] * b[i];
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn div_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    let eps = vdupq_n_f64(1e-15);
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        let abs_b = vabsq_f64(vb);
        let near_zero = vcltq_f64(abs_b, eps);
        let div_result = vdivq_f64(va, vb);
        let nan_vec = vdupq_n_f64(f64::NAN);
        let blended = vbslq_f64(near_zero, nan_vec, div_result);
        vst1q_f64(result.as_mut_ptr().add(off), blended);
    }
    for i in (chunks * 2)..len {
        result[i] = if b[i].abs() < 1e-15 { f64::NAN } else { a[i] / b[i] };
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn abs_neon(data: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = data.len().min(result.len());
    let chunks = len / 2;
    for i in 0..chunks {
        let off = i * 2;
        let v = vld1q_f64(data.as_ptr().add(off));
        vst1q_f64(result.as_mut_ptr().add(off), vabsq_f64(v));
    }
    for i in (chunks * 2)..len {
        result[i] = data[i].abs();
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn max_elementwise_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        vst1q_f64(result.as_mut_ptr().add(off), vmaxq_f64(va, vb));
    }
    for i in (chunks * 2)..len {
        result[i] = a[i].max(b[i]);
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn min_elementwise_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        vst1q_f64(result.as_mut_ptr().add(off), vminq_f64(va, vb));
    }
    for i in (chunks * 2)..len {
        result[i] = a[i].min(b[i]);
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn gt_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    let ones = vdupq_n_f64(1.0);
    let zeros = vdupq_n_f64(0.0);
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        let cmp = vcgtq_f64(va, vb);
        vst1q_f64(result.as_mut_ptr().add(off), vbslq_f64(cmp, ones, zeros));
    }
    for i in (chunks * 2)..len {
        result[i] = if a[i] > b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn lt_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    let ones = vdupq_n_f64(1.0);
    let zeros = vdupq_n_f64(0.0);
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        let cmp = vcltq_f64(va, vb);
        vst1q_f64(result.as_mut_ptr().add(off), vbslq_f64(cmp, ones, zeros));
    }
    for i in (chunks * 2)..len {
        result[i] = if a[i] < b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn gte_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    let ones = vdupq_n_f64(1.0);
    let zeros = vdupq_n_f64(0.0);
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        let cmp = vcgeq_f64(va, vb);
        vst1q_f64(result.as_mut_ptr().add(off), vbslq_f64(cmp, ones, zeros));
    }
    for i in (chunks * 2)..len {
        result[i] = if a[i] >= b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn lte_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    let ones = vdupq_n_f64(1.0);
    let zeros = vdupq_n_f64(0.0);
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        let cmp = vcleq_f64(va, vb);
        vst1q_f64(result.as_mut_ptr().add(off), vbslq_f64(cmp, ones, zeros));
    }
    for i in (chunks * 2)..len {
        result[i] = if a[i] <= b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn eq_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    let ones = vdupq_n_f64(1.0);
    let zeros = vdupq_n_f64(0.0);
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        let cmp = vceqq_f64(va, vb);
        vst1q_f64(result.as_mut_ptr().add(off), vbslq_f64(cmp, ones, zeros));
    }
    for i in (chunks * 2)..len {
        result[i] = if a[i] == b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn neq_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    let ones = vdupq_n_f64(1.0);
    let zeros = vdupq_n_f64(0.0);
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        let eq = vceqq_f64(va, vb);
        let neq = vmvnq_u8(vreinterpretq_u8_u64(eq));
        let neq64 = vreinterpretq_u64_u8(neq);
        vst1q_f64(result.as_mut_ptr().add(off), vbslq_f64(neq64, ones, zeros));
    }
    for i in (chunks * 2)..len {
        result[i] = if a[i] != b[i] { 1.0 } else { 0.0 };
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn mod_neon(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 2;
    let eps = vdupq_n_f64(1e-15);
    let nan_vec = vdupq_n_f64(f64::NAN);
    for i in 0..chunks {
        let off = i * 2;
        let va = vld1q_f64(a.as_ptr().add(off));
        let vb = vld1q_f64(b.as_ptr().add(off));
        let abs_b = vabsq_f64(vb);
        let near_zero = vcltq_f64(abs_b, eps);
        let quot = vdivq_f64(va, vb);
        let floored = vrndmq_f64(quot);
        let mod_result = vsubq_f64(va, vmulq_f64(floored, vb));
        let blended = vbslq_f64(near_zero, nan_vec, mod_result);
        vst1q_f64(result.as_mut_ptr().add(off), blended);
    }
    for i in (chunks * 2)..len {
        result[i] = scalar::element_mod(a[i], b[i]);
    }
}

#[cfg(target_arch = "aarch64")]
#[allow(clippy::needless_range_loop)]
unsafe fn hhv_neon(data: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = data.len().min(result.len());
    if period == 0 || len == 0 {
        return;
    }
    for i in 0..len {
        if i + 1 < period {
            result[i] = f64::NAN;
        } else {
            let start = i + 1 - period;
            let wlen = period;
            let chunks = wlen / 2;
            if chunks > 0 {
                let mut vmax = vld1q_f64(data.as_ptr().add(start));
                for j in 1..chunks {
                    let v = vld1q_f64(data.as_ptr().add(start + j * 2));
                    vmax = vmaxq_f64(vmax, v);
                }
                let mut buf = [0.0f64; 2];
                vst1q_f64(buf.as_mut_ptr(), vmax);
                let mut m = buf[0].max(buf[1]);
                for j in (chunks * 2)..wlen {
                    m = m.max(data[start + j]);
                }
                result[i] = m;
            } else {
                let mut m = data[start];
                for j in 1..wlen {
                    m = m.max(data[start + j]);
                }
                result[i] = m;
            }
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[allow(clippy::needless_range_loop)]
unsafe fn llv_neon(data: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::aarch64::*;
    let len = data.len().min(result.len());
    if period == 0 || len == 0 {
        return;
    }
    for i in 0..len {
        if i + 1 < period {
            result[i] = f64::NAN;
        } else {
            let start = i + 1 - period;
            let wlen = period;
            let chunks = wlen / 2;
            if chunks > 0 {
                let mut vmin = vld1q_f64(data.as_ptr().add(start));
                for j in 1..chunks {
                    let v = vld1q_f64(data.as_ptr().add(start + j * 2));
                    vmin = vminq_f64(vmin, v);
                }
                let mut buf = [0.0f64; 2];
                vst1q_f64(buf.as_mut_ptr(), vmin);
                let mut m = buf[0].min(buf[1]);
                for j in (chunks * 2)..wlen {
                    m = m.min(data[start + j]);
                }
                result[i] = m;
            } else {
                let mut m = data[start];
                for j in 1..wlen {
                    m = m.min(data[start + j]);
                }
                result[i] = m;
            }
        }
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn select_neon(
    condition: &[f64],
    then_val: &[f64],
    else_val: &[f64],
    result: &mut [f64],
    len: usize,
) {
    use core::arch::aarch64::*;
    let zero = vdupq_n_f64(0.0);
    let chunks = len / 2;
    for i in 0..chunks {
        let off = i * 2;
        let cond = vld1q_f64(condition.as_ptr().add(off));
        let mask = vceqq_f64(cond, zero);
        let tv = vld1q_f64(then_val.as_ptr().add(off));
        let ev = vld1q_f64(else_val.as_ptr().add(off));
        let neq_mask = vmvnq_u8(vreinterpretq_u8_u64(mask));
        let neq64 = vreinterpretq_u64_u8(neq_mask);
        let blended = vbslq_f64(neq64, tv, ev);
        vst1q_f64(result.as_mut_ptr().add(off), blended);
    }
    for i in (chunks * 2)..len {
        result[i] = if condition[i] != 0.0 { then_val[i] } else { else_val[i] };
    }
}

// ───────────────────── Fallback (scalar) ─────────────────────

fn add_fallback(a: &[f64], b: &[f64], result: &mut [f64]) {
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in (0..chunks * 4).step_by(4) {
        result[i] = a[i] + b[i];
        result[i + 1] = a[i + 1] + b[i + 1];
        result[i + 2] = a[i + 2] + b[i + 2];
        result[i + 3] = a[i + 3] + b[i + 3];
    }
    for i in (chunks * 4)..len {
        result[i] = a[i] + b[i];
    }
}

fn sub_fallback(a: &[f64], b: &[f64], result: &mut [f64]) {
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in (0..chunks * 4).step_by(4) {
        result[i] = a[i] - b[i];
        result[i + 1] = a[i + 1] - b[i + 1];
        result[i + 2] = a[i + 2] - b[i + 2];
        result[i + 3] = a[i + 3] - b[i + 3];
    }
    for i in (chunks * 4)..len {
        result[i] = a[i] - b[i];
    }
}

fn mul_fallback(a: &[f64], b: &[f64], result: &mut [f64]) {
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in (0..chunks * 4).step_by(4) {
        result[i] = a[i] * b[i];
        result[i + 1] = a[i + 1] * b[i + 1];
        result[i + 2] = a[i + 2] * b[i + 2];
        result[i + 3] = a[i + 3] * b[i + 3];
    }
    for i in (chunks * 4)..len {
        result[i] = a[i] * b[i];
    }
}

fn div_fallback(a: &[f64], b: &[f64], result: &mut [f64]) {
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in (0..chunks * 4).step_by(4) {
        result[i] = if b[i].abs() < 1e-15 {
            f64::NAN
        } else {
            a[i] / b[i]
        };
        result[i + 1] = if b[i + 1].abs() < 1e-15 {
            f64::NAN
        } else {
            a[i + 1] / b[i + 1]
        };
        result[i + 2] = if b[i + 2].abs() < 1e-15 {
            f64::NAN
        } else {
            a[i + 2] / b[i + 2]
        };
        result[i + 3] = if b[i + 3].abs() < 1e-15 {
            f64::NAN
        } else {
            a[i + 3] / b[i + 3]
        };
    }
    for i in (chunks * 4)..len {
        result[i] = if b[i].abs() < 1e-15 {
            f64::NAN
        } else {
            a[i] / b[i]
        };
    }
}

fn cmp_fallback(a: &[f64], b: &[f64], result: &mut [f64], op: impl Fn(f64, f64) -> bool) {
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in (0..chunks * 4).step_by(4) {
        result[i] = if op(a[i], b[i]) { 1.0 } else { 0.0 };
        result[i + 1] = if op(a[i + 1], b[i + 1]) { 1.0 } else { 0.0 };
        result[i + 2] = if op(a[i + 2], b[i + 2]) { 1.0 } else { 0.0 };
        result[i + 3] = if op(a[i + 3], b[i + 3]) { 1.0 } else { 0.0 };
    }
    for i in (chunks * 4)..len {
        result[i] = if op(a[i], b[i]) { 1.0 } else { 0.0 };
    }
}

fn abs_fallback(data: &[f64], result: &mut [f64]) {
    let len = data.len().min(result.len());
    let chunks = len / 4;
    for i in (0..chunks * 4).step_by(4) {
        result[i] = data[i].abs();
        result[i + 1] = data[i + 1].abs();
        result[i + 2] = data[i + 2].abs();
        result[i + 3] = data[i + 3].abs();
    }
    for i in (chunks * 4)..len {
        result[i] = data[i].abs();
    }
}

fn max_elementwise_fallback(a: &[f64], b: &[f64], result: &mut [f64]) {
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in (0..chunks * 4).step_by(4) {
        result[i] = a[i].max(b[i]);
        result[i + 1] = a[i + 1].max(b[i + 1]);
        result[i + 2] = a[i + 2].max(b[i + 2]);
        result[i + 3] = a[i + 3].max(b[i + 3]);
    }
    for i in (chunks * 4)..len {
        result[i] = a[i].max(b[i]);
    }
}

fn min_elementwise_fallback(a: &[f64], b: &[f64], result: &mut [f64]) {
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in (0..chunks * 4).step_by(4) {
        result[i] = a[i].min(b[i]);
        result[i + 1] = a[i + 1].min(b[i + 1]);
        result[i + 2] = a[i + 2].min(b[i + 2]);
        result[i + 3] = a[i + 3].min(b[i + 3]);
    }
    for i in (chunks * 4)..len {
        result[i] = a[i].min(b[i]);
    }
}

fn hhv_fallback(data: &[f64], period: usize, result: &mut [f64]) {
    let len = data.len().min(result.len());
    if period == 0 || len == 0 {
        return;
    }
    for (i, r) in result.iter_mut().enumerate().take(len) {
        if i + 1 < period {
            *r = f64::NAN;
        } else {
            let start = i + 1 - period;
            let mut m = data[start];
            for j in 1..period {
                m = m.max(data[start + j]);
            }
            *r = m;
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn mod_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    let sign_mask = _mm256_castsi256_pd(_mm256_set1_epi64x(i64::MIN));
    let eps = _mm256_set1_pd(1e-15);
    let nan = _mm256_set1_pd(f64::NAN);
    for i in 0..chunks {
        let off = i * 4;
        let va = _mm256_loadu_pd(a.as_ptr().add(off));
        let vb = _mm256_loadu_pd(b.as_ptr().add(off));
        let abs_b = _mm256_andnot_pd(sign_mask, vb);
        let near_zero = _mm256_cmp_pd(abs_b, eps, _CMP_LT_OS);
        let quot = _mm256_div_pd(va, vb);
        let floored = _mm256_round_pd(quot, _MM_FROUND_FLOOR);
        let mod_result = _mm256_sub_pd(va, _mm256_mul_pd(floored, vb));
        let blended = _mm256_blendv_pd(mod_result, nan, near_zero);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), blended);
    }
    for i in (chunks * 4)..len {
        result[i] = scalar::element_mod(a[i], b[i]);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn pow_avx2(a: &[f64], b: &[f64], result: &mut [f64]) {
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        result[off] = scalar::element_pow(a[off], b[off]);
        result[off + 1] = scalar::element_pow(a[off + 1], b[off + 1]);
        result[off + 2] = scalar::element_pow(a[off + 2], b[off + 2]);
        result[off + 3] = scalar::element_pow(a[off + 3], b[off + 3]);
    }
    for i in (chunks * 4)..len {
        result[i] = scalar::element_pow(a[i], b[i]);
    }
}

fn mod_fallback(a: &[f64], b: &[f64], result: &mut [f64]) {
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in (0..chunks * 4).step_by(4) {
        result[i] = scalar::element_mod(a[i], b[i]);
        result[i + 1] = scalar::element_mod(a[i + 1], b[i + 1]);
        result[i + 2] = scalar::element_mod(a[i + 2], b[i + 2]);
        result[i + 3] = scalar::element_mod(a[i + 3], b[i + 3]);
    }
    for i in (chunks * 4)..len {
        result[i] = scalar::element_mod(a[i], b[i]);
    }
}

fn pow_fallback(a: &[f64], b: &[f64], result: &mut [f64]) {
    let len = a.len().min(b.len()).min(result.len());
    let chunks = len / 4;
    for i in (0..chunks * 4).step_by(4) {
        result[i] = scalar::element_pow(a[i], b[i]);
        result[i + 1] = scalar::element_pow(a[i + 1], b[i + 1]);
        result[i + 2] = scalar::element_pow(a[i + 2], b[i + 2]);
        result[i + 3] = scalar::element_pow(a[i + 3], b[i + 3]);
    }
    for i in (chunks * 4)..len {
        result[i] = scalar::element_pow(a[i], b[i]);
    }
}

fn llv_fallback(data: &[f64], period: usize, result: &mut [f64]) {
    let len = data.len().min(result.len());
    if period == 0 || len == 0 {
        return;
    }
    for (i, r) in result.iter_mut().enumerate().take(len) {
        if i + 1 < period {
            *r = f64::NAN;
        } else {
            let start = i + 1 - period;
            let mut m = data[start];
            for j in 1..period {
                m = m.min(data[start + j]);
            }
            *r = m;
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn select_avx2(
    condition: &[f64],
    then_val: &[f64],
    else_val: &[f64],
    result: &mut [f64],
    len: usize,
) {
    use core::arch::x86_64::*;
    let zero = _mm256_setzero_pd();
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        let cond = _mm256_loadu_pd(condition.as_ptr().add(off));
        let mask = _mm256_cmp_pd(cond, zero, _CMP_NEQ_UQ);
        let tv = _mm256_loadu_pd(then_val.as_ptr().add(off));
        let ev = _mm256_loadu_pd(else_val.as_ptr().add(off));
        let blended = _mm256_blendv_pd(ev, tv, mask);
        _mm256_storeu_pd(result.as_mut_ptr().add(off), blended);
    }
    for i in (chunks * 4)..len {
        result[i] = if condition[i] != 0.0 {
            then_val[i]
        } else {
            else_val[i]
        };
    }
}

pub struct SimdOps;

impl SimdOps {
    #[inline]
    pub fn add(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { add_avx512(a, b, result) },
            SimdLevel::Avx2 => return unsafe { add_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { add_neon(a, b, result) };
        }
        add_fallback(a, b, result)
    }

    #[inline]
    pub fn sub(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { sub_avx512(a, b, result) },
            SimdLevel::Avx2 => return unsafe { sub_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { sub_neon(a, b, result) };
        }
        sub_fallback(a, b, result)
    }

    #[inline]
    pub fn mul(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { mul_avx512(a, b, result) },
            SimdLevel::Avx2 => return unsafe { mul_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { mul_neon(a, b, result) };
        }
        mul_fallback(a, b, result)
    }

    #[inline]
    pub fn div(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { div_avx512(a, b, result) },
            SimdLevel::Avx2 => return unsafe { div_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { div_neon(a, b, result) };
        }
        div_fallback(a, b, result)
    }

    #[inline]
    pub fn simd_mod(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            SimdLevel::Avx2 => return unsafe { mod_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { mod_neon(a, b, result) };
        }
        mod_fallback(a, b, result)
    }

    #[inline]
    pub fn simd_pow(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            SimdLevel::Avx2 => return unsafe { pow_avx2(a, b, result) },
            _ => {}
        }
        pow_fallback(a, b, result)
    }

    #[inline]
    pub fn gt(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { gt_avx512(a, b, result) },
            SimdLevel::Avx2 => return unsafe { gt_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { gt_neon(a, b, result) };
        }
        cmp_fallback(a, b, result, |a, b| a > b)
    }

    #[inline]
    pub fn lt(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { lt_avx512(a, b, result) },
            SimdLevel::Avx2 => return unsafe { lt_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { lt_neon(a, b, result) };
        }
        cmp_fallback(a, b, result, |a, b| a < b)
    }

    #[inline]
    pub fn gte(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { gte_avx512(a, b, result) },
            SimdLevel::Avx2 => return unsafe { gte_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { gte_neon(a, b, result) };
        }
        cmp_fallback(a, b, result, |a, b| a >= b)
    }

    #[inline]
    pub fn lte(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { lte_avx512(a, b, result) },
            SimdLevel::Avx2 => return unsafe { lte_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { lte_neon(a, b, result) };
        }
        cmp_fallback(a, b, result, |a, b| a <= b)
    }

    #[inline]
    pub fn eq(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { eq_avx512(a, b, result) },
            SimdLevel::Avx2 => return unsafe { eq_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { eq_neon(a, b, result) };
        }
        cmp_fallback(a, b, result, |a, b| a == b)
    }

    #[inline]
    pub fn neq(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { neq_avx512(a, b, result) },
            SimdLevel::Avx2 => return unsafe { neq_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { neq_neon(a, b, result) };
        }
        cmp_fallback(a, b, result, |a, b| a != b)
    }

    #[inline]
    pub fn abs(data: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { abs_avx512(data, result) },
            SimdLevel::Avx2 => return unsafe { abs_avx2(data, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { abs_neon(data, result) };
        }
        abs_fallback(data, result)
    }

    #[inline]
    pub fn max_elementwise(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { max_elementwise_avx512(a, b, result) },
            SimdLevel::Avx2 => return unsafe { max_elementwise_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { max_elementwise_neon(a, b, result) };
        }
        max_elementwise_fallback(a, b, result)
    }

    #[inline]
    pub fn min_elementwise(a: &[f64], b: &[f64], result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { min_elementwise_avx512(a, b, result) },
            SimdLevel::Avx2 => return unsafe { min_elementwise_avx2(a, b, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { min_elementwise_neon(a, b, result) };
        }
        min_elementwise_fallback(a, b, result)
    }

    #[inline]
    pub fn sma(data: &[f64], period: usize, result: &mut [f64]) {
        let len = data.len().min(result.len());
        if period == 0 || len == 0 {
            return;
        }
        let mut window_sum = 0.0f64;
        for i in 0..len {
            window_sum += data[i];
            if i + 1 < period {
                result[i] = f64::NAN;
            } else {
                result[i] = window_sum / period as f64;
                window_sum -= data[i + 1 - period];
            }
        }
    }

    #[inline]
    pub fn ema(data: &[f64], period: usize, result: &mut [f64]) {
        let len = data.len().min(result.len());
        if period == 0 || len == 0 {
            return;
        }
        let k = 2.0 / (period as f64 + 1.0);
        let mut sum = 0.0f64;
        for i in 0..period.min(len) {
            sum += data[i];
            result[i] = f64::NAN;
        }
        if len >= period {
            result[period - 1] = sum / period as f64;
            for i in period..len {
                result[i] = data[i] * k + result[i - 1] * (1.0 - k);
            }
        }
    }

    #[inline]
    pub fn ref_value(data: &[f64], n: usize, result: &mut [f64]) {
        let len = data.len().min(result.len());
        for i in 0..len {
            result[i] = if i >= n { data[i - n] } else { f64::NAN };
        }
    }

    #[inline]
    pub fn hhv(data: &[f64], period: usize, result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { hhv_avx512(data, period, result) },
            SimdLevel::Avx2 => return unsafe { hhv_avx2(data, period, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { hhv_neon(data, period, result) };
        }
        hhv_fallback(data, period, result)
    }

    #[inline]
    pub fn llv(data: &[f64], period: usize, result: &mut [f64]) {
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { llv_avx512(data, period, result) },
            SimdLevel::Avx2 => return unsafe { llv_avx2(data, period, result) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { llv_neon(data, period, result) };
        }
        llv_fallback(data, period, result)
    }

    #[inline]
    pub fn sum(data: &[f64], period: usize, result: &mut [f64]) {
        let len = data.len().min(result.len());
        if period == 0 || len == 0 {
            return;
        }
        let mut window_sum = 0.0f64;
        for i in 0..len {
            window_sum += data[i];
            if i + 1 < period {
                result[i] = f64::NAN;
            } else {
                result[i] = window_sum;
                window_sum -= data[i + 1 - period];
            }
        }
    }

    #[inline]
    pub fn count(condition: &[f64], period: usize, result: &mut [f64]) {
        let len = condition.len().min(result.len());
        if period == 0 || len == 0 {
            return;
        }
        let mut window_count = 0.0f64;
        for i in 0..len {
            if condition[i] != 0.0 {
                window_count += 1.0;
            }
            if i + 1 < period {
                result[i] = f64::NAN;
            } else {
                result[i] = window_count;
                if condition[i + 1 - period] != 0.0 {
                    window_count -= 1.0;
                }
            }
        }
    }

    pub fn simd_add_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::add(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_sub_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::sub(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_mul_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::mul(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_div_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::div(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_mod_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::simd_mod(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_pow_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::simd_pow(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_gt_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::gt(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_lt_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::lt(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_gte_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::gte(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_lte_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::lte(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_eq_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::eq(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_neq_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::neq(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_abs_array(data: &Array1<f64>) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::abs(data.as_slice().unwrap(), result.as_slice_mut().unwrap());
        result
    }

    pub fn simd_max_elementwise_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::max_elementwise(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_min_elementwise_arrays(a: &Array1<f64>, b: &Array1<f64>) -> Array1<f64> {
        let len = a.len();
        let mut result = Array1::zeros(len);
        Self::min_elementwise(
            a.as_slice().unwrap(),
            b.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_sma_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::sma(
            data.as_slice().unwrap(),
            period,
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_ema_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::ema(
            data.as_slice().unwrap(),
            period,
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_ref_array(data: &Array1<f64>, n: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::ref_value(data.as_slice().unwrap(), n, result.as_slice_mut().unwrap());
        result
    }

    pub fn simd_hhv_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::hhv(
            data.as_slice().unwrap(),
            period,
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_llv_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::llv(
            data.as_slice().unwrap(),
            period,
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_sum_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::sum(
            data.as_slice().unwrap(),
            period,
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_count_array(condition: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = condition.len();
        let mut result = Array1::zeros(len);
        Self::count(
            condition.as_slice().unwrap(),
            period,
            result.as_slice_mut().unwrap(),
        );
        result
    }

    pub fn simd_ma_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = vec![f64::NAN; len];
        if period == 0 || len < period {
            return Array1::from_vec(result);
        }
        let data_slice = data.as_slice().unwrap();
        let mut window_sum: f64 = data_slice[..period].iter().sum();
        result[period - 1] = window_sum / period as f64;
        for i in period..len {
            window_sum += data[i] - data[i - period];
            result[i] = window_sum / period as f64;
        }
        Array1::from_vec(result)
    }

    pub fn simd_ema_array_opt(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = vec![f64::NAN; len];
        if period == 0 || len < period {
            return Array1::from_vec(result);
        }
        let k = 2.0 / (period as f64 + 1.0);
        let data_slice = data.as_slice().unwrap();
        let sum: f64 = data_slice[..period].iter().sum();
        result[period - 1] = sum / period as f64;
        for i in period..len {
            result[i] = data[i] * k + result[i - 1] * (1.0 - k);
        }
        Array1::from_vec(result)
    }

    pub fn logical_and(a: &[f64], b: &[f64], result: &mut [f64]) {
        let len = a.len().min(b.len()).min(result.len());
        let chunks = len / 4;
        for i in 0..chunks {
            let off = i * 4;
            result[off] = if a[off] != 0.0 && b[off] != 0.0 { 1.0 } else { 0.0 };
            result[off + 1] = if a[off + 1] != 0.0 && b[off + 1] != 0.0 { 1.0 } else { 0.0 };
            result[off + 2] = if a[off + 2] != 0.0 && b[off + 2] != 0.0 { 1.0 } else { 0.0 };
            result[off + 3] = if a[off + 3] != 0.0 && b[off + 3] != 0.0 { 1.0 } else { 0.0 };
        }
        for i in (chunks * 4)..len {
            result[i] = if a[i] != 0.0 && b[i] != 0.0 { 1.0 } else { 0.0 };
        }
    }

    pub fn logical_or(a: &[f64], b: &[f64], result: &mut [f64]) {
        let len = a.len().min(b.len()).min(result.len());
        let chunks = len / 4;
        for i in 0..chunks {
            let off = i * 4;
            result[off] = if a[off] != 0.0 || b[off] != 0.0 { 1.0 } else { 0.0 };
            result[off + 1] = if a[off + 1] != 0.0 || b[off + 1] != 0.0 { 1.0 } else { 0.0 };
            result[off + 2] = if a[off + 2] != 0.0 || b[off + 2] != 0.0 { 1.0 } else { 0.0 };
            result[off + 3] = if a[off + 3] != 0.0 || b[off + 3] != 0.0 { 1.0 } else { 0.0 };
        }
        for i in (chunks * 4)..len {
            result[i] = if a[i] != 0.0 || b[i] != 0.0 { 1.0 } else { 0.0 };
        }
    }

    pub fn logical_not(data: &[f64], result: &mut [f64]) {
        let len = data.len().min(result.len());
        let chunks = len / 4;
        for i in 0..chunks {
            let off = i * 4;
            result[off] = if data[off] != 0.0 { 0.0 } else { 1.0 };
            result[off + 1] = if data[off + 1] != 0.0 { 0.0 } else { 1.0 };
            result[off + 2] = if data[off + 2] != 0.0 { 0.0 } else { 1.0 };
            result[off + 3] = if data[off + 3] != 0.0 { 0.0 } else { 1.0 };
        }
        for i in (chunks * 4)..len {
            result[i] = if data[i] != 0.0 { 0.0 } else { 1.0 };
        }
    }

    pub fn logical_xor(a: &[f64], b: &[f64], result: &mut [f64]) {
        let len = a.len().min(b.len()).min(result.len());
        let chunks = len / 4;
        for i in 0..chunks {
            let off = i * 4;
            result[off] = if (a[off] != 0.0) != (b[off] != 0.0) { 1.0 } else { 0.0 };
            result[off + 1] = if (a[off + 1] != 0.0) != (b[off + 1] != 0.0) { 1.0 } else { 0.0 };
            result[off + 2] = if (a[off + 2] != 0.0) != (b[off + 2] != 0.0) { 1.0 } else { 0.0 };
            result[off + 3] = if (a[off + 3] != 0.0) != (b[off + 3] != 0.0) { 1.0 } else { 0.0 };
        }
        for i in (chunks * 4)..len {
            result[i] = if (a[i] != 0.0) != (b[i] != 0.0) { 1.0 } else { 0.0 };
        }
    }

    pub fn select(condition: &[f64], then_val: &[f64], else_val: &[f64], result: &mut [f64]) {
        let len = condition
            .len()
            .min(then_val.len())
            .min(else_val.len())
            .min(result.len());
        #[cfg(target_arch = "x86_64")]
        match simd_level() {
            #[cfg(feature = "nightly-avx512")]
            SimdLevel::Avx512 => return unsafe { select_avx512(condition, then_val, else_val, result, len) },
            SimdLevel::Avx2 => return unsafe { select_avx2(condition, then_val, else_val, result, len) },
            _ => {}
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { select_neon(condition, then_val, else_val, result, len) };
        }
        for i in 0..len {
            result[i] = if condition[i] != 0.0 {
                then_val[i]
            } else {
                else_val[i]
            };
        }
    }

    pub fn simd_select_arrays(
        condition: &Array1<f64>,
        then_val: &Array1<f64>,
        else_val: &Array1<f64>,
    ) -> Array1<f64> {
        let len = condition.len();
        let mut result = Array1::zeros(len);
        Self::select(
            condition.as_slice().unwrap(),
            then_val.as_slice().unwrap(),
            else_val.as_slice().unwrap(),
            result.as_slice_mut().unwrap(),
        );
        result
    }

    #[inline]
    pub fn stddev(data: &[f64], period: usize, result: &mut [f64]) {
        let len = data.len().min(result.len());
        if period < 2 || len == 0 {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }
        if len < period {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }

        let inv_w = 1.0 / period as f64;
        let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

        let mut sum: f64 = data[..period].iter().sum();
        let mut sum_sq: f64 = data[..period].iter().map(|x| x * x).sum();
        let mean = sum * inv_w;
        let var = (sum_sq - sum * mean) * inv_w_minus_1;
        result[period - 1] = var.max(0.0).sqrt();

        for i in period..len {
            let old = data[i - period];
            let new = data[i];
            sum += new - old;
            sum_sq += new * new - old * old;
            let m = sum * inv_w;
            let var = (sum_sq - sum * m) * inv_w_minus_1;
            result[i] = var.max(0.0).sqrt();
        }

        for r in result.iter_mut().take(period - 1) {
            *r = f64::NAN;
        }
    }

    #[inline]
    pub fn zscore_optimized(data: &[f64], period: usize, result: &mut [f64]) {
        let len = data.len().min(result.len());
        if period < 2 || len == 0 {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }
        if len < period {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }

        let inv_w = 1.0 / period as f64;

        let mut sum: f64 = data[..period].iter().sum();
        let mut sum_sq: f64 = data[..period].iter().map(|x| x * x).sum();
        let mean = sum * inv_w;
        let var = (sum_sq - sum * mean) / (period as f64 - 1.0);
        let std = var.max(0.0).sqrt();
        result[period - 1] = if std.abs() < 1e-15 {
            0.0
        } else {
            (data[period - 1] - mean) / std
        };

        for i in period..len {
            let old = data[i - period];
            let new = data[i];
            sum += new - old;
            sum_sq += new * new - old * old;
            let m = sum * inv_w;
            let var = (sum_sq - sum * m) / (period as f64 - 1.0);
            let std = var.max(0.0).sqrt();
            result[i] = if std.abs() < 1e-15 {
                0.0
            } else {
                (data[i] - m) / std
            };
        }

        for r in result.iter_mut().take(period - 1) {
            *r = f64::NAN;
        }
    }

    #[inline]
    pub fn correl(x: &[f64], y: &[f64], period: usize, result: &mut [f64]) {
        let len = x.len().min(y.len()).min(result.len());
        if period < 2 || len == 0 {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }
        if len < period {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }

        let inv_w = 1.0 / period as f64;
        let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

        let mut sum_x: f64 = x[..period].iter().sum();
        let mut sum_y: f64 = y[..period].iter().sum();
        let mut sum_xy: f64 = x[..period].iter().zip(y[..period].iter()).map(|(xi, yi)| xi * yi).sum();
        let mut sum_x2: f64 = x[..period].iter().map(|xi| xi * xi).sum();
        let mut sum_y2: f64 = y[..period].iter().map(|yi| yi * yi).sum();

        let mean_x = sum_x * inv_w;
        let mean_y = sum_y * inv_w;
        let cov = (sum_xy - sum_x * mean_y) * inv_w_minus_1;
        let var_x = (sum_x2 - sum_x * mean_x) * inv_w_minus_1;
        let var_y = (sum_y2 - sum_y * mean_y) * inv_w_minus_1;

        let denom = (var_x * var_y).sqrt();
        result[period - 1] = if denom.abs() < 1e-15 {
            f64::NAN
        } else {
            cov / denom
        };

        for i in period..len {
            let old_x = x[i - period];
            let old_y = y[i - period];
            let new_x = x[i];
            let new_y = y[i];

            sum_x += new_x - old_x;
            sum_y += new_y - old_y;
            sum_xy += new_x * new_y - old_x * old_y;
            sum_x2 += new_x * new_x - old_x * old_x;
            sum_y2 += new_y * new_y - old_y * old_y;

            let mean_x = sum_x * inv_w;
            let mean_y = sum_y * inv_w;
            let cov = (sum_xy - sum_x * mean_y) * inv_w_minus_1;
            let var_x = (sum_x2 - sum_x * mean_x) * inv_w_minus_1;
            let var_y = (sum_y2 - sum_y * mean_y) * inv_w_minus_1;

            let denom = (var_x.max(0.0) * var_y.max(0.0)).sqrt();
            result[i] = if denom.abs() < 1e-15 {
                f64::NAN
            } else {
                cov / denom
            };
        }

        for r in result.iter_mut().take(period - 1) {
            *r = f64::NAN;
        }
    }

    #[inline]
    pub fn beta(asset: &[f64], benchmark: &[f64], period: usize, result: &mut [f64]) {
        let len = asset.len().min(benchmark.len()).min(result.len());
        if period < 2 || len == 0 {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }
        if len < period {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }

        let inv_w = 1.0 / period as f64;
        let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

        let mut sum_a: f64 = asset[..period].iter().sum();
        let mut sum_b: f64 = benchmark[..period].iter().sum();
        let mut sum_ab: f64 = asset[..period].iter().zip(benchmark[..period].iter()).map(|(a, b)| a * b).sum();
        let mut sum_b2: f64 = benchmark[..period].iter().map(|b| b * b).sum();

        let _mean_a = sum_a * inv_w;
        let mean_b = sum_b * inv_w;
        let cov = (sum_ab - sum_a * mean_b) * inv_w_minus_1;
        let var_b = (sum_b2 - sum_b * mean_b) * inv_w_minus_1;

        result[period - 1] = if var_b.abs() < 1e-15 {
            f64::NAN
        } else {
            cov / var_b
        };

        for i in period..len {
            let old_a = asset[i - period];
            let old_b = benchmark[i - period];
            let new_a = asset[i];
            let new_b = benchmark[i];

            sum_a += new_a - old_a;
            sum_b += new_b - old_b;
            sum_ab += new_a * new_b - old_a * old_b;
            sum_b2 += new_b * new_b - old_b * old_b;

            let _mean_a = sum_a * inv_w;
            let mean_b = sum_b * inv_w;
            let cov = (sum_ab - sum_a * mean_b) * inv_w_minus_1;
            let var_b = (sum_b2 - sum_b * mean_b) * inv_w_minus_1;

            result[i] = if var_b.abs() < 1e-15 {
                f64::NAN
            } else {
                cov / var_b.max(0.0)
            };
        }

        for r in result.iter_mut().take(period - 1) {
            *r = f64::NAN;
        }
    }

    #[inline]
    pub fn linear_reg_slope(data: &[f64], period: usize, result: &mut [f64]) {
        let len = data.len().min(result.len());
        if period < 2 || len == 0 {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }
        if len < period {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }

        let p = period as f64;
        let sum_x = p * (p - 1.0) / 2.0;
        let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
        let denom = p * sum_x2 - sum_x * sum_x;

        let mut sum_y: f64 = data[..period].iter().sum();
        let mut sum_xy: f64 = 0.0;
        for (j, &val) in data[..period].iter().enumerate() {
            sum_xy += j as f64 * val;
        }
        result[period - 1] = (p * sum_xy - sum_x * sum_y) / denom;

        for i in period..len {
            let old_val = data[i - period];
            let new_val = data[i];
            sum_xy += (period - 1) as f64 * new_val - (sum_y - old_val);
            sum_y += new_val - old_val;
            result[i] = (p * sum_xy - sum_x * sum_y) / denom;
        }

        for r in result.iter_mut().take(period - 1) {
            *r = f64::NAN;
        }
    }

    #[inline]
    pub fn linear_reg_intercept(data: &[f64], period: usize, result: &mut [f64]) {
        let len = data.len().min(result.len());
        if period < 2 || len == 0 {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }
        if len < period {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }

        let p = period as f64;
        let sum_x = p * (p - 1.0) / 2.0;
        let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
        let denom = p * sum_x2 - sum_x * sum_x;

        let mut sum_y: f64 = data[..period].iter().sum();
        let mut sum_xy: f64 = 0.0;
        for (j, &val) in data[..period].iter().enumerate() {
            sum_xy += j as f64 * val;
        }
        let slope = (p * sum_xy - sum_x * sum_y) / denom;
        result[period - 1] = (sum_y - slope * sum_x) / p;

        for i in period..len {
            let old_val = data[i - period];
            let new_val = data[i];
            sum_xy += (period - 1) as f64 * new_val - (sum_y - old_val);
            sum_y += new_val - old_val;
            let slope = (p * sum_xy - sum_x * sum_y) / denom;
            result[i] = (sum_y - slope * sum_x) / p;
        }

        for r in result.iter_mut().take(period - 1) {
            *r = f64::NAN;
        }
    }

    #[inline]
    pub fn linear_reg(data: &[f64], period: usize, result: &mut [f64]) {
        let len = data.len().min(result.len());
        if period < 2 || len == 0 {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }
        if len < period {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }

        let p = period as f64;
        let sum_x = p * (p - 1.0) / 2.0;
        let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
        let denom = p * sum_x2 - sum_x * sum_x;
        let last_x = (period - 1) as f64;

        let mut sum_y: f64 = data[..period].iter().sum();
        let mut sum_xy: f64 = 0.0;
        for (j, &val) in data[..period].iter().enumerate() {
            sum_xy += j as f64 * val;
        }
        let slope = (p * sum_xy - sum_x * sum_y) / denom;
        let intercept = (sum_y - slope * sum_x) / p;
        result[period - 1] = slope * last_x + intercept;

        for i in period..len {
            let old_val = data[i - period];
            let new_val = data[i];
            sum_xy += last_x * new_val - (sum_y - old_val);
            sum_y += new_val - old_val;
            let slope = (p * sum_xy - sum_x * sum_y) / denom;
            let intercept = (sum_y - slope * sum_x) / p;
            result[i] = slope * last_x + intercept;
        }

        for r in result.iter_mut().take(period - 1) {
            *r = f64::NAN;
        }
    }

    #[inline]
    pub fn linear_reg_angle(data: &[f64], period: usize, result: &mut [f64]) {
        let len = data.len().min(result.len());
        if period < 2 || len == 0 {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }

        let mut slope_result = vec![f64::NAN; len];
        Self::linear_reg_slope(data, period, &mut slope_result);

        for i in 0..len {
            if !slope_result[i].is_nan() {
                result[i] = slope_result[i].atan() * 180.0 / std::f64::consts::PI;
            } else {
                result[i] = f64::NAN;
            }
        }
    }

    #[inline]
    pub fn linear_reg_r2(data: &[f64], period: usize, result: &mut [f64]) {
        let len = data.len().min(result.len());
        if period < 2 || len == 0 {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }
        if len < period {
            for r in result.iter_mut().take(len) {
                *r = f64::NAN;
            }
            return;
        }

        let p = period as f64;
        let sum_x = p * (p - 1.0) / 2.0;
        let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
        let denom = p * sum_x2 - sum_x * sum_x;

        for val in result.iter_mut().take(period - 1) {
            *val = f64::NAN;
        }

        for i in period - 1..len {
            let start = i + 1 - period;
            let window = &data[start..=i];

            let sum_y: f64 = window.iter().sum();
            let mean_y = sum_y / p;

            let mut sum_xy: f64 = 0.0;
            for (j, &val) in window.iter().enumerate() {
                sum_xy += j as f64 * val;
            }

            let slope = (p * sum_xy - sum_x * sum_y) / denom;
            let intercept = (sum_y - slope * sum_x) / p;

            let ss_tot: f64 = window.iter().map(|yi| (yi - mean_y).powi(2)).sum();
            let ss_res: f64 = window.iter().enumerate().map(|(j, yi)| {
                let predicted = slope * j as f64 + intercept;
                (yi - predicted).powi(2)
            }).sum();

            result[i] = if ss_tot.abs() < 1e-15 {
                1.0
            } else {
                1.0 - ss_res / ss_tot
            };
        }
    }

    pub fn simd_stddev_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::stddev(data.as_slice().unwrap(), period, result.as_slice_mut().unwrap());
        result
    }

    pub fn simd_zscore_optimized_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::zscore_optimized(data.as_slice().unwrap(), period, result.as_slice_mut().unwrap());
        result
    }

    pub fn simd_correl_array(x: &Array1<f64>, y: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = x.len();
        let mut result = Array1::zeros(len);
        Self::correl(x.as_slice().unwrap(), y.as_slice().unwrap(), period, result.as_slice_mut().unwrap());
        result
    }

    pub fn simd_beta_array(asset: &Array1<f64>, benchmark: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = asset.len();
        let mut result = Array1::zeros(len);
        Self::beta(asset.as_slice().unwrap(), benchmark.as_slice().unwrap(), period, result.as_slice_mut().unwrap());
        result
    }

    pub fn simd_linear_reg_slope_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::linear_reg_slope(data.as_slice().unwrap(), period, result.as_slice_mut().unwrap());
        result
    }

    pub fn simd_linear_reg_intercept_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::linear_reg_intercept(data.as_slice().unwrap(), period, result.as_slice_mut().unwrap());
        result
    }

    pub fn simd_linear_reg_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::linear_reg(data.as_slice().unwrap(), period, result.as_slice_mut().unwrap());
        result
    }

    pub fn simd_linear_reg_angle_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::linear_reg_angle(data.as_slice().unwrap(), period, result.as_slice_mut().unwrap());
        result
    }

    pub fn simd_linear_reg_r2_array(data: &Array1<f64>, period: usize) -> Array1<f64> {
        let len = data.len();
        let mut result = Array1::zeros(len);
        Self::linear_reg_r2(data.as_slice().unwrap(), period, result.as_slice_mut().unwrap());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_add() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0];
        let b = [10.0, 20.0, 30.0, 40.0, 50.0];
        let mut result = [0.0; 5];
        SimdOps::add(&a, &b, &mut result);
        assert!((result[0] - 11.0).abs() < 1e-10);
        assert!((result[1] - 22.0).abs() < 1e-10);
        assert!((result[2] - 33.0).abs() < 1e-10);
        assert!((result[3] - 44.0).abs() < 1e-10);
        assert!((result[4] - 55.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_sub() {
        let a = [10.0, 20.0, 30.0];
        let b = [1.0, 2.0, 3.0];
        let mut result = [0.0; 3];
        SimdOps::sub(&a, &b, &mut result);
        assert!((result[0] - 9.0).abs() < 1e-10);
        assert!((result[1] - 18.0).abs() < 1e-10);
        assert!((result[2] - 27.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_mul() {
        let a = [2.0, 3.0, 4.0];
        let b = [5.0, 6.0, 7.0];
        let mut result = [0.0; 3];
        SimdOps::mul(&a, &b, &mut result);
        assert!((result[0] - 10.0).abs() < 1e-10);
        assert!((result[1] - 18.0).abs() < 1e-10);
        assert!((result[2] - 28.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_div() {
        let a = [10.0, 20.0, 30.0];
        let b = [2.0, 5.0, 10.0];
        let mut result = [0.0; 3];
        SimdOps::div(&a, &b, &mut result);
        assert!((result[0] - 5.0).abs() < 1e-10);
        assert!((result[1] - 4.0).abs() < 1e-10);
        assert!((result[2] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_div_by_zero() {
        let a = [10.0, 20.0];
        let b = [2.0, 0.0];
        let mut result = [0.0; 2];
        SimdOps::div(&a, &b, &mut result);
        assert!((result[0] - 5.0).abs() < 1e-10);
        assert!(result[1].is_nan());
    }

    #[test]
    fn test_simd_mod() {
        let a = [10.0, 7.0, 5.5, 8.0];
        let b = [3.0, 3.0, 2.0, 4.0];
        let mut result = [0.0; 4];
        SimdOps::simd_mod(&a, &b, &mut result);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 1.0).abs() < 1e-10);
        assert!((result[2] - 1.5).abs() < 1e-10);
        assert!((result[3] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_mod_by_zero() {
        let a = [10.0, 20.0];
        let b = [3.0, 0.0];
        let mut result = [0.0; 2];
        SimdOps::simd_mod(&a, &b, &mut result);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!(result[1].is_nan());
    }

    #[test]
    fn test_simd_pow() {
        let a = [2.0, 3.0, 4.0, 9.0];
        let b = [3.0, 2.0, 0.5, 0.5];
        let mut result = [0.0; 4];
        SimdOps::simd_pow(&a, &b, &mut result);
        assert!((result[0] - 8.0).abs() < 1e-10);
        assert!((result[1] - 9.0).abs() < 1e-10);
        assert!((result[2] - 2.0).abs() < 1e-10);
        assert!((result[3] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_scalar_mod_and_pow() {
        assert!((scalar::element_mod(10.0, 3.0) - 1.0).abs() < 1e-10);
        assert!((scalar::element_pow(2.0, 3.0) - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_mod_arrays() {
        let a = Array1::from_vec(vec![10.0, 7.0, 5.5]);
        let b = Array1::from_vec(vec![3.0, 3.0, 2.0]);
        let result = SimdOps::simd_mod_arrays(&a, &b);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 1.0).abs() < 1e-10);
        assert!((result[2] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_simd_pow_arrays() {
        let a = Array1::from_vec(vec![2.0, 3.0, 4.0]);
        let b = Array1::from_vec(vec![3.0, 2.0, 0.5]);
        let result = SimdOps::simd_pow_arrays(&a, &b);
        assert!((result[0] - 8.0).abs() < 1e-10);
        assert!((result[1] - 9.0).abs() < 1e-10);
        assert!((result[2] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_gt() {
        let a = [10.0, 5.0, 20.0];
        let b = [5.0, 10.0, 20.0];
        let mut result = [0.0; 3];
        SimdOps::gt(&a, &b, &mut result);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 0.0).abs() < 1e-10);
        assert!((result[2] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_lt() {
        let a = [5.0, 10.0, 20.0];
        let b = [10.0, 5.0, 20.0];
        let mut result = [0.0; 3];
        SimdOps::lt(&a, &b, &mut result);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 0.0).abs() < 1e-10);
        assert!((result[2] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_gte() {
        let a = [10.0, 5.0, 20.0];
        let b = [5.0, 10.0, 20.0];
        let mut result = [0.0; 3];
        SimdOps::gte(&a, &b, &mut result);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 0.0).abs() < 1e-10);
        assert!((result[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_lte() {
        let a = [5.0, 10.0, 20.0];
        let b = [10.0, 5.0, 20.0];
        let mut result = [0.0; 3];
        SimdOps::lte(&a, &b, &mut result);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 0.0).abs() < 1e-10);
        assert!((result[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_eq() {
        let a = [10.0, 5.0, 20.0];
        let b = [10.0, 10.0, 20.0];
        let mut result = [0.0; 3];
        SimdOps::eq(&a, &b, &mut result);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 0.0).abs() < 1e-10);
        assert!((result[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_neq() {
        let a = [10.0, 5.0, 20.0];
        let b = [10.0, 10.0, 20.0];
        let mut result = [0.0; 3];
        SimdOps::neq(&a, &b, &mut result);
        assert!((result[0] - 0.0).abs() < 1e-10);
        assert!((result[1] - 1.0).abs() < 1e-10);
        assert!((result[2] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_sma() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut result = [0.0; 5];
        SimdOps::sma(&data, 3, &mut result);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 2.0).abs() < 1e-10);
        assert!((result[3] - 3.0).abs() < 1e-10);
        assert!((result[4] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_ema() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let mut result = [0.0; 10];
        SimdOps::ema(&data, 5, &mut result);
        assert!(result[0].is_nan());
        assert!(result[3].is_nan());
        assert!(!result[4].is_nan());
        let first_ema = (1.0 + 2.0 + 3.0 + 4.0 + 5.0) / 5.0;
        assert!((result[4] - first_ema).abs() < 1e-10);
    }

    #[test]
    fn test_simd_add_arrays() {
        let a = Array1::from_vec(vec![1.0, 2.0, 3.0]);
        let b = Array1::from_vec(vec![10.0, 20.0, 30.0]);
        let result = SimdOps::simd_add_arrays(&a, &b);
        assert!((result[0] - 11.0).abs() < 1e-10);
        assert!((result[1] - 22.0).abs() < 1e-10);
        assert!((result[2] - 33.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_sma_array() {
        let data = Array1::from_vec(vec![2.0, 4.0, 6.0, 8.0, 10.0]);
        let result = SimdOps::simd_sma_array(&data, 3);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 4.0).abs() < 1e-10);
        assert!((result[3] - 6.0).abs() < 1e-10);
        assert!((result[4] - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_ema_array() {
        let data = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
        let result = SimdOps::simd_ema_array(&data, 3);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(!result[2].is_nan());
    }

    #[test]
    fn test_simd_abs() {
        let data = [-1.0, 2.0, -3.0, 0.0];
        let mut result = [0.0; 4];
        SimdOps::abs(&data, &mut result);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 2.0).abs() < 1e-10);
        assert!((result[2] - 3.0).abs() < 1e-10);
        assert!((result[3] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_max_elementwise() {
        let a = [1.0, 5.0, 3.0];
        let b = [2.0, 4.0, 6.0];
        let mut result = [0.0; 3];
        SimdOps::max_elementwise(&a, &b, &mut result);
        assert!((result[0] - 2.0).abs() < 1e-10);
        assert!((result[1] - 5.0).abs() < 1e-10);
        assert!((result[2] - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_min_elementwise() {
        let a = [1.0, 5.0, 3.0];
        let b = [2.0, 4.0, 6.0];
        let mut result = [0.0; 3];
        SimdOps::min_elementwise(&a, &b, &mut result);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 4.0).abs() < 1e-10);
        assert!((result[2] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_sma_period_one() {
        let data = [1.0, 2.0, 3.0];
        let mut result = [0.0; 3];
        SimdOps::sma(&data, 1, &mut result);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 2.0).abs() < 1e-10);
        assert!((result[2] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_empty_slices() {
        let a: [f64; 0] = [];
        let b: [f64; 0] = [];
        let mut result: [f64; 0] = [];
        SimdOps::add(&a, &b, &mut result);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_simd_ref_value() {
        let data = [10.0, 20.0, 30.0, 40.0, 50.0];
        let mut result = [0.0; 5];
        SimdOps::ref_value(&data, 2, &mut result);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 10.0).abs() < 1e-10);
        assert!((result[3] - 20.0).abs() < 1e-10);
        assert!((result[4] - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_ref_value_zero() {
        let data = [10.0, 20.0, 30.0];
        let mut result = [0.0; 3];
        SimdOps::ref_value(&data, 0, &mut result);
        assert!((result[0] - 10.0).abs() < 1e-10);
        assert!((result[1] - 20.0).abs() < 1e-10);
        assert!((result[2] - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_hhv() {
        let data = [3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let mut result = [0.0; 8];
        SimdOps::hhv(&data, 3, &mut result);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 4.0).abs() < 1e-10);
        assert!((result[3] - 4.0).abs() < 1e-10);
        assert!((result[4] - 5.0).abs() < 1e-10);
        assert!((result[5] - 9.0).abs() < 1e-10);
        assert!((result[6] - 9.0).abs() < 1e-10);
        assert!((result[7] - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_llv() {
        let data = [3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let mut result = [0.0; 8];
        SimdOps::llv(&data, 3, &mut result);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 1.0).abs() < 1e-10);
        assert!((result[3] - 1.0).abs() < 1e-10);
        assert!((result[4] - 1.0).abs() < 1e-10);
        assert!((result[5] - 1.0).abs() < 1e-10);
        assert!((result[6] - 2.0).abs() < 1e-10);
        assert!((result[7] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_sum() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut result = [0.0; 5];
        SimdOps::sum(&data, 3, &mut result);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 6.0).abs() < 1e-10);
        assert!((result[3] - 9.0).abs() < 1e-10);
        assert!((result[4] - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_count() {
        let condition = [1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        let mut result = [0.0; 6];
        SimdOps::count(&condition, 3, &mut result);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 2.0).abs() < 1e-10);
        assert!((result[3] - 2.0).abs() < 1e-10);
        assert!((result[4] - 2.0).abs() < 1e-10);
        assert!((result[5] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_hhv_period_one() {
        let data = [3.0, 1.0, 4.0];
        let mut result = [0.0; 3];
        SimdOps::hhv(&data, 1, &mut result);
        assert!((result[0] - 3.0).abs() < 1e-10);
        assert!((result[1] - 1.0).abs() < 1e-10);
        assert!((result[2] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_llv_period_one() {
        let data = [3.0, 1.0, 4.0];
        let mut result = [0.0; 3];
        SimdOps::llv(&data, 1, &mut result);
        assert!((result[0] - 3.0).abs() < 1e-10);
        assert!((result[1] - 1.0).abs() < 1e-10);
        assert!((result[2] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_sum_period_one() {
        let data = [3.0, 1.0, 4.0];
        let mut result = [0.0; 3];
        SimdOps::sum(&data, 1, &mut result);
        assert!((result[0] - 3.0).abs() < 1e-10);
        assert!((result[1] - 1.0).abs() < 1e-10);
        assert!((result[2] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_count_all_true() {
        let condition = [1.0, 1.0, 1.0, 1.0];
        let mut result = [0.0; 4];
        SimdOps::count(&condition, 2, &mut result);
        assert!(result[0].is_nan());
        assert!((result[1] - 2.0).abs() < 1e-10);
        assert!((result[2] - 2.0).abs() < 1e-10);
        assert!((result[3] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_ref_array() {
        let data = Array1::from_vec(vec![10.0, 20.0, 30.0, 40.0]);
        let result = SimdOps::simd_ref_array(&data, 1);
        assert!(result[0].is_nan());
        assert!((result[1] - 10.0).abs() < 1e-10);
        assert!((result[2] - 20.0).abs() < 1e-10);
        assert!((result[3] - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_hhv_array() {
        let data = Array1::from_vec(vec![3.0, 1.0, 4.0, 1.0, 5.0]);
        let result = SimdOps::simd_hhv_array(&data, 3);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_llv_array() {
        let data = Array1::from_vec(vec![3.0, 1.0, 4.0, 1.0, 5.0]);
        let result = SimdOps::simd_llv_array(&data, 3);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_sum_array() {
        let data = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let result = SimdOps::simd_sum_array(&data, 3);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_count_array() {
        let condition = Array1::from_vec(vec![1.0, 0.0, 1.0, 1.0, 0.0]);
        let result = SimdOps::simd_count_array(&condition, 3);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_gt_arrays() {
        let a = Array1::from_vec(vec![10.0, 5.0, 20.0]);
        let b = Array1::from_vec(vec![5.0, 10.0, 20.0]);
        let result = SimdOps::simd_gt_arrays(&a, &b);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 0.0).abs() < 1e-10);
        assert!((result[2] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_abs_array() {
        let data = Array1::from_vec(vec![-1.0, 2.0, -3.0]);
        let result = SimdOps::simd_abs_array(&data);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 2.0).abs() < 1e-10);
        assert!((result[2] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_max_elementwise_arrays() {
        let a = Array1::from_vec(vec![1.0, 5.0, 3.0]);
        let b = Array1::from_vec(vec![2.0, 4.0, 6.0]);
        let result = SimdOps::simd_max_elementwise_arrays(&a, &b);
        assert!((result[0] - 2.0).abs() < 1e-10);
        assert!((result[1] - 5.0).abs() < 1e-10);
        assert!((result[2] - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_min_elementwise_arrays() {
        let a = Array1::from_vec(vec![1.0, 5.0, 3.0]);
        let b = Array1::from_vec(vec![2.0, 4.0, 6.0]);
        let result = SimdOps::simd_min_elementwise_arrays(&a, &b);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 4.0).abs() < 1e-10);
        assert!((result[2] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_select() {
        let condition = [1.0, 0.0, 1.0, 0.0];
        let then_val = [10.0, 20.0, 30.0, 40.0];
        let else_val = [100.0, 200.0, 300.0, 400.0];
        let mut result = [0.0; 4];
        SimdOps::select(&condition, &then_val, &else_val, &mut result);
        assert!((result[0] - 10.0).abs() < 1e-10);
        assert!((result[1] - 200.0).abs() < 1e-10);
        assert!((result[2] - 30.0).abs() < 1e-10);
        assert!((result[3] - 400.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_ma_array() {
        let data = Array1::from_vec(vec![2.0, 4.0, 6.0, 8.0, 10.0]);
        let result = SimdOps::simd_ma_array(&data, 3);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 4.0).abs() < 1e-10);
        assert!((result[3] - 6.0).abs() < 1e-10);
        assert!((result[4] - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_simd_ema_array_opt() {
        let data = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
        let result = SimdOps::simd_ema_array_opt(&data, 5);
        assert!(result[0].is_nan());
        assert!(result[3].is_nan());
        assert!(!result[4].is_nan());
        let first_ema = (1.0 + 2.0 + 3.0 + 4.0 + 5.0) / 5.0;
        assert!((result[4] - first_ema).abs() < 1e-10);
    }
}
