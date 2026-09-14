use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KlineData {
    pub dates: Vec<String>,
    /// Optional strictly increasing Unix-second timestamps. Empty means the
    /// compatibility date-only mode is active.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timestamps: Vec<i64>,
    pub opens: Vec<f64>,
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
    pub closes: Vec<f64>,
    pub volumes: Vec<f64>,
    /// Monotonically increasing source revision used by render caches.
    #[serde(default)]
    pub revision: u64,
}

/// One validated live OHLCV quote used by batch chart updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KlineBar {
    pub date: String,
    /// Optional Unix-second timestamp for calendar-aware live updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl KlineBar {
    pub fn new(
        date: impl Into<String>,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Self {
        Self {
            date: date.into(),
            timestamp: None,
            open,
            high,
            low,
            close,
            volume,
        }
    }

    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
}

/// Shared, revisioned OHLCV frame for repeated viewport renders.
///
/// `KlineData` remains the compatibility and FFI-friendly representation;
/// this frame avoids cloning numeric columns when several charts or views
/// consume the same source data.
#[derive(Debug, Clone, PartialEq)]
pub struct KlineFrame {
    pub timestamps: Arc<[i64]>,
    pub dates: Arc<[String]>,
    pub opens: Arc<[f64]>,
    pub highs: Arc<[f64]>,
    pub lows: Arc<[f64]>,
    pub closes: Arc<[f64]>,
    pub volumes: Arc<[f64]>,
    pub revision: u64,
}

impl KlineFrame {
    pub fn new(
        timestamps: Vec<i64>,
        dates: Vec<String>,
        opens: Vec<f64>,
        highs: Vec<f64>,
        lows: Vec<f64>,
        closes: Vec<f64>,
        volumes: Vec<f64>,
        revision: u64,
    ) -> Self {
        Self {
            timestamps: timestamps.into(),
            dates: dates.into(),
            opens: opens.into(),
            highs: highs.into(),
            lows: lows.into(),
            closes: closes.into(),
            volumes: volumes.into(),
            revision,
        }
    }

    pub fn from_kline_data(data: &KlineData, timestamps: Vec<i64>) -> Option<Self> {
        if timestamps.len() != data.len()
            || !timestamps.windows(2).all(|window| window[0] < window[1])
            || !data.validate()
        {
            return None;
        }
        Some(Self::new(
            timestamps,
            data.dates.clone(),
            data.opens.clone(),
            data.highs.clone(),
            data.lows.clone(),
            data.closes.clone(),
            data.volumes.clone(),
            data.revision,
        ))
    }

    pub fn len(&self) -> usize {
        self.dates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn validate_timestamps(&self) -> bool {
        self.timestamps.len() == self.len()
            && self
                .timestamps
                .windows(2)
                .all(|window| window[0] < window[1])
    }

    pub fn to_kline_data(&self) -> KlineData {
        KlineData {
            dates: self.dates.to_vec(),
            timestamps: self.timestamps.to_vec(),
            opens: self.opens.to_vec(),
            highs: self.highs.to_vec(),
            lows: self.lows.to_vec(),
            closes: self.closes.to_vec(),
            volumes: self.volumes.to_vec(),
            revision: self.revision,
        }
    }
}

/// OHLCV data reduced into time buckets while retaining source ranges.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregatedKline {
    pub data: KlineData,
    pub source_ranges: Vec<(usize, usize)>,
}

impl KlineData {
    pub fn new(
        dates: Vec<String>,
        opens: Vec<f64>,
        highs: Vec<f64>,
        lows: Vec<f64>,
        closes: Vec<f64>,
        volumes: Vec<f64>,
    ) -> Self {
        Self {
            dates,
            timestamps: Vec::new(),
            opens,
            highs,
            lows,
            closes,
            volumes,
            revision: 0,
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    pub fn opens(&self) -> &[f64] {
        &self.opens
    }

    pub fn highs(&self) -> &[f64] {
        &self.highs
    }

    pub fn lows(&self) -> &[f64] {
        &self.lows
    }

    pub fn closes(&self) -> &[f64] {
        &self.closes
    }

    pub fn volumes(&self) -> &[f64] {
        &self.volumes
    }

    pub fn dates(&self) -> &[String] {
        &self.dates
    }

    pub fn timestamps(&self) -> Option<&[i64]> {
        (self.timestamps.len() == self.len()).then_some(self.timestamps.as_slice())
    }

    pub fn validate_timestamps(&self) -> bool {
        self.timestamps.is_empty()
            || (self.timestamps.len() == self.len()
                && self.timestamps.windows(2).all(|pair| pair[0] < pair[1]))
    }

    /// Attach a strictly increasing Unix-second index to the compatibility
    /// K-line data. Passing an empty vector returns to date-only mode.
    pub fn set_timestamps(&mut self, timestamps: Vec<i64>) -> bool {
        if timestamps.is_empty()
            || (timestamps.len() == self.len()
                && timestamps.windows(2).all(|pair| pair[0] < pair[1]))
        {
            self.timestamps = timestamps;
            self.bump_revision();
            true
        } else {
            false
        }
    }

    pub fn push(&mut self, date: String, open: f64, high: f64, low: f64, close: f64, volume: f64) {
        self.dates.push(date);
        self.opens.push(open);
        self.highs.push(high);
        self.lows.push(low);
        self.closes.push(close);
        self.volumes.push(volume);
        // The compatibility live-update API has no timestamp parameter. Keep
        // the data valid by dropping an attached time index; callers that need
        // timestamp-preserving updates should use `push_timestamped`.
        if !self.timestamps.is_empty() {
            self.timestamps.clear();
        }
        self.bump_revision();
    }

    pub fn push_timestamped(
        &mut self,
        timestamp: i64,
        date: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> bool {
        if self.timestamps.len() != self.len() {
            return false;
        }
        if self
            .timestamps
            .last()
            .is_some_and(|last| timestamp <= *last)
        {
            return false;
        }
        self.timestamps.push(timestamp);
        self.dates.push(date);
        self.opens.push(open);
        self.highs.push(high);
        self.lows.push(low);
        self.closes.push(close);
        self.volumes.push(volume);
        self.bump_revision();
        true
    }

    pub fn slice(&self, start: usize, end: usize) -> KlineData {
        KlineData {
            dates: self.dates[start..end].to_vec(),
            timestamps: self.timestamps.get(start..end).unwrap_or(&[]).to_vec(),
            opens: self.opens[start..end].to_vec(),
            highs: self.highs[start..end].to_vec(),
            lows: self.lows[start..end].to_vec(),
            closes: self.closes[start..end].to_vec(),
            volumes: self.volumes[start..end].to_vec(),
            revision: self.revision,
        }
    }

    /// Aggregates consecutive source bars into semantic OHLCV buckets.
    ///
    /// The first open, highest high, lowest low, last close and summed volume
    /// are retained. `source_ranges` makes the display row traceable to the
    /// original data and is used by hit testing and Chan overlays.
    pub fn aggregate(&self, start: usize, end: usize, bucket: usize) -> AggregatedKline {
        let start = start.min(self.len());
        let end = end.min(self.len()).max(start);
        let bucket = bucket.max(1);
        let count = end.saturating_sub(start).div_ceil(bucket);
        let mut dates = Vec::with_capacity(count);
        let mut timestamps = Vec::with_capacity(count);
        let mut opens = Vec::with_capacity(count);
        let mut highs = Vec::with_capacity(count);
        let mut lows = Vec::with_capacity(count);
        let mut closes = Vec::with_capacity(count);
        let mut volumes = Vec::with_capacity(count);
        let mut source_ranges = Vec::with_capacity(count);

        for bucket_start in (start..end).step_by(bucket) {
            let bucket_end = (bucket_start + bucket).min(end);
            dates.push(self.dates[bucket_start].clone());
            if let Some(value) = self.timestamps.get(bucket_start) {
                timestamps.push(*value);
            }
            opens.push(self.opens[bucket_start]);
            highs.push(
                self.highs[bucket_start..bucket_end]
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max),
            );
            lows.push(
                self.lows[bucket_start..bucket_end]
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min),
            );
            closes.push(self.closes[bucket_end - 1]);
            volumes.push(self.volumes[bucket_start..bucket_end].iter().sum());
            source_ranges.push((bucket_start, bucket_end));
        }

        AggregatedKline {
            data: {
                let mut data = Self::new(dates, opens, highs, lows, closes, volumes);
                if timestamps.len() == data.len() {
                    data.timestamps = timestamps;
                }
                data.revision = self.revision;
                data
            },
            source_ranges,
        }
    }

    pub fn from_json(json_str: &str) -> crate::error::Result<Self> {
        serde_json::from_str(json_str).map_err(|e| {
            crate::error::VisualizationError::SerializationError {
                message: format!("Failed to parse JSON: {}", e),
            }
        })
    }

    pub fn from_csv(csv_str: &str) -> crate::error::Result<Self> {
        let mut dates = Vec::new();
        let mut opens = Vec::new();
        let mut highs = Vec::new();
        let mut lows = Vec::new();
        let mut closes = Vec::new();
        let mut volumes = Vec::new();

        for (i, line) in csv_str.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 6 {
                continue;
            }
            if i == 0 {
                let first = parts[0].trim().to_lowercase();
                if first == "date" || first == "日期" {
                    continue;
                }
            }
            dates.push(parts[0].trim().to_string());
            opens.push(parts[1].trim().parse::<f64>().map_err(|e| {
                crate::error::VisualizationError::ConversionError {
                    message: format!("Failed to parse open at line {}: {}", i + 1, e),
                }
            })?);
            highs.push(parts[2].trim().parse::<f64>().map_err(|e| {
                crate::error::VisualizationError::ConversionError {
                    message: format!("Failed to parse high at line {}: {}", i + 1, e),
                }
            })?);
            lows.push(parts[3].trim().parse::<f64>().map_err(|e| {
                crate::error::VisualizationError::ConversionError {
                    message: format!("Failed to parse low at line {}: {}", i + 1, e),
                }
            })?);
            closes.push(parts[4].trim().parse::<f64>().map_err(|e| {
                crate::error::VisualizationError::ConversionError {
                    message: format!("Failed to parse close at line {}: {}", i + 1, e),
                }
            })?);
            volumes.push(parts[5].trim().parse::<f64>().map_err(|e| {
                crate::error::VisualizationError::ConversionError {
                    message: format!("Failed to parse volume at line {}: {}", i + 1, e),
                }
            })?);
        }

        Ok(Self::new(dates, opens, highs, lows, closes, volumes))
    }

    pub fn len(&self) -> usize {
        self.dates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dates.is_empty()
    }

    pub fn validate(&self) -> bool {
        let len = self.dates.len();
        len > 0
            && len == self.opens.len()
            && len == self.highs.len()
            && len == self.lows.len()
            && len == self.closes.len()
            && len == self.volumes.len()
    }

    /// Validate both column shape and ordinary OHLCV invariants.
    ///
    /// `validate` intentionally remains the compatibility shape check. Use
    /// this stricter method at ingestion boundaries to catch malformed feeds
    /// before indicators, Chan analysis or rendering consume them.
    pub fn validate_ohlcv(&self) -> bool {
        self.validation_errors().is_empty()
    }

    /// Return actionable ingestion errors instead of a single boolean.
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let expected = self.dates.len();
        if expected == 0 {
            errors.push("data must contain at least one row".to_string());
            return errors;
        }
        for (name, actual) in [
            ("opens", self.opens.len()),
            ("highs", self.highs.len()),
            ("lows", self.lows.len()),
            ("closes", self.closes.len()),
            ("volumes", self.volumes.len()),
        ] {
            if actual != expected {
                errors.push(format!(
                    "column {name} has {actual} rows, expected {expected}"
                ));
            }
        }
        if !errors.is_empty() {
            return errors;
        }
        if !self.timestamps.is_empty()
            && (self.timestamps.len() != expected
                || !self.timestamps.windows(2).all(|pair| pair[0] < pair[1]))
        {
            errors.push(
                "timestamps must be empty or strictly increasing with one value per row"
                    .to_string(),
            );
        }
        for index in 0..expected {
            let values = [
                ("open", self.opens[index]),
                ("high", self.highs[index]),
                ("low", self.lows[index]),
                ("close", self.closes[index]),
                ("volume", self.volumes[index]),
            ];
            for (name, value) in values {
                if !value.is_finite() {
                    errors.push(format!("{name} at row {index} is not finite"));
                }
            }
            if self.highs[index] < self.opens[index]
                || self.highs[index] < self.closes[index]
                || self.lows[index] > self.opens[index]
                || self.lows[index] > self.closes[index]
                || self.highs[index] < self.lows[index]
            {
                errors.push(format!("OHLC range is invalid at row {index}"));
            }
            if self.volumes[index] < 0.0 {
                errors.push(format!("volume at row {index} is negative"));
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kline_data_new() {
        let data = KlineData::new(
            vec!["2024-01-01".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        );
        assert_eq!(data.len(), 1);
        assert!(data.validate());
    }

    #[test]
    fn test_kline_data_empty() {
        let data = KlineData::new(vec![], vec![], vec![], vec![], vec![], vec![]);
        assert!(data.is_empty());
        assert!(!data.validate());
        assert!(!data.validate_ohlcv());
    }

    #[test]
    fn test_kline_data_serialize() {
        let data = KlineData::new(
            vec!["2024-01-01".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        );
        let json = serde_json::to_string(&data).expect("finkit-visualization: unexpected None/Err in visualization/src/data.rs (A5 governance)");
        let deserialized: KlineData = serde_json::from_str(&json).expect("finkit-visualization: unexpected None/Err in visualization/src/data.rs (A5 governance)");
        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_kline_data_accessors() {
        let data = KlineData::new(
            vec!["2024-01-01".to_string(), "2024-01-02".to_string()],
            vec![100.0, 101.0],
            vec![105.0, 106.0],
            vec![98.0, 99.0],
            vec![103.0, 104.0],
            vec![1000.0, 1100.0],
        );
        assert_eq!(data.opens(), &[100.0, 101.0]);
        assert_eq!(data.highs(), &[105.0, 106.0]);
        assert_eq!(data.lows(), &[98.0, 99.0]);
        assert_eq!(data.closes(), &[103.0, 104.0]);
        assert_eq!(data.volumes(), &[1000.0, 1100.0]);
        assert_eq!(data.dates().len(), 2);
    }

    #[test]
    fn test_kline_data_push() {
        let mut data = KlineData::new(
            vec!["2024-01-01".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        );
        data.push("2024-01-02".to_string(), 103.0, 108.0, 101.0, 107.0, 1200.0);
        assert_eq!(data.len(), 2);
        assert_eq!(data.opens()[1], 103.0);
        assert_eq!(data.revision(), 1);
        assert_eq!(data.closes()[1], 107.0);
    }

    #[test]
    fn test_kline_data_slice() {
        let data = KlineData::new(
            vec![
                "2024-01-01".to_string(),
                "2024-01-02".to_string(),
                "2024-01-03".to_string(),
            ],
            vec![100.0, 101.0, 102.0],
            vec![105.0, 106.0, 107.0],
            vec![98.0, 99.0, 100.0],
            vec![103.0, 104.0, 105.0],
            vec![1000.0, 1100.0, 1200.0],
        );
        let sliced = data.slice(1, 3);
        assert_eq!(sliced.len(), 2);
        assert_eq!(sliced.opens(), &[101.0, 102.0]);
        assert_eq!(sliced.dates()[0], "2024-01-02");
    }

    #[test]
    fn test_kline_data_aggregate_preserves_ohlcv_semantics() {
        let data = KlineData::new(
            vec!["a".into(), "b".into(), "c".into(), "d".into()],
            vec![10.0, 11.0, 12.0, 13.0],
            vec![12.0, 15.0, 14.0, 16.0],
            vec![9.0, 10.0, 11.0, 12.0],
            vec![11.0, 13.0, 12.0, 15.0],
            vec![100.0, 200.0, 300.0, 400.0],
        );
        let aggregated = data.aggregate(0, 4, 2);
        assert_eq!(aggregated.source_ranges, vec![(0, 2), (2, 4)]);
        assert_eq!(aggregated.data.opens, vec![10.0, 12.0]);
        assert_eq!(aggregated.data.highs, vec![15.0, 16.0]);
        assert_eq!(aggregated.data.lows, vec![9.0, 11.0]);
        assert_eq!(aggregated.data.closes, vec![13.0, 15.0]);
        assert_eq!(aggregated.data.volumes, vec![300.0, 700.0]);
    }

    #[test]
    fn test_timestamp_index_roundtrips_and_aggregates() {
        let mut data = KlineData::new(
            vec!["a".into(), "b".into(), "c".into(), "d".into()],
            vec![10.0, 11.0, 12.0, 13.0],
            vec![12.0, 15.0, 14.0, 16.0],
            vec![9.0, 10.0, 11.0, 12.0],
            vec![11.0, 13.0, 12.0, 15.0],
            vec![100.0, 200.0, 300.0, 400.0],
        );
        assert!(data.set_timestamps(vec![10, 20, 30, 40]));
        assert_eq!(data.timestamps(), Some(&[10, 20, 30, 40][..]));
        let aggregated = data.aggregate(0, 4, 2);
        assert_eq!(aggregated.data.timestamps, vec![10, 30]);
        assert!(data.push_timestamped(50, "e".into(), 15.0, 17.0, 14.0, 16.0, 500.0));
        assert!(!data.push_timestamped(50, "duplicate".into(), 15.0, 17.0, 14.0, 16.0, 500.0));
        assert_eq!(data.timestamps(), Some(&[10, 20, 30, 40, 50][..]));
    }

    #[test]
    fn test_kline_data_strict_validation_reports_ohlcv_errors() {
        let invalid = KlineData::new(
            vec!["2024-01-01".into()],
            vec![100.0],
            vec![99.0],
            vec![98.0],
            vec![101.0],
            vec![-1.0],
        );
        assert!(!invalid.validate_ohlcv());
        let errors = invalid.validation_errors();
        assert!(errors.iter().any(|error| error.contains("OHLC")));
        assert!(errors.iter().any(|error| error.contains("negative")));
    }

    #[test]
    fn test_kline_data_strict_validation_accepts_valid_row() {
        let data = KlineData::new(
            vec!["2024-01-01".into()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        );
        assert!(data.validate_ohlcv());
        assert!(data.validation_errors().is_empty());
    }

    #[test]
    fn test_kline_frame_shares_columns_and_roundtrips() {
        let mut data = KlineData::new(
            vec!["2024-01-01".into()],
            vec![10.0],
            vec![12.0],
            vec![9.0],
            vec![11.0],
            vec![100.0],
        );
        data.push("2024-01-02".into(), 11.0, 13.0, 10.0, 12.0, 120.0);
        assert!(data.set_timestamps(vec![1_704_067_200, 1_704_153_600]));
        let frame = KlineFrame::from_kline_data(&data, data.timestamps.clone())
            .expect("valid shared frame");
        assert_eq!(frame.len(), 2);
        assert_eq!(frame.revision, data.revision);
        assert_eq!(frame.to_kline_data(), data);
        assert!(frame.validate_timestamps());
        assert!(KlineFrame::from_kline_data(&data, vec![2, 1]).is_none());
    }

    #[test]
    fn test_kline_data_from_json() {
        let json = r#"{"dates":["2024-01-01"],"opens":[100.0],"highs":[105.0],"lows":[98.0],"closes":[103.0],"volumes":[1000.0]}"#;
        let data = KlineData::from_json(json).expect("finkit-visualization: unexpected None/Err in visualization/src/data.rs (A5 governance)");
        assert_eq!(data.len(), 1);
        assert_eq!(data.closes()[0], 103.0);
    }

    #[test]
    fn test_kline_data_from_csv() {
        let csv = "date,open,high,low,close,volume\n2024-01-01,100.0,105.0,98.0,103.0,1000.0\n2024-01-02,103.0,108.0,101.0,107.0,1200.0";
        let data = KlineData::from_csv(csv).expect("finkit-visualization: unexpected None/Err in visualization/src/data.rs (A5 governance)");
        assert_eq!(data.len(), 2);
        assert_eq!(data.opens()[0], 100.0);
        assert_eq!(data.closes()[1], 107.0);
    }

    #[test]
    fn test_kline_data_from_csv_no_header() {
        let csv = "2024-01-01,100.0,105.0,98.0,103.0,1000.0";
        let data = KlineData::from_csv(csv).expect("finkit-visualization: unexpected None/Err in visualization/src/data.rs (A5 governance)");
        assert_eq!(data.len(), 1);
        assert_eq!(data.dates()[0], "2024-01-01");
    }
}
