//! Canonical ranking primitives with explicit tie handling.

/// Tie handling for equal values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TiePolicy {
    Average,
    Min,
    Max,
    First,
    Dense,
}

/// Rank finite values starting at 1. Non-finite inputs remain `NaN`.
pub fn ranks(values: &[f64], policy: TiePolicy) -> Vec<f64> {
    let mut finite: Vec<(usize, f64)> = values
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, value)| value.is_finite())
        .collect();
    finite.sort_by(|left, right| left.1.total_cmp(&right.1).then(left.0.cmp(&right.0)));

    let mut out = vec![f64::NAN; values.len()];
    let mut start = 0usize;
    let mut dense_rank = 1usize;
    while start < finite.len() {
        let mut end = start + 1;
        while end < finite.len() && finite[end].1 == finite[start].1 {
            end += 1;
        }
        match policy {
            TiePolicy::First => {
                for (offset, &(idx, _)) in finite[start..end].iter().enumerate() {
                    out[idx] = (start + offset + 1) as f64;
                }
            }
            TiePolicy::Dense => {
                for &(idx, _) in &finite[start..end] {
                    out[idx] = dense_rank as f64;
                }
                dense_rank += 1;
            }
            TiePolicy::Average | TiePolicy::Min | TiePolicy::Max => {
                let rank = match policy {
                    TiePolicy::Average => (start + 1 + end) as f64 / 2.0,
                    TiePolicy::Min => (start + 1) as f64,
                    TiePolicy::Max => end as f64,
                    _ => unreachable!(),
                };
                for &(idx, _) in &finite[start..end] {
                    out[idx] = rank;
                }
            }
        }
        start = end;
    }
    out
}

/// Fractional ranks, matching the common Spearman tie convention.
pub fn fractional_ranks(values: &[f64]) -> Vec<f64> {
    ranks(values, TiePolicy::Average)
}

/// Percentile ranks in `[0, 1]`, averaging ties. A single finite value maps to `0.5`.
pub fn percentile_rank(values: &[f64]) -> Vec<f64> {
    let finite_count = values.iter().filter(|value| value.is_finite()).count();
    if finite_count == 0 {
        return vec![f64::NAN; values.len()];
    }
    if finite_count == 1 {
        return values
            .iter()
            .map(|value| if value.is_finite() { 0.5 } else { f64::NAN })
            .collect();
    }
    let denominator = (finite_count - 1) as f64;
    ranks(values, TiePolicy::Average)
        .into_iter()
        .map(|rank| {
            if rank.is_finite() {
                (rank - 1.0) / denominator
            } else {
                f64::NAN
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_ties_are_fractional() {
        let r = fractional_ranks(&[10.0, 20.0, 20.0, 30.0]);
        assert_eq!(r, vec![1.0, 2.5, 2.5, 4.0]);
    }

    #[test]
    fn percentile_rank_preserves_nan() {
        let r = percentile_rank(&[1.0, f64::NAN, 3.0]);
        assert_eq!(r[0], 0.0);
        assert!(r[1].is_nan());
        assert_eq!(r[2], 1.0);
    }
}
