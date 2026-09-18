//! Lightweight Charts data adapter.
//!
//! The adapter is intentionally renderer-agnostic: the Rust side validates
//! and shapes chart data, while the small frontend helper owns the
//! Lightweight Charts instance. Existing SVG/Canvas/PNG renderers remain
//! available for native and headless output.

use crate::data::KlineData;
use crate::error::{Result, VisualizationError};
use crate::scene::{ChartScene, HitTarget, PanelId};
use crate::viewport::Viewport;
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

/// A panel descriptor consumed by the web adapter. Absolute rectangles are
/// retained as metadata; Lightweight Charts owns the actual pane layout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightweightPanel {
    /// Stable panel identifier.
    pub id: String,
    /// Source scene rectangle, when available.
    pub rect: LightweightRect,
    /// Whether the panel is visible.
    pub visible: bool,
}

/// A render layer descriptor retained for frontend visibility controls.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightweightLayer {
    /// Stable layer identifier.
    pub id: String,
    /// Owning panel identifier.
    pub panel: String,
    /// Draw ordering inherited from the semantic scene.
    pub z_index: i32,
    /// Current visibility.
    pub visible: bool,
    /// Opacity in the source scene.
    pub opacity: f32,
}

/// Rectangular scene metadata. Lightweight Charts does not position panes by
/// absolute pixels, but other web adapters can use the same scene payload.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct LightweightRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A semantic event or signal marker mapped to a candle time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightweightMarker {
    /// Marker time.
    pub time: LightweightTime,
    /// `aboveBar`, `belowBar`, or `inBar`.
    pub position: String,
    /// Lightweight Charts marker shape.
    pub shape: String,
    /// CSS color.
    pub color: String,
    /// Optional short label or tooltip text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Viewport metadata for logical-range synchronization.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct LightweightViewport {
    pub start: usize,
    pub end: usize,
    pub follow_latest: bool,
}

/// Semantic scene portion of a Lightweight Charts payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightweightScene {
    pub panels: Vec<LightweightPanel>,
    pub layers: Vec<LightweightLayer>,
    pub markers: Vec<LightweightMarker>,
    pub viewport: LightweightViewport,
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
    /// Optional semantic scene data for interactive web adapters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<LightweightScene>,
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
            scene: None,
        })
    }

    /// Build a payload that includes semantic panels, layers, markers and
    /// viewport metadata from the renderer-neutral chart scene.
    pub fn from_kline_scene(
        data: &KlineData,
        scene: &ChartScene,
        viewport: Viewport,
    ) -> Result<Self> {
        let mut payload = Self::from_kline(data)?;
        let markers = scene_markers(data, &payload.candles, scene)?;
        let panels = scene
            .panels
            .iter()
            .map(|panel| LightweightPanel {
                id: panel_id(panel.id),
                rect: LightweightRect {
                    x: panel.rect.x,
                    y: panel.rect.y,
                    width: panel.rect.width,
                    height: panel.rect.height,
                },
                visible: panel.visible,
            })
            .collect();
        let layers = scene
            .layers
            .iter()
            .map(|layer| LightweightLayer {
                id: layer.id.clone(),
                panel: panel_id(layer.panel),
                z_index: layer.z_index,
                visible: layer.visible,
                opacity: layer.opacity,
            })
            .collect();
        let (start, end) = viewport.resolve(data.len());
        payload.scene = Some(LightweightScene {
            panels,
            layers,
            markers,
            viewport: LightweightViewport {
                start,
                end,
                follow_latest: viewport.follow_latest,
            },
        });
        Ok(payload)
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

fn panel_id(panel: PanelId) -> String {
    match panel {
        PanelId::Main => "main".to_string(),
        PanelId::Volume => "volume".to_string(),
        PanelId::Indicator(index) => format!("indicator:{index}"),
    }
}

fn scene_markers(
    data: &KlineData,
    candles: &[LightweightCandle],
    scene: &ChartScene,
) -> Result<Vec<LightweightMarker>> {
    let mut markers = Vec::new();
    for region in &scene.hit_regions {
        let (index, position, shape, color, fallback_text) = match &region.target {
            HitTarget::Event { index, kind } => {
                (*index, "aboveBar", "circle", "#f59e0b", kind.clone())
            }
            HitTarget::ChanSignal { index, kind } => {
                let is_sell = kind.to_ascii_uppercase().starts_with('S');
                (
                    *index,
                    if is_sell { "aboveBar" } else { "belowBar" },
                    if is_sell { "arrowDown" } else { "arrowUp" },
                    if is_sell { "#ef5350" } else { "#26a69a" },
                    kind.clone(),
                )
            }
            HitTarget::ChanDivergence { index, kind } => {
                (*index, "aboveBar", "square", "#ab47bc", kind.clone())
            }
            _ => continue,
        };
        if index >= data.len() || index >= candles.len() {
            return Err(VisualizationError::ConversionError {
                message: format!("scene marker index {index} exceeds kline length"),
            });
        }
        markers.push((
            index,
            LightweightMarker {
                time: candles[index].time.clone(),
                position: position.to_string(),
                shape: shape.to_string(),
                color: color.to_string(),
                text: region.tooltip.clone().or(Some(fallback_text)),
            },
        ));
    }
    markers.sort_by_key(|(index, _)| *index);
    Ok(markers.into_iter().map(|(_, marker)| marker).collect())
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

    #[test]
    fn scene_payload_maps_markers_panels_layers_and_viewport() {
        use crate::geometry::Rect;
        use crate::scene::{ChartMetadata, HitRegion, LayerDescriptor, PanelDescriptor};

        let data = data();
        let scene = ChartScene {
            panels: vec![PanelDescriptor {
                id: PanelId::Main,
                rect: Rect::new(0.0, 0.0, 800.0, 500.0),
                visible: true,
            }],
            layers: vec![LayerDescriptor::new("price", PanelId::Main, 10)],
            hit_regions: vec![HitRegion {
                rect: Rect::zero(),
                target: HitTarget::Event {
                    index: 1,
                    kind: "signal".to_string(),
                },
                priority: 1,
                tooltip: Some("event".to_string()),
            }],
            metadata: ChartMetadata::default(),
        };
        let mut payload =
            LightweightChartsPayload::from_kline_scene(&data, &scene, Viewport::new(0, 1)).unwrap();
        let scene = payload.scene.take().unwrap();
        assert_eq!(scene.panels[0].id, "main");
        assert_eq!(scene.layers[0].id, "price");
        assert_eq!(scene.markers[0].text.as_deref(), Some("event"));
        assert_eq!(scene.viewport.end, 1);
    }
}
