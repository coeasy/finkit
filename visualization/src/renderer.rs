use crate::config::{ChartConfig, IndicatorConfig};
use crate::data::KlineData;
use crate::error::{Result, VisualizationError};
use crate::language::LanguageResource;
use crate::lightweight::LightweightChartsPayload;
use serde_json::json;

const LIGHTWEIGHT_CHARTS_CDN: &str =
    "https://unpkg.com/lightweight-charts@5.0.0/dist/lightweight-charts.standalone.production.mjs";
const LIGHTWEIGHT_ADAPTER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/frontend/lightweight-charts-adapter.js"
));

pub struct RenderContext {
    pub config: ChartConfig,
    pub resource: &'static LanguageResource,
}

impl RenderContext {
    pub fn new(config: ChartConfig) -> Self {
        let resource = LanguageResource::from_language(&config.language);
        Self { config, resource }
    }
}

pub trait Renderer {
    fn render(&self, data: &KlineData, indicators: &[IndicatorConfig]) -> Result<String>;
    fn render_html(&self, data: &KlineData, indicators: &[IndicatorConfig]) -> Result<String>;
}

pub struct ChartRenderer {
    context: RenderContext,
}

impl ChartRenderer {
    pub fn new(config: ChartConfig) -> Self {
        Self {
            context: RenderContext::new(config),
        }
    }

    pub fn config(&self) -> &ChartConfig {
        &self.context.config
    }

    pub fn context(&self) -> &RenderContext {
        &self.context
    }

    fn validate_data(&self, data: &KlineData) -> Result<()> {
        if data.is_empty() {
            return Err(VisualizationError::EmptyData);
        }
        if !data.validate() {
            return Err(VisualizationError::ConversionError {
                message: "Data arrays have inconsistent lengths".to_string(),
            });
        }
        Ok(())
    }
}

impl Renderer for ChartRenderer {
    fn render(&self, data: &KlineData, indicators: &[IndicatorConfig]) -> Result<String> {
        self.validate_data(data)?;
        let json_data =
            serde_json::to_string(&data).map_err(|e| VisualizationError::SerializationError {
                message: e.to_string(),
            })?;
        let json_indicators = serde_json::to_string(&indicators.to_vec()).map_err(|e| {
            VisualizationError::SerializationError {
                message: e.to_string(),
            }
        })?;

        Ok(format!(
            "{{\"data\":{},\"indicators\":{},\"config\":{}}}",
            json_data,
            json_indicators,
            serde_json::to_string(&self.context.config).map_err(|e| {
                VisualizationError::SerializationError {
                    message: e.to_string(),
                }
            })?
        ))
    }

    fn render_html(&self, data: &KlineData, indicators: &[IndicatorConfig]) -> Result<String> {
        self.validate_data(data)?;
        let mut payload = LightweightChartsPayload::from_kline(data)?;
        payload.add_indicator_lines(data, indicators)?;
        let payload = script_json(&payload.to_json_string()?);
        let options_json = serde_json::to_string(&json!({
            "width": self.context.config.width,
            "height": self.context.config.height,
            "layout": {
                "background": { "color": self.context.config.theme_config.background_color },
                "textColor": self.context.config.theme_config.font_color,
            },
            "grid": {
                "vertLines": { "color": self.context.config.theme_config.grid_color },
                "horzLines": { "color": self.context.config.theme_config.grid_color },
            },
            "crosshair": {
                "mode": if self.context.config.interaction.show_crosshair { 0 } else { 2 },
            },
            "rightPriceScale": { "borderColor": self.context.config.theme_config.axis_line_color },
            "timeScale": { "borderColor": self.context.config.theme_config.axis_line_color },
        }))
        .map_err(|error| VisualizationError::SerializationError {
            message: error.to_string(),
        })?;
        let options = script_json(&options_json);
        let title = html_escape(&self.context.config.title);

        Ok(format!(
            r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <style>html,body,#finkit-chart{{margin:0;width:100%;height:100%;min-height:{height}px;}}</style>
</head>
<body>
  <div id="finkit-chart" aria-label="{title}"></div>
  <script type="module">
    import * as LightweightCharts from "{cdn}";
{adapter}
    const payload = {payload};
    const chartOptions = {options};
    window.finkitChart = createFinkitLightweightChart(
      document.getElementById("finkit-chart"),
      payload,
      LightweightCharts,
      {{ chart: chartOptions }},
    );
  </script>
</body>
</html>"#,
            title = title,
            height = self.context.config.height,
            cdn = LIGHTWEIGHT_CHARTS_CDN,
            adapter = LIGHTWEIGHT_ADAPTER,
            payload = payload,
            options = options,
        ))
    }
}

fn script_json(value: &str) -> String {
    value
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChartConfig, IndicatorConfig, IndicatorType};
    use crate::data::KlineData;
    use crate::language::Language;

    #[test]
    fn test_renderer_creation() {
        let config = ChartConfig::default();
        let renderer = ChartRenderer::new(config);
        assert_eq!(renderer.config().width, 1200);
    }

    #[test]
    fn test_validate_data_empty() {
        let renderer = ChartRenderer::new(ChartConfig::default());
        let empty_data = KlineData::new(vec![], vec![], vec![], vec![], vec![], vec![]);
        assert!(renderer.validate_data(&empty_data).is_err());
    }

    #[test]
    fn test_validate_data_valid() {
        let renderer = ChartRenderer::new(ChartConfig::default());
        let data = KlineData::new(
            vec!["2024-01-01".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        );
        assert!(renderer.validate_data(&data).is_ok());
    }

    #[test]
    fn test_render_json() {
        let renderer = ChartRenderer::new(ChartConfig::new("Test", Language::ZhCn));
        let data = KlineData::new(
            vec!["2024-01-01".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        );
        let indicators = vec![IndicatorConfig::new(IndicatorType::MA, vec![5.0])];
        let result = renderer.render(&data, &indicators);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_html_uses_lightweight_charts_payload() {
        let renderer = ChartRenderer::new(ChartConfig::new("<Test>", Language::ZhCn));
        let mut data = KlineData::new(
            vec!["2024-01-01".to_string(), "2024-01-02".to_string()],
            vec![100.0, 103.0],
            vec![105.0, 108.0],
            vec![98.0, 101.0],
            vec![103.0, 106.0],
            vec![1000.0, 1200.0],
        );
        data.revision = 7;
        let indicators = vec![IndicatorConfig::new(IndicatorType::SMA, vec![2.0])];
        let result = renderer.render_html(&data, &indicators).unwrap();

        assert!(result.contains("createFinkitLightweightChart"));
        assert!(result.contains("\"schema_version\":1"));
        assert!(result.contains("\"name\":\"SMA2\""));
        assert!(result.contains("&lt;Test&gt;"));
        assert!(!result.contains("HTML rendering is not yet implemented"));
    }
}
