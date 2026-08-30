use std::time::Instant;

use alpha_ta_visualization::chart::KlineChart;
use alpha_ta_visualization::config::{
    ChartConfigBuilder, ChartType, DecimateStrategy, IndicatorConfig, IndicatorType,
};
use alpha_ta_visualization::data::KlineData;
use alpha_ta_visualization::decimate::decimate;

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

fn bench_svg_render(n: usize) -> std::time::Duration {
    let data = make_test_data(n);
    let config = ChartConfigBuilder::new()
        .with_chart_type(ChartType::Candlestick)
        .with_dimensions(1200, 600)
        .build();
    let indicators = vec![IndicatorConfig::new(
        IndicatorType::MA,
        vec![5.0, 10.0, 20.0],
    )];

    let start = Instant::now();
    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &indicators).unwrap();
    let _svg = chart.to_svg_string().unwrap();
    start.elapsed()
}

fn bench_decimate(n: usize, target: u32) -> std::time::Duration {
    let data = make_test_data(n);
    let start = Instant::now();
    let _result = decimate(&data, &DecimateStrategy::LTTB, target);
    start.elapsed()
}

#[test]
fn bench_1000_klines() {
    let duration = bench_svg_render(1000);
    println!(
        "1000 K线 SVG渲染时间: {:.2}ms",
        duration.as_secs_f64() * 1000.0
    );
    assert!(duration.as_secs() < 10, "1000 K线渲染应在10秒内完成");
}

#[test]
fn bench_10000_klines() {
    let duration = bench_svg_render(10000);
    println!(
        "10000 K线 SVG渲染时间: {:.2}ms",
        duration.as_secs_f64() * 1000.0
    );
    assert!(duration.as_secs() < 30, "10000 K线渲染应在30秒内完成");
}

#[test]
fn bench_100000_klines() {
    let duration = bench_svg_render(100000);
    println!(
        "100000 K线 SVG渲染时间: {:.2}ms",
        duration.as_secs_f64() * 1000.0
    );
    assert!(duration.as_secs() < 60, "100000 K线渲染应在60秒内完成");
}

#[test]
fn bench_decimate_1m_klines() {
    let duration = bench_decimate(1_000_000, 1200);
    println!(
        "1000000 K线降采样时间: {:.2}ms",
        duration.as_secs_f64() * 1000.0
    );
    assert!(duration.as_secs() < 30, "100万K线降采样应在30秒内完成");
}

#[test]
fn bench_summary() {
    let d1 = bench_svg_render(1000);
    let d2 = bench_svg_render(10000);
    let d3 = bench_svg_render(100000);
    let d4 = bench_decimate(1_000_000, 1200);

    println!("\n========== 性能基准测试结果 ==========");
    println!("1000 K线 SVG渲染:      {:.2}ms", d1.as_secs_f64() * 1000.0);
    println!("10000 K线 SVG渲染:     {:.2}ms", d2.as_secs_f64() * 1000.0);
    println!("100000 K线 SVG渲染:    {:.2}ms", d3.as_secs_f64() * 1000.0);
    println!("1000000 K线降采样:     {:.2}ms", d4.as_secs_f64() * 1000.0);
    println!("=========================================\n");
}
