use crate::config::{ChartConfig, IndicatorConfig};
use crate::data::KlineData;
use crate::error::{Result, VisualizationError};
use crate::language::LanguageResource;

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

    fn render_html(&self, _data: &KlineData, _indicators: &[IndicatorConfig]) -> Result<String> {
        Err(VisualizationError::RenderError {
            message: "HTML rendering is not yet implemented".to_string(),
        })
    }
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
}
