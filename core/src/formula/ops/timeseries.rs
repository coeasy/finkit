use ndarray::{s, Array1, ArrayView1};

#[inline]
fn has_nan(values: &[f64]) -> bool {
    values.iter().any(|v| v.is_nan())
}

fn rolling_window_start(i: usize, window: usize) -> usize {
    (i + 1).saturating_sub(window)
}

pub fn delay(input: ArrayView1<'_, f64>, period: usize) -> Array1<f64> {
    let len = input.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if period >= len {
        return out;
    }
    for i in period..len {
        out[i] = input[i - period];
    }
    out
}

pub fn delta(input: ArrayView1<'_, f64>, period: usize) -> Array1<f64> {
    let delayed = delay(input, period);
    let mut out = Array1::from_elem(input.len(), f64::NAN);
    for i in 0..input.len() {
        if input[i].is_nan() || delayed[i].is_nan() {
            continue;
        }
        out[i] = input[i] - delayed[i];
    }
    out
}

pub fn ts_argmax(input: ArrayView1<'_, f64>, window: usize) -> Array1<f64> {
    let len = input.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window == 0 {
        return out;
    }
    for i in 0..len {
        let start = rolling_window_start(i, window);
        let slice = input.slice(s![start..=i]);
        let mut max_v = f64::NEG_INFINITY;
        let mut max_idx = None;
        for (j, v) in slice.iter().enumerate() {
            if !v.is_nan() && *v >= max_v {
                max_v = *v;
                max_idx = Some(j);
            }
        }
        if let Some(idx) = max_idx {
            out[i] = idx as f64;
        }
    }
    out
}

pub fn ts_argmin(input: ArrayView1<'_, f64>, window: usize) -> Array1<f64> {
    let len = input.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window == 0 {
        return out;
    }
    for i in 0..len {
        let start = rolling_window_start(i, window);
        let slice = input.slice(s![start..=i]);
        let mut min_v = f64::INFINITY;
        let mut min_idx = None;
        for (j, v) in slice.iter().enumerate() {
            if !v.is_nan() && *v <= min_v {
                min_v = *v;
                min_idx = Some(j);
            }
        }
        if let Some(idx) = min_idx {
            out[i] = idx as f64;
        }
    }
    out
}

pub fn ts_rank(input: ArrayView1<'_, f64>, window: usize) -> Array1<f64> {
    let len = input.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window == 0 {
        return out;
    }

    for i in 0..len {
        let start = rolling_window_start(i, window);
        let slice = input.slice(s![start..=i]);
        let current = input[i];
        if current.is_nan() {
            continue;
        }
        let mut valid = 0usize;
        let mut less_or_equal = 0usize;
        for v in slice.iter() {
            if v.is_nan() {
                continue;
            }
            valid += 1;
            if *v <= current {
                less_or_equal += 1;
            }
        }
        if valid > 0 {
            out[i] = less_or_equal as f64 / valid as f64;
        }
    }
    out
}

pub fn covariance(left: ArrayView1<'_, f64>, right: ArrayView1<'_, f64>, window: usize) -> Array1<f64> {
    let len = left.len().min(right.len());
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 2 {
        return out;
    }

    for i in 0..len {
        if i + 1 < window {
            continue;
        }
        let start = i + 1 - window;
        let xs = left.slice(s![start..=i]).to_vec();
        let ys = right.slice(s![start..=i]).to_vec();
        if has_nan(&xs) || has_nan(&ys) {
            continue;
        }
        let mean_x = xs.iter().sum::<f64>() / window as f64;
        let mean_y = ys.iter().sum::<f64>() / window as f64;
        let mut cov = 0.0;
        for j in 0..window {
            cov += (xs[j] - mean_x) * (ys[j] - mean_y);
        }
        out[i] = cov / window as f64;
    }
    out
}

pub fn correlation(left: ArrayView1<'_, f64>, right: ArrayView1<'_, f64>, window: usize) -> Array1<f64> {
    let len = left.len().min(right.len());
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 2 {
        return out;
    }

    for i in 0..len {
        if i + 1 < window {
            continue;
        }
        let start = i + 1 - window;
        let xs = left.slice(s![start..=i]).to_vec();
        let ys = right.slice(s![start..=i]).to_vec();
        if has_nan(&xs) || has_nan(&ys) {
            continue;
        }
        let mean_x = xs.iter().sum::<f64>() / window as f64;
        let mean_y = ys.iter().sum::<f64>() / window as f64;
        let mut cov = 0.0;
        let mut var_x = 0.0;
        let mut var_y = 0.0;
        for j in 0..window {
            let dx = xs[j] - mean_x;
            let dy = ys[j] - mean_y;
            cov += dx * dy;
            var_x += dx * dx;
            var_y += dy * dy;
        }
        if var_x > 0.0 && var_y > 0.0 {
            out[i] = cov / (var_x.sqrt() * var_y.sqrt());
        }
    }
    out
}

pub fn decay_linear(input: ArrayView1<'_, f64>, window: usize) -> Array1<f64> {
    let len = input.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window == 0 {
        return out;
    }

    let weight_sum = (window * (window + 1) / 2) as f64;
    for i in 0..len {
        if i + 1 < window {
            continue;
        }
        let start = i + 1 - window;
        let xs = input.slice(s![start..=i]).to_vec();
        if has_nan(&xs) {
            continue;
        }
        let mut weighted = 0.0;
        for (j, x) in xs.iter().enumerate() {
            weighted += (j + 1) as f64 * *x;
        }
        out[i] = weighted / weight_sum;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_delay_and_delta() {
        let x = array![1.0, 2.0, 3.0, 4.0, 5.0];
        let delayed = delay(x.view(), 2);
        assert!(delayed[0].is_nan());
        assert!(delayed[1].is_nan());
        assert_eq!(delayed[2], 1.0);
        let d = delta(x.view(), 2);
        assert!(d[0].is_nan());
        assert!(d[1].is_nan());
        assert_eq!(d[4], 2.0);
    }

    #[test]
    fn test_ts_rank_arg() {
        let x = array![1.0, 3.0, 2.0, 5.0, 4.0];
        let r = ts_rank(x.view(), 3);
        assert!((r[2] - (2.0 / 3.0)).abs() < 1e-10);
        assert!((r[3] - 1.0).abs() < 1e-10);

        let amax = ts_argmax(x.view(), 3);
        assert_eq!(amax[3], 2.0);
        let amin = ts_argmin(x.view(), 3);
        assert_eq!(amin[3], 1.0);
    }

    #[test]
    fn test_cov_corr_decay() {
        let x = array![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = array![2.0, 4.0, 6.0, 8.0, 10.0];
        let cov = covariance(x.view(), y.view(), 3);
        assert!(cov[1].is_nan());
        assert!((cov[4] - (4.0 / 3.0)).abs() < 1e-10);

        let corr = correlation(x.view(), y.view(), 3);
        assert!((corr[4] - 1.0).abs() < 1e-10);

        let dec = decay_linear(x.view(), 3);
        let expected = (1.0 * 3.0 + 2.0 * 4.0 + 3.0 * 5.0) / 6.0;
        assert!((dec[4] - expected).abs() < 1e-10);
    }
}
