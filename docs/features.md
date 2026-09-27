# Feature Engineering Module

The `features` module provides a comprehensive toolkit for transforming raw financial data and technical indicators into feature matrices suitable for machine learning models and quantitative strategy development.

## Quick Start

```rust
use finkit::features::*;

// Generate multi-period SMA features
let close = vec![100.0, 101.0, 102.0, 101.5, 103.0, 104.0, 103.5, 105.0, 106.0, 107.0,
                 108.0, 109.0, 108.5, 110.0, 111.0, 112.0, 113.0, 114.0, 115.0, 116.0];

let mut engine = FeatureSet::new();
engine.add_indicator("sma", &[5, 10, 20]);
engine.add_indicator("rsi", &[7, 14]);
let matrix = engine.generate(&close);

println!("Generated {} features x {} rows", matrix.cols(), matrix.rows());
```

## Module Overview

| Sub-module | Purpose |
|-----------|---------|
| `matrix` | 2D feature storage with column-oriented access |
| `engine` | `FeatureEngine` trait and `FeatureSet` container |
| `multi_period` | Generate indicators across multiple lookback periods |
| `signals` | Crossover, crossunder, and divergence detection |
| `timeseries` | Lag, lead, diff, pct_change, rolling_apply |
| `rolling_stats` | Skewness, kurtosis, entropy, z-score, percentile |
| `normalization` | Z-score, min-max, robust scaler, rank normalization |
| `labels` | Forward return, triple barrier, binary labels |
| `combinations` | Feature ratio, spread, rolling correlation matrix |
| `selection` | Variance threshold, correlation filter, mutual information |
| `export` | CSV, JSON Lines, Arrow IPC export |
| `simd_opt` | SIMD-optimized batch operations |

## Feature Matrix

The `FeatureMatrix` is the central data structure:

```rust
use finkit::features::FeatureMatrix;

let mut matrix = FeatureMatrix::new();
matrix.add_column("sma_5", vec![f64::NAN; 4].into_iter().chain(vec![101.0, 102.0]).collect());
matrix.add_column("rsi_14", vec![50.0, 55.0, 60.0, 45.0, 70.0, 65.0]);

// Access by name
let sma_col = matrix.column_by_name("sma_5").unwrap();

// Select subset
let subset = matrix.select(&["sma_5"]);

// Merge two matrices
let mut other = FeatureMatrix::new();
other.add_column("volume_sma", vec![1000.0; 6]);
let merged = matrix.merge(&other);
```

## Multi-Period Feature Generation

Generate the same indicator with different lookback windows:

```rust
use finkit::features::{FeatureEngine, MultiPeriodFeature};

let close = vec![/* ... price data ... */];

// Predefined period templates. The methods are `fast` / `medium` / `slow` /
// `all_periods`; each accepts `impl Into<String>` for the indicator name.
let fast = MultiPeriodFeature::fast("ema");       // periods [5, 8, 13]
let medium = MultiPeriodFeature::medium("rsi");   // periods [14, 21, 34]
let slow = MultiPeriodFeature::slow("sma");       // periods [50, 100, 200]
let all = MultiPeriodFeature::all_periods("sma"); // the three sets concatenated

// Custom periods. `new` takes an owned `String`, so `.into()` is required for
// a string literal.
let custom = MultiPeriodFeature::new("sma".into(), vec![10, 20, 30]);

// `generate` comes from the `FeatureEngine` trait, which must be in scope.
let matrix = custom.generate(&close);
```

`generate` produces one column per period, named `<indicator>_<period>`.
Periods whose indicator call fails are skipped rather than emitted as an empty
column.

Supported indicator names (the exact match arms in
`core/src/features/multi_period.rs`; anything else is skipped):

`sma`, `ema`, `wma`, `dema`, `tema`, `trima`, `kama`, `rsi`, `roc`, `mom`,
`cmo`, `trix`, `cfo` (alias `chande_forecast`), `qstick`, `stddev`, `jma`,
`efficiency_ratio`, `hv` (alias `historical_volatility`), `volume_momentum`,
`volume_roc`.

## Signal Detection

```rust
use finkit::features::{crossover, crossunder, threshold_cross, divergence};

let fast_ma = vec![100.0, 101.0, 102.0, 101.5, 103.0];
let slow_ma = vec![100.5, 100.8, 101.5, 102.0, 102.5];

// Detect crossovers (fast crosses above slow)
let signals = crossover(&fast_ma, &slow_ma);
for signal in &signals {
    println!("Crossover at index {}", signal.index);
}

// Threshold crossings
let rsi = vec![30.0, 35.0, 45.0, 55.0, 70.0, 75.0, 65.0];
let crosses = threshold_cross(&rsi, 50.0);

// Divergence detection (price vs indicator). Signature is
// `divergence(price, indicator, lookback, min_distance)`.
let price = vec![100.0, 102.0, 101.0, 103.0, 102.0, 104.0];
let indicator = vec![50.0, 55.0, 52.0, 48.0, 45.0, 43.0];
let divergences = divergence(&price, &indicator, 3, 2);
```

## Time Series Transformations

```rust
use finkit::features::{lag, lead, diff, pct_change, multi_lag, rolling_apply};

let data = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0];

let lagged = lag(&data, 2);        // [NaN, NaN, 100, 101, 102, 103]
let led = lead(&data, 1);          // [101, 102, 103, 104, 105, NaN]
let changes = diff(&data, 1);      // [NaN, 1, 1, 1, 1, 1]
let returns = pct_change(&data, 1); // [NaN, 0.01, 0.0099, ...]

// Multiple lags at once
let multi = multi_lag(&data, &[1, 2, 5]);

// Custom rolling window function
let rolling_range = rolling_apply(&data, 3, |window| {
    window.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    - window.iter().cloned().fold(f64::INFINITY, f64::min)
});
```

## Rolling Statistics

```rust
use finkit::features::{
    rolling_skewness, rolling_kurtosis, rolling_entropy,
    rolling_zscore, rolling_percentile
};

let data = vec![/* ... 100+ data points ... */];

let skew = rolling_skewness(&data, 20);
let kurt = rolling_kurtosis(&data, 20);
let entropy = rolling_entropy(&data, 20, 10); // 10 bins
let zscore = rolling_zscore(&data, 20);
let pctl = rolling_percentile(&data, 20);
```

## Normalization

```rust
use finkit::features::{
    rolling_zscore_normalize, rolling_minmax, robust_scaler, rank_normalize
};

let data = vec![/* ... data ... */];

let zscored = rolling_zscore_normalize(&data, 50);
let minmaxed = rolling_minmax(&data, 50);
let robust = robust_scaler(&data, 50);     // Uses median and IQR
let ranked = rank_normalize(&data, 50);     // Rank within window / window_size
```

## ML Label Generation

```rust
use finkit::features::{forward_return, triple_barrier, binary_label, fixed_horizon_label};

let close = vec![/* ... price data ... */];
let high = vec![/* ... high prices ... */];
let low = vec![/* ... low prices ... */];

// Simple forward return (log)
let fwd_ret = forward_return(&close, 5);

// Triple barrier method (López de Prado)
let labels = triple_barrier(&close, &high, &low, 2.0, 2.0, 20);
// Returns BarrierLabel { label: 1/-1/0, duration, ret }

// Binary classification label
let binary = binary_label(&close, 5, 0.01); // 1 if return > 1%, else 0

// Fixed horizon discrete labels
let discrete = fixed_horizon_label(&close, 10, 0.005); // -1, 0, 1
```

## Feature Combinations

```rust
use finkit::features::{
    feature_ratio, feature_spread, rolling_correlation, rolling_correlation_matrix
};

let feat_a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let feat_b = vec![2.0, 4.0, 6.0, 8.0, 10.0];

let ratio = feature_ratio(&feat_a, &feat_b);   // [0.5, 0.5, 0.5, ...]
let spread = feature_spread(&feat_a, &feat_b); // [-1, -2, -3, ...]
let corr = rolling_correlation(&feat_a, &feat_b, 3);

// Correlation matrix for multiple features. Takes named columns, not a
// `Vec<Vec<f64>>`.
let corr_matrix = rolling_correlation_matrix(
    &[("a", feat_a.as_slice()), ("b", feat_b.as_slice())],
    3,
);
```

## Feature Selection

All three selectors take a `&FeatureMatrix`, not a `Vec<Vec<f64>>` — build the
matrix first (see [Export](#export) for the column API).

```rust
use finkit::features::{variance_threshold, correlation_filter, mutual_information};

let mut matrix = FeatureMatrix::new();
matrix.add_column(Feature::new("f_a", "custom", 0), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
matrix.add_column(Feature::new("f_const", "custom", 0), vec![1.0, 1.0, 1.0, 1.0, 1.0]);
matrix.add_column(Feature::new("f_b", "custom", 0), vec![2.0, 4.0, 6.0, 8.0, 10.0]);

// Remove near-zero variance features. Returns a new FeatureMatrix.
let kept = variance_threshold(&matrix, 0.01);

// Remove highly correlated features. Returns a new FeatureMatrix.
let uncorr = correlation_filter(&matrix, 0.95);

// Mutual information against a target. Returns a `FeatureRanking`, not a
// bare score vector.
let target = vec![0.0, 1.0, 1.0, 0.0, 1.0];
let ranking = mutual_information(&matrix, &target, 5);
for (name, score) in ranking.top_k(2) {
    println!("{name}: {score}");
}
```

## Export

The exporters **write to a path** and return `std::io::Result<()>` — they do not
return a `String` that you write yourself.

```rust
use finkit::features::{Feature, FeatureMatrix, to_csv, to_json_lines, to_arrow_ipc};

let mut matrix = FeatureMatrix::new();
// `Feature::new` takes (name, category, period); the values go in `add_column`.
matrix.add_column(Feature::new("sma_5", "overlap", 5), vec![1.0, 2.0, 3.0]);
matrix.add_column(Feature::new("rsi_14", "momentum", 14), vec![55.0, 60.0, 48.0]);

to_csv(&matrix, "features.csv")?;            // CSV
to_json_lines(&matrix, "features.jsonl")?;   // JSON Lines, one object per row
to_arrow_ipc(&matrix, "features.arrow")?;    // Arrow IPC
```

## SIMD-Optimized Operations

For batch processing large datasets:

```rust
use finkit::features::{batch_zscore_simd, batch_minmax_simd, correlation_simd};

let data: Vec<f64> = (0..100_000).map(|i| i as f64 * 0.01).collect();

// ~2-4x faster than naive implementation for large arrays
let zscored = batch_zscore_simd(&data);
let normalized = batch_minmax_simd(&data);

let other: Vec<f64> = (0..100_000).map(|i| (i as f64 * 0.02).sin()).collect();
let corr = correlation_simd(&data, &other);
```

## Complete Pipeline Example

```rust
use finkit::features::*;
use finkit::indicators;

fn build_ml_features(close: &[f64], high: &[f64], low: &[f64], volume: &[f64]) -> FeatureMatrix {
    let mut matrix = FeatureMatrix::new();

    // 1. Multi-period indicators
    let mut engine = FeatureSet::new();
    engine.add_indicator("sma", &[5, 10, 20, 50]);
    engine.add_indicator("rsi", &[7, 14, 21]);
    engine.add_indicator("roc", &[5, 10, 20]);
    let indicator_features = engine.generate(close);
    let matrix = matrix.merge(&indicator_features);

    // 2. Rolling statistics
    let skew = rolling_skewness(close, 20);
    let kurt = rolling_kurtosis(close, 20);
    let mut matrix = matrix;
    matrix.add_column("skewness_20", skew);
    matrix.add_column("kurtosis_20", kurt);

    // 3. Normalized features
    let zscore = rolling_zscore_normalize(close, 50);
    matrix.add_column("price_zscore_50", zscore);

    // 4. Time series features
    let ret_1 = pct_change(close, 1);
    let ret_5 = pct_change(close, 5);
    matrix.add_column("return_1", ret_1);
    matrix.add_column("return_5", ret_5);

    // 5. Labels
    let fwd = forward_return(close, 5);
    matrix.add_column("target_fwd_5", fwd);

    // 6. Remove low-variance features
    matrix.drop_all_nan_columns();

    matrix
}
```

## Performance

On 100K data points (measured with Criterion):
- Multi-period SMA (4 periods): ~50ms
- Rolling skewness (window=20): ~8ms
- Batch z-score SIMD: ~0.15ms
- Correlation SIMD: ~0.2ms
- Forward return: ~0.05ms
