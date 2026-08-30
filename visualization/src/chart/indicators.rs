use crate::config::{ChartConfig, IndicatorConfig};
use crate::data::KlineData;
use crate::geometry::{Point, Rect, Scale};
use crate::layout::ChartLayout;
use crate::primitive::{Color, DrawList, LineStyle, Primitive, Style};
use alpha_ta_core::indicators;

fn data_len_from_layout(layout: &ChartLayout) -> usize {
    let max_tick = layout.x_axis.max;
    if max_tick >= 0.0 {
        (max_tick as usize) + 1
    } else {
        0
    }
}

#[allow(clippy::needless_range_loop)]
fn draw_indicator_line(
    draw_list: &mut DrawList,
    values: &[f64],
    plot_area: &Rect,
    y_scale: &Scale,
    bar_width: f64,
    style: Style,
) {
    let n = values.len();
    if n == 0 {
        return;
    }

    let mut segment: Vec<Point> = Vec::new();

    for i in 0..n {
        let v = values[i];
        if v.is_nan() {
            if segment.len() >= 2 {
                draw_list.push(Primitive::Path {
                    points: segment.clone(),
                    style: style.clone(),
                    close: false,
                });
            }
            segment.clear();
            continue;
        }

        let x = plot_area.x + i as f64 * bar_width + bar_width / 2.0;
        let y = y_scale.data_to_pixel(v);
        segment.push(Point::new(x, y));
    }

    if segment.len() >= 2 {
        draw_list.push(Primitive::Path {
            points: segment,
            style,
            close: false,
        });
    }
}

pub fn render_ma(
    draw_list: &mut DrawList,
    data: &KlineData,
    layout: &ChartLayout,
    _config: &ChartConfig,
    periods: &[usize],
) {
    let n = data.len();
    if n == 0 {
        return;
    }

    let plot_area = &layout.main_panel.plot_area;
    let y_scale = &layout.main_panel.y_scale;
    let bar_width = plot_area.width / n as f64;

    let ma_colors = ["#f39c12", "#e74c3c", "#3498db", "#9b59b6"];

    for (idx, &period) in periods.iter().enumerate() {
        if period == 0 || period > n {
            continue;
        }

        let ma_values = match indicators::sma(&data.closes, period) {
            Ok(v) => v.to_vec(),
            Err(_) => continue,
        };

        let color_hex = ma_colors[idx % ma_colors.len()];
        let style = Style::new()
            .with_stroke(Color::from_hex(color_hex))
            .with_line_width(1.5)
            .with_fill(Color::TRANSPARENT);

        draw_indicator_line(draw_list, &ma_values, plot_area, y_scale, bar_width, style);
    }
}

pub fn render_ema(
    draw_list: &mut DrawList,
    data: &KlineData,
    layout: &ChartLayout,
    _config: &ChartConfig,
    periods: &[usize],
) {
    let n = data.len();
    if n == 0 {
        return;
    }

    let plot_area = &layout.main_panel.plot_area;
    let y_scale = &layout.main_panel.y_scale;
    let bar_width = plot_area.width / n as f64;

    let ema_colors = ["#f39c12", "#e74c3c", "#3498db", "#9b59b6"];

    for (idx, &period) in periods.iter().enumerate() {
        if period == 0 || period > n {
            continue;
        }

        let ema_values = match indicators::ema(&data.closes, period) {
            Ok(v) => v.to_vec(),
            Err(_) => continue,
        };

        let color_hex = ema_colors[idx % ema_colors.len()];
        let style = Style::new()
            .with_stroke(Color::from_hex(color_hex))
            .with_line_width(1.5)
            .with_fill(Color::TRANSPARENT);

        draw_indicator_line(draw_list, &ema_values, plot_area, y_scale, bar_width, style);
    }
}

pub fn render_boll(
    draw_list: &mut DrawList,
    data: &KlineData,
    layout: &ChartLayout,
    _config: &ChartConfig,
    period: usize,
    nb_dev: f64,
) {
    let n = data.len();
    if n == 0 {
        return;
    }

    let result = match indicators::bbands(&data.closes, period, nb_dev, nb_dev) {
        Ok(r) => r,
        Err(_) => return,
    };

    let upper: Vec<f64> = result.upper.to_vec();
    let middle: Vec<f64> = result.middle.to_vec();
    let lower: Vec<f64> = result.lower.to_vec();

    let plot_area = &layout.main_panel.plot_area;
    let y_scale = &layout.main_panel.y_scale;
    let bar_width = plot_area.width / n as f64;

    let mut fill_points: Vec<Point> = Vec::with_capacity(n * 2);
    for i in 0..n {
        if !upper[i].is_nan() && !lower[i].is_nan() {
            let x = plot_area.x + i as f64 * bar_width + bar_width / 2.0;
            let y_upper = y_scale.data_to_pixel(upper[i]);
            fill_points.push(Point::new(x, y_upper));
        }
    }
    for i in (0..n).rev() {
        if !upper[i].is_nan() && !lower[i].is_nan() {
            let x = plot_area.x + i as f64 * bar_width + bar_width / 2.0;
            let y_lower = y_scale.data_to_pixel(lower[i]);
            fill_points.push(Point::new(x, y_lower));
        }
    }

    if fill_points.len() >= 3 {
        let fill_style = Style::new()
            .with_fill(Color::from_hex("#3498db").with_alpha(30))
            .with_stroke(Color::TRANSPARENT);
        draw_list.push(Primitive::Polygon {
            points: fill_points,
            style: fill_style,
        });
    }

    let upper_style = Style::new()
        .with_stroke(Color::from_hex("#3498db"))
        .with_line_width(1.0)
        .with_fill(Color::TRANSPARENT);
    draw_indicator_line(
        draw_list,
        &upper,
        plot_area,
        y_scale,
        bar_width,
        upper_style,
    );

    let middle_style = Style::new()
        .with_stroke(Color::from_hex("#3498db"))
        .with_line_width(1.5)
        .with_fill(Color::TRANSPARENT);
    draw_indicator_line(
        draw_list,
        &middle,
        plot_area,
        y_scale,
        bar_width,
        middle_style,
    );

    let lower_style = Style::new()
        .with_stroke(Color::from_hex("#3498db"))
        .with_line_width(1.0)
        .with_fill(Color::TRANSPARENT);
    draw_indicator_line(
        draw_list,
        &lower,
        plot_area,
        y_scale,
        bar_width,
        lower_style,
    );
}

#[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
pub fn render_macd(
    draw_list: &mut DrawList,
    data: &KlineData,
    layout: &ChartLayout,
    _config: &ChartConfig,
    fast: usize,
    slow: usize,
    signal: usize,
    panel_index: usize,
) {
    let n = data.len();
    if n == 0 || panel_index >= layout.sub_panels.len() {
        return;
    }

    let result = match indicators::macd(&data.closes, fast, slow, signal) {
        Ok(r) => r,
        Err(_) => return,
    };

    let dif: Vec<f64> = result.macd.to_vec();
    let dea: Vec<f64> = result.signal.to_vec();
    let hist: Vec<f64> = result.hist.to_vec();

    let sub_panel = &layout.sub_panels[panel_index];
    let plot_area = &sub_panel.plot_area;
    let y_scale = &sub_panel.y_scale;
    let bar_width = plot_area.width / n as f64;

    let dif_style = Style::new()
        .with_stroke(Color::from_hex("#f39c12"))
        .with_line_width(1.5)
        .with_fill(Color::TRANSPARENT);
    draw_indicator_line(draw_list, &dif, plot_area, y_scale, bar_width, dif_style);

    let dea_style = Style::new()
        .with_stroke(Color::from_hex("#3498db"))
        .with_line_width(1.5)
        .with_fill(Color::TRANSPARENT);
    draw_indicator_line(draw_list, &dea, plot_area, y_scale, bar_width, dea_style);

    let hist_bar_width = (bar_width * 0.7).max(1.0);
    let gap = (bar_width - hist_bar_width) / 2.0;
    let y_zero = y_scale.data_to_pixel(0.0);

    for i in 0..n {
        let v = hist[i];
        if v.is_nan() {
            continue;
        }

        let color = if v >= 0.0 {
            Color::from_hex("#ef4444")
        } else {
            Color::from_hex("#22c55e")
        };

        let y_val = y_scale.data_to_pixel(v);
        let (y_top, bar_height) = if v >= 0.0 {
            (y_val, (y_zero - y_val).max(0.0))
        } else {
            (y_zero, (y_val - y_zero).max(0.0))
        };

        let x = plot_area.x + i as f64 * bar_width + gap;
        draw_list.push(Primitive::FilledRect {
            rect: Rect::new(x, y_top, hist_bar_width, bar_height),
            fill: color,
            stroke: None,
        });
    }
}

pub fn render_rsi(
    draw_list: &mut DrawList,
    data: &KlineData,
    layout: &ChartLayout,
    _config: &ChartConfig,
    period: usize,
    panel_index: usize,
) {
    let n = data.len();
    if n == 0 || panel_index >= layout.sub_panels.len() {
        return;
    }

    let rsi_values = match indicators::rsi(&data.closes, period) {
        Ok(v) => v.to_vec(),
        Err(_) => return,
    };

    let sub_panel = &layout.sub_panels[panel_index];
    let plot_area = &sub_panel.plot_area;
    let y_scale = &sub_panel.y_scale;
    let bar_width = plot_area.width / n as f64;

    let rsi_style = Style::new()
        .with_stroke(Color::from_hex("#9b59b6"))
        .with_line_width(1.5)
        .with_fill(Color::TRANSPARENT);
    draw_indicator_line(
        draw_list,
        &rsi_values,
        plot_area,
        y_scale,
        bar_width,
        rsi_style,
    );

    let overbought = 70.0;
    let oversold = 30.0;

    let line_style = Style::new()
        .with_stroke(Color::from_hex("#888888"))
        .with_line_width(0.5)
        .with_line_style(LineStyle::Dashed)
        .with_fill(Color::TRANSPARENT);

    let y_ob = y_scale.data_to_pixel(overbought);
    if y_ob >= plot_area.y && y_ob <= plot_area.bottom() {
        draw_list.push(Primitive::Line {
            p1: Point::new(plot_area.x, y_ob),
            p2: Point::new(plot_area.right(), y_ob),
            style: line_style.clone(),
        });
    }

    let y_os = y_scale.data_to_pixel(oversold);
    if y_os >= plot_area.y && y_os <= plot_area.bottom() {
        draw_list.push(Primitive::Line {
            p1: Point::new(plot_area.x, y_os),
            p2: Point::new(plot_area.right(), y_os),
            style: line_style,
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_kdj(
    draw_list: &mut DrawList,
    data: &KlineData,
    layout: &ChartLayout,
    _config: &ChartConfig,
    fast_k: usize,
    slow_k: usize,
    slow_d: usize,
    panel_index: usize,
) {
    let n = data.len();
    if n == 0 || panel_index >= layout.sub_panels.len() {
        return;
    }

    let result = match indicators::stoch(
        &data.highs,
        &data.lows,
        &data.closes,
        fast_k,
        slow_k,
        slow_d,
    ) {
        Ok(r) => r,
        Err(_) => return,
    };

    let k_values: Vec<f64> = result.k.to_vec();
    let d_values: Vec<f64> = result.d.to_vec();

    let j_values: Vec<f64> = k_values
        .iter()
        .zip(d_values.iter())
        .map(|(&k, &d)| {
            if k.is_nan() || d.is_nan() {
                f64::NAN
            } else {
                3.0 * k - 2.0 * d
            }
        })
        .collect();

    let sub_panel = &layout.sub_panels[panel_index];
    let plot_area = &sub_panel.plot_area;
    let y_scale = &sub_panel.y_scale;
    let bar_width = plot_area.width / n as f64;

    let k_style = Style::new()
        .with_stroke(Color::from_hex("#f39c12"))
        .with_line_width(1.5)
        .with_fill(Color::TRANSPARENT);
    draw_indicator_line(draw_list, &k_values, plot_area, y_scale, bar_width, k_style);

    let d_style = Style::new()
        .with_stroke(Color::from_hex("#3498db"))
        .with_line_width(1.5)
        .with_fill(Color::TRANSPARENT);
    draw_indicator_line(draw_list, &d_values, plot_area, y_scale, bar_width, d_style);

    let j_style = Style::new()
        .with_stroke(Color::from_hex("#9b59b6"))
        .with_line_width(1.5)
        .with_fill(Color::TRANSPARENT);
    draw_indicator_line(draw_list, &j_values, plot_area, y_scale, bar_width, j_style);
}

#[allow(clippy::needless_range_loop)]
pub fn render_sar(
    draw_list: &mut DrawList,
    data: &KlineData,
    layout: &ChartLayout,
    _config: &ChartConfig,
    acceleration: f64,
    maximum: f64,
) {
    let n = data.len();
    if n == 0 {
        return;
    }

    let result = match indicators::sar(&data.highs, &data.lows, acceleration, maximum) {
        Ok(r) => r,
        Err(_) => return,
    };

    let sar_values: Vec<f64> = result.sar.to_vec();

    let plot_area = &layout.main_panel.plot_area;
    let y_scale = &layout.main_panel.y_scale;
    let bar_width = plot_area.width / n as f64;

    let style = Style::new()
        .with_fill(Color::from_hex("#e74c3c"))
        .with_stroke(Color::from_hex("#e74c3c"));

    for i in 0..n {
        let v = sar_values[i];
        if v.is_nan() {
            continue;
        }

        let x = plot_area.x + i as f64 * bar_width + bar_width / 2.0;
        let y = y_scale.data_to_pixel(v);

        draw_list.push(Primitive::Circle {
            center: Point::new(x, y),
            radius: 2.0,
            style: style.clone(),
        });
    }
}

pub fn render_grid(draw_list: &mut DrawList, layout: &ChartLayout, config: &ChartConfig) {
    let plot_area = &layout.main_panel.plot_area;
    let grid_color = Color::from_hex(&config.theme_config.grid_color);

    let h_style = Style::new()
        .with_stroke(grid_color)
        .with_line_width(0.5)
        .with_line_style(LineStyle::Dashed)
        .with_fill(Color::TRANSPARENT);

    if let Some(y_axis) = layout.y_axes.first() {
        for &tick in &y_axis.ticks {
            let y = layout.main_panel.y_scale.data_to_pixel(tick);
            if y >= plot_area.y && y <= plot_area.bottom() {
                draw_list.push(Primitive::Line {
                    p1: Point::new(plot_area.x, y),
                    p2: Point::new(plot_area.right(), y),
                    style: h_style.clone(),
                });
            }
        }
    }

    let v_style = Style::new()
        .with_stroke(grid_color)
        .with_line_width(0.5)
        .with_line_style(LineStyle::Dashed)
        .with_fill(Color::TRANSPARENT);

    let n = layout.x_axis.ticks.len();
    if n > 0 {
        let data_len = data_len_from_layout(layout);
        let bar_width = if data_len > 0 {
            plot_area.width / data_len as f64
        } else {
            plot_area.width
        };

        for &tick in &layout.x_axis.ticks {
            let idx = tick as usize;
            let x = plot_area.x + idx as f64 * bar_width + bar_width / 2.0;
            if x >= plot_area.x && x <= plot_area.right() {
                draw_list.push(Primitive::Line {
                    p1: Point::new(x, plot_area.y),
                    p2: Point::new(x, plot_area.bottom()),
                    style: v_style.clone(),
                });
            }
        }
    }
}

pub fn render_axes(draw_list: &mut DrawList, layout: &ChartLayout, config: &ChartConfig) {
    let font_color = Color::from_hex(&config.theme_config.font_color);
    let axis_color = Color::from_hex(&config.theme_config.axis_line_color);
    let plot_area = &layout.main_panel.plot_area;

    let axis_style = Style::new()
        .with_stroke(axis_color)
        .with_line_width(1.0)
        .with_fill(Color::TRANSPARENT);

    draw_list.push(Primitive::Line {
        p1: Point::new(plot_area.x, plot_area.y),
        p2: Point::new(plot_area.x, plot_area.bottom()),
        style: axis_style.clone(),
    });

    draw_list.push(Primitive::Line {
        p1: Point::new(plot_area.x, plot_area.bottom()),
        p2: Point::new(plot_area.right(), plot_area.bottom()),
        style: axis_style,
    });

    let label_style = Style::new()
        .with_fill(font_color)
        .with_stroke(font_color)
        .with_font_size(10.0);

    if let Some(y_axis) = layout.y_axes.first() {
        for (i, &tick) in y_axis.ticks.iter().enumerate() {
            let y = layout.main_panel.y_scale.data_to_pixel(tick);
            if y >= plot_area.y && y <= plot_area.bottom() {
                let label = &y_axis.labels[i];
                draw_list.push(Primitive::Text {
                    position: Point::new(plot_area.x - 5.0, y + 4.0),
                    content: label.clone(),
                    style: label_style.clone(),
                });
            }
        }
    }

    let data_len = data_len_from_layout(layout);
    let bar_width = if data_len > 0 {
        plot_area.width / data_len as f64
    } else {
        plot_area.width
    };

    for (i, &tick) in layout.x_axis.ticks.iter().enumerate() {
        let idx = tick as usize;
        let x = plot_area.x + idx as f64 * bar_width + bar_width / 2.0;
        if x >= plot_area.x && x <= plot_area.right() {
            let label = &layout.x_axis.labels[i];
            draw_list.push(Primitive::Text {
                position: Point::new(x - 15.0, plot_area.bottom() + 15.0),
                content: label.clone(),
                style: label_style.clone(),
            });
        }
    }
}

pub fn render_title(draw_list: &mut DrawList, layout: &ChartLayout, config: &ChartConfig) {
    if config.title.is_empty() {
        return;
    }

    let font_color = Color::from_hex(&config.theme_config.font_color);
    let title_style = Style::new()
        .with_fill(font_color)
        .with_stroke(font_color)
        .with_font_size(16.0);

    let position = if let Some(title_rect) = layout.title_rect {
        Point::new(title_rect.x, title_rect.y + title_rect.height * 0.7)
    } else {
        Point::new(layout.main_panel.plot_area.x, 25.0)
    };

    draw_list.push(Primitive::Text {
        position,
        content: config.title.clone(),
        style: title_style,
    });
}

pub fn render_legend(
    draw_list: &mut DrawList,
    layout: &ChartLayout,
    config: &ChartConfig,
    indicators: &[IndicatorConfig],
) {
    if !config.show_legend {
        return;
    }

    let font_color = Color::from_hex(&config.theme_config.font_color);
    let legend_style = Style::new()
        .with_fill(font_color)
        .with_stroke(font_color)
        .with_font_size(11.0);

    let legend_y = if let Some(legend_rect) = layout.legend_rect {
        legend_rect.y + legend_rect.height * 0.7
    } else {
        35.0
    };

    let start_x = layout.main_panel.plot_area.x;
    let mut x_offset = start_x;

    let resource = crate::language::LanguageResource::from_language(&config.language);
    let labels = [
        ("O", resource.legend_open),
        ("H", resource.legend_high),
        ("L", resource.legend_low),
        ("C", resource.legend_close),
        ("V", resource.legend_volume),
    ];

    for (prefix, label) in &labels {
        let text = format!("{}:{}", prefix, label);
        let text_len = text.len();
        draw_list.push(Primitive::Text {
            position: Point::new(x_offset, legend_y),
            content: text,
            style: legend_style.clone(),
        });
        x_offset += text_len as f64 * 7.0 + 15.0;
    }

    for indicator in indicators {
        if !indicator.visible {
            continue;
        }
        let indicator_color = Color::from_hex(&indicator.color);
        let line_style = Style::new()
            .with_stroke(indicator_color)
            .with_line_width(indicator.line_width)
            .with_fill(Color::TRANSPARENT);

        draw_list.push(Primitive::Line {
            p1: Point::new(x_offset, legend_y - 3.0),
            p2: Point::new(x_offset + 15.0, legend_y - 3.0),
            style: line_style,
        });

        let param_str = indicator
            .params
            .iter()
            .map(|p| format!("{}", *p as usize))
            .collect::<Vec<_>>()
            .join(",");

        let text = format!("{}({})", indicator.name, param_str);
        let text_width = text.len() as f64 * 7.0;
        draw_list.push(Primitive::Text {
            position: Point::new(x_offset + 20.0, legend_y),
            content: text,
            style: legend_style.clone(),
        });
        x_offset += 20.0 + text_width + 15.0;
    }
}

pub fn render_volume(
    draw_list: &mut DrawList,
    data: &KlineData,
    layout: &ChartLayout,
    config: &ChartConfig,
) {
    if !config.show_volume || layout.sub_panels.is_empty() {
        return;
    }

    let n = data.len();
    if n == 0 {
        return;
    }

    let sub_panel = &layout.sub_panels[0];
    let plot_area = &sub_panel.plot_area;
    let y_scale = &sub_panel.y_scale;

    let plot_width = plot_area.width;
    let bar_width = plot_width / n as f64;
    let vol_bar_width = (bar_width * 0.7).max(1.0);
    let gap = (bar_width - vol_bar_width) / 2.0;

    let up_color = Color::from_hex(config.color_scheme.up_color()).with_alpha(180);
    let down_color = Color::from_hex(config.color_scheme.down_color()).with_alpha(180);

    for i in 0..n {
        let close = data.closes[i];
        let open = data.opens[i];
        let volume = data.volumes[i];

        let is_up = close >= open;
        let color = if is_up { up_color } else { down_color };

        let y_top = y_scale.data_to_pixel(volume);
        let y_bottom = y_scale.data_to_pixel(0.0);
        let bar_height = (y_bottom - y_top).max(0.0);

        let x = plot_area.x + i as f64 * bar_width + gap;

        draw_list.push(Primitive::FilledRect {
            rect: crate::geometry::Rect::new(x, y_top, vol_bar_width, bar_height),
            fill: color,
            stroke: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChartConfigBuilder, IndicatorType};
    use crate::layout::LayoutCalculator;

    fn make_test_data() -> KlineData {
        KlineData::new(
            vec![
                "2024-01-02".to_string(),
                "2024-01-03".to_string(),
                "2024-01-04".to_string(),
                "2024-01-05".to_string(),
                "2024-01-08".to_string(),
            ],
            vec![100.0, 102.0, 101.0, 103.0, 105.0],
            vec![105.0, 106.0, 104.0, 107.0, 108.0],
            vec![98.0, 100.0, 99.0, 101.0, 103.0],
            vec![103.0, 104.0, 100.0, 105.0, 107.0],
            vec![1000.0, 1200.0, 800.0, 1500.0, 2000.0],
        )
    }

    fn make_large_test_data() -> KlineData {
        let n = 40;
        let dates: Vec<String> = (0..n).map(|i| format!("2024-01-{:02}", i + 1)).collect();
        let closes: Vec<f64> = (0..n)
            .map(|i| 100.0 + (i as f64 * 0.5).sin() * 10.0 + i as f64 * 0.3)
            .collect();
        let opens: Vec<f64> = closes.iter().map(|&c| c - 1.0).collect();
        let highs: Vec<f64> = closes.iter().map(|&c| c + 2.0).collect();
        let lows: Vec<f64> = closes.iter().map(|&c| c - 2.0).collect();
        let volumes: Vec<f64> = (0..n).map(|i| 1000.0 + (i as f64 * 100.0)).collect();
        KlineData::new(dates, opens, highs, lows, closes, volumes)
    }

    fn make_layout(data: &KlineData, config: &ChartConfig) -> ChartLayout {
        LayoutCalculator::calculate(data, config, 1)
    }

    fn make_layout_with_sub(
        data: &KlineData,
        config: &ChartConfig,
        sub_count: usize,
    ) -> ChartLayout {
        LayoutCalculator::calculate(data, config, sub_count)
    }

    #[test]
    fn test_grid_draw_list_not_empty() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_grid(&mut draw_list, &layout, &config);
        assert!(!draw_list.is_empty());
    }

    #[test]
    fn test_axes_draw_list_not_empty() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_axes(&mut draw_list, &layout, &config);
        assert!(!draw_list.is_empty());
    }

    #[test]
    fn test_title_draw_list_not_empty() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().with_title("Test Title").build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_title(&mut draw_list, &layout, &config);
        assert!(!draw_list.is_empty());
    }

    #[test]
    fn test_title_empty_skipped() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_title(&mut draw_list, &layout, &config);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_legend_draw_list_not_empty() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().show_legend(true).build();
        let layout = make_layout(&data, &config);
        let indicators = vec![IndicatorConfig::new(IndicatorType::MA, vec![5.0])];
        let mut draw_list = DrawList::new();
        render_legend(&mut draw_list, &layout, &config, &indicators);
        assert!(!draw_list.is_empty());
    }

    #[test]
    fn test_legend_hidden() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().show_legend(false).build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_legend(&mut draw_list, &layout, &config, &[]);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_volume_draw_list_not_empty() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().show_volume(true).build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_volume(&mut draw_list, &data, &layout, &config);
        assert!(!draw_list.is_empty());
    }

    #[test]
    fn test_volume_hidden() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().show_volume(false).build();
        let layout = LayoutCalculator::calculate(&data, &config, 0);
        let mut draw_list = DrawList::new();
        render_volume(&mut draw_list, &data, &layout, &config);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_volume_bar_count() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().show_volume(true).build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_volume(&mut draw_list, &data, &layout, &config);
        assert_eq!(draw_list.len(), 5);
    }

    #[test]
    fn test_render_ma() {
        let data = make_large_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_ma(&mut draw_list, &data, &layout, &config, &[5, 10]);
        assert!(!draw_list.is_empty());
        let path_count = draw_list
            .primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Path { .. }))
            .count();
        assert_eq!(path_count, 2);
    }

    #[test]
    fn test_render_ma_empty_data() {
        let data = KlineData::new(vec![], vec![], vec![], vec![], vec![], vec![]);
        let config = ChartConfigBuilder::new().build();
        let layout = LayoutCalculator::calculate(&data, &config, 0);
        let mut draw_list = DrawList::new();
        render_ma(&mut draw_list, &data, &layout, &config, &[5]);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_render_ma_period_exceeds_data() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_ma(&mut draw_list, &data, &layout, &config, &[100]);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_render_ema() {
        let data = make_large_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_ema(&mut draw_list, &data, &layout, &config, &[5, 10]);
        assert!(!draw_list.is_empty());
        let path_count = draw_list
            .primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Path { .. }))
            .count();
        assert_eq!(path_count, 2);
    }

    #[test]
    fn test_render_ema_empty_data() {
        let data = KlineData::new(vec![], vec![], vec![], vec![], vec![], vec![]);
        let config = ChartConfigBuilder::new().build();
        let layout = LayoutCalculator::calculate(&data, &config, 0);
        let mut draw_list = DrawList::new();
        render_ema(&mut draw_list, &data, &layout, &config, &[5]);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_render_boll() {
        let data = make_large_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_boll(&mut draw_list, &data, &layout, &config, 20, 2.0);
        assert!(!draw_list.is_empty());
        let polygon_count = draw_list
            .primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Polygon { .. }))
            .count();
        assert_eq!(polygon_count, 1);
        let path_count = draw_list
            .primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Path { .. }))
            .count();
        assert_eq!(path_count, 3);
    }

    #[test]
    fn test_render_boll_empty_data() {
        let data = KlineData::new(vec![], vec![], vec![], vec![], vec![], vec![]);
        let config = ChartConfigBuilder::new().build();
        let layout = LayoutCalculator::calculate(&data, &config, 0);
        let mut draw_list = DrawList::new();
        render_boll(&mut draw_list, &data, &layout, &config, 20, 2.0);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_render_macd() {
        let data = make_large_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout_with_sub(&data, &config, 2);
        let mut draw_list = DrawList::new();
        render_macd(&mut draw_list, &data, &layout, &config, 12, 26, 9, 1);
        assert!(!draw_list.is_empty());
        let path_count = draw_list
            .primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Path { .. }))
            .count();
        assert_eq!(path_count, 2);
        let rect_count = draw_list
            .primitives
            .iter()
            .filter(|p| matches!(p, Primitive::FilledRect { .. }))
            .count();
        assert!(rect_count > 0);
    }

    #[test]
    fn test_render_macd_invalid_panel() {
        let data = make_large_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_macd(&mut draw_list, &data, &layout, &config, 12, 26, 9, 5);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_render_rsi() {
        let data = make_large_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout_with_sub(&data, &config, 2);
        let mut draw_list = DrawList::new();
        render_rsi(&mut draw_list, &data, &layout, &config, 14, 1);
        assert!(!draw_list.is_empty());
        let path_count = draw_list
            .primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Path { .. }))
            .count();
        assert_eq!(path_count, 1);
        let line_count = draw_list
            .primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Line { .. }))
            .count();
        assert_eq!(line_count, 2);
    }

    #[test]
    fn test_render_rsi_invalid_panel() {
        let data = make_large_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_rsi(&mut draw_list, &data, &layout, &config, 14, 5);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_render_kdj() {
        let data = make_large_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout_with_sub(&data, &config, 2);
        let mut draw_list = DrawList::new();
        render_kdj(&mut draw_list, &data, &layout, &config, 9, 3, 3, 1);
        assert!(!draw_list.is_empty());
        let path_count = draw_list
            .primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Path { .. }))
            .count();
        assert_eq!(path_count, 3);
    }

    #[test]
    fn test_render_kdj_invalid_panel() {
        let data = make_large_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_kdj(&mut draw_list, &data, &layout, &config, 9, 3, 3, 5);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_render_sar() {
        let data = make_large_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_sar(&mut draw_list, &data, &layout, &config, 0.02, 0.2);
        assert!(!draw_list.is_empty());
        let circle_count = draw_list
            .primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Circle { .. }))
            .count();
        assert!(circle_count > 0);
    }

    #[test]
    fn test_render_sar_empty_data() {
        let data = KlineData::new(vec![], vec![], vec![], vec![], vec![], vec![]);
        let config = ChartConfigBuilder::new().build();
        let layout = LayoutCalculator::calculate(&data, &config, 0);
        let mut draw_list = DrawList::new();
        render_sar(&mut draw_list, &data, &layout, &config, 0.02, 0.2);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_draw_indicator_line_with_nan() {
        let values = vec![1.0, 2.0, f64::NAN, 4.0, 5.0];
        let plot_area = Rect::new(0.0, 0.0, 100.0, 100.0);
        let y_scale = Scale::linear_scale(0.0, 10.0, 100.0, 0.0);
        let style = Style::new()
            .with_stroke(Color::RED)
            .with_fill(Color::TRANSPARENT);
        let mut draw_list = DrawList::new();
        draw_indicator_line(&mut draw_list, &values, &plot_area, &y_scale, 20.0, style);
        assert_eq!(draw_list.len(), 2);
    }

    #[test]
    fn test_draw_indicator_line_all_nan() {
        let values = vec![f64::NAN, f64::NAN, f64::NAN];
        let plot_area = Rect::new(0.0, 0.0, 100.0, 100.0);
        let y_scale = Scale::linear_scale(0.0, 10.0, 100.0, 0.0);
        let style = Style::new()
            .with_stroke(Color::RED)
            .with_fill(Color::TRANSPARENT);
        let mut draw_list = DrawList::new();
        draw_indicator_line(&mut draw_list, &values, &plot_area, &y_scale, 20.0, style);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_draw_indicator_line_empty() {
        let values: Vec<f64> = vec![];
        let plot_area = Rect::new(0.0, 0.0, 100.0, 100.0);
        let y_scale = Scale::linear_scale(0.0, 10.0, 100.0, 0.0);
        let style = Style::new()
            .with_stroke(Color::RED)
            .with_fill(Color::TRANSPARENT);
        let mut draw_list = DrawList::new();
        draw_indicator_line(&mut draw_list, &values, &plot_area, &y_scale, 20.0, style);
        assert!(draw_list.is_empty());
    }

    #[test]
    fn test_draw_indicator_line_single_point() {
        let values = vec![5.0];
        let plot_area = Rect::new(0.0, 0.0, 100.0, 100.0);
        let y_scale = Scale::linear_scale(0.0, 10.0, 100.0, 0.0);
        let style = Style::new()
            .with_stroke(Color::RED)
            .with_fill(Color::TRANSPARENT);
        let mut draw_list = DrawList::new();
        draw_indicator_line(&mut draw_list, &values, &plot_area, &y_scale, 20.0, style);
        assert!(draw_list.is_empty());
    }
}
