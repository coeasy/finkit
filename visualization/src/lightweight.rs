//! Lightweight Charts data adapter.
//!
//! The adapter is intentionally renderer-agnostic: the Rust side validates
//! and shapes chart data, while the small frontend helper owns the
//! Lightweight Charts instance. Existing SVG/Canvas/PNG renderers remain
//! available for native and headless output.

use crate::data::KlineData;
use crate::error::{Result, VisualizationError};
use serde::{Deserialize, Serialize};

/// Time value accepted by Lightweight Charts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum LightweightTime {
    /// Unix timestamp in seconds.
    Timestamp(i64),
    /// ISO-like business date string.
    Date(String),
}

/// One candlestick point in the Lightweight Charts data model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightweightCandle {
    /// Bar time.
    pub time: LightweightTime,
    /// Opening price.
    pub open: f64,
    /// Highest price.
    pub high: f64,
    /// Lowest price.
    pub low: f64,
    /// Closing price.
    pub close: f64,
}

/// One volume histogram point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightweightVolume {
    /// Bar time.
    pub time: LightweightTime,
    /// Volume value.
    pub value: f64,
    /// Optional up/down color selected from the OHLC direction.
    pub color: String,
}

/// One nullable line point. Non-finite indicator values become JSON `null`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightweightLinePoint {
    /// Point time.
    pub time: LightweightTime,
    /// Finite value, or `null` for a warm-up/missing point.
    pub value: Option<f64>,
}

/// Named indicator line consumed by the frontend adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightweightLine {
    /// Stable frontend series name.
    pub name: String,
    /// Ordered line points.
    pub data: Vec<LightweightLinePoint>,
}

/// Versioned payload shared by Lightweight Charts and other web frontends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightweightChartsPayload {
    /// Payload schema version, independent from the crate version.
    pub schema_version: u16,
    /// Source data revision.
    pub revision: u64,
    /// Price candles.
    pub candles: Vec<LightweightCandle>,
    /// Volume histogram points.
    pub volume: Vec<LightweightVolume>,
    /// Additional named indicator lines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<LightweightLine>,
}

impl LightweightChartsPayload {
    /// Build a validated payload from OHLCV data.
    pub fn from_kline(data: &KlineData) -> Result<Self> {
        if !data.validate_ohlcv() {
            return Err(VisualizationError::ConversionError {
                message: data.validation_errors().join("; "),
            });
        }
        let times = times_for(data)?;
        let candles = times
            .iter()
            .cloned()
            .zip(data.opens().iter().copied())
            .zip(data.highs().iter().copied())
            .zip(data.lows().iter().copied())
            .zip(data.closes().iter().copied())
            .map(|((((time, open), high), low), close)| LightweightCandle {
                time,
                open,
                high,
                low,
                close,
            })
            .collect();
        let volume = times
            .iter()
            .cloned()
            .zip(data.volumes().iter().copied())
            .zip(data.opens().iter().copied())
            .zip(data.closes().iter().copied())
            .map(|(((time, value), open), close)| LightweightVolume {
                time,
                value,
                color: if close >= open {
                    "#26a69a".to_string()
                } else {
                    "#ef5350".to_string()
                },
            })
            .collect();
        Ok(Self {
            schema_version: 1,
            revision: data.revision(),
            candles,
            volume,
            lines: Vec::new(),
        })
    }

    /// Add one aligned nullable indicator line.
    pub fn add_line(&mut self, name: impl Into<String>, values: &[f64]) -> Result<()> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(VisualizationError::ConversionError {
                message: "Lightweight Charts line name must not be empty".to_string(),
            });
        }
        if self.lines.iter().any(|line| line.name == name) {
            return Err(VisualizationError::ConversionError {
                message: format!("duplicate Lightweight Charts line: {name}"),
            });
        }
        if values.len() != self.candles.len() {
            return Err(VisualizationError::ConversionError {
                message: format!(
                    "Lightweight Charts line {name} has {} values, expected {}",
                    values.len(),
                    self.candles.len()
                ),
            });
        }
        let data = self
            .candles
            .iter()
            .zip(values.iter().copied())
            .map(|(candle, value)| LightweightLinePoint {
                time: candle.time.clone(),
                value: value.is_finite().then_some(value),
            })
            .collect();
        self.lines.push(LightweightLine { name, data });
        Ok(())
    }

    /// Serialize the versioned payload for a web adapter.
    pub fn to_json_string(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|error| VisualizationError::SerializationError {
            message: error.to_string(),
        })
    }
}

fn times_for(data: &KlineData) -> Result<Vec<LightweightTime>> {
    if let Some(timestamps) = data.timestamps() {
        return Ok(timestamps
            .iter()
            .copied()
            .map(LightweightTime::Timestamp)
            .collect());
    }
    if data.dates().iter().any(|date| date.trim().is_empty()) {
        return Err(VisualizationError::ConversionError {
            message: "Lightweight Charts dates must not be empty".to_string(),
        });
    }
    Ok(data
        .dates()
        .iter()
        .cloned()
        .map(LightweightTime::Date)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> KlineData {
        let mut data = KlineData::new(
            vec!["2024-01-01".into(), "2024-01-02".into()],
            vec![10.0, 11.0],
            vec![12.0, 13.0],
            vec![9.0, 10.0],
            vec![11.0, 10.5],
            vec![100.0, 120.0],
        );
        assert!(data.set_timestamps(vec![1_704_067_200, 1_704_153_600]));
        data
    }

    #[test]
    fn payload_uses_numeric_timestamps_and_nullable_line_values() {
        let mut payload = LightweightChartsPayload::from_kline(&data()).unwrap();
        payload.add_line("EMA", &[f64::NAN, 10.75]).unwrap();
        let json = payload.to_json_string().unwrap();
        assert!(json.contains("\"time\":1704067200"));
        assert!(json.contains("\"value\":null"));
        assert!(json.contains("\"name\":\"EMA\""));
    }

    #[test]
    fn payload_falls_back_to_date_strings_without_timestamp_index() {
        let data = KlineData::new(
            vec!["2024-01-01".into()],
            vec![10.0],
            vec![11.0],
            vec![9.0],
            vec![10.5],
            vec![1.0],
        );
        let payload = LightweightChartsPayload::from_kline(&data).unwrap();
        assert_eq!(
            payload.candles[0].time,
            LightweightTime::Date("2024-01-01".to_string())
        );
    }

    #[test]
    fn payload_rejects_malformed_ohlcv() {
        let data = KlineData::new(
            vec!["2024-01-01".into()],
            vec![10.0],
            vec![9.0],
            vec![8.0],
            vec![10.5],
            vec![1.0],
        );
        assert!(matches!(
            LightweightChartsPayload::from_kline(&data),
            Err(VisualizationError::ConversionError { .. })
        ));
    }
}
