use alpha_ta_visualization::chart::KlineChart;
use alpha_ta_visualization::config::{
    ChartConfigBuilder, ChartType, IndicatorConfig, IndicatorType,
};
use alpha_ta_visualization::data::KlineData;
use alpha_ta_visualization::language::Language;

fn main() {
    let mut data = KlineData::new(
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );

    let mut price = 100.0_f64;
    for i in 0..100 {
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

    let config = ChartConfigBuilder::new()
        .with_title("K线图示例")
        .with_language(Language::ZhCn)
        .with_chart_type(ChartType::Candlestick)
        .with_dimensions(1200, 600)
        .build();

    let indicators = vec![
        IndicatorConfig::new(IndicatorType::MA, vec![5.0, 10.0, 20.0]),
        IndicatorConfig::new(IndicatorType::MACD, vec![12.0, 26.0, 9.0]),
        IndicatorConfig::new(IndicatorType::RSI, vec![14.0]),
    ];

    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &indicators).unwrap();

    chart.save_as_svg("basic_chart.svg").unwrap();
    println!("SVG saved to: basic_chart.svg");

    let svg_str = chart.to_svg_string().unwrap();
    assert!(svg_str.starts_with("<svg"));
    assert!(svg_str.contains("</svg>"));
    println!("SVG validation passed, length: {}", svg_str.len());
}
