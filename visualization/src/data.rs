use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KlineData {
    pub dates: Vec<String>,
    pub opens: Vec<f64>,
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
    pub closes: Vec<f64>,
    pub volumes: Vec<f64>,
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
            opens,
            highs,
            lows,
            closes,
            volumes,
        }
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

    pub fn push(&mut self, date: String, open: f64, high: f64, low: f64, close: f64, volume: f64) {
        self.dates.push(date);
        self.opens.push(open);
        self.highs.push(high);
        self.lows.push(low);
        self.closes.push(close);
        self.volumes.push(volume);
    }

    pub fn slice(&self, start: usize, end: usize) -> KlineData {
        KlineData {
            dates: self.dates[start..end].to_vec(),
            opens: self.opens[start..end].to_vec(),
            highs: self.highs[start..end].to_vec(),
            lows: self.lows[start..end].to_vec(),
            closes: self.closes[start..end].to_vec(),
            volumes: self.volumes[start..end].to_vec(),
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
