use alpha_ta_visualization::chart::KlineChart;
use alpha_ta_visualization::config::{
    ChartConfigBuilder, ChartType, ColorScheme, DecimateStrategy, IndicatorConfig, IndicatorType,
    Theme,
};
use alpha_ta_visualization::data::KlineData;
use alpha_ta_visualization::decimate::decimate;
use alpha_ta_visualization::language::Language;
use alpha_ta_visualization::layout::LayoutCalculator;
use alpha_ta_visualization::primitive::DrawList;
use alpha_ta_visualization::render::{Renderer, SvgRenderer};

fn make_test_data(n: usize) -> KlineData {
    let mut data = KlineData::new(
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    let mut price = 100.0_f64;
    for i in 0..n {
        let day = i + 1;
        let month = (day - 1) / 30 + 1;
        let d = (day - 1) % 30 + 1;
        let date = format!("2024-{:02}-{:02}", month, d);
        let change = (i as f64 * 0.1).sin() * 2.0 + ((i % 7) as f64 - 3.0) * 0.5;
        let open = price;
        let close = price + change;
        let high = open.max(close) + change.abs() * 0.3;
        let low = open.min(close) - change.abs() * 0.3;
        let volume = 1000.0 + (i as f64 * 0.2).sin() * 500.0 + i as f64 * 10.0;
        data.push(date, open, high, low, close, volume);
        price = close;
    }
    data
}

#[test]
fn test_full_render_pipeline() {
    let data = make_test_data(50);
    let config = ChartConfigBuilder::new()
        .with_title("Integration Test")
        .with_chart_type(ChartType::Candlestick)
        .build();

    let mut chart = KlineChart::new(config);
    let indicators = vec![IndicatorConfig::new(IndicatorType::MA, vec![5.0, 10.0])];
    chart.build_draw_list(&data, &indicators).unwrap();

    let svg = chart.to_svg_string().unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("</svg>"));
    assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains(">Integration Test</text>"));
    assert!(svg.contains("<line"));
    assert!(svg.contains("<rect"));
}

#[test]
fn test_render_pipeline_save_svg() {
    let data = make_test_data(20);
    let config = ChartConfigBuilder::new().with_title("Save Test").build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();

    let path = std::env::temp_dir().join("fta_integration_test.svg");
    chart.save_as_svg(path.to_str().unwrap()).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.starts_with("<svg"));
    assert!(content.contains("</svg>"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_chart_type_candlestick() {
    let data = make_test_data(30);
    let config = ChartConfigBuilder::new()
        .with_chart_type(ChartType::Candlestick)
        .build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("<rect"));
}

#[test]
fn test_chart_type_ohlc_bar() {
    let data = make_test_data(30);
    let config = ChartConfigBuilder::new()
        .with_chart_type(ChartType::Bar)
        .build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("<line"));
}

#[test]
fn test_chart_type_line() {
    let data = make_test_data(30);
    let config = ChartConfigBuilder::new()
        .with_chart_type(ChartType::Line)
        .build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("<path"));
}

#[test]
fn test_chart_type_area() {
    let data = make_test_data(30);
    let config = ChartConfigBuilder::new()
        .with_chart_type(ChartType::Area)
        .build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("<polygon") || svg.contains("<path"));
}

#[test]
fn test_indicator_ma() {
    let data = make_test_data(50);
    let config = ChartConfigBuilder::new().build();
    let mut chart = KlineChart::new(config);
    let indicators = vec![IndicatorConfig::new(
        IndicatorType::MA,
        vec![5.0, 10.0, 20.0],
    )];
    chart.build_draw_list(&data, &indicators).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("<path"));
}

#[test]
fn test_indicator_ema() {
    let data = make_test_data(50);
    let config = ChartConfigBuilder::new().build();
    let mut chart = KlineChart::new(config);
    let indicators = vec![IndicatorConfig::new(IndicatorType::EMA, vec![12.0, 26.0])];
    chart.build_draw_list(&data, &indicators).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("<path"));
}

#[test]
fn test_indicator_boll() {
    let data = make_test_data(50);
    let config = ChartConfigBuilder::new().build();
    let mut chart = KlineChart::new(config);
    let indicators = vec![IndicatorConfig::new(IndicatorType::BOLL, vec![20.0, 2.0])];
    chart.build_draw_list(&data, &indicators).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("<polygon") || svg.contains("<path"));
}

#[test]
fn test_indicator_macd() {
    let data = make_test_data(50);
    let config = ChartConfigBuilder::new().build();
    let mut chart = KlineChart::new(config);
    let indicators = vec![IndicatorConfig::new(
        IndicatorType::MACD,
        vec![12.0, 26.0, 9.0],
    )];
    chart.build_draw_list(&data, &indicators).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("<path"));
}

#[test]
fn test_indicator_rsi() {
    let data = make_test_data(50);
    let config = ChartConfigBuilder::new().build();
    let mut chart = KlineChart::new(config);
    let indicators = vec![IndicatorConfig::new(IndicatorType::RSI, vec![14.0])];
    chart.build_draw_list(&data, &indicators).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("<path") || svg.contains("<line"));
}

#[test]
fn test_indicator_kdj() {
    let data = make_test_data(50);
    let config = ChartConfigBuilder::new().build();
    let mut chart = KlineChart::new(config);
    let indicators = vec![IndicatorConfig::new(
        IndicatorType::KDJ,
        vec![9.0, 3.0, 3.0],
    )];
    chart.build_draw_list(&data, &indicators).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("<path"));
}

#[test]
fn test_indicator_sar() {
    let data = make_test_data(50);
    let config = ChartConfigBuilder::new().build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    chart.add_sar(&data, 0.02, 0.2);
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("<circle"));
}

#[test]
fn test_multiple_indicators_combined() {
    let data = make_test_data(60);
    let config = ChartConfigBuilder::new()
        .with_title("Multi Indicator")
        .build();
    let mut chart = KlineChart::new(config);
    let indicators = vec![
        IndicatorConfig::new(IndicatorType::MA, vec![5.0, 10.0]),
        IndicatorConfig::new(IndicatorType::BOLL, vec![20.0, 2.0]),
        IndicatorConfig::new(IndicatorType::MACD, vec![12.0, 26.0, 9.0]),
        IndicatorConfig::new(IndicatorType::RSI, vec![14.0]),
    ];
    chart.build_draw_list(&data, &indicators).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("</svg>"));
}

#[test]
fn test_language_chinese() {
    let data = make_test_data(20);
    let config = ChartConfigBuilder::new()
        .with_title("中文测试")
        .with_language(Language::ZhCn)
        .build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains(">中文测试</text>"));
    assert!(matches!(
        chart.config().color_scheme,
        ColorScheme::ChinaMode
    ));
}

#[test]
fn test_language_english() {
    let data = make_test_data(20);
    let config = ChartConfigBuilder::new()
        .with_title("English Test")
        .with_language(Language::EnUs)
        .build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains(">English Test</text>"));
    assert!(matches!(
        chart.config().color_scheme,
        ColorScheme::InternationalMode
    ));
}

#[test]
fn test_theme_light() {
    let data = make_test_data(20);
    let config = ChartConfigBuilder::new().with_theme(Theme::Light).build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("#ffffff"));
}

#[test]
fn test_theme_dark() {
    let data = make_test_data(20);
    let config = ChartConfigBuilder::new().with_theme(Theme::Dark).build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.contains("#1a1a2e"));
}

#[test]
fn test_layout_calculation() {
    let data = make_test_data(30);
    let config = ChartConfigBuilder::new()
        .with_title("Layout Test")
        .show_volume(true)
        .build();
    let layout = LayoutCalculator::calculate(&data, &config, 1);
    assert!(layout.main_panel.plot_area.width > 0.0);
    assert!(layout.main_panel.plot_area.height > 0.0);
    assert_eq!(layout.sub_panels.len(), 1);
    assert!(!layout.x_axis.ticks.is_empty());
    assert!(!layout.y_axes.is_empty());
}

#[test]
fn test_decimate_integration() {
    let data = make_test_data(5000);
    let result = decimate(&data, &DecimateStrategy::LTTB, 1200);
    assert!(result.len() <= 1202);
    assert_eq!(result.original_len, 5000);
    assert_eq!(result.indices[0], 0);
    assert_eq!(*result.indices.last().unwrap(), 4999);
}

#[test]
fn test_svg_renderer_direct() {
    let draw_list = DrawList::new();
    let config = ChartConfigBuilder::new().build();
    let renderer = SvgRenderer::new();
    let result = renderer.render(&draw_list, &config).unwrap();
    assert!(result.starts_with("<svg"));
    assert!(result.contains("</svg>"));
}

#[test]
fn test_empty_data_error() {
    let data = KlineData::new(vec![], vec![], vec![], vec![], vec![], vec![]);
    let config = ChartConfigBuilder::new().build();
    let mut chart = KlineChart::new(config);
    let result = chart.build_draw_list(&data, &[]);
    assert!(result.is_err());
}

#[test]
fn test_incremental_render() {
    let data = make_test_data(30);
    let config = ChartConfigBuilder::new()
        .with_chart_type(ChartType::Candlestick)
        .build();
    let mut chart = KlineChart::new(config);
    chart.set_data(data);
    let result = chart.render_incremental();
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn test_kline_data_from_json() {
    let json = r#"{"dates":["2024-01-01"],"opens":[100.0],"highs":[105.0],"lows":[98.0],"closes":[103.0],"volumes":[1000.0]}"#;
    let data = KlineData::from_json(json).unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data.closes()[0], 103.0);
}

#[test]
fn test_kline_data_from_csv() {
    let csv = "date,open,high,low,close,volume\n2024-01-01,100.0,105.0,98.0,103.0,1000.0\n2024-01-02,103.0,108.0,101.0,107.0,1200.0";
    let data = KlineData::from_csv(csv).unwrap();
    assert_eq!(data.len(), 2);
    let config = ChartConfigBuilder::new().build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    let svg = chart.to_svg_string().unwrap();
    assert!(svg.starts_with("<svg"));
}

#[cfg(feature = "html")]
#[test]
fn test_html_output() {
    let data = make_test_data(20);
    let config = ChartConfigBuilder::new().with_title("HTML Test").build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    let html = chart.to_html_string().unwrap();
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<html>"));
    assert!(html.contains("</html>"));
    assert!(html.contains("<svg"));
    assert!(html.contains("</svg>"));
    assert!(html.contains("<style>"));
    assert!(html.contains("<script>"));
    assert!(html.contains("crosshair"));
    assert!(html.contains("tooltip"));
}

#[cfg(feature = "html")]
#[test]
fn test_html_save() {
    let data = make_test_data(20);
    let config = ChartConfigBuilder::new().build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    let path = std::env::temp_dir().join("fta_integration_test.html");
    chart.save_as_html(path.to_str().unwrap()).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<!DOCTYPE html>"));
    let _ = std::fs::remove_file(&path);
}

#[cfg(not(feature = "html"))]
#[test]
fn test_html_not_available() {
    let data = make_test_data(20);
    let config = ChartConfigBuilder::new().build();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &[]).unwrap();
    let result = chart.save_as_html("test.html");
    assert!(result.is_err());
}
