//! Population and characteristic stability indices (PSI / CSI) for distribution drift.

const EPSILON: f64 = 1e-10;

/// Histogram binning strategy for stability metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinningMethod {
    /// Fixed-width bins spanning the expected sample range.
    EqualWidth,
    /// Quantile-based bins from the expected sample.
    EqualFrequency,
    /// Placeholder for user-defined edges; currently uses equal-width on expected.
    Custom,
}

/// Population Stability Index between two distributions.
///
/// Compares histograms of `expected` (baseline) and `actual` using bins derived from
/// `expected` according to `method`. Values above 0.2 typically indicate significant drift.
pub fn psi(expected: &[f64], actual: &[f64], bins: usize, method: BinningMethod) -> f64 {
    if bins == 0 || expected.is_empty() || actual.is_empty() {
        return f64::NAN;
    }
    let edges = bin_edges(expected, bins, method);
    let expected_pct = histogram_proportions(expected, &edges, bins);
    let actual_pct = histogram_proportions(actual, &edges, bins);
    stability_index(&expected_pct, &actual_pct)
}

/// Characteristic Stability Index for a single feature.
///
/// Uses equal-frequency binning on the expected (baseline) distribution.
pub fn csi(expected: &[f64], actual: &[f64], bins: usize) -> f64 {
    psi(expected, actual, bins, BinningMethod::EqualFrequency)
}

/// Rolling PSI: each window is compared to the first `window`-length segment as baseline.
pub fn rolling_psi(
    data: &[f64],
    window: usize,
    bins: usize,
    method: BinningMethod,
) -> Vec<f64> {
    if window == 0 || bins == 0 || data.len() < window {
        return Vec::new();
    }
    let baseline = &data[0..window];
    (0..=data.len() - window)
        .map(|start| psi(baseline, &data[start..start + window], bins, method))
        .collect()
}

fn stability_index(expected_pct: &[f64], actual_pct: &[f64]) -> f64 {
    expected_pct
        .iter()
        .zip(actual_pct.iter())
        .map(|(&e, &a)| {
            let e = e.max(EPSILON);
            let a = a.max(EPSILON);
            (a - e) * (a / e).ln()
        })
        .sum()
}

fn bin_edges(expected: &[f64], bins: usize, method: BinningMethod) -> Vec<f64> {
    match method {
        BinningMethod::EqualWidth | BinningMethod::Custom => equal_width_edges(expected, bins),
        BinningMethod::EqualFrequency => equal_frequency_edges(expected, bins),
    }
}

fn equal_width_edges(data: &[f64], bins: usize) -> Vec<f64> {
    let min = data.iter().copied().fold(f64::INFINITY, f64::min);
    let max = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if (max - min).abs() < EPSILON {
        return vec![min, max + EPSILON];
    }
    let width = (max - min) / bins as f64;
    (0..=bins)
        .map(|i| min + width * i as f64)
        .collect()
}

fn equal_frequency_edges(data: &[f64], bins: usize) -> Vec<f64> {
    let mut sorted: Vec<f64> = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let min = sorted[0];
    let max = sorted[n - 1];
    if (max - min).abs() < EPSILON {
        return vec![min, max + EPSILON];
    }
    let mut edges = Vec::with_capacity(bins + 1);
    edges.push(min);
    for i in 1..bins {
        let idx = n * i / bins;
        let idx = idx.min(n - 1);
        let edge = sorted[idx];
        if edge > *edges.last().unwrap_or(&min) {
            edges.push(edge);
        }
    }
    edges.push(max + EPSILON);
    edges
}

fn histogram_proportions(data: &[f64], edges: &[f64], bins: usize) -> Vec<f64> {
    let effective_bins = edges.len().saturating_sub(1).max(1).min(bins);
    let mut counts = vec![0.0_f64; effective_bins];
    let n = data.len() as f64;
    if n == 0.0 {
        return counts;
    }
    for &x in data {
        let bin = assign_bin(x, edges, effective_bins);
        counts[bin] += 1.0;
    }
    counts.iter().map(|&c| c / n).collect()
}

fn assign_bin(x: f64, edges: &[f64], bins: usize) -> usize {
    if bins <= 1 {
        return 0;
    }
    if x <= edges[0] {
        return 0;
    }
    for i in 0..bins - 1 {
        if x < edges[i + 1] {
            return i;
        }
    }
    bins - 1
}

// ─── Factor Turnover Ratio ─────────────────────────────────────────

/// Factor Turnover Ratio between two cross-sectional rank vectors.
///
/// Measures how much factor rankings changed between two periods:
/// `turnover = sum(|rank(t) - rank(t-1)|) / N`
///
/// # Arguments
/// * `ranks_prev` - Previous period's factor rankings (or raw values to be ranked)
/// * `ranks_curr` - Current period's factor rankings (or raw values to be ranked)
///
/// # Returns
/// Turnover ratio (0 = no change, higher = more turnover). NaN if empty.
pub fn turnover_ratio(ranks_prev: &[f64], ranks_curr: &[f64]) -> f64 {
    let n = ranks_prev.len();
    if n == 0 || n != ranks_curr.len() {
        return f64::NAN;
    }

    let sum_abs_diff: f64 = ranks_prev
        .iter()
        .zip(ranks_curr.iter())
        .map(|(&a, &b)| (a - b).abs())
        .sum();

    sum_abs_diff / n as f64
}

/// Weighted turnover ratio with per-asset weights.
///
/// `turnover = sum(weight_i * |rank_i(t) - rank_i(t-1)|)`
///
/// Weights are normalized to sum to 1.
pub fn turnover_ratio_weighted(ranks_prev: &[f64], ranks_curr: &[f64], weights: &[f64]) -> f64 {
    let n = ranks_prev.len();
    if n == 0 || n != ranks_curr.len() || n != weights.len() {
        return f64::NAN;
    }

    let w_sum: f64 = weights.iter().sum();
    if w_sum.abs() < EPSILON {
        return f64::NAN;
    }

    let weighted_diff: f64 = ranks_prev
        .iter()
        .zip(ranks_curr.iter())
        .zip(weights.iter())
        .map(|((&a, &b), &w)| w * (a - b).abs())
        .sum();

    weighted_diff / w_sum
}

/// Rolling turnover ratio over a sequence of cross-sectional snapshots.
///
/// Each element of `snapshots` is one time period's factor values (same length N).
/// Returns a Vec of length T where `result[0] = NaN` (no previous) and
/// `result[t] = turnover_ratio(snapshots[t-1], snapshots[t])` for t >= 1.
///
/// # Arguments
/// * `snapshots` - Slice of time-ordered cross-sectional factor value slices
///
/// # Returns
/// Vec of turnover ratios per period
pub fn rolling_turnover(snapshots: &[&[f64]]) -> Vec<f64> {
    let t = snapshots.len();
    if t == 0 {
        return Vec::new();
    }

    let mut output = vec![f64::NAN; t];

    for i in 1..t {
        output[i] = turnover_ratio(snapshots[i - 1], snapshots[i]);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_psi_identical_distributions() {
        let data: Vec<f64> = (0..500).map(|i| i as f64 * 0.01).collect();
        let psi_val = psi(&data, &data, 10, BinningMethod::EqualWidth);
        assert_relative_eq!(psi_val, 0.0, epsilon = 0.01);
    }

    #[test]
    fn test_psi_shifted_distribution() {
        let expected: Vec<f64> = (0..1000).map(|i| i as f64 / 1000.0).collect();
        let actual: Vec<f64> = (0..1000).map(|i| i as f64 / 1000.0 + 0.6).collect();
        let psi_val = psi(&expected, &actual, 10, BinningMethod::EqualWidth);
        assert!(psi_val > 0.2, "shifted distribution PSI = {psi_val}");
    }

    #[test]
    fn test_csi_basic() {
        let expected: Vec<f64> = (0..200).map(|i| (i as f64).sin()).collect();
        let actual: Vec<f64> = (0..200).map(|i| (i as f64).sin() + 0.3).collect();
        let csi_val = csi(&expected, &actual, 8);
        assert!(csi_val.is_finite());
        assert!(csi_val > 0.0);
        let psi_ef = psi(&expected, &actual, 8, BinningMethod::EqualFrequency);
        assert_relative_eq!(csi_val, psi_ef, epsilon = 1e-12);
    }

    #[test]
    fn test_rolling_psi() {
        let data: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let window = 10;
        let result = rolling_psi(&data, window, 5, BinningMethod::EqualWidth);
        assert_eq!(result.len(), data.len() - window + 1);
        assert_relative_eq!(result[0], 0.0, epsilon = 0.01);
        let drifted: Vec<f64> = (0..50)
            .map(|i| if i < 25 { i as f64 } else { i as f64 + 20.0 })
            .collect();
        let drift_result = rolling_psi(&drifted, window, 5, BinningMethod::EqualWidth);
        assert_eq!(drift_result.len(), drifted.len() - window + 1);
        assert!(drift_result.last().copied().unwrap_or(0.0) > drift_result[0]);
    }

    #[test]
    fn test_turnover_ratio_no_change() {
        // Same rankings → turnover = 0
        let ranks_t0: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ranks_t1: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let tr = turnover_ratio(&ranks_t0, &ranks_t1);
        assert_relative_eq!(tr, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_turnover_ratio_full_reversal() {
        // Complete reversal of rankings
        let ranks_t0: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ranks_t1: Vec<f64> = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let tr = turnover_ratio(&ranks_t0, &ranks_t1);
        // |1-5|+|2-4|+|3-3|+|4-2|+|5-1| = 4+2+0+2+4 = 12, /5 = 2.4
        assert_relative_eq!(tr, 2.4, epsilon = 1e-10);
    }

    #[test]
    fn test_turnover_ratio_empty() {
        let tr = turnover_ratio(&[], &[]);
        assert!(tr.is_nan());
    }

    #[test]
    fn test_rolling_turnover_basic() {
        // 5 assets over 4 time periods
        let data = [
            vec![1.0, 2.0, 3.0, 4.0, 5.0], // t=0
            vec![1.0, 2.0, 3.0, 4.0, 5.0], // t=1 (no change)
            vec![5.0, 4.0, 3.0, 2.0, 1.0], // t=2 (reversal)
            vec![5.0, 4.0, 3.0, 2.0, 1.0], // t=3 (no change from t=2)
        ];
        let refs: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();
        let result = rolling_turnover(&refs);
        // t=0: NaN (no prev)
        // t=1: 0.0 (no change)
        // t=2: 2.4 (full reversal)
        // t=3: 0.0 (no change)
        assert_eq!(result.len(), 4);
        assert!(result[0].is_nan());
        assert_relative_eq!(result[1], 0.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 2.4, epsilon = 1e-10);
        assert_relative_eq!(result[3], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rolling_turnover_single_period() {
        let data = [vec![1.0, 2.0, 3.0]];
        let refs: Vec<&[f64]> = data.iter().map(|v| v.as_slice()).collect();
        let result = rolling_turnover(&refs);
        assert_eq!(result.len(), 1);
        assert!(result[0].is_nan());
    }
}
