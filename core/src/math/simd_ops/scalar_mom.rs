//! Portable scalar fallbacks for momentum and price averages.

/// Wilder-style recursive smoothing: `out[period-1] = mean(tr[0..period])`,
/// then `out[i] = (out[i-1] * (period-1) + tr[i]) / period`.
// Only reached from `simd_atr`, which is `std`-gated; without `std` this is
// unreferenced, hence the allow.
#[allow(dead_code)]
pub(crate) fn atr_wilder_scalar(tr: &[f64], period: usize, result: &mut [f64]) {
    let len = tr.len().min(result.len());
    if len < period || period == 0 {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }
    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
    // Seed with simple mean of the first `period` TRs.
    let mut sum = 0.0f64;
    for i in 0..period {
        sum += tr[i];
    }
    result[period - 1] = sum / period as f64;
    let p = period as f64;
    for i in period..len {
        result[i] = (result[i - 1] * (p - 1.0) + tr[i]) / p;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
pub(crate) unsafe fn mom_sse2(input: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = input.len().min(result.len());
    if period == 0 || len <= period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    for r in result.iter_mut().take(period) {
        *r = f64::NAN;
    }

    let input_ptr = input.as_ptr();
    let result_ptr = result.as_mut_ptr();
    let mut i = period;
    let unrolled_end = period + ((len - period) / 16) * 16;
    while i < unrolled_end {
        macro_rules! mom_sse2_pair {
            ($offset:expr) => {
                let current = _mm_loadu_pd(input_ptr.add(i + $offset));
                let previous = _mm_loadu_pd(input_ptr.add(i + $offset - period));
                _mm_storeu_pd(result_ptr.add(i + $offset), _mm_sub_pd(current, previous));
            };
        }
        mom_sse2_pair!(0);
        mom_sse2_pair!(2);
        mom_sse2_pair!(4);
        mom_sse2_pair!(6);
        mom_sse2_pair!(8);
        mom_sse2_pair!(10);
        mom_sse2_pair!(12);
        mom_sse2_pair!(14);
        i += 16;
    }
    while i + 1 < len {
        let current = _mm_loadu_pd(input_ptr.add(i));
        let previous = _mm_loadu_pd(input_ptr.add(i - period));
        _mm_storeu_pd(result_ptr.add(i), _mm_sub_pd(current, previous));
        i += 2;
    }
    while i < len {
        *result_ptr.add(i) = *input_ptr.add(i) - *input_ptr.add(i - period);
        i += 1;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
pub(crate) unsafe fn mom10_sse2(input: &[f64], result: &mut [f64]) {
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

    let input_ptr = input.as_ptr();
    let result_ptr = result.as_mut_ptr();
    let unrolled_end = PERIOD + ((len - PERIOD) / 16) * 16;
    let mut i = PERIOD;
    while i < unrolled_end {
        macro_rules! mom10_sse2_pair {
            ($offset:expr) => {
                let current = _mm_loadu_pd(input_ptr.add(i + $offset));
                let previous = _mm_loadu_pd(input_ptr.add(i + $offset - PERIOD));
                _mm_storeu_pd(result_ptr.add(i + $offset), _mm_sub_pd(current, previous));
            };
        }
        mom10_sse2_pair!(0);
        mom10_sse2_pair!(2);
        mom10_sse2_pair!(4);
        mom10_sse2_pair!(6);
        mom10_sse2_pair!(8);
        mom10_sse2_pair!(10);
        mom10_sse2_pair!(12);
        mom10_sse2_pair!(14);
        i += 16;
    }
    while i + 1 < len {
        let current = _mm_loadu_pd(input_ptr.add(i));
        let previous = _mm_loadu_pd(input_ptr.add(i - PERIOD));
        _mm_storeu_pd(result_ptr.add(i), _mm_sub_pd(current, previous));
        i += 2;
    }
    while i < len {
        *result_ptr.add(i) = *input_ptr.add(i) - *input_ptr.add(i - PERIOD);
        i += 1;
    }
}

// The scalar fallback behind `simd_mom`/`simd_mom10`. Those call it only under
// `cfg(not(all(feature = "std", target_arch = "x86_64")))`, so an x86_64 build
// never reaches it -- which is what makes it look unused here.
#[allow(dead_code)]
pub(crate) fn mom_scalar(input: &[f64], period: usize, result: &mut [f64]) {
    let len = input.len().min(result.len());
    if period == 0 || len <= period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    for r in result.iter_mut().take(period) {
        *r = f64::NAN;
    }

    // Keep the portable fallback allocation-free and bounds-check-light for
    // x86 runners without AVX2. The fixed unroll mirrors the SIMD path while
    // remaining valid on every supported target.
    let input_ptr = input.as_ptr();
    let result_ptr = result.as_mut_ptr();
    let mut i = period;
    let unrolled_end = period + ((len - period) / 8) * 8;
    while i < unrolled_end {
        unsafe {
            *result_ptr.add(i) = *input_ptr.add(i) - *input_ptr.add(i - period);
            *result_ptr.add(i + 1) = *input_ptr.add(i + 1) - *input_ptr.add(i + 1 - period);
            *result_ptr.add(i + 2) = *input_ptr.add(i + 2) - *input_ptr.add(i + 2 - period);
            *result_ptr.add(i + 3) = *input_ptr.add(i + 3) - *input_ptr.add(i + 3 - period);
            *result_ptr.add(i + 4) = *input_ptr.add(i + 4) - *input_ptr.add(i + 4 - period);
            *result_ptr.add(i + 5) = *input_ptr.add(i + 5) - *input_ptr.add(i + 5 - period);
            *result_ptr.add(i + 6) = *input_ptr.add(i + 6) - *input_ptr.add(i + 6 - period);
            *result_ptr.add(i + 7) = *input_ptr.add(i + 7) - *input_ptr.add(i + 7 - period);
        }
        i += 8;
    }
    while i < len {
        unsafe {
            *result_ptr.add(i) = *input_ptr.add(i) - *input_ptr.add(i - period);
        }
        i += 1;
    }
}

pub(crate) fn bop_scalar(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    result: &mut [f64],
) {
    let len = open
        .len()
        .min(high.len())
        .min(low.len())
        .min(close.len())
        .min(result.len());

    for i in 0..len {
        let range = high[i] - low[i];
        if range.abs() > crate::utils::NUMERIC_EPSILON {
            result[i] = (close[i] - open[i]) / range;
        } else {
            result[i] = 0.0;
        }
    }
}

pub(crate) fn avgprice_scalar(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    result: &mut [f64],
) {
    let len = open
        .len()
        .min(high.len())
        .min(low.len())
        .min(close.len())
        .min(result.len());

    for i in 0..len {
        result[i] = (open[i] + high[i] + low[i] + close[i]) / 4.0;
    }
}
