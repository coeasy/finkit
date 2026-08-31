//! Feature engineering pipeline example.
//!
//! Demonstrates building a complete ML feature matrix from raw price data.
//!
//! Run: cargo run --example feature_pipeline -p finkit

use finkit::features::*;

fn generate_sample_data(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut close = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut volume = Vec::with_capacity(n);

    for i in 0..n {
        let t = i as f64;
        let trend = 100.0 + t * 0.02;
        let noise = (t * 0.1).sin() * 3.0 + (t * 0.37).cos() * 2.0;
        let price = trend + noise;
        close.push(price);
        high.push(price + 1.0 + (t * 0.3).sin().abs());
        low.push(price - 1.0 - (t * 0.5).cos().abs());
        volume.push(10000.0 + (t * 0.7).sin() * 3000.0);
    }

    (close, high, low, volume)
}

fn main() {
    let (close, high, low, _volume) = generate_sample_data(500);

    println!("=== FTA Feature Engineering Pipeline ===\n");

    // 1. Multi-period indicator features
    println!("1. Generating multi-period indicators...");
    let mut engine = FeatureSet::new();
    engine.add_indicator("sma", &[5, 10, 20, 50]);
    engine.add_indicator("ema", &[5, 12, 26]);
    engine.add_indicator("rsi", &[7, 14, 21]);
    engine.add_indicator("roc", &[5, 10, 20]);
    let mut matrix = engine.generate(&close);
    println!("   Generated {} indicator features", matrix.cols());

    // 2. Rolling statistics
    println!("2. Computing rolling statistics...");
    let skew = rolling_skewness(&close, 20);
    let kurt = rolling_kurtosis(&close, 20);
    let ent = rolling_entropy(&close, 20, 10);
    matrix.add_column(Feature::new("skewness_20", "statistics", 20), skew.to_vec());
    matrix.add_column(Feature::new("kurtosis_20", "statistics", 20), kurt.to_vec());
    matrix.add_column(Feature::new("entropy_20", "statistics", 20), ent.to_vec());
    println!("   Added 3 statistical features");

    // 3. Normalization
    println!("3. Applying normalizations...");
    let zscore = rolling_zscore_normalize(&close, 50);
    let minmax = rolling_minmax(&close, 50);
    matrix.add_column(
        Feature::new("price_zscore_50", "normalization", 50),
        zscore.to_vec(),
    );
    matrix.add_column(
        Feature::new("price_minmax_50", "normalization", 50),
        minmax.to_vec(),
    );
    println!("   Added 2 normalized features");

    // 4. Time series features
    println!("4. Creating time series features...");
    let ret1 = pct_change(&close, 1);
    let ret5 = pct_change(&close, 5);
    let d1 = diff(&close, 1);
    let lag5 = lag(&close, 5);
    matrix.add_column(Feature::new("return_1d", "timeseries", 1), ret1.to_vec());
    matrix.add_column(Feature::new("return_5d", "timeseries", 5), ret5.to_vec());
    matrix.add_column(Feature::new("diff_1", "timeseries", 1), d1.to_vec());
    matrix.add_column(Feature::new("lag_5", "timeseries", 5), lag5.to_vec());
    println!("   Added 4 time series features");

    // 5. Signal detection
    println!("5. Detecting signals...");
    let sma5: Vec<f64> = finkit::indicators::sma(&close, 5).unwrap().to_vec();
    let sma20: Vec<f64> = finkit::indicators::sma(&close, 20).unwrap().to_vec();
    let crossovers = crossover(&sma5, &sma20);
    let crossunders = crossunder(&sma5, &sma20);
    println!(
        "   Found {} golden crosses, {} death crosses",
        crossovers.len(),
        crossunders.len()
    );

    // 6. Labels for ML
    println!("6. Generating ML labels...");
    let fwd_ret = forward_return(&close, 5);
    let binary = binary_label(&close, 5, 0.01);
    let barriers = triple_barrier(&close, &high, &low, 2.0, 2.0, 20);
    matrix.add_column(
        Feature::new("target_fwd_return_5", "label", 5),
        fwd_ret.to_vec(),
    );
    matrix.add_column(
        Feature::new("target_binary_1pct", "label", 5),
        binary.to_vec(),
    );
    let barrier_labels: Vec<f64> = barriers.iter().map(|b| b.label as f64).collect();
    matrix.add_column(
        Feature::new("target_triple_barrier", "label", 20),
        barrier_labels,
    );
    println!("   Added 3 label columns");

    // 7. Feature combinations
    println!("7. Computing feature combinations...");
    let ratio = feature_ratio(&sma5, &sma20);
    let spread = feature_spread(&sma5, &sma20);
    matrix.add_column(
        Feature::new("sma5_20_ratio", "combination", 0),
        ratio.to_vec(),
    );
    matrix.add_column(
        Feature::new("sma5_20_spread", "combination", 0),
        spread.to_vec(),
    );
    println!("   Added 2 combination features");

    // Summary
    println!("\n=== Feature Matrix Summary ===");
    println!("   Rows: {}", matrix.rows());
    println!("   Columns: {}", matrix.cols());
    println!("   Column names (first 10):");
    for name in matrix.column_names().iter().take(10) {
        println!("     - {}", name);
    }
    if matrix.cols() > 10 {
        println!("     ... and {} more", matrix.cols() - 10);
    }

    // 8. Export
    println!("\n8. Exporting...");
    to_csv(&matrix, "target/features_output.csv").unwrap();
    println!("   CSV exported to target/features_output.csv");

    // 9. SIMD batch processing demo
    println!("\n9. SIMD batch operations...");
    let batch_z = batch_zscore_simd(&close);
    let batch_mm = batch_minmax_simd(&close);
    println!("   batch_zscore: mean={:.6}", batch_z.mean().unwrap());
    println!(
        "   batch_minmax: min={:.4}, max={:.4}",
        batch_mm[0],
        batch_mm[batch_mm.len() - 1]
    );

    println!("\n=== Pipeline Complete ===");
}
