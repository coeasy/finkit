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
