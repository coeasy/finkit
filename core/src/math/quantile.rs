//! Canonical quantile and binning primitives.

/// Quantile interpolation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantileInterpolation {
    Linear,
    Lower,
    Higher,
    Midpoint,
    Nearest,
}

/// Histogram/binning policy.
#[derive(Debug, Clone, PartialEq)]
pub enum BinPolicy {
    EqualWidth(usize),
    EqualFrequency(usize),
    ExplicitEdges(Vec<f64>),
}

/// Quantile from an already sorted finite slice.
pub fn quantile_sorted(values: &[f64], q: f64, interpolation: QuantileInterpolation) -> f64 {
    if values.is_empty() || !q.is_finite() || !(0.0..=1.0).contains(&q) {
        return f64::NAN;
    }
    if values.len() == 1 {
        return values[0];
    }
    let pos = q * (values.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    match interpolation {
        QuantileInterpolation::Linear => {
            let fraction = pos - lo as f64;
            values[lo] * (1.0 - fraction) + values[hi] * fraction
        }
        QuantileInterpolation::Lower => values[lo],
        QuantileInterpolation::Higher => values[hi],
        QuantileInterpolation::Midpoint => (values[lo] + values[hi]) * 0.5,
        QuantileInterpolation::Nearest => values[pos.round() as usize],
    }
}

/// Quantile over finite values only.
pub fn quantile(values: &[f64], q: f64, interpolation: QuantileInterpolation) -> f64 {
    let mut sorted: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    sorted.sort_by(f64::total_cmp);
    quantile_sorted(&sorted, q, interpolation)
}

/// Build bin edges using one canonical implementation.
pub fn bin_edges(values: &[f64], policy: &BinPolicy) -> Vec<f64> {
    let mut finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() {
        return Vec::new();
    }
    finite.sort_by(f64::total_cmp);
    match policy {
        BinPolicy::ExplicitEdges(edges) => {
            if edges.len() < 2
                || edges.iter().any(|v| !v.is_finite())
                || edges.windows(2).any(|w| w[0] >= w[1])
            {
                Vec::new()
            } else {
                edges.clone()
            }
        }
        BinPolicy::EqualWidth(bins) => {
            if *bins == 0 {
                return Vec::new();
            }
            let min = finite[0];
            let max = *finite.last().unwrap();
            if (max - min).abs() <= f64::EPSILON {
                return vec![min, max + f64::EPSILON];
            }
            let width = (max - min) / *bins as f64;
            (0..=*bins).map(|i| min + width * i as f64).collect()
        }
        BinPolicy::EqualFrequency(bins) => {
            if *bins == 0 {
                return Vec::new();
            }
            let min = finite[0];
            let max = *finite.last().unwrap();
            if (max - min).abs() <= f64::EPSILON {
                return vec![min, max + f64::EPSILON];
            }
            let mut edges = Vec::with_capacity(*bins + 1);
            edges.push(min);
            for i in 1..*bins {
                let edge = quantile_sorted(
                    &finite,
                    i as f64 / *bins as f64,
                    QuantileInterpolation::Linear,
                );
                if edge > *edges.last().unwrap() {
                    edges.push(edge);
                }
            }
            edges.push(max + f64::EPSILON);
            edges
        }
    }
}

/// Assign zero-based bins. Non-finite values receive `None`.
pub fn discretize(values: &[f64], policy: &BinPolicy) -> Vec<Option<usize>> {
    let edges = bin_edges(values, policy);
    if edges.len() < 2 {
        return vec![None; values.len()];
    }
    let bins = edges.len() - 1;
    values
        .iter()
        .map(|&value| {
            if !value.is_finite() {
                return None;
            }
            let idx = edges.partition_point(|edge| *edge <= value);
            Some(idx.saturating_sub(1).min(bins - 1))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_frequency_bins_cover_values() {
        let values: Vec<f64> = (0..100).map(|v| v as f64).collect();
        let bins = discretize(&values, &BinPolicy::EqualFrequency(5));
        assert!(bins.iter().all(Option::is_some));
        assert!(bins.iter().flatten().all(|b| *b < 5));
    }

    #[test]
    fn explicit_edges_validate() {
        let values = [0.2, 0.8];
        assert!(bin_edges(&values, &BinPolicy::ExplicitEdges(vec![0.0, 0.0])).is_empty());
    }
}
