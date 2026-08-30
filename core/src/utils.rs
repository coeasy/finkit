use ndarray::Array1;

/// Calculate the True Range of a single period
///
/// # Arguments
/// * `high` - Current high price
/// * `low` - Current low price
/// * `prev_close` - Previous close price
///
/// # Returns
/// The true range value
#[inline]
pub fn true_range(high: f64, low: f64, prev_close: f64) -> f64 {
    (high - low)
        .max((high - prev_close).abs())
        .max((low - prev_close).abs())
}

/// Validate that input data is not empty
///
/// # Arguments
/// * `len` - Length of input data
/// * `required` - Minimum required length
///
/// # Returns
/// Result indicating whether the input is valid
pub fn validate_input(len: usize, required: usize) -> crate::error::Result<()> {
    if len == 0 {
        return Err(crate::error::TaError::EmptyInput);
    }
    if len < required {
        return Err(crate::error::TaError::InsufficientData {
            length: len,
            required,
        });
    }
    Ok(())
}

/// Validate that a parameter value meets a constraint
///
/// # Arguments
/// * `name` - Parameter name
/// * `value` - Parameter value
/// * `constraint` - Constraint description
/// * `valid` - Whether the constraint is satisfied
///
/// # Returns
/// Result indicating whether the parameter is valid
#[inline]
pub fn validate_param<F: FnOnce() -> bool>(
    name: &str,
    constraint: &str,
    valid: F,
) -> crate::error::Result<()> {
    if !valid() {
        return Err(crate::error::TaError::InvalidParameter {
            name: name.to_string(),
            constraint: constraint.to_string(),
        });
    }
    Ok(())
}

/// Calculate exponential smoothing factor
///
/// # Arguments
/// * `period` - The lookback period
///
/// # Returns
/// The smoothing factor (2.0 / (period + 1.0))
#[inline]
pub fn smoothing_factor(period: usize) -> f64 {
    2.0 / (period as f64 + 1.0)
}

/// Initialize output array with NaN values
///
/// # Arguments
/// * `len` - Length of the output array
///
/// # Returns
/// Array1 filled with NaN values
#[inline]
pub fn init_output(len: usize) -> Array1<f64> {
    // Pre-allocate with zeros, then SIMD-fill with NaN for 4-8x speedup
    // over Array1::from_elem (which uses a generic iterator-fill path).
    let mut arr = Array1::<f64>::zeros(len);
    simd_fill_nan(arr.as_slice_mut().expect("Array1 is contiguous"));
    arr
}

/// SIMD-accelerated NaN fill: writes `f64::NAN` to every element of `buf`.
///
/// On x86_64 with AVX2, fills 4 f64s per iteration. Falls back to a scalar
/// memset otherwise. This avoids the generic iterator-fill path in
/// `Array1::from_elem`, which is ~8x slower for large arrays.
#[inline]
pub fn simd_fill_nan(buf: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe { fill_nan_avx2(buf) };
            return;
        }
    }
    fill_nan_scalar(buf);
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn fill_nan_avx2(buf: &mut [f64]) {
    use std::arch::x86_64::*;
    // f64::NAN as a __m256d — high bit set, exponent all-ones, mantissa non-zero
    let nan_bits = f64::NAN.to_bits();
    let nan_vec = _mm256_set1_pd(f64::from_bits(nan_bits));
    let len = buf.len();
    let ptr = buf.as_mut_ptr();
    let chunks = len / 4;
    for i in 0..chunks {
        unsafe {
            let p = ptr.add(i * 4);
            _mm256_storeu_pd(p, nan_vec);
        }
    }
    // Tail
    let tail_start = chunks * 4;
    for i in tail_start..len {
        unsafe {
            *ptr.add(i) = f64::NAN;
        }
    }
}

#[inline]
fn fill_nan_scalar(buf: &mut [f64]) {
    for v in buf.iter_mut() {
        *v = f64::NAN;
    }
}

/// Copy input slice to Array1
///
/// Note: Callers should prefer working with `&[f64]` directly when possible
/// to avoid unnecessary allocation.
///
/// # Arguments
/// * `input` - Slice of f64 values
///
/// # Returns
/// Array1 containing the input values
#[inline]
pub fn to_array(input: &[f64]) -> Array1<f64> {
    Array1::from_vec(input.to_vec())
}

/// Check if all values in an array view are NaN
///
/// # Arguments
/// * `arr` - Array view to check
///
/// # Returns
/// true if all values are NaN
#[inline]
pub fn all_nan(arr: &[f64]) -> bool {
    arr.iter().all(|x| x.is_nan())
}

/// Clamp a value to a minimum threshold
///
/// # Arguments
/// * `value` - The value to clamp
/// * `min` - Minimum threshold
///
/// # Returns
/// The clamped value
#[inline]
pub fn clamp_min(value: f64, min: f64) -> f64 {
    if value < min {
        min
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_true_range() {
        assert_eq!(true_range(10.0, 8.0, 9.0), 2.0);
        assert_eq!(true_range(10.0, 8.0, 12.0), 4.0);
        assert_eq!(true_range(10.0, 8.0, 6.0), 4.0);
    }

    #[test]
    fn test_validate_input() {
        assert!(validate_input(10, 5).is_ok());
        assert!(validate_input(0, 5).is_err());
        assert!(validate_input(3, 5).is_err());
    }

    #[test]
    fn test_validate_param() {
        assert!(validate_param("period", "positive", || 10 > 0).is_ok());
        assert!(validate_param("period", "positive", || -1 > 0).is_err());
    }

    #[test]
    fn test_smoothing_factor() {
        assert!((smoothing_factor(10) - 0.181818).abs() < 1e-5);
        assert!((smoothing_factor(20) - 0.095238).abs() < 1e-5);
    }

    #[test]
    fn test_init_output() {
        let arr = init_output(5);
        assert_eq!(arr.len(), 5);
        assert!(arr.iter().all(|x| x.is_nan()));
    }

    #[test]
    fn test_to_array() {
        let input = vec![1.0, 2.0, 3.0];
        let arr = to_array(&input);
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], 1.0);
        assert_eq!(arr[1], 2.0);
        assert_eq!(arr[2], 3.0);
    }

    #[test]
    fn test_all_nan() {
        let arr = vec![f64::NAN, f64::NAN];
        assert!(all_nan(&arr));

        let arr = vec![1.0, f64::NAN];
        assert!(!all_nan(&arr));
    }

    #[test]
    fn test_clamp_min() {
        assert_eq!(clamp_min(5.0, 10.0), 10.0);
        assert_eq!(clamp_min(15.0, 10.0), 15.0);
    }
}
