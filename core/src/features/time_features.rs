//! Cyclical time encoding and trading-session classification features.

use std::f64::consts::PI;

use super::{Feature, FeatureEngine, FeatureMatrix};

/// Encode Unix timestamps as sin/cos pairs for a cyclical period (e.g. 86400 for daily).
///
/// Phase is `2π * (timestamp mod period) / period`, so epoch midnight yields sin(0)=0, cos(1)=1
/// when `period` is 86400.
pub fn cyclical_time_encoding(timestamps: &[i64], period: f64) -> (Vec<f64>, Vec<f64>) {
    let mut sin_encoded = Vec::with_capacity(timestamps.len());
    let mut cos_encoded = Vec::with_capacity(timestamps.len());
    for &ts in timestamps {
        let phase = 2.0 * PI * (ts as f64).rem_euclid(period) / period;
        sin_encoded.push(phase.sin());
        cos_encoded.push(phase.cos());
    }
    (sin_encoded, cos_encoded)
}

/// Classify UTC hours into trading sessions: 0=Asia (0-7), 1=Europe (8-15), 2=Americas (16-23).
pub fn trading_session_features(hour_of_day: &[u8]) -> Vec<u8> {
    hour_of_day
        .iter()
        .map(|&h| match h {
            0..=7 => 0,
            8..=15 => 1,
            _ => 2,
        })
        .collect()
}

/// Feature engine for cyclical time and trading-session columns.
pub struct TimeFeatureEngine {
    timestamps: Vec<i64>,
    hours: Vec<u8>,
    period: f64,
}

impl TimeFeatureEngine {
    /// Create a time-feature engine with aligned timestamp and hour series.
    pub fn new(timestamps: Vec<i64>, hours: Vec<u8>, period: f64) -> Self {
        Self {
            timestamps,
            hours,
            period,
        }
    }

    fn period_label(&self) -> String {
        if (self.period - 86_400.0).abs() < 1.0 {
            "daily".to_string()
        } else if (self.period - 604_800.0).abs() < 1.0 {
            "weekly".to_string()
        } else {
            format!("p{}", self.period as u64)
        }
    }
}

impl FeatureEngine for TimeFeatureEngine {
    fn generate(&self, close: &[f64]) -> FeatureMatrix {
        let len = close.len();
        let ts: Vec<i64> = self.timestamps.iter().copied().take(len).collect();
        let hrs: Vec<u8> = self.hours.iter().copied().take(len).collect();

        let (sin_vals, cos_vals) = cyclical_time_encoding(&ts, self.period);
        let sessions = trading_session_features(&hrs);
        let session_f64: Vec<f64> = sessions.into_iter().map(f64::from).collect();

        let label = self.period_label();
        let period_usize = self.period as usize;

        let mut matrix = FeatureMatrix::with_capacity(len, 3);
        matrix.add_column(
            Feature::new(format!("time_sin_{label}"), "time", period_usize),
            sin_vals,
        );
        matrix.add_column(
            Feature::new(format!("time_cos_{label}"), "time", period_usize),
            cos_vals,
        );
        matrix.add_column(
            Feature::new("trading_session".to_string(), "time", 0),
            session_f64,
        );
        matrix
    }

    fn feature_names(&self) -> Vec<String> {
        let label = self.period_label();
        vec![
            format!("time_sin_{label}"),
            format!("time_cos_{label}"),
            "trading_session".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY_SECS: f64 = 86_400.0;

    #[test]
    fn cyclical_midnight_daily() {
        let timestamps = [0_i64];
        let (sin_vals, cos_vals) = cyclical_time_encoding(&timestamps, DAY_SECS);
        assert!((sin_vals[0] - 0.0).abs() < 1e-10);
        assert!((cos_vals[0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cyclical_quarter_day() {
        let timestamps = [21_600_i64];
        let (sin_vals, cos_vals) = cyclical_time_encoding(&timestamps, DAY_SECS);
        assert!((sin_vals[0] - 1.0).abs() < 1e-10);
        assert!(cos_vals[0].abs() < 1e-10);
    }

    #[test]
    fn trading_session_asia_europe_americas() {
        assert_eq!(trading_session_features(&[0, 7]), vec![0, 0]);
        assert_eq!(trading_session_features(&[8, 15]), vec![1, 1]);
        assert_eq!(trading_session_features(&[16, 23]), vec![2, 2]);
    }

    #[test]
    fn time_feature_engine_columns() {
        let close = vec![100.0, 101.0, 102.0];
        let timestamps = vec![0_i64, 21_600, 43_200];
        let hours = vec![0_u8, 10, 20];
        let engine = TimeFeatureEngine::new(timestamps, hours, DAY_SECS);
        let matrix = engine.generate(&close);

        assert_eq!(matrix.cols(), 3);
        assert_eq!(matrix.rows(), 3);
        assert_eq!(
            engine.feature_names(),
            vec![
                "time_sin_daily".to_string(),
                "time_cos_daily".to_string(),
                "trading_session".to_string(),
            ]
        );

        assert!((matrix.get(0, 0) - 0.0).abs() < 1e-10);
        assert!((matrix.get(0, 1) - 1.0).abs() < 1e-10);
        assert!((matrix.get(0, 2) - 0.0).abs() < 1e-10);

        assert!((matrix.get(1, 0) - 1.0).abs() < 1e-10);
        assert!((matrix.get(1, 2) - 1.0).abs() < 1e-10);

        assert!((matrix.get(2, 2) - 2.0).abs() < 1e-10);
    }
}
