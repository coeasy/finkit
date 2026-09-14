use finkit::chan::ChanConfig;
use finkit_visualization::chart::{EventMarker, KlineChart};
use finkit_visualization::config::{ChartConfigBuilder, ChartType, IndicatorConfig, IndicatorType};
use finkit_visualization::data::KlineData;
use finkit_visualization::language::Language;
use finkit_visualization::viewport::{LodLevel, LodPolicy, Viewport};

fn main() {
    let mut data = KlineData::new(
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let mut price = 100.0;
    for index in 0..180 {
        let change = (index as f64 * 0.21).sin() * 2.0 + (index % 9) as f64 * 0.08 - 0.3;
        let open = price;
        let close = price + change;
        let high = open.max(close) + 0.8;
        let low = open.min(close) - 0.8;
        data.push(
            format!("2024-{:03}", index + 1),
            open,
            high,
            low,
            close,
            1_000.0 + (index as f64 * 0.17).sin().abs() * 800.0,
        );
        price = close;
    }

    let mut config = ChartConfigBuilder::new()
        .with_title("Finkit LOD + Chan 交互示例")
        .with_language(Language::ZhCn)
        .with_chart_type(ChartType::Candlestick)
        .with_dimensions(1200, 600)
        .show_volume(true)
        .show_chan(true)
        .build();
    config.chan.min_stroke_bars = 2;
    config.chan.show_labels = true;
    config.chan.label_min_pixel_gap = 18.0;

    let mut chart = KlineChart::new(config);
    chart.set_data(data.clone());
    chart.add_event_marker(
        EventMarker::new(150, "突破候选")
            .with_value(data.highs[150])
            .with_color(finkit_visualization::primitive::Color::from_hex("#f59e0b")),
    );
    chart.set_viewport(Viewport::latest(60).with_pixels(80, 600).with_overscan(4));
    chart.set_lod_policy(LodPolicy::Fixed(LodLevel::Overview));
    chart
        .analyze_and_set_chan(ChanConfig {
            min_stroke_bars: 2,
            ..ChanConfig::default()
        })
        .expect("Chan analysis should succeed");
    chart
        .build_draw_list(
            &data,
            &[IndicatorConfig::new(IndicatorType::MA, vec![5.0, 20.0])],
        )
        .expect("chart should render");

    chart
        .save_as_svg("improved_chart.svg")
        .expect("SVG should save");
    chart
        .save_as_canvas_html("improved_chart_canvas.html")
        .expect("Canvas HTML should save");
    #[cfg(feature = "html")]
    chart
        .save_as_html("improved_chart.html")
        .expect("HTML should save");
    #[cfg(feature = "html")]
    chart
        .save_as_webgpu_html("improved_chart_webgpu.html")
        .expect("WebGPU/WebGL HTML should save");
    std::fs::write(
        "improved_chart.json",
        chart.to_json_string().expect("JSON scene should serialize"),
    )
    .expect("JSON should save");
    println!(
        "improved_chart.svg generated; rendered bars: {}",
        chart.scene().hit_regions.len()
    );
}
