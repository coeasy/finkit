//! Time-series cross-validation splitters for financial ML.
//!
//! Implements purged k-fold, embargo, combinatorial purged CV, and walk-forward
//! expanding-window splits following López de Prado, *Advances in Financial Machine Learning*.

/// Contiguous fold boundary for fold `fold` out of `n_splits`.
fn fold_bounds(n_samples: usize, n_splits: usize, fold: usize) -> (usize, usize) {
    if n_splits == 0 || n_samples == 0 {
        return (0, 0);
    }
    let base = n_samples / n_splits;
    let rem = n_samples % n_splits;
    let start = fold * base + fold.min(rem);
    let end = start + base + usize::from(fold < rem);
    (start, end.min(n_samples))
}

/// Indices in `[0, n_samples)` that are not in `excluded`.
fn complement_indices(n_samples: usize, excluded: &[bool]) -> Vec<usize> {
    (0..n_samples)
        .filter(|&i| i < excluded.len() && !excluded[i])
        .collect()
}

/// Mark purge zones around a contiguous test block `[test_start, test_end)`.
fn mark_purge_zone(excluded: &mut [bool], test_start: usize, test_end: usize, purge_gap: usize) {
    let purge_lo = test_start.saturating_sub(purge_gap);
    let purge_hi = test_end.saturating_add(purge_gap).min(excluded.len());
    #[allow(clippy::needless_range_loop)]
    for i in purge_lo..purge_hi {
        excluded[i] = true;
    }
}

/// Generate all `k`-combinations of `0..n`.
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    if k == 0 || k > n {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut combo: Vec<usize> = (0..k).collect();
    loop {
        result.push(combo.clone());
        let mut i = k;
        while i > 0 {
            i -= 1;
            if combo[i] != i + n - k {
                break;
            }
            if i == 0 {
                return result;
            }
        }
        combo[i] += 1;
        for j in i + 1..k {
            combo[j] = combo[j - 1] + 1;
        }
    }
}

/// Purged k-fold cross-validation for ordered time series.
///
/// Each fold uses one contiguous test block. Training indices exclude the test
/// block and a purge buffer of `purge_gap` samples on each side of the test block
/// to reduce label-overlap leakage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgedKFold {
    n_splits: usize,
    purge_gap: usize,
}

impl PurgedKFold {
    /// Create a purged k-fold splitter.
    pub fn new(n_splits: usize, purge_gap: usize) -> Self {
        Self {
            n_splits,
            purge_gap,
        }
    }

    /// Return `(train_indices, test_indices)` pairs for `n_samples` observations.
    pub fn split(&self, n_samples: usize) -> Vec<(Vec<usize>, Vec<usize>)> {
        if n_samples == 0 || self.n_splits == 0 {
            return Vec::new();
        }

        let mut splits = Vec::with_capacity(self.n_splits);
        for fold in 0..self.n_splits {
            let (test_start, test_end) = fold_bounds(n_samples, self.n_splits, fold);
            if test_start >= test_end {
                continue;
            }

            let mut excluded = vec![false; n_samples];
            #[allow(clippy::needless_range_loop)]
            for i in test_start..test_end {
                excluded[i] = true;
            }
            mark_purge_zone(&mut excluded, test_start, test_end, self.purge_gap);

            let test_indices: Vec<usize> = (test_start..test_end).collect();
            let train_indices = complement_indices(n_samples, &excluded);
            splits.push((train_indices, test_indices));
        }
        splits
    }
}

/// K-fold with purge buffer and post-test embargo.
///
/// `embargo_pct` is the fraction of the test-block length used as an embargo
/// period immediately after the test set (training samples in that window are removed).
#[derive(Debug, Clone, PartialEq)]
pub struct EmbargoKFold {
    n_splits: usize,
    embargo_pct: f64,
}

impl EmbargoKFold {
    /// Create an embargo k-fold splitter.
    pub fn new(n_splits: usize, embargo_pct: f64) -> Self {
        Self {
            n_splits,
            embargo_pct: embargo_pct.max(0.0),
        }
    }

    /// Return `(train_indices, test_indices)` pairs for `n_samples` observations.
    pub fn split(&self, n_samples: usize) -> Vec<(Vec<usize>, Vec<usize>)> {
        if n_samples == 0 || self.n_splits == 0 {
            return Vec::new();
        }

        let mut splits = Vec::with_capacity(self.n_splits);
        for fold in 0..self.n_splits {
            let (test_start, test_end) = fold_bounds(n_samples, self.n_splits, fold);
            if test_start >= test_end {
                continue;
            }

            let test_len = test_end - test_start;
            let embargo = ((test_len as f64) * self.embargo_pct).ceil() as usize;
            let embargo_hi = (test_end + embargo).min(n_samples);

            let mut excluded = vec![false; n_samples];
            #[allow(clippy::needless_range_loop)]
            for i in test_start..test_end {
                excluded[i] = true;
            }
            #[allow(clippy::needless_range_loop)]
            for i in test_end..embargo_hi {
                excluded[i] = true;
            }

            let test_indices: Vec<usize> = (test_start..test_end).collect();
            let train_indices = complement_indices(n_samples, &excluded);
            splits.push((train_indices, test_indices));
        }
        splits
    }
}

/// Combinatorial purged cross-validation (CPCV).
///
/// The sample path is partitioned into `n_splits` contiguous groups. Every
/// combination of `n_test_splits` groups forms a test set; remaining groups are
/// training, with purge gaps applied around each test segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinatorialPurgedCV {
    n_splits: usize,
    n_test_splits: usize,
    purge_gap: usize,
}

impl CombinatorialPurgedCV {
    /// Create a combinatorial purged CV splitter.
    pub fn new(n_splits: usize, n_test_splits: usize, purge_gap: usize) -> Self {
        Self {
            n_splits,
            n_test_splits,
            purge_gap,
        }
    }

    /// Return `(train_indices, test_indices)` pairs for `n_samples` observations.
    pub fn split(&self, n_samples: usize) -> Vec<(Vec<usize>, Vec<usize>)> {
        if n_samples == 0
            || self.n_splits == 0
            || self.n_test_splits == 0
            || self.n_test_splits > self.n_splits
        {
            return Vec::new();
        }

        let mut splits = Vec::new();
        for test_groups in combinations(self.n_splits, self.n_test_splits) {
            let mut excluded = vec![false; n_samples];
            let mut test_indices = Vec::new();

            for &group in &test_groups {
                let (test_start, test_end) = fold_bounds(n_samples, self.n_splits, group);
                if test_start >= test_end {
                    continue;
                }
                #[allow(clippy::needless_range_loop)]
                for i in test_start..test_end {
                    excluded[i] = true;
                    test_indices.push(i);
                }
                mark_purge_zone(&mut excluded, test_start, test_end, self.purge_gap);
            }

            test_indices.sort_unstable();
            test_indices.dedup();

            let train_indices = complement_indices(n_samples, &excluded);
            if !train_indices.is_empty() && !test_indices.is_empty() {
                splits.push((train_indices, test_indices));
            }
        }
        splits
    }
}

/// Expanding-window walk-forward cross-validation.
///
/// Training always starts at index `0` and grows by one test window per fold.
/// The first fold uses `min_train_size` training samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkForwardSplit {
    n_splits: usize,
    min_train_size: usize,
}

impl WalkForwardSplit {
    /// Create a walk-forward splitter.
    pub fn new(n_splits: usize, min_train_size: usize) -> Self {
        Self {
            n_splits,
            min_train_size,
        }
    }

    /// Return `(train_indices, test_indices)` pairs for `n_samples` observations.
    pub fn split(&self, n_samples: usize) -> Vec<(Vec<usize>, Vec<usize>)> {
        if n_samples == 0 || self.n_splits == 0 || self.min_train_size >= n_samples {
            return Vec::new();
        }

        let remaining = n_samples - self.min_train_size;
        let test_size = remaining / self.n_splits;
        if test_size == 0 {
            return Vec::new();
        }

        let mut splits = Vec::with_capacity(self.n_splits);
        for fold in 0..self.n_splits {
            let train_end = self.min_train_size + fold * test_size;
            let test_start = train_end;
            let test_end = if fold + 1 == self.n_splits {
                n_samples
            } else {
                train_end + test_size
            };
            if test_start >= test_end || train_end == 0 {
                continue;
            }

            let train_indices: Vec<usize> = (0..train_end).collect();
            let test_indices: Vec<usize> = (test_start..test_end).collect();
            splits.push((train_indices, test_indices));
        }
        splits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_overlap(train: &[usize], test: &[usize]) -> bool {
        use std::collections::HashSet;
        let test_set: HashSet<_> = test.iter().copied().collect();
        train.iter().all(|i| !test_set.contains(i))
    }

    fn contiguous_segments(indices: &[usize]) -> Vec<(usize, usize)> {
        if indices.is_empty() {
            return Vec::new();
        }
        let mut segments = Vec::new();
        let mut start = indices[0];
        let mut prev = indices[0];
        for &idx in &indices[1..] {
            if idx == prev + 1 {
                prev = idx;
            } else {
                segments.push((start, prev + 1));
                start = idx;
                prev = idx;
            }
        }
        segments.push((start, prev + 1));
        segments
    }

    fn purge_gap_ok(train: &[usize], test: &[usize], purge_gap: usize) -> bool {
        for (seg_start, seg_end) in contiguous_segments(test) {
            let lo = seg_start.saturating_sub(purge_gap);
            let hi = seg_end + purge_gap;
            if !train.iter().all(|&t| t < lo || t >= hi) {
                return false;
            }
        }
        true
    }

    #[test]
    fn test_purged_kfold_no_overlap() {
        let cv = PurgedKFold::new(5, 3);
        let splits = cv.split(100);
        assert_eq!(splits.len(), 5);

        for (train, test) in splits {
            assert!(!train.is_empty());
            assert!(!test.is_empty());
            assert!(no_overlap(&train, &test));
            assert!(purge_gap_ok(&train, &test, 3));
        }
    }

    #[test]
    fn test_embargo_kfold_embargo_applied() {
        let cv = EmbargoKFold::new(4, 0.25);
        let splits = cv.split(80);
        assert_eq!(splits.len(), 4);

        for (train, test) in splits {
            assert!(no_overlap(&train, &test));
            let test_max = *test.iter().max().unwrap();
            let test_len = test.len();
            let embargo = ((test_len as f64) * 0.25).ceil() as usize;
            for &t in &train {
                assert!(
                    t <= test_max || t > test_max + embargo,
                    "train index {t} falls inside embargo after test_max={test_max}, embargo={embargo}"
                );
            }
        }
    }

    #[test]
    fn test_walk_forward_expanding() {
        let cv = WalkForwardSplit::new(4, 20);
        let splits = cv.split(100);
        assert!(!splits.is_empty());

        let mut prev_train_len = 0;
        for (train, test) in &splits {
            assert!(train.len() > prev_train_len, "train window should expand");
            prev_train_len = train.len();
            assert!(no_overlap(train, test));
            assert_eq!(train.first(), Some(&0));
            assert_eq!(
                train.last().copied().unwrap() + 1,
                test.first().copied().unwrap()
            );
        }
    }

    #[test]
    fn test_combinatorial_purged() {
        let cv = CombinatorialPurgedCV::new(6, 2, 2);
        let splits = cv.split(120);
        // C(6,2) = 15
        assert_eq!(splits.len(), 15);

        for (train, test) in splits {
            assert!(!train.is_empty());
            assert!(!test.is_empty());
            assert!(no_overlap(&train, &test));
            assert!(purge_gap_ok(&train, &test, 2));
            for &i in train.iter().chain(test.iter()) {
                assert!(i < 120);
            }
        }
    }
}
