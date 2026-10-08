//! AVX2 kernels for rolling statistics and linear regression.

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn stddev_avx2(data: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let inv_w = 1.0 / period as f64;
    let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;
    let chunks = period / 4;
    for c in 0..chunks {
        let off = c * 4;
        let v = _mm256_loadu_pd(data.as_ptr().add(off));
        sum_sq += horizontal_sum_avx2(_mm256_mul_pd(v, v));
        sum += horizontal_sum_avx2(v);
    }
    for &val in data.iter().take(period).skip(chunks * 4) {
        sum += val;
        sum_sq += val * val;
    }

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

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn horizontal_sum_avx2(v: core::arch::x86_64::__m256d) -> f64 {
    use core::arch::x86_64::*;
    let v_low = _mm256_castpd256_pd128(v);
    let v_high = _mm256_extractf128_pd(v, 1);
    let v_sum = _mm_add_pd(v_low, v_high);
    let v_sum2 = _mm_unpackhi_pd(v_sum, v_sum);
    let v_sum3 = _mm_add_sd(v_sum, v_sum2);
    _mm_cvtsd_f64(v_sum3)
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn variance_avx2(data: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let inv_w = 1.0 / period as f64;
    let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;
    let chunks = period / 4;
    for c in 0..chunks {
        let off = c * 4;
        let v = _mm256_loadu_pd(data.as_ptr().add(off));
        sum_sq += horizontal_sum_avx2(_mm256_mul_pd(v, v));
        sum += horizontal_sum_avx2(v);
    }
    for &val in data.iter().take(period).skip(chunks * 4) {
        sum += val;
        sum_sq += val * val;
    }

    let mean = sum * inv_w;
    let var = (sum_sq - sum * mean) * inv_w_minus_1;
    result[period - 1] = var.max(0.0);

    for i in period..len {
        let old = data[i - period];
        let new = data[i];
        sum += new - old;
        sum_sq += new * new - old * old;
        let m = sum * inv_w;
        let var = (sum_sq - sum * m) * inv_w_minus_1;
        result[i] = var.max(0.0);
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn zscore_optimized_avx2(data: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let inv_w = 1.0 / period as f64;
    let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;
    let chunks = period / 4;
    for c in 0..chunks {
        let off = c * 4;
        let v = _mm256_loadu_pd(data.as_ptr().add(off));
        sum_sq += horizontal_sum_avx2(_mm256_mul_pd(v, v));
        sum += horizontal_sum_avx2(v);
    }
    for &val in data.iter().take(period).skip(chunks * 4) {
        sum += val;
        sum_sq += val * val;
    }

    let mean = sum * inv_w;
    let var = (sum_sq - sum * mean) * inv_w_minus_1;
    let std = var.max(0.0).sqrt();
    result[period - 1] = if std.abs() < crate::utils::NUMERIC_EPSILON {
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
        let var = (sum_sq - sum * m) * inv_w_minus_1;
        let std = var.max(0.0).sqrt();
        result[i] = if std.abs() < crate::utils::NUMERIC_EPSILON {
            0.0
        } else {
            (data[i] - m) / std
        };
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn correl_avx2(x: &[f64], y: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = x.len().min(y.len()).min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let inv_w = 1.0 / period as f64;
    let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

    let mut sum_x: f64 = 0.0;
    let mut sum_y: f64 = 0.0;
    let mut sum_xy: f64 = 0.0;
    let mut sum_x2: f64 = 0.0;
    let mut sum_y2: f64 = 0.0;

    let chunks = period / 4;
    for c in 0..chunks {
        let off = c * 4;
        let vx = _mm256_loadu_pd(x.as_ptr().add(off));
        let vy = _mm256_loadu_pd(y.as_ptr().add(off));
        sum_x += horizontal_sum_avx2(vx);
        sum_y += horizontal_sum_avx2(vy);
        sum_xy += horizontal_sum_avx2(_mm256_mul_pd(vx, vy));
        sum_x2 += horizontal_sum_avx2(_mm256_mul_pd(vx, vx));
        sum_y2 += horizontal_sum_avx2(_mm256_mul_pd(vy, vy));
    }
    for i in chunks * 4..period {
        sum_x += x[i];
        sum_y += y[i];
        sum_xy += x[i] * y[i];
        sum_x2 += x[i] * x[i];
        sum_y2 += y[i] * y[i];
    }

    let mean_x = sum_x * inv_w;
    let mean_y = sum_y * inv_w;
    let cov = (sum_xy - sum_x * mean_y) * inv_w_minus_1;
    let var_x = (sum_x2 - sum_x * mean_x) * inv_w_minus_1;
    let var_y = (sum_y2 - sum_y * mean_y) * inv_w_minus_1;
    let denom = (var_x.max(0.0) * var_y.max(0.0)).sqrt();
    result[period - 1] = if denom.abs() < crate::utils::NUMERIC_EPSILON {
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
        result[i] = if denom.abs() < crate::utils::NUMERIC_EPSILON {
            f64::NAN
        } else {
            cov / denom
        };
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn beta_avx2(
    asset: &[f64],
    benchmark: &[f64],
    period: usize,
    result: &mut [f64],
) {
    use core::arch::x86_64::*;
    let len = asset.len().min(benchmark.len()).min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let inv_w = 1.0 / period as f64;
    let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

    let mut sum_a: f64 = 0.0;
    let mut sum_b: f64 = 0.0;
    let mut sum_ab: f64 = 0.0;
    let mut sum_b2: f64 = 0.0;

    let chunks = period / 4;
    for c in 0..chunks {
        let off = c * 4;
        let va = _mm256_loadu_pd(asset.as_ptr().add(off));
        let vb = _mm256_loadu_pd(benchmark.as_ptr().add(off));
        sum_a += horizontal_sum_avx2(va);
        sum_b += horizontal_sum_avx2(vb);
        sum_ab += horizontal_sum_avx2(_mm256_mul_pd(va, vb));
        sum_b2 += horizontal_sum_avx2(_mm256_mul_pd(vb, vb));
    }
    for i in chunks * 4..period {
        sum_a += asset[i];
        sum_b += benchmark[i];
        sum_ab += asset[i] * benchmark[i];
        sum_b2 += benchmark[i] * benchmark[i];
    }

    let _mean_a = sum_a * inv_w;
    let mean_b = sum_b * inv_w;
    let cov = (sum_ab - sum_a * mean_b) * inv_w_minus_1;
    let var_b = (sum_b2 - sum_b * mean_b) * inv_w_minus_1;
    result[period - 1] = if var_b.abs() < crate::utils::NUMERIC_EPSILON {
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
        result[i] = if var_b.abs() < crate::utils::NUMERIC_EPSILON {
            f64::NAN
        } else {
            cov / var_b.max(0.0)
        };
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn linreg_slope_avx2(data: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let p = period as f64;
    let p1 = (period - 1) as f64;
    // Weights run **oldest first**: the oldest bar of the window carries
    // `period - 1` and the newest carries 0. That is TA-Lib's orientation, and
    // it is the whole reason this kernel was 47% slower before — it is not a
    // cosmetic re-indexing. In this orientation the per-bar advance is
    //
    //     sum_xy += sum_y - period * old
    //
    // whereas the other orientation needs
    //
    //     sum_xy += (period - 1) * new - (sum_y - old)
    //
    // which puts two dependent subtractions between `sum_y` and the accumulator
    // instead of one addition. Both are the same number; only the rounding and
    // the dependency graph differ.
    //
    // `divisor` is TA-Lib's `SumX * SumX - period * SumXSqr`, the negative of
    // the other orientation's denominator. The two sign flips (numerator terms
    // reverse with the weights) cancel, so the reported slope is unchanged —
    // `core/examples/talib_gap_probe.rs` asserts the two forms agree to 1e-12
    // over the whole series rather than arguing it.
    //
    // Measured on the 10,000-bar series, seeding and output held identical:
    // 30.4 us for the old orientation against 20.6 us for this one, and 21.6 us
    // for TA-Lib's own C loop — i.e. this form is *ahead* of the reference it
    // was 42% behind. See V4 plan §44.
    let sum_x = p * p1 / 2.0;
    let sum_x2 = p * p1 * (2.0 * p - 1.0) / 6.0;
    let divisor = sum_x * sum_x - p * sum_x2;
    let inv_divisor = 1.0 / divisor;

    let mut sum_y: f64 = 0.0;
    let mut sum_xy: f64 = 0.0;

    let chunks = period / 4;
    let indices: [f64; 4] = [0.0, 1.0, 2.0, 3.0];
    for c in 0..chunks {
        let off = c * 4;
        let v_data = _mm256_loadu_pd(data.as_ptr().add(off));
        // Lane weights for this chunk are `p1 - (off + lane)`, so the vector is
        // `(p1 - off) - [0, 1, 2, 3]` rather than `off + [0, 1, 2, 3]`.
        let v_w = _mm256_sub_pd(
            _mm256_set1_pd(p1 - off as f64),
            _mm256_loadu_pd(indices.as_ptr()),
        );
        sum_y += horizontal_sum_avx2(v_data);
        sum_xy += horizontal_sum_avx2(_mm256_mul_pd(v_data, v_w));
    }
    for (i, &val) in data.iter().enumerate().take(period).skip(chunks * 4) {
        sum_y += val;
        sum_xy += (p1 - i as f64) * val;
    }

    result[period - 1] = (p * sum_xy - sum_x * sum_y) * inv_divisor;

    for i in period..len {
        let old_val = data[i - period];
        let new_val = data[i];
        sum_xy += sum_y - p * old_val;
        sum_y += new_val - old_val;
        result[i] = (p * sum_xy - sum_x * sum_y) * inv_divisor;
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn linreg_avx2(data: &[f64], period: usize, result: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let p = period as f64;
    let last_x = (period - 1) as f64;
    // Same oldest-first weight orientation as `linreg_slope_avx2`; see that
    // kernel for why the direction is load-bearing. `linreg` folds slope and
    // intercept into the emitted value, but it accumulates `sum_xy` the same
    // way, so the two have to agree or the family stops having one answer.
    let sum_x = p * last_x / 2.0;
    let sum_x2 = p * last_x * (2.0 * p - 1.0) / 6.0;
    let divisor = sum_x * sum_x - p * sum_x2;
    // See `linreg_scalar`: reciprocal multiplies for both loop-invariant divisors.
    let inv_divisor = 1.0 / divisor;
    let inv_p = 1.0 / p;

    let mut sum_y: f64 = 0.0;
    let mut sum_xy: f64 = 0.0;

    let chunks = period / 4;
    // Same constant-vector trick as `linreg_slope_avx2`: the lane indices are
    // known at compile time, so shifting them by the chunk offset beats
    // materialising a `Vec<f64>` of `period` values on every call.
    let indices: [f64; 4] = [0.0, 1.0, 2.0, 3.0];
    for c in 0..chunks {
        let off = c * 4;
        let v_data = _mm256_loadu_pd(data.as_ptr().add(off));
        // Lane weights are `last_x - (off + lane)`, i.e. `(last_x - off) - [0,1,2,3]`.
        let v_w = _mm256_sub_pd(
            _mm256_set1_pd(last_x - off as f64),
            _mm256_loadu_pd(indices.as_ptr()),
        );
        sum_y += horizontal_sum_avx2(v_data);
        sum_xy += horizontal_sum_avx2(_mm256_mul_pd(v_data, v_w));
    }
    for (i, &val) in data.iter().enumerate().take(period).skip(chunks * 4) {
        sum_y += val;
        sum_xy += (last_x - i as f64) * val;
    }

    let slope = (p * sum_xy - sum_x * sum_y) * inv_divisor;
    let intercept = (sum_y - slope * sum_x) * inv_p;
    result[period - 1] = slope * last_x + intercept;

    for i in period..len {
        let old_val = data[i - period];
        let new_val = data[i];
        sum_xy += sum_y - p * old_val;
        sum_y += new_val - old_val;
        let slope = (p * sum_xy - sum_x * sum_y) * inv_divisor;
        let intercept = (sum_y - slope * sum_x) * inv_p;
        result[i] = slope * last_x + intercept;
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}
