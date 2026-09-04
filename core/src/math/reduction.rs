//! Allocation-free scalar reduction kernels used by language bindings and hot paths.
//!
//! These functions intentionally operate on borrowed slices and return scalars.  They
//! form the canonical reduction layer for the Architecture 3.0 typed fast path: f64
//! input stays f64, f32 input stays f32, and no temporary Vec/ndarray is allocated.

/// Sum an f64 slice with four independent accumulators so LLVM can vectorize the loop.
#[inline]
pub fn sum_f64(input: &[f64]) -> f64 {
    let mut s0 = 0.0;
    let mut s1 = 0.0;
    let mut s2 = 0.0;
    let mut s3 = 0.0;
    let mut chunks = input.chunks_exact(4);
    for c in &mut chunks {
        s0 += c[0];
        s1 += c[1];
        s2 += c[2];
        s3 += c[3];
    }
    let mut sum = (s0 + s1) + (s2 + s3);
    for &value in chunks.remainder() {
        sum += value;
    }
    sum
}

/// Sum an f32 slice without promoting it to f64.
#[inline]
pub fn sum_f32(input: &[f32]) -> f32 {
    let mut s0 = 0.0f32;
    let mut s1 = 0.0f32;
    let mut s2 = 0.0f32;
    let mut s3 = 0.0f32;
    let mut chunks = input.chunks_exact(4);
    for c in &mut chunks {
        s0 += c[0];
        s1 += c[1];
        s2 += c[2];
        s3 += c[3];
    }
    let mut sum = (s0 + s1) + (s2 + s3);
    for &value in chunks.remainder() {
        sum += value;
    }
    sum
}

/// Arithmetic mean for f64 input. Empty input returns NaN at the raw-kernel layer.
#[inline]
pub fn mean_f64(input: &[f64]) -> f64 {
    if input.is_empty() {
        f64::NAN
    } else {
        sum_f64(input) / input.len() as f64
    }
}

/// Arithmetic mean for f32 input. Empty input returns NaN at the raw-kernel layer.
#[inline]
pub fn mean_f32(input: &[f32]) -> f32 {
    if input.is_empty() {
        f32::NAN
    } else {
        sum_f32(input) / input.len() as f32
    }
}

/// Maximum value for f64 input. NaN propagates; empty input returns NaN.
#[inline]
pub fn max_f64(input: &[f64]) -> f64 {
    let Some((&first, rest)) = input.split_first() else {
        return f64::NAN;
    };
    if first.is_nan() {
        return f64::NAN;
    }
    let mut max = first;
    for &value in rest {
        if value.is_nan() {
            return f64::NAN;
        }
        if value > max {
            max = value;
        }
    }
    max
}

/// Maximum value for f32 input. NaN propagates; empty input returns NaN.
#[inline]
pub fn max_f32(input: &[f32]) -> f32 {
    let Some((&first, rest)) = input.split_first() else {
        return f32::NAN;
    };
    if first.is_nan() {
        return f32::NAN;
    }
    let mut max = first;
    for &value in rest {
        if value.is_nan() {
            return f32::NAN;
        }
        if value > max {
            max = value;
        }
    }
    max
}

/// Minimum value for f64 input. NaN propagates; empty input returns NaN.
#[inline]
pub fn min_f64(input: &[f64]) -> f64 {
    let Some((&first, rest)) = input.split_first() else {
        return f64::NAN;
    };
    if first.is_nan() {
        return f64::NAN;
    }
    let mut min = first;
    for &value in rest {
        if value.is_nan() {
            return f64::NAN;
        }
        if value < min {
            min = value;
        }
    }
    min
}

/// Minimum value for f32 input. NaN propagates; empty input returns NaN.
#[inline]
pub fn min_f32(input: &[f32]) -> f32 {
    let Some((&first, rest)) = input.split_first() else {
        return f32::NAN;
    };
    if first.is_nan() {
        return f32::NAN;
    }
    let mut min = first;
    for &value in rest {
        if value.is_nan() {
            return f32::NAN;
        }
        if value < min {
            min = value;
        }
    }
    min
}

/// Population variance for f64 input using a two-pass algorithm.
///
/// Two passes are deliberate: the hot path remains allocation free while avoiding the
/// large cancellation error of `E[x^2] - E[x]^2` for price series with a large offset.
#[inline]
pub fn variance_f64(input: &[f64]) -> f64 {
    if input.is_empty() {
        return f64::NAN;
    }
    let mean = mean_f64(input);
    let mut s0 = 0.0;
    let mut s1 = 0.0;
    let mut s2 = 0.0;
    let mut s3 = 0.0;
    let mut chunks = input.chunks_exact(4);
    for c in &mut chunks {
        let d0 = c[0] - mean;
        let d1 = c[1] - mean;
        let d2 = c[2] - mean;
        let d3 = c[3] - mean;
        s0 = d0.mul_add(d0, s0);
        s1 = d1.mul_add(d1, s1);
        s2 = d2.mul_add(d2, s2);
        s3 = d3.mul_add(d3, s3);
    }
    let mut sum_sq = (s0 + s1) + (s2 + s3);
    for &value in chunks.remainder() {
        let d = value - mean;
        sum_sq = d.mul_add(d, sum_sq);
    }
    sum_sq / input.len() as f64
}

/// Population variance for f32 input without a mandatory f64 promotion.
#[inline]
pub fn variance_f32(input: &[f32]) -> f32 {
    if input.is_empty() {
        return f32::NAN;
    }
    let mean = mean_f32(input);
    let mut s0 = 0.0f32;
    let mut s1 = 0.0f32;
    let mut s2 = 0.0f32;
    let mut s3 = 0.0f32;
    let mut chunks = input.chunks_exact(4);
    for c in &mut chunks {
        let d0 = c[0] - mean;
        let d1 = c[1] - mean;
        let d2 = c[2] - mean;
        let d3 = c[3] - mean;
        s0 = d0.mul_add(d0, s0);
        s1 = d1.mul_add(d1, s1);
        s2 = d2.mul_add(d2, s2);
        s3 = d3.mul_add(d3, s3);
    }
    let mut sum_sq = (s0 + s1) + (s2 + s3);
    for &value in chunks.remainder() {
        let d = value - mean;
        sum_sq = d.mul_add(d, sum_sq);
    }
    sum_sq / input.len() as f32
}

/// Population standard deviation for f64 input.
#[inline]
pub fn stddev_f64(input: &[f64]) -> f64 {
    variance_f64(input).sqrt()
}

/// Population standard deviation for f32 input.
#[inline]
pub fn stddev_f32(input: &[f32]) -> f32 {
    variance_f32(input).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_reductions_match_expected_values() {
        let f64s = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(sum_f64(&f64s), 10.0);
        assert_eq!(mean_f64(&f64s), 2.5);
        assert_eq!(min_f64(&f64s), 1.0);
        assert_eq!(max_f64(&f64s), 4.0);
        assert!((stddev_f64(&f64s) - 1.118_033_988_749_895).abs() < 1e-12);

        let f32s = [1.0f32, 2.0, 3.0, 4.0];
        assert_eq!(sum_f32(&f32s), 10.0);
        assert_eq!(mean_f32(&f32s), 2.5);
        assert_eq!(min_f32(&f32s), 1.0);
        assert_eq!(max_f32(&f32s), 4.0);
        assert!((stddev_f32(&f32s) - 1.118_034).abs() < 1e-5);
    }

    #[test]
    fn empty_and_nan_contract_is_explicit() {
        assert!(mean_f64(&[]).is_nan());
        assert!(min_f64(&[]).is_nan());
        assert!(max_f64(&[]).is_nan());
        assert!(stddev_f64(&[]).is_nan());
        assert!(max_f64(&[1.0, f64::NAN, 2.0]).is_nan());
        assert!(min_f32(&[1.0, f32::NAN, 2.0]).is_nan());
    }
}
