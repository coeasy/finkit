//! Feature engine trait and FeatureSet container.

use super::FeatureMatrix;

/// Trait for feature generators that produce one or more columns from input data.
pub trait FeatureEngine: Send + Sync {
    /// Generate features from close price data.
    fn generate(&self, close: &[f64]) -> FeatureMatrix;

    /// Generate features from full OHLCV data.
    fn generate_ohlcv(
        &self,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
    ) -> FeatureMatrix {
        let _ = (open, high, low, volume);
        self.generate(close)
    }

    /// Get names of features this engine will produce.
    fn feature_names(&self) -> Vec<String>;
}

/// Composable collection of feature generators.
///
/// Chains multiple feature engines and merges their outputs into a single matrix.
pub struct FeatureSet {
    engines: Vec<Box<dyn FeatureEngine>>,
}

impl FeatureSet {
    /// Create a new empty feature set.
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
        }
    }

    /// Add a feature engine.
    pub fn add(&mut self, engine: Box<dyn FeatureEngine>) -> &mut Self {
        self.engines.push(engine);
        self
    }

    /// Add a multi-period indicator feature by name and period list.
    pub fn add_indicator(&mut self, name: &str, periods: &[usize]) -> &mut Self {
        self.engines.push(Box::new(super::MultiPeriodFeature::new(
            name.to_string(),
            periods.to_vec(),
        )));
        self
    }

    /// Add pairwise cross-product features from named column snapshots.
    pub fn add_cross(&mut self, columns: &[(&str, &[f64])]) -> &mut Self {
        self.engines.push(super::cross_feature_engine(columns));
        self
    }

    /// Add cyclical time and trading-session features.
    pub fn add_time_features(
        &mut self,
        timestamps: Vec<i64>,
        hours: Vec<u8>,
        period: f64,
    ) -> &mut Self {
        self.engines.push(Box::new(super::TimeFeatureEngine::new(
            timestamps, hours, period,
        )));
        self
    }

    /// Generate all features from close prices.
    ///
    /// When the `rayon` feature is enabled, engines run in parallel; otherwise
    /// execution is sequential.
    pub fn generate(&self, close: &[f64]) -> FeatureMatrix {
        #[cfg(feature = "rayon")]
        {
            super::generate_parallel(&self.engines, close)
        }
        #[cfg(not(feature = "rayon"))]
        {
            let mut result = FeatureMatrix::new();
            for engine in &self.engines {
                let m = engine.generate(close);
                result.merge(m);
            }
            result
        }
    }

    /// Generate all features from full OHLCV data.
    ///
    /// When the `rayon` feature is enabled, engines run in parallel; otherwise
    /// execution is sequential.
    pub fn generate_ohlcv(
        &self,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
    ) -> FeatureMatrix {
        #[cfg(feature = "rayon")]
        {
            use rayon::prelude::*;
            let matrices: Vec<FeatureMatrix> = self
                .engines
                .par_iter()
                .map(|engine| engine.generate_ohlcv(open, high, low, close, volume))
                .collect();
            let mut result = FeatureMatrix::new();
            for matrix in matrices {
                result.merge(matrix);
            }
            result
        }
        #[cfg(not(feature = "rayon"))]
        {
            let mut result = FeatureMatrix::new();
            for engine in &self.engines {
                let m = engine.generate_ohlcv(open, high, low, close, volume);
                result.merge(m);
            }
            result
        }
    }

    /// Get all feature names that will be produced.
    pub fn feature_names(&self) -> Vec<String> {
        self.engines
            .iter()
            .flat_map(|e| e.feature_names())
            .collect()
    }
}

impl Default for FeatureSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_set_empty() {
        let set = FeatureSet::new();
        let close = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let m = set.generate(&close);
        assert_eq!(m.cols(), 0);
    }

    #[test]
    fn test_feature_set_with_indicator() {
        let close = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let mut set = FeatureSet::new();
        set.add_indicator("sma", &[3, 5]);
        let m = set.generate(&close);
        assert_eq!(m.cols(), 2);
        assert_eq!(m.rows(), 10);
    }

    #[test]
    fn test_feature_set_add_cross() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let mut set = FeatureSet::new();
        set.add_cross(&[("a", a.as_slice()), ("b", b.as_slice())]);
        let close = vec![0.0; 3];
        let m = set.generate(&close);
        assert_eq!(m.cols(), 1);
        assert_eq!(m.column(0)[0], 4.0);
    }

    #[test]
    fn test_feature_set_add_time_features() {
        let close = vec![100.0, 101.0];
        let mut set = FeatureSet::new();
        set.add_time_features(vec![0_i64, 86_400], vec![0_u8, 16], 86_400.0);
        let m = set.generate(&close);
        assert_eq!(m.cols(), 3);
        assert_eq!(m.rows(), 2);
        assert!((m.column_by_name("time_sin_daily").unwrap()[0] - 0.0).abs() < 1e-10);
        assert!((m.column_by_name("trading_session").unwrap()[1] - 2.0).abs() < 1e-10);
    }
}
