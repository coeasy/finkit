use ndarray::{Array1, ArrayView1};
use std::collections::HashMap;

pub fn rank(input: ArrayView1<'_, f64>) -> Array1<f64> {
    let len = input.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    let mut pairs: Vec<(usize, f64)> = input
        .iter()
        .enumerate()
        .filter_map(|(idx, v)| if v.is_nan() { None } else { Some((idx, *v)) })
        .collect();
    pairs.sort_by(|a, b| a.1.total_cmp(&b.1));
    let n = pairs.len();
    if n == 0 {
        return out;
    }

    for (rank_idx, (idx, _)) in pairs.iter().enumerate() {
        out[*idx] = (rank_idx + 1) as f64 / n as f64;
    }
    out
}

/// Percentile rank finite values into `[0, 1]`, averaging ties.
pub fn percentile_rank(input: ArrayView1<'_, f64>) -> Array1<f64> {
    let finite: Vec<(usize, f64)> = input
        .iter()
        .enumerate()
        .filter_map(|(idx, value)| value.is_finite().then_some((idx, *value)))
        .collect();
    let count = finite.len();
    let mut sorted: Vec<f64> = finite.iter().map(|(_, value)| *value).collect();
    sorted.sort_by(f64::total_cmp);
    let mut output = Array1::from_elem(input.len(), f64::NAN);
    if count == 0 {
        return output;
    }
    for (index, value) in finite {
        let below = sorted.partition_point(|candidate| *candidate < value);
        let equal_end = sorted.partition_point(|candidate| *candidate <= value);
        let midpoint = (below + equal_end - 1) as f64 / 2.0;
        output[index] = if count == 1 {
            0.0
        } else {
            midpoint / (count - 1) as f64
        };
    }
    output
}

/// Population z-score over the current cross-sectional row.
pub fn zscore(input: ArrayView1<'_, f64>) -> Array1<f64> {
    let finite: Vec<f64> = input
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();
    if finite.is_empty() {
        return Array1::from_elem(input.len(), f64::NAN);
    }
    let mean = finite.iter().sum::<f64>() / finite.len() as f64;
    let variance = finite
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / finite.len() as f64;
    let std = variance.sqrt();
    input
        .iter()
        .map(|value| {
            if !value.is_finite() {
                f64::NAN
            } else if std <= f64::EPSILON {
                0.0
            } else {
                (value - mean) / std
            }
        })
        .collect()
}

pub fn scale(input: ArrayView1<'_, f64>, k: f64) -> Array1<f64> {
    let denom: f64 = input.iter().filter(|v| !v.is_nan()).map(|v| v.abs()).sum();
    if denom <= f64::EPSILON {
        return Array1::from_elem(input.len(), 0.0);
    }
    input
        .iter()
        .map(|v| if v.is_nan() { f64::NAN } else { v * k / denom })
        .collect()
}

pub fn signed_power(input: ArrayView1<'_, f64>, a: f64) -> Array1<f64> {
    input
        .iter()
        .map(|v| {
            if v.is_nan() {
                f64::NAN
            } else {
                v.signum() * v.abs().powf(a)
            }
        })
        .collect()
}

pub fn indneutralize(input: ArrayView1<'_, f64>, groups: ArrayView1<'_, f64>) -> Array1<f64> {
    let len = input.len().min(groups.len());
    let mut out = Array1::from_elem(len, f64::NAN);
    let mut grouped_sum: HashMap<i64, (f64, usize)> = HashMap::new();

    for i in 0..len {
        let v = input[i];
        let g = groups[i];
        if v.is_nan() || g.is_nan() {
            continue;
        }
        let key = g as i64;
        let entry = grouped_sum.entry(key).or_insert((0.0, 0));
        entry.0 += v;
        entry.1 += 1;
    }

    for i in 0..len {
        let v = input[i];
        let g = groups[i];
        if v.is_nan() || g.is_nan() {
            continue;
        }
        let key = g as i64;
        if let Some((sum, cnt)) = grouped_sum.get(&key) {
            out[i] = v - (sum / *cnt as f64);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_rank_scale() {
        let x = array![3.0, 1.0, 2.0, 4.0];
        let r = rank(x.view());
        assert!((r[1] - 0.25).abs() < 1e-10);
        assert!((r[3] - 1.0).abs() < 1e-10);

        let s = scale(x.view(), 1.0);
        let sum_abs: f64 = s.iter().map(|v| v.abs()).sum();
        assert!((sum_abs - 1.0).abs() < 1e-10);

        let percentile = percentile_rank(x.view());
        assert_eq!(percentile[1], 0.0);
        assert_eq!(percentile[3], 1.0);

        let z = zscore(x.view());
        let endpoint = (9.0_f64 / 5.0).sqrt();
        assert!((z[1] + endpoint).abs() < 1e-10);
        assert!((z[3] - endpoint).abs() < 1e-10);
    }

    #[test]
    fn test_signed_power() {
        let x = array![-2.0, -1.0, 0.0, 2.0];
        let y = signed_power(x.view(), 2.0);
        assert_eq!(y[0], -4.0);
        assert_eq!(y[1], -1.0);
        assert_eq!(y[3], 4.0);
    }

    #[test]
    fn test_indneutralize() {
        let x = array![10.0, 14.0, 21.0, 25.0, 30.0];
        let g = array![1.0, 1.0, 2.0, 2.0, 2.0];
        let y = indneutralize(x.view(), g.view());
        assert!((y[0] + 2.0).abs() < 1e-10);
        assert!((y[1] - 2.0).abs() < 1e-10);
        assert!((y[2] + 4.3333333333).abs() < 1e-8);
        assert!((y[3] + 0.3333333333).abs() < 1e-8);
        assert!((y[4] - 4.6666666667).abs() < 1e-8);
    }
}
