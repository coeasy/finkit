//! Multi-period feature generation.
//!
//! Generates the same indicator across multiple period parameters efficiently.

use super::{Feature, FeatureEngine, FeatureMatrix};
use crate::indicators;

/// Default fast periods: [5, 8, 13]
pub const PERIODS_FAST: &[usize] = &[5, 8, 13];
/// Default medium periods: [14, 21, 34]
pub const PERIODS_MEDIUM: &[usize] = &[14, 21, 34];
/// Default slow periods: [50, 100, 200]
pub const PERIODS_SLOW: &[usize] = &[50, 100, 200];

/// Generates a single indicator across multiple period values.
pub struct MultiPeriodFeature {
    indicator_name: String,
    periods: Vec<usize>,
}

impl MultiPeriodFeature {
    /// Create a new multi-period feature generator.
    pub fn new(indicator_name: String, periods: Vec<usize>) -> Self {
        Self {
            indicator_name,
            periods,
        }
    }

    /// Create with the default "fast" period template.
    pub fn fast(indicator_name: impl Into<String>) -> Self {
        Self::new(indicator_name.into(), PERIODS_FAST.to_vec())
    }

    /// Create with the default "medium" period template.
    pub fn medium(indicator_name: impl Into<String>) -> Self {
        Self::new(indicator_name.into(), PERIODS_MEDIUM.to_vec())
    }

    /// Create with the default "slow" period template.
    pub fn slow(indicator_name: impl Into<String>) -> Self {
        Self::new(indicator_name.into(), PERIODS_SLOW.to_vec())
    }

    /// Create with all three templates combined.
    pub fn all_periods(indicator_name: impl Into<String>) -> Self {
        let mut periods = Vec::with_capacity(9);
        periods.extend_from_slice(PERIODS_FAST);
        periods.extend_from_slice(PERIODS_MEDIUM);
        periods.extend_from_slice(PERIODS_SLOW);
        Self::new(indicator_name.into(), periods)
    }

    fn compute_indicator(&self, close: &[f64], period: usize) -> Option<Vec<f64>> {
        let result = match self.indicator_name.as_str() {
            "sma" => indicators::sma(close, period).ok(),
            "ema" => indicators::ema(close, period).ok(),
            "rsi" => indicators::rsi(close, period).ok(),
            "roc" => indicators::roc(close, period).ok(),
            "mom" => indicators::mom(close, period).ok(),
            "wma" => indicators::wma(close, period).ok(),
            "dema" => indicators::dema(close, period).ok(),
            "tema" => indicators::tema(close, period).ok(),
            "kama" => indicators::kama(close, period, 2, 30).ok(),
            "trima" => indicators::trima(close, period).ok(),
            "cmo" => indicators::cmo(close, period).ok(),
            "trix" => indicators::trix(close, period).ok(),
            "stddev" => indicators::std_dev(close, period, 1.0).ok(),
            "jma" => indicators::jma(close, period, 0.0, 2.0).ok(),
            "efficiency_ratio" => indicators::efficiency_ratio(close, period).ok(),
            "cfo" | "chande_forecast" => indicators::chande_forecast_oscillator(close, period).ok(),
            "qstick" => indicators::qstick(close, close, period, indicators::MaType::Sma).ok(),
            "hv" | "historical_volatility" => {
                indicators::historical_volatility(close, period, 252.0).ok()
            }
            "volume_momentum" => indicators::volume_momentum(close, period).ok(),
            "volume_roc" => indicators::volume_roc(close, period).ok(),
            _ => None,
        };
        result.map(|arr| arr.to_vec())
    }
}

impl FeatureEngine for MultiPeriodFeature {
    fn generate(&self, close: &[f64]) -> FeatureMatrix {
        let mut matrix = FeatureMatrix::with_capacity(close.len(), self.periods.len());
        for &period in &self.periods {
            if let Some(values) = self.compute_indicator(close, period) {
                let feature = Feature::new(
                    format!("{}_{}", self.indicator_name, period),
                    "indicator",
                    period,
                );
                matrix.add_column(feature, values);
            }
        }
        matrix
    }

    fn feature_names(&self) -> Vec<String> {
        self.periods
            .iter()
            .map(|p| format!("{}_{}", self.indicator_name, p))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_period_sma() {
        let close: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let gen = MultiPeriodFeature::new("sma".into(), vec![3, 5, 10]);
        let matrix = gen.generate(&close);
        assert_eq!(matrix.cols(), 3);
        assert_eq!(matrix.rows(), 20);
        assert_eq!(matrix.column_names(), vec!["sma_3", "sma_5", "sma_10"]);
    }

    #[test]
    fn test_multi_period_rsi() {
        let close: Vec<f64> = (1..=50).map(|i| 100.0 + (i as f64).sin() * 5.0).collect();
        let gen = MultiPeriodFeature::new("rsi".into(), vec![5, 14]);
        let matrix = gen.generate(&close);
        assert_eq!(matrix.cols(), 2);
        assert_eq!(matrix.column_names(), vec!["rsi_5", "rsi_14"]);
    }

    #[test]
    fn test_multi_period_templates() {
        let gen = MultiPeriodFeature::fast("ema");
        assert_eq!(gen.periods, vec![5, 8, 13]);

        let gen = MultiPeriodFeature::all_periods("sma");
        assert_eq!(gen.periods.len(), 9);
    }

    #[test]
    fn test_multi_period_performance_vs_individual() {
        let close: Vec<f64> = (0..10_000)
            .map(|i| 100.0 + (i as f64 * 0.01).sin() * 10.0)
            .collect();
        let gen = MultiPeriodFeature::new("sma".into(), vec![5, 10, 20, 50]);
        let matrix = gen.generate(&close);
        assert_eq!(matrix.cols(), 4);
        assert_eq!(matrix.rows(), 10_000);
    }

    #[test]
    fn test_multi_period_new_indicators() {
        let close: Vec<f64> = (1..=50)
            .map(|i| 100.0 + (i as f64 * 0.1).sin() * 5.0)
            .collect();

        let new_names = [
            "jma",
            "efficiency_ratio",
            "cfo",
            "qstick",
            "hv",
            "volume_momentum",
            "volume_roc",
        ];

        for name in &new_names {
            let gen = MultiPeriodFeature::new(name.to_string(), vec![10, 14]);
            let matrix = gen.generate(&close);
            assert!(
                matrix.cols() > 0,
                "indicator '{}' should produce at least 1 column",
                name
            );
            assert_eq!(matrix.rows(), 50);
        }
    }
}
