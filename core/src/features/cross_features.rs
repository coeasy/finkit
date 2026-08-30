//! Feature crossing and deviation features for ML pipelines.
//!
//! Provides element-wise cross products, automatic pairwise crossing,
//! price deviation from moving average, and exponential time-decay weighting.

use ndarray::Array1;

use crate::indicators::{atr, sma};

use super::{feature_ratio, Feature, FeatureEngine, FeatureMatrix};

/// Compute element-wise cross product: `a[i] * b[i]`.
pub fn feature_cross(a: &[f64], b: &[f64]) -> Array1<f64> {
    assert_eq!(a.len(), b.len());
    Array1::from_iter(a.iter().zip(b.iter()).map(|(&x, &y)| x * y))
}

/// Automatically generate all pairwise cross-product features.
///
/// For `n` input columns, produces `n * (n - 1) / 2` cross columns named
/// `cross_{name_a}_{name_b}`.
pub fn auto_cross(columns: &[(&str, &[f64])]) -> FeatureMatrix {
    let n = columns.len();
    let mut matrix = FeatureMatrix::new();

    for i in 0..n {
        for j in (i + 1)..n {
            let cross = feature_cross(columns[i].1, columns[j].1);
            matrix.add_column(
                Feature::new(
                    format!("cross_{}_{}", columns[i].0, columns[j].0),
                    "cross",
                    0,
                ),
                cross.to_vec(),
            );
        }
    }
    matrix
}

/// Expand features into polynomial terms up to the given degree.
///
/// For `degree == 1`, returns the original columns. For `degree == 2`, adds
/// squared terms and pairwise products. Higher degrees include all monomials
/// with total exponent sum from 1 through `degree`.
pub fn polynomial_expand(columns: &[(&str, &[f64])], degree: usize) -> FeatureMatrix {
    let mut matrix = FeatureMatrix::new();
    if columns.is_empty() || degree == 0 {
        return matrix;
    }

    let n = columns.len();
    for d in 1..=degree {
        let mut indices = Vec::with_capacity(d);
        gen_monomial_indices(n, d, 0, &mut indices, &mut |idxs| {
            let values = monomial_values(columns, idxs);
            let name = monomial_name(columns, idxs);
            matrix.add_column(Feature::new(name, "polynomial", d), values);
        });
    }
    matrix
}

/// Generate ratio interaction features for all ordered pairs: `a / b` and `b / a`.
///
/// Column names follow `{name_a}_x_{name_b}_ratio` (e.g. `rsi_14_x_ema_20_ratio`).
pub fn interaction_features(columns: &[(&str, &[f64])]) -> FeatureMatrix {
    let n = columns.len();
    let mut matrix = FeatureMatrix::new();

    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let ratio = feature_ratio(columns[i].1, columns[j].1);
            matrix.add_column(
                Feature::new(
                    format!("{}_x_{}_ratio", columns[i].0, columns[j].0),
                    "interaction",
                    0,
                ),
                ratio.to_vec(),
            );
        }
    }
    matrix
}

fn gen_monomial_indices(
    n: usize,
    degree: usize,
    start: usize,
    current: &mut Vec<usize>,
    emit: &mut impl FnMut(&[usize]),
) {
    if current.len() == degree {
        emit(current);
        return;
    }
    for i in start..n {
        current.push(i);
        gen_monomial_indices(n, degree, i, current, emit);
        current.pop();
    }
}

fn monomial_values(columns: &[(&str, &[f64])], indices: &[usize]) -> Vec<f64> {
    let len = columns[0].1.len();
    let mut out = vec![1.0; len];
    for &idx in indices {
        let col = columns[idx].1;
        for (value, &v) in out.iter_mut().zip(col.iter()) {
            *value *= v;
        }
    }
    out
}

fn monomial_name(columns: &[(&str, &[f64])], indices: &[usize]) -> String {
    if indices.len() == 1 {
        return columns[indices[0]].0.to_string();
    }

    let n = columns.len();
    let mut counts = vec![0usize; n];
    for &idx in indices {
        counts[idx] += 1;
    }

    let mut parts = Vec::new();
    for (i, &count) in counts.iter().enumerate() {
        if count == 0 {
            continue;
        }
        if count == 1 {
            parts.push(columns[i].0.to_string());
        } else {
            parts.push(format!("{}_sq{}", columns[i].0, count));
        }
    }
    parts.join("_x_")
}

/// Price deviation from moving average, optionally normalized by ATR.
pub struct DeviationFeature {
    /// SMA lookback period.
    pub period: usize,
    /// ATR period used for normalized deviation (OHLCV path only).
    pub atr_period: usize,
}

impl DeviationFeature {
    /// Create a deviation feature generator.
    pub fn new(period: usize, atr_period: usize) -> Self {
        Self { period, atr_period }
    }

    fn raw_deviation(&self, close: &[f64]) -> Array1<f64> {
        let ma = sma(close, self.period).unwrap_or_else(|_| Array1::from_elem(close.len(), f64::NAN));
        Array1::from_iter(close.iter().zip(ma.iter()).map(|(&p, &m)| p - m))
    }
}

impl FeatureEngine for DeviationFeature {
    fn generate(&self, close: &[f64]) -> FeatureMatrix {
        let dev = self.raw_deviation(close);
        let mut matrix = FeatureMatrix::with_capacity(close.len(), 1);
        matrix.add_column(
            Feature::new(format!("dev_sma_{}", self.period), "deviation", self.period),
            dev.to_vec(),
        );
        matrix
    }

    fn generate_ohlcv(
        &self,
        _open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        _volume: &[f64],
    ) -> FeatureMatrix {
        let dev = self.raw_deviation(close);
        let atr_vals =
            atr(high, low, close, self.atr_period).unwrap_or_else(|_| Array1::from_elem(close.len(), f64::NAN));

        let norm_dev = Array1::from_iter(dev.iter().zip(atr_vals.iter()).map(|(&d, &a)| {
            if a.abs() > 1e-15 {
                d / a
            } else {
                f64::NAN
            }
        }));

        let mut matrix = FeatureMatrix::with_capacity(close.len(), 2);
        matrix.add_column(
            Feature::new(format!("dev_sma_{}", self.period), "deviation", self.period),
            dev.to_vec(),
        );
        matrix.add_column(
            Feature::new(
                format!("dev_sma_{}_atr_{}", self.period, self.atr_period),
                "deviation",
                self.atr_period,
            ),
            norm_dev.to_vec(),
        );
        matrix
    }

    fn feature_names(&self) -> Vec<String> {
        vec![
            format!("dev_sma_{}", self.period),
            format!("dev_sma_{}_atr_{}", self.period, self.atr_period),
        ]
    }
}

/// Exponential time-decay weighted feature.
///
/// At each index `T`, computes a weighted average:
/// `sum(exp(-lambda * (T - t)) * data[t]) / sum(exp(-lambda * (T - t)))`.
pub struct TimeDecayFeature {
    /// Decay rate (higher = more weight on recent observations).
    pub lambda: f64,
}

impl TimeDecayFeature {
    /// Create a time-decay feature generator with the given decay rate.
    pub fn new(lambda: f64) -> Self {
        Self { lambda }
    }

    fn compute_decay(&self, data: &[f64]) -> Array1<f64> {
        let len = data.len();
        let mut out = Array1::from_elem(len, f64::NAN);
        if len == 0 {
            return out;
        }

        out[0] = data[0];
        for t in 1..len {
            let mut weighted_sum = 0.0;
            let mut weight_sum = 0.0;
            for (s, &val) in data.iter().enumerate().take(t + 1) {
                let w = (-self.lambda * (t - s) as f64).exp();
                weighted_sum += w * val;
                weight_sum += w;
            }
            out[t] = if weight_sum > 1e-15 {
                weighted_sum / weight_sum
            } else {
                f64::NAN
            };
        }
        out
    }
}

impl FeatureEngine for TimeDecayFeature {
    fn generate(&self, close: &[f64]) -> FeatureMatrix {
        let values = self.compute_decay(close);
        let mut matrix = FeatureMatrix::with_capacity(close.len(), 1);
        matrix.add_column(
            Feature::new(format!("time_decay_{}", self.lambda), "decay", 0),
            values.to_vec(),
        );
        matrix
    }

    fn feature_names(&self) -> Vec<String> {
        vec![format!("time_decay_{}", self.lambda)]
    }
}

/// Feature engine wrapping precomputed columns for pairwise crossing.
struct CrossFeatureEngine {
    columns: Vec<(String, Vec<f64>)>,
}

impl FeatureEngine for CrossFeatureEngine {
    fn generate(&self, _close: &[f64]) -> FeatureMatrix {
        let cols: Vec<(&str, &[f64])> = self
            .columns
            .iter()
            .map(|(name, data)| (name.as_str(), data.as_slice()))
            .collect();
        auto_cross(&cols)
    }

    fn feature_names(&self) -> Vec<String> {
        let n = self.columns.len();
        let mut names = Vec::with_capacity(n * (n - 1) / 2);
        for i in 0..n {
            for j in (i + 1)..n {
                names.push(format!("cross_{}_{}", self.columns[i].0, self.columns[j].0));
            }
        }
        names
    }
}

/// Build a cross-feature engine from named column snapshots.
pub fn cross_feature_engine(columns: &[(&str, &[f64])]) -> Box<dyn FeatureEngine> {
    let owned: Vec<(String, Vec<f64>)> = columns
        .iter()
        .map(|(name, data)| (name.to_string(), data.to_vec()))
        .collect();
    Box::new(CrossFeatureEngine { columns: owned })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_cross_basic() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let result = feature_cross(&a, &b);
        assert_eq!(result[0], 4.0);
        assert_eq!(result[1], 10.0);
        assert_eq!(result[2], 18.0);
    }

    #[test]
    fn test_feature_cross_zeros() {
        let a = vec![0.0, 2.0, -3.0];
        let b = vec![5.0, 0.0, 4.0];
        let result = feature_cross(&a, &b);
        assert_eq!(result[0], 0.0);
        assert_eq!(result[1], 0.0);
        assert_eq!(result[2], -12.0);
    }

    #[test]
    fn test_auto_cross_pair_count() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let c = vec![7.0, 8.0, 9.0];
        let cols = vec![("a", a.as_slice()), ("b", b.as_slice()), ("c", c.as_slice())];
        let matrix = auto_cross(&cols);
        assert_eq!(matrix.cols(), 3);
        assert_eq!(matrix.column_names(), vec!["cross_a_b", "cross_a_c", "cross_b_c"]);
    }

    #[test]
    fn test_auto_cross_values() {
        let a = vec![2.0, 3.0];
        let b = vec![4.0, 5.0];
        let cols = vec![("x", a.as_slice()), ("y", b.as_slice())];
        let matrix = auto_cross(&cols);
        assert_eq!(matrix.cols(), 1);
        assert_eq!(matrix.column(0)[0], 8.0);
        assert_eq!(matrix.column(0)[1], 15.0);
    }

    #[test]
    fn test_deviation_feature_close_only() {
        let close: Vec<f64> = (1..=10).map(|i| i as f64).collect();
        let engine = DeviationFeature::new(3, 5);
        let matrix = engine.generate(&close);
        assert_eq!(matrix.cols(), 1);
        assert_eq!(matrix.column_names(), vec!["dev_sma_3"]);
        assert!((matrix.column(0)[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_deviation_feature_ohlcv_normalized() {
        let close: Vec<f64> = (1..=20).map(|i| 100.0 + i as f64).collect();
        let high: Vec<f64> = close.iter().map(|c| c + 1.0).collect();
        let low: Vec<f64> = close.iter().map(|c| c - 1.0).collect();
        let open = close.clone();
        let volume = vec![1000.0; 20];
        let engine = DeviationFeature::new(5, 5);
        let matrix = engine.generate_ohlcv(&open, &high, &low, &close, &volume);
        assert_eq!(matrix.cols(), 2);
        assert!(matrix.column(1)[19].is_finite());
    }

    #[test]
    fn test_time_decay_recent_weight() {
        let data = vec![1.0, 1.0, 1.0, 10.0];
        let engine = TimeDecayFeature::new(1.0);
        let matrix = engine.generate(&data);
        let values = matrix.column(0);
        assert_eq!(values[0], 1.0);
        assert!(values[3] > 1.0);
        assert!(values[3] < 10.0);
    }

    #[test]
    fn test_time_decay_constant_series() {
        let data = vec![5.0; 8];
        let engine = TimeDecayFeature::new(0.5);
        let matrix = engine.generate(&data);
        let values = matrix.column(0);
        for &v in values {
            assert!((v - 5.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_cross_feature_engine_names() {
        let a = vec![1.0, 2.0];
        let b = vec![3.0, 4.0];
        let engine = cross_feature_engine(&[("fast", a.as_slice()), ("slow", b.as_slice())]);
        assert_eq!(engine.feature_names(), vec!["cross_fast_slow"]);
    }

    #[test]
    fn test_polynomial_expand_degree_2() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let c = vec![7.0, 8.0, 9.0];
        let cols = vec![("a", a.as_slice()), ("b", b.as_slice()), ("c", c.as_slice())];
        let matrix = polynomial_expand(&cols, 2);
        // 3 originals + 3 squared + 3 pairwise products
        assert_eq!(matrix.cols(), 9);
    }

    #[test]
    fn test_interaction_features_names() {
        let rsi = vec![70.0, 80.0];
        let ema = vec![100.0, 110.0];
        let cols = vec![("rsi_14", rsi.as_slice()), ("ema_20", ema.as_slice())];
        let matrix = interaction_features(&cols);
        assert_eq!(matrix.cols(), 2);
        assert_eq!(
            matrix.column_names(),
            vec!["rsi_14_x_ema_20_ratio", "ema_20_x_rsi_14_ratio"]
        );
    }
}

/// Correlation method for cross-correlation matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CorrelationMethod {
    Pearson,
    Spearman,
}

/// Cross-correlation matrix result.
#[derive(Debug, Clone)]
pub struct CrossCorrelationResult {
    /// N×N correlation matrix (flattened row-major)
    pub matrix: Vec<f64>,
    /// Number of series (N)
    pub n: usize,
}

impl CrossCorrelationResult {
    /// Get correlation between series i and j.
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.matrix[i * self.n + j]
    }
}

/// Compute cross-correlation matrix for multiple time series.
///
/// Given N series of equal length, computes the N×N correlation matrix
/// using the specified method (Pearson or Spearman).
///
/// # Arguments
/// * `series` - Slice of time series (all must have same length)
/// * `method` - Correlation method (Pearson or Spearman)
///
/// # Returns
/// CrossCorrelationResult with N×N correlation matrix
pub fn cross_correlation_matrix(
    series: &[&[f64]],
    method: CorrelationMethod,
) -> Result<CrossCorrelationResult> {
    use crate::error::TaError;
    use crate::math::statistics::spearman_rank;

    let num_series = series.len();
    if num_series < 2 {
        return Err(TaError::InvalidParameter {
            name: "series".to_string(),
            constraint: "must have at least 2 series".to_string(),
        });
    }
    let len = series[0].len();
    if len < 2 {
        return Err(TaError::InvalidParameter {
            name: "series".to_string(),
            constraint: "series length must be >= 2".to_string(),
        });
    }
    for s in series.iter() {
        if s.len() != len {
            return Err(TaError::InvalidParameter {
                name: "series".to_string(),
                constraint: "all series must have the same length".to_string(),
            });
        }
    }

    let mut matrix = vec![0.0; num_series * num_series];

    for i in 0..num_series {
        matrix[i * num_series + i] = 1.0; // diagonal
        for j in (i + 1)..num_series {
            let corr = match method {
                CorrelationMethod::Pearson => pearson_corr(series[i], series[j]),
                CorrelationMethod::Spearman => spearman_rank(series[i], series[j]).unwrap_or(0.0),
            };
            matrix[i * num_series + j] = corr;
            matrix[j * num_series + i] = corr;
        }
    }

    Ok(CrossCorrelationResult {
        matrix,
        n: num_series,
    })
}

/// Rolling cross-correlation matrix.
///
/// For each window position, computes the N×N cross-correlation matrix.
///
/// # Arguments
/// * `series` - Slice of time series (all same length)
/// * `window` - Rolling window size
/// * `method` - Correlation method
///
/// # Returns
/// Vec of CrossCorrelationResult, one per valid window position (NaN before warm-up implied by length)
pub fn rolling_cross_correlation_matrix(
    series: &[&[f64]],
    window: usize,
    method: CorrelationMethod,
) -> Result<Vec<Option<CrossCorrelationResult>>> {
    use crate::error::TaError;

    let num_series = series.len();
    if num_series < 2 {
        return Err(TaError::InvalidParameter {
            name: "series".to_string(),
            constraint: "must have at least 2 series".to_string(),
        });
    }
    if window < 2 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= 2".to_string(),
        });
    }
    let len = series[0].len();
    if len < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "series length must be >= window".to_string(),
        });
    }

    let mut results = Vec::with_capacity(len);

    for i in 0..len {
        if i < window - 1 {
            results.push(None);
            continue;
        }
        let start = i + 1 - window;
        let slices: Vec<&[f64]> = series.iter().map(|s| &s[start..=i]).collect();
        match cross_correlation_matrix(&slices, method) {
            Ok(r) => results.push(Some(r)),
            Err(_) => results.push(None),
        }
    }

    Ok(results)
}

fn pearson_corr(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let mean_x = x.iter().sum::<f64>() / n;
    let mean_y = y.iter().sum::<f64>() / n;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for i in 0..x.len() {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    if denom < 1e-15 {
        return 0.0;
    }
    cov / denom
}

use crate::error::Result;

#[cfg(test)]
mod cross_corr_tests {
    use super::*;

    #[test]
    fn test_cross_correlation_pearson() {
        let a: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let b: Vec<f64> = (0..20).map(|i| i as f64 * 2.0 + 1.0).collect();
        let c: Vec<f64> = (0..20).map(|i| 100.0 - i as f64).collect();

        let result = cross_correlation_matrix(
            &[&a, &b, &c],
            CorrelationMethod::Pearson,
        )
        .unwrap();

        assert_eq!(result.n, 3);
        // a and b are perfectly correlated
        assert!((result.get(0, 1) - 1.0).abs() < 1e-10);
        // a and c are perfectly anti-correlated
        assert!((result.get(0, 2) + 1.0).abs() < 1e-10);
        // diagonal = 1
        assert!((result.get(0, 0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cross_correlation_spearman() {
        let a: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let b: Vec<f64> = (0..20).map(|i| (i as f64).powi(2)).collect();

        let result = cross_correlation_matrix(
            &[&a, &b],
            CorrelationMethod::Spearman,
        )
        .unwrap();

        assert_eq!(result.n, 2);
        // Monotone relationship => Spearman = 1
        assert!((result.get(0, 1) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cross_correlation_invalid() {
        let a = [1.0; 10];
        assert!(cross_correlation_matrix(&[&a[..]], CorrelationMethod::Pearson).is_err());
    }

    #[test]
    fn test_rolling_cross_correlation() {
        let a: Vec<f64> = (0..30).map(|i| i as f64).collect();
        let b: Vec<f64> = (0..30).map(|i| i as f64 * 2.0).collect();

        let results = rolling_cross_correlation_matrix(
            &[&a, &b],
            10,
            CorrelationMethod::Pearson,
        )
        .unwrap();

        assert_eq!(results.len(), 30);
        assert!(results[8].is_none());
        let r9 = results[9].as_ref().unwrap();
        assert!((r9.get(0, 1) - 1.0).abs() < 1e-10);
    }
}
