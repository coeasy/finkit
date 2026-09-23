//! Canonical ranking primitives with explicit tie handling.

use crate::error::{Result, TaError};

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

/// Rolling percentile rank written directly into caller-owned output.
///
/// The value at each bar is the rank of that bar's value **within its own
/// window**, scaled to `(0, 1]`: `ranks(window, Average)[last] / window`. Ties
/// take the average rank, and the denominator is the number of observations
/// rather than `n - 1` — that is pandas' `rolling(N).rank(pct=True)`, which is
/// what Qlib's `Rank` operator calls, and it is deliberately *not*
/// [`percentile_rank`], whose `(n - 1)` denominator and `0.5` single-value
/// convention serve a different contract.
///
/// A bar is `NaN` unless its window is full and every value in it is finite;
/// see [`crate::math::quantile::rolling_quantile_into`] for why finkit's window
/// convention differs from Qlib's `min_periods=1`.
///
/// # Errors
///
/// Returns [`TaError::EmptyInput`] for empty `data`, [`TaError::InvalidParameter`]
/// for `window == 0` or an `output` that does not match `data` in length.
pub fn rolling_rank_pct_into(data: &[f64], window: usize, output: &mut [f64]) -> Result<()> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if window == 0 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    if output.len() != data.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as data".to_string(),
        });
    }

    output.fill(f64::NAN);
    if window > data.len() {
        return Ok(());
    }
    #[allow(clippy::cast_precision_loss)] // `window` is a bar count, far below 2^53
    let scale = window as f64;
    let mut scratch: Vec<f64> = Vec::with_capacity(window);
    for index in window - 1..data.len() {
        let slice = &data[index + 1 - window..=index];
        if slice.iter().any(|value| !value.is_finite()) {
            continue;
        }
        scratch.clear();
        scratch.extend_from_slice(slice);
        // The last element of `ranks` is this bar's rank within the window.
        output[index] = ranks(&scratch, TiePolicy::Average)[window - 1] / scale;
    }
    Ok(())
}

/// Rolling percentile rank as an owned series.
///
/// # Errors
///
/// Propagates [`rolling_rank_pct_into`]'s errors.
pub fn rolling_rank_pct(data: &[f64], window: usize) -> Result<Vec<f64>> {
    let mut output = vec![f64::NAN; data.len()];
    rolling_rank_pct_into(data, window, &mut output)?;
    Ok(output)
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
