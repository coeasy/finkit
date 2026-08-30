use crate::data::KlineData;
use crate::language::LanguageResource;
use crate::layout::ChartLayout;

#[derive(Debug, Clone, PartialEq)]
pub struct CrosshairInfo {
    pub index: usize,
    pub x: f64,
    pub y: f64,
    pub ohlcv: (f64, f64, f64, f64, f64),
}

pub fn find_nearest_kline(cursor_x: f64, layout: &ChartLayout, data_len: usize) -> usize {
    if data_len == 0 {
        return 0;
    }
    let plot_area = &layout.main_panel.plot_area;
    let plot_width = plot_area.width;
    if plot_width <= 0.0 || data_len == 0 {
        return 0;
    }
    let bar_width = plot_width / data_len as f64;
    let relative_x = cursor_x - plot_area.x;
    let index = if bar_width > 0.0 {
        (relative_x / bar_width).floor() as usize
    } else {
        0
    };
    index.min(data_len - 1)
}

pub fn format_tooltip(index: usize, data: &KlineData, resource: &LanguageResource) -> String {
    if index >= data.len() {
        return resource.no_data.to_string();
    }
    format!(
        "{}: {}\n{}: {:.2}\n{}: {:.2}\n{}: {:.2}\n{}: {:.2}\n{}: {:.2}",
        resource.tooltip_date,
        data.dates[index],
        resource.tooltip_open,
        data.opens[index],
        resource.tooltip_high,
        data.highs[index],
        resource.tooltip_low,
        data.lows[index],
        resource.tooltip_close,
        data.closes[index],
        resource.tooltip_volume,
        data.volumes[index],
    )
}

pub fn create_crosshair_info(
    index: usize,
    cursor_x: f64,
    cursor_y: f64,
    data: &KlineData,
) -> Option<CrosshairInfo> {
    if index >= data.len() {
        return None;
    }
    Some(CrosshairInfo {
        index,
        x: cursor_x,
        y: cursor_y,
        ohlcv: (
            data.opens[index],
            data.highs[index],
            data.lows[index],
            data.closes[index],
            data.volumes[index],
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ChartConfigBuilder;

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

    fn make_test_layout() -> ChartLayout {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().with_dimensions(1200, 800).build();
        crate::layout::LayoutCalculator::calculate(&data, &config, 0)
    }

    #[test]
    fn test_find_nearest_kline_first() {
        let layout = make_test_layout();
        let plot_x = layout.main_panel.plot_area.x;
        let index = find_nearest_kline(plot_x + 1.0, &layout, 5);
        assert_eq!(index, 0);
    }

    #[test]
    fn test_find_nearest_kline_last() {
        let layout = make_test_layout();
        let plot_right = layout.main_panel.plot_area.right();
        let index = find_nearest_kline(plot_right - 1.0, &layout, 5);
        assert_eq!(index, 4);
    }

    #[test]
    fn test_find_nearest_kline_middle() {
        let layout = make_test_layout();
        let plot_x = layout.main_panel.plot_area.x;
        let plot_w = layout.main_panel.plot_area.width;
        let index = find_nearest_kline(plot_x + plot_w * 0.5, &layout, 5);
        assert!(index <= 3);
    }

    #[test]
    fn test_find_nearest_kline_zero_data() {
        let layout = make_test_layout();
        let index = find_nearest_kline(100.0, &layout, 0);
        assert_eq!(index, 0);
    }

    #[test]
    fn test_find_nearest_kline_before_plot() {
        let layout = make_test_layout();
        let index = find_nearest_kline(0.0, &layout, 5);
        assert_eq!(index, 0);
    }

    #[test]
    fn test_find_nearest_kline_after_plot() {
        let layout = make_test_layout();
        let index = find_nearest_kline(2000.0, &layout, 5);
        assert_eq!(index, 4);
    }

    #[test]
    fn test_format_tooltip_zh() {
        use crate::language::ZH_CN_RESOURCE;
        let data = make_test_data();
        let result = format_tooltip(0, &data, &ZH_CN_RESOURCE);
        assert!(result.contains("日期"));
        assert!(result.contains("开盘"));
        assert!(result.contains("2024-01-02"));
        assert!(result.contains("100.00"));
    }

    #[test]
    fn test_format_tooltip_en() {
        use crate::language::EN_US_RESOURCE;
        let data = make_test_data();
        let result = format_tooltip(0, &data, &EN_US_RESOURCE);
        assert!(result.contains("Date"));
        assert!(result.contains("Open"));
        assert!(result.contains("2024-01-02"));
    }

    #[test]
    fn test_format_tooltip_out_of_bounds() {
        use crate::language::ZH_CN_RESOURCE;
        let data = make_test_data();
        let result = format_tooltip(100, &data, &ZH_CN_RESOURCE);
        assert_eq!(result, ZH_CN_RESOURCE.no_data);
    }

    #[test]
    fn test_create_crosshair_info_valid() {
        let data = make_test_data();
        let info = create_crosshair_info(0, 100.0, 200.0, &data);
        assert!(info.is_some());
        let info = info.expect("alpha-ta-visualization: unexpected None/Err in visualization/src/interaction/crosshair.rs (A5 governance)");
        assert_eq!(info.index, 0);
        assert!((info.x - 100.0).abs() < 1e-10);
        assert!((info.y - 200.0).abs() < 1e-10);
        assert!((info.ohlcv.0 - 100.0).abs() < 1e-10);
        assert!((info.ohlcv.1 - 105.0).abs() < 1e-10);
        assert!((info.ohlcv.2 - 98.0).abs() < 1e-10);
        assert!((info.ohlcv.3 - 103.0).abs() < 1e-10);
        assert!((info.ohlcv.4 - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_create_crosshair_info_out_of_bounds() {
        let data = make_test_data();
        let info = create_crosshair_info(100, 100.0, 200.0, &data);
        assert!(info.is_none());
    }

    #[test]
    fn test_crosshair_info_fields() {
        let data = make_test_data();
        let info = create_crosshair_info(2, 50.0, 150.0, &data).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/interaction/crosshair.rs (A5 governance)");
        assert_eq!(info.index, 2);
        assert!((info.ohlcv.0 - 101.0).abs() < 1e-10);
        assert!((info.ohlcv.3 - 100.0).abs() < 1e-10);
    }
}
