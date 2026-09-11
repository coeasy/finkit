//! Validation and overfitting-control utilities.

pub use finkit::features::{CombinatorialPurgedCV, EmbargoKFold, PurgedKFold, WalkForwardSplit};

/// Benjamini-Hochberg FDR adjusted p-values in original input order.
pub fn benjamini_hochberg(p_values: &[f64]) -> Vec<f64> {
    let mut indexed: Vec<(usize, f64)> = p_values
        .iter()
        .copied()
        .enumerate()
        .map(|(i, p)| (i, p.clamp(0.0, 1.0)))
        .collect();
    indexed.sort_by(|a, b| a.1.total_cmp(&b.1));
    let m = indexed.len();
    let mut adjusted_sorted = vec![1.0; m];
    let mut running = 1.0_f64;
    for rank0 in (0..m).rev() {
        let rank = rank0 + 1;
        let candidate = indexed[rank0].1 * m as f64 / rank as f64;
        running = running.min(candidate).min(1.0);
        adjusted_sorted[rank0] = running;
    }
    let mut out = vec![1.0; m];
    for (rank0, &(original, _)) in indexed.iter().enumerate() {
        out[original] = adjusted_sorted[rank0];
    }
    out
}

/// Holm step-down adjusted p-values.
pub fn holm(p_values: &[f64]) -> Vec<f64> {
    let mut indexed: Vec<(usize, f64)> = p_values.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| a.1.total_cmp(&b.1));
    let m = indexed.len();
    let mut out = vec![1.0; m];
    let mut running = 0.0_f64;
    for (rank0, &(original, p)) in indexed.iter().enumerate() {
        let adjusted = ((m - rank0) as f64 * p.clamp(0.0, 1.0)).min(1.0);
        running = running.max(adjusted);
        out[original] = running;
    }
    out
}

/// Interval metadata for label-aware purging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelInterval {
    pub start: i64,
    pub end: i64,
}

impl LabelInterval {
    pub fn overlaps(self, other: Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

/// Purge train indices whose label intervals overlap any test interval.
pub fn purge_overlapping_intervals(
    intervals: &[LabelInterval],
    train_indices: &[usize],
    test_indices: &[usize],
) -> Vec<usize> {
    let tests: Vec<LabelInterval> = test_indices.iter().filter_map(|&i| intervals.get(i).copied()).collect();
    train_indices
        .iter()
        .copied()
        .filter(|&i| intervals.get(i).is_some_and(|candidate| !tests.iter().any(|test| candidate.overlaps(*test))))
        .collect()
}

/// Deterministic block-bootstrap sample indices using a tiny LCG.
pub fn block_bootstrap_indices(n: usize, block: usize, seed: u64) -> Vec<usize> {
    if n == 0 || block == 0 { return Vec::new(); }
    let mut state = seed.max(1);
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let start = (state as usize) % n;
        for offset in 0..block {
            if out.len() == n { break; }
            out.push((start + offset) % n);
        }
    }
    out
}

/// Deterministic permutation of `0..n`.
pub fn permutation_indices(n: usize, seed: u64) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..n).collect();
    let mut state = seed.max(1);
    for i in (1..n).rev() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (state as usize) % (i + 1);
        indices.swap(i, j);
    }
    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bh_is_monotone_in_sorted_order() {
        let adjusted = benjamini_hochberg(&[0.01, 0.04, 0.03, 0.2]);
        assert!(adjusted.iter().all(|p| (0.0..=1.0).contains(p)));
        assert!(adjusted[0] <= adjusted[3]);
    }

    #[test]
    fn interval_purge_removes_overlap() {
        let intervals = [LabelInterval { start: 0, end: 5 }, LabelInterval { start: 6, end: 8 }, LabelInterval { start: 4, end: 7 }];
        assert_eq!(purge_overlapping_intervals(&intervals, &[0,1], &[2]), Vec::<usize>::new());
    }
}
