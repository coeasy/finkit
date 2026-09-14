use finkit_visualization::chart::KlineChart;
use finkit_visualization::config::{ChartConfigBuilder, ChartType};
use finkit_visualization::data::KlineData;
use finkit_visualization::viewport::{LodLevel, LodPolicy};

fn main() {
    let count = 20_000;
    let mut data = KlineData::new(
        Vec::with_capacity(count),
        Vec::with_capacity(count),
        Vec::with_capacity(count),
        Vec::with_capacity(count),
        Vec::with_capacity(count),
        Vec::with_capacity(count),
    );
    let mut price = 100.0;
    for index in 0..count {
        let change = (index as f64 * 0.013).sin() * 1.8 + (index % 17) as f64 * 0.02 - 0.16;
        let open = price;
        let close = price + change;
        let high = open.max(close) + 0.4;
        let low = open.min(close) - 0.4;
        data.push(
            format!("2024-{index:05}"),
            open,
            high,
            low,
            close,
            1_000.0 + (index as f64 * 0.031).cos().abs() * 2_000.0,
        );
        price = close;
    }

    let config = ChartConfigBuilder::new()
        .with_title("Finkit WebGPU/WebGL2 大数据 LOD 示例")
        .with_chart_type(ChartType::Candlestick)
        .with_dimensions(1200, 600)
        .show_volume(true)
        .build();
    let mut chart = KlineChart::new(config);
    chart.set_lod_policy(LodPolicy::Fixed(LodLevel::Raw));
    chart.set_data(data.clone());
    chart
        .build_draw_list(&data, &[])
        .expect("large GPU chart should render");
    chart
        .save_as_webgpu_html("gpu_large_chart.html")
        .expect("GPU HTML should save");
    println!("generated gpu_large_chart.html with {count} source bars");
}
