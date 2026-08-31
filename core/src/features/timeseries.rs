//! Time series feature transforms: lag, lead, diff, rolling_apply.

use ndarray::Array1;

/// Shift data forward by n positions (lag). First n values become NaN.
pub fn lag(data: &[f64], n: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    for i in n..len {
        out[i] = data[i - n];
    }
    out
}

/// Shift data backward by n positions (lead). Last n values become NaN.
pub fn lead(data: &[f64], n: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    for i in 0..len.saturating_sub(n) {
        out[i] = data[i + n];
    }
    out
}

/// N-th order difference: data[i] - data[i-n].
pub fn diff(data: &[f64], n: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    for i in n..len {
        out[i] = data[i] - data[i - n];
    }
    out
}

/// Percentage change: (data[i] - data[i-n]) / data[i-n].
pub fn pct_change(data: &[f64], n: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    for i in n..len {
        if data[i - n].abs() > 1e-15 {
            out[i] = (data[i] - data[i - n]) / data[i - n];
        } else {
            out[i] = 0.0;
        }
    }
    out
}

/// Apply a custom function over a rolling window.
pub fn rolling_apply<F>(data: &[f64], window: usize, f: F) -> Array1<f64>
where
    F: Fn(&[f64]) -> f64,
{
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    for i in (window - 1)..len {
        let start = i + 1 - window;
        out[i] = f(&data[start..=i]);
    }
    out
}

/// Generate multiple lag features at once.
pub fn multi_lag(data: &[f64], lags: &[usize]) -> Vec<(String, Array1<f64>)> {
    lags.iter()
        .map(|&n| (format!("lag_{}", n), lag(data, n)))
        .collect()
}

/// Generate multiple diff features at once.
pub fn multi_diff(data: &[f64], orders: &[usize]) -> Vec<(String, Array1<f64>)> {
    orders
        .iter()
        .map(|&n| (format!("diff_{}", n), diff(data, n)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lag() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = lag(&data, 2);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_eq!(result[2], 1.0);
        assert_eq!(result[3], 2.0);
        assert_eq!(result[4], 3.0);
    }

    #[test]
    fn test_lead() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = lead(&data, 2);
        assert_eq!(result[0], 3.0);
        assert_eq!(result[1], 4.0);
        assert_eq!(result[2], 5.0);
        assert!(result[3].is_nan());
        assert!(result[4].is_nan());
    }

    #[test]
    fn test_diff() {
        let data = vec![1.0, 3.0, 6.0, 10.0, 15.0];
        let result = diff(&data, 1);
        assert!(result[0].is_nan());
        assert_eq!(result[1], 2.0);
        assert_eq!(result[2], 3.0);
        assert_eq!(result[3], 4.0);
        assert_eq!(result[4], 5.0);
    }

    #[test]
    fn test_diff_order_2() {
        let data = vec![1.0, 3.0, 6.0, 10.0, 15.0];
        let result = diff(&data, 2);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_eq!(result[2], 5.0);
        assert_eq!(result[3], 7.0);
        assert_eq!(result[4], 9.0);
    }

    #[test]
    fn test_pct_change() {
        let data = vec![100.0, 110.0, 99.0, 110.0];
        let result = pct_change(&data, 1);
        assert!(result[0].is_nan());
        assert!((result[1] - 0.1).abs() < 1e-10);
        assert!((result[2] - (-0.1)).abs() < 1e-10);
    }

    #[test]
    fn test_rolling_apply() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = rolling_apply(&data, 3, |w| w.iter().sum::<f64>());
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_eq!(result[2], 6.0);
        assert_eq!(result[3], 9.0);
        assert_eq!(result[4], 12.0);
    }
}
