use serde::{Deserialize, Serialize};

use crate::language::Language;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeConfig {
    pub background_color: String,
    pub grid_color: String,
    pub font_color: String,
    pub font_size: u32,
    pub crosshair_color: String,
    pub axis_line_color: String,
}

impl ThemeConfig {
    pub fn light() -> Self {
        Self {
            background_color: "#ffffff".to_string(),
            grid_color: "#f0f0f0".to_string(),
            font_color: "#333333".to_string(),
            font_size: 12,
            crosshair_color: "#cccccc".to_string(),
            axis_line_color: "#dddddd".to_string(),
        }
    }

    pub fn dark() -> Self {
        Self {
            background_color: "#1a1a2e".to_string(),
            grid_color: "#2d2d44".to_string(),
            font_color: "#d1d1e9".to_string(),
            font_size: 12,
            crosshair_color: "#555577".to_string(),
            axis_line_color: "#444466".to_string(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self::light()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum ColorScheme {
    #[default]
    ChinaMode,
    InternationalMode,
    Custom {
        up_color: String,
        down_color: String,
    },
}

impl ColorScheme {
    pub fn from_language(lang: &Language) -> Self {
        match lang {
            Language::ZhCn => Self::ChinaMode,
            Language::EnUs => Self::InternationalMode,
        }
    }

    pub fn up_color(&self) -> &str {
        match self {
            Self::ChinaMode => "#ef4444",
            Self::InternationalMode => "#22c55e",
            Self::Custom { up_color, .. } => up_color,
        }
    }

    pub fn down_color(&self) -> &str {
        match self {
            Self::ChinaMode => "#22c55e",
            Self::InternationalMode => "#ef4444",
            Self::Custom { down_color, .. } => down_color,
        }
    }

    pub fn with_colors(up_color: &str, down_color: &str) -> Self {
        Self::Custom {
            up_color: up_color.to_string(),
            down_color: down_color.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum ChartType {
    #[default]
    Candlestick,
    Line,
    Bar,
    Area,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    pub fn config(&self) -> ThemeConfig {
        match self {
            Self::Light => ThemeConfig::light(),
            Self::Dark => ThemeConfig::dark(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub enum LineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
    DashDot,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub enum DecimateStrategy {
    LTTB,
    MinMax,
    EveryNth,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Margin {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

impl Default for Margin {
    fn default() -> Self {
        Self {
            top: 40,
            right: 40,
            bottom: 40,
            left: 60,
        }
    }
}

impl Margin {
    pub fn new(top: u32, right: u32, bottom: u32, left: u32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn uniform(value: u32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChartConfig {
    pub title: String,
    pub show_legend: bool,
    pub show_volume: bool,
    pub chart_type: ChartType,
    pub theme: Theme,
    pub theme_config: ThemeConfig,
    pub language: Language,
    pub color_scheme: ColorScheme,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub decimate_strategy: DecimateStrategy,
    pub margins: Margin,
}

impl Default for ChartConfig {
    fn default() -> Self {
        let language = Language::default();
        let theme = Theme::default();
        let theme_config = theme.config();
        Self {
            title: String::new(),
            show_legend: true,
            show_volume: true,
            chart_type: ChartType::default(),
            theme,
            theme_config,
            language,
            color_scheme: ColorScheme::from_language(&language),
            width: 1200,
            height: 600,
            dpi: 144,
            decimate_strategy: DecimateStrategy::default(),
            margins: Margin::default(),
        }
    }
}

impl ChartConfig {
    pub fn new(title: &str, language: Language) -> Self {
        let theme = Theme::default();
        Self {
            title: title.to_string(),
            language,
            color_scheme: ColorScheme::from_language(&language),
            theme_config: theme.config(),
            ..Default::default()
        }
    }

    pub fn set_language(&mut self, language: Language) {
        self.color_scheme = ColorScheme::from_language(&language);
        self.language = language;
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme_config = theme.config();
        self.theme = theme;
    }
}

pub struct ChartConfigBuilder {
    config: ChartConfig,
}

impl ChartConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: ChartConfig::default(),
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.config.title = title.to_string();
        self
    }

    pub fn with_language(mut self, language: Language) -> Self {
        self.config.language = language;
        self.config.color_scheme = ColorScheme::from_language(&language);
        self
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.config.theme_config = theme.config();
        self.config.theme = theme;
        self
    }

    pub fn with_chart_type(mut self, chart_type: ChartType) -> Self {
        self.config.chart_type = chart_type;
        self
    }

    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    pub fn show_legend(mut self, show: bool) -> Self {
        self.config.show_legend = show;
        self
    }

    pub fn show_volume(mut self, show: bool) -> Self {
        self.config.show_volume = show;
        self
    }

    pub fn with_color_scheme(mut self, scheme: ColorScheme) -> Self {
        self.config.color_scheme = scheme;
        self
    }

    pub fn with_dpi(mut self, dpi: u32) -> Self {
        self.config.dpi = dpi;
        self
    }

    pub fn with_decimate_strategy(mut self, strategy: DecimateStrategy) -> Self {
        self.config.decimate_strategy = strategy;
        self
    }

    pub fn with_margins(mut self, margins: Margin) -> Self {
        self.config.margins = margins;
        self
    }

    pub fn build(self) -> ChartConfig {
        self.config
    }
}

impl Default for ChartConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndicatorType {
    MA,
    EMA,
    SMA,
    RSI,
    MACD,
    BOLL,
    KDJ,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndicatorConfig {
    pub indicator_type: IndicatorType,
    pub name: String,
    pub params: Vec<f64>,
    pub color: String,
    pub line_width: f32,
    pub visible: bool,
}

impl IndicatorConfig {
    pub fn new(indicator_type: IndicatorType, params: Vec<f64>) -> Self {
        let name = match &indicator_type {
            IndicatorType::MA => "MA".to_string(),
            IndicatorType::EMA => "EMA".to_string(),
            IndicatorType::SMA => "SMA".to_string(),
            IndicatorType::RSI => "RSI".to_string(),
            IndicatorType::MACD => "MACD".to_string(),
            IndicatorType::BOLL => "BOLL".to_string(),
            IndicatorType::KDJ => "KDJ".to_string(),
            IndicatorType::Custom(name) => name.clone(),
        };

        Self {
            indicator_type,
            name,
            params,
            color: "#000000".to_string(),
            line_width: 1.5,
            visible: true,
        }
    }

    pub fn with_color(mut self, color: &str) -> Self {
        self.color = color.to_string();
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_config_default() {
        let config = ChartConfig::default();
        assert!(config.show_legend);
        assert!(config.show_volume);
        assert_eq!(config.width, 1200);
        assert_eq!(config.height, 600);
        assert_eq!(config.dpi, 144);
        assert!(matches!(config.decimate_strategy, DecimateStrategy::Auto));
        assert_eq!(config.margins.top, 40);
        assert_eq!(config.margins.left, 60);
    }

    #[test]
    fn test_chart_config_new() {
        let config = ChartConfig::new("Test Chart", Language::ZhCn);
        assert_eq!(config.title, "Test Chart");
        assert!(matches!(config.language, Language::ZhCn));
    }

    #[test]
    fn test_indicator_config_new() {
        let indicator = IndicatorConfig::new(IndicatorType::RSI, vec![14.0]);
        assert_eq!(indicator.name, "RSI");
        assert_eq!(indicator.params, vec![14.0]);
        assert!(indicator.visible);
    }

    #[test]
    fn test_indicator_config_with_color() {
        let indicator = IndicatorConfig::new(IndicatorType::EMA, vec![12.0])
            .with_color("#ff0000")
            .with_visible(false);
        assert_eq!(indicator.color, "#ff0000");
        assert!(!indicator.visible);
    }

    #[test]
    fn test_theme_config() {
        let light = ThemeConfig::light();
        assert_eq!(light.background_color, "#ffffff");
        assert_eq!(light.font_color, "#333333");

        let dark = ThemeConfig::dark();
        assert_eq!(dark.background_color, "#1a1a2e");
        assert_eq!(dark.font_color, "#d1d1e9");
    }

    #[test]
    fn test_color_scheme() {
        let china = ColorScheme::ChinaMode;
        assert_eq!(china.up_color(), "#ef4444");
        assert_eq!(china.down_color(), "#22c55e");

        let intl = ColorScheme::InternationalMode;
        assert_eq!(intl.up_color(), "#22c55e");
        assert_eq!(intl.down_color(), "#ef4444");

        let custom = ColorScheme::with_colors("#00ff00", "#ff0000");
        assert_eq!(custom.up_color(), "#00ff00");
        assert_eq!(custom.down_color(), "#ff0000");
    }

    #[test]
    fn test_color_scheme_from_language() {
        let china = ColorScheme::from_language(&Language::ZhCn);
        assert!(matches!(china, ColorScheme::ChinaMode));

        let intl = ColorScheme::from_language(&Language::EnUs);
        assert!(matches!(intl, ColorScheme::InternationalMode));
    }

    #[test]
    fn test_chart_config_set_language() {
        let mut config = ChartConfig::new("Test", Language::ZhCn);
        assert!(matches!(config.color_scheme, ColorScheme::ChinaMode));

        config.set_language(Language::EnUs);
        assert!(matches!(
            config.color_scheme,
            ColorScheme::InternationalMode
        ));
        assert!(matches!(config.language, Language::EnUs));
    }

    #[test]
    fn test_chart_config_set_theme() {
        let mut config = ChartConfig::default();
        assert_eq!(config.theme_config.background_color, "#ffffff");

        config.set_theme(Theme::Dark);
        assert_eq!(config.theme_config.background_color, "#1a1a2e");
        assert!(matches!(config.theme, Theme::Dark));
    }

    #[test]
    fn test_chart_config_builder() {
        let config = ChartConfigBuilder::new()
            .with_title("My Chart")
            .with_language(Language::EnUs)
            .with_theme(Theme::Dark)
            .with_chart_type(ChartType::Line)
            .with_dimensions(800, 400)
            .show_legend(false)
            .show_volume(true)
            .build();

        assert_eq!(config.title, "My Chart");
        assert!(matches!(config.language, Language::EnUs));
        assert!(matches!(
            config.color_scheme,
            ColorScheme::InternationalMode
        ));
        assert!(matches!(config.theme, Theme::Dark));
        assert_eq!(config.theme_config.background_color, "#1a1a2e");
        assert!(matches!(config.chart_type, ChartType::Line));
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 400);
        assert!(!config.show_legend);
        assert!(config.show_volume);
    }

    #[test]
    fn test_line_style() {
        assert!(matches!(LineStyle::default(), LineStyle::Solid));
    }

    #[test]
    fn test_decimate_strategy() {
        assert!(matches!(
            DecimateStrategy::default(),
            DecimateStrategy::Auto
        ));
    }

    #[test]
    fn test_margin_default() {
        let m = Margin::default();
        assert_eq!(m.top, 40);
        assert_eq!(m.right, 40);
        assert_eq!(m.bottom, 40);
        assert_eq!(m.left, 60);
    }

    #[test]
    fn test_margin_uniform() {
        let m = Margin::uniform(20);
        assert_eq!(m.top, 20);
        assert_eq!(m.right, 20);
        assert_eq!(m.bottom, 20);
        assert_eq!(m.left, 20);
    }

    #[test]
    fn test_margin_new() {
        let m = Margin::new(10, 20, 30, 40);
        assert_eq!(m.top, 10);
        assert_eq!(m.right, 20);
        assert_eq!(m.bottom, 30);
        assert_eq!(m.left, 40);
    }

    #[test]
    fn test_builder_with_dpi_and_margins() {
        let config = ChartConfigBuilder::new()
            .with_dpi(300)
            .with_decimate_strategy(DecimateStrategy::LTTB)
            .with_margins(Margin::uniform(10))
            .build();
        assert_eq!(config.dpi, 300);
        assert!(matches!(config.decimate_strategy, DecimateStrategy::LTTB));
        assert_eq!(config.margins.top, 10);
    }
}
