use crate::config::ChartConfig;
use crate::data::KlineData;
use crate::geometry::Point;
use crate::layout::ChartLayout;
use crate::primitive::{Color, DrawList, Primitive, Style};

pub fn render_line(
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

    let line_color = Color::from_hex(config.color_scheme.up_color());

    let mut points = Vec::with_capacity(n);
    for i in 0..n {
        let x = plot_area.x + i as f64 * bar_width + bar_width / 2.0;
        let y = y_scale.data_to_pixel(data.closes[i]);
        points.push(Point::new(x, y));
    }

    let style = Style::new()
        .with_stroke(line_color)
        .with_line_width(1.5)
        .with_fill(Color::TRANSPARENT);

    draw_list.push(Primitive::Path {
        points,
        style,
        close: false,
    });
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
    fn test_line_draw_list_not_empty() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_line(&mut draw_list, &data, &layout, &config);
        assert!(!draw_list.is_empty());
    }

    #[test]
    fn test_line_single_path_primitive() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().build();
        let layout = make_layout(&data, &config);
        let mut draw_list = DrawList::new();
        render_line(&mut draw_list, &data, &layout, &config);
        assert_eq!(draw_list.len(), 1);
        if let Primitive::Path { points, close, .. } = &draw_list.primitives[0] {
            assert_eq!(points.len(), 3);
            assert!(!close);
        } else {
            panic!("Expected Path primitive");
        }
    }

    #[test]
    fn test_line_empty_data() {
        let data = KlineData::new(vec![], vec![], vec![], vec![], vec![], vec![]);
        let config = ChartConfigBuilder::new().build();
        let layout = LayoutCalculator::calculate(&data, &config, 1);
        let mut draw_list = DrawList::new();
        render_line(&mut draw_list, &data, &layout, &config);
        assert!(draw_list.is_empty());
    }
}
