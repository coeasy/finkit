//! Parallel feature matrix generation using rayon.

use super::{FeatureEngine, FeatureMatrix};

/// Generate features from multiple engines in parallel, merging all columns.
///
/// Each generator runs independently (read-only `close` slice, owned output matrix),
/// so there are no data races. Column merge happens sequentially after collection.
#[cfg(feature = "rayon")]
pub fn generate_parallel(generators: &[Box<dyn FeatureEngine>], close: &[f64]) -> FeatureMatrix {
    use rayon::prelude::*;

    let matrices: Vec<FeatureMatrix> = generators
        .par_iter()
        .map(|engine| engine.generate(close))
        .collect();

    let mut result = FeatureMatrix::new();
    for matrix in matrices {
        result.merge(matrix);
    }
    result
}

/// Serial fallback when the `rayon` feature is disabled.
#[cfg(not(feature = "rayon"))]
pub fn generate_parallel(generators: &[Box<dyn FeatureEngine>], close: &[f64]) -> FeatureMatrix {
    let mut result = FeatureMatrix::new();
    for engine in generators {
        result.merge(engine.generate(close));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::MultiPeriodFeature;

    fn generate_serial(generators: &[Box<dyn FeatureEngine>], close: &[f64]) -> FeatureMatrix {
        let mut result = FeatureMatrix::new();
        for engine in generators {
            result.merge(engine.generate(close));
        }
        result
    }

    fn matrices_equal(a: &FeatureMatrix, b: &FeatureMatrix) {
        assert_eq!(a.rows(), b.rows());
        assert_eq!(a.cols(), b.cols());
        assert_eq!(a.column_names(), b.column_names());
        for col in 0..a.cols() {
            for (av, bv) in a.column(col).iter().zip(b.column(col).iter()) {
                if av.is_nan() {
                    assert!(bv.is_nan());
                } else {
                    assert!((av - bv).abs() < 1e-10);
                }
            }
        }
    }

    #[test]
    fn test_generate_parallel_matches_serial() {
        let close = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ];
        let generators: Vec<Box<dyn FeatureEngine>> = vec![
            Box::new(MultiPeriodFeature::new("sma".to_string(), vec![3, 5])),
            Box::new(MultiPeriodFeature::new("ema".to_string(), vec![7])),
            Box::new(MultiPeriodFeature::new("rsi".to_string(), vec![6])),
        ];

        let serial = generate_serial(&generators, &close);
        let parallel = generate_parallel(&generators, &close);
        matrices_equal(&serial, &parallel);
    }

    #[test]
    fn test_generate_parallel_empty() {
        let generators: Vec<Box<dyn FeatureEngine>> = vec![];
        let close = vec![1.0, 2.0, 3.0];
        let matrix = generate_parallel(&generators, &close);
        assert_eq!(matrix.cols(), 0);
        assert_eq!(matrix.rows(), 0);
    }
}
