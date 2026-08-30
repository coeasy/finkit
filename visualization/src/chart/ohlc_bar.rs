use crate::config::ChartConfig;
use crate::data::KlineData;
use crate::geometry::Point;
use crate::layout::ChartLayout;
use crate::primitive::{Color, DrawList, Primitive, Style};

pub fn render_ohlc_bar(
    draw_list: &mut DrawList,
    data: &KlineData,
    layout: &ChartLayout,
    config: &ChartConfig,
) {
    let n = data.len();
    if n == 0 {
        return;
    }

    let plot_area = &layout.main_panel.plot_area;
    let y_scale = &layout.main_panel.y_scale;

    let plot_width = plot_area.width;
    let bar_width = plot_width / n as f64;
    let tick_len = (bar_width / 3.0).max(2.0);

    let up_color = Color::from_hex(config.color_scheme.up_color());
    let down_color = Color::from_hex(config.color_scheme.down_color());

    for i in 0..n {
        let open = data.opens[i];
        let high = data.highs[i];
        let low = data.lows[i];
        let close = data.closes[i];

        let is_up = close >= open;
        let color = if is_up { up_color } else { down_color };

        let x_center = plot_area.x + i as f64 * bar_width + bar_width / 2.0;

        let y_high = y_scale.data_to_pixel(high);
        let y_low = y_scale.data_to_pixel(low);
        let y_open = y_scale.data_to_pixel(open);
        let y_close = y_scale.data_to_pixel(close);

        let line_style = Style::new()
            .with_stroke(color)
            .with_line_width(1.0)
            .with_fill(Color::TRANSPARENT);

        draw_list.push(Primitive::Line {
            p1: Point::new(x_center, y_high),
            p2: Point::new(x_center, y_low),
            style: line_style.clone(),
        });

        draw_list.push(Primitive::Line {
            p1: Point::new(x_center - tick_len, y_open),
            p2: Point::new(x_center, y_open),
            style: line_style.clone(),
        });

        draw_list.push(Primitive::Line {
            p1: Point::new(x_center, y_close),
            p2: Point::new(x_center + tick_len, y_close),
            style: line_style,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ChartConfigBuilder;
    use crate::layout::LayoutCalculator;

    fn make_test_data() -> KlineData {
        KlineData::new(
            vec![
                "2024-01-02".to_string(),
                "2024-01-03".to_string(),
                "2024-01-04".to_string(),
            ],
            vec![100.0, 102.0, 101.0],
            vec![105.0, 106.0, 104.0],
            vec![98.0, 100.0, 99.0],
            vec![103.0, 104.0, 100.0],
            vec![1000.0, 1200.0, 800.0],
        )
    }

    fn make_layout(data: &KlineData, config: &ChartConfig) -> ChartLayout {
        LayoutCalculator::calculate(data, config, 1)
    }

    #[test]
    fn test_ohlc_bar_draw_list_not_empty() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_ohlc_bar(&mut draw_list, &data, &layout, &config);
        assert!(!draw_list.is_empty());
    }

    #[test]
    fn test_ohlc_bar_primitives_per_bar() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_ohlc_bar(&mut draw_list, &data, &layout, &config);
        assert_eq!(draw_list.len(), 9);
    }

    #[test]
    fn test_ohlc_bar_empty_data() {
        let data = KlineData::new(vec![], vec![], vec![], vec![], vec![], vec![]);
        let config = ChartConfigBuilder::new().build();
        let layout = LayoutCalculator::calculate(&data, &config, 1);
        let mut draw_list = DrawList::new();
        render_ohlc_bar(&mut draw_list, &data, &layout, &config);
        assert!(draw_list.is_empty());
    }
}
