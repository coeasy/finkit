use crate::config::ChartConfig;
use crate::data::KlineData;
use crate::geometry::{Rect, Scale};

#[derive(Debug, Clone, PartialEq)]
pub struct PanelRect {
    pub plot_area: Rect,
    pub full_area: Rect,
    pub y_scale: Scale,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AxisInfo {
    pub ticks: Vec<f64>,
    pub labels: Vec<String>,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChartLayout {
    pub total_rect: Rect,
    pub main_panel: PanelRect,
    pub sub_panels: Vec<PanelRect>,
    pub x_axis: AxisInfo,
    pub y_axes: Vec<AxisInfo>,
    pub legend_rect: Option<Rect>,
    pub title_rect: Option<Rect>,
}

pub struct LayoutCalculator;

impl LayoutCalculator {
    const LEFT_MARGIN: f64 = 80.0;
    const RIGHT_MARGIN: f64 = 20.0;
    const TOP_MARGIN: f64 = 10.0;
    const BOTTOM_MARGIN: f64 = 30.0;
    const TITLE_HEIGHT: f64 = 30.0;
    const LEGEND_HEIGHT: f64 = 25.0;
    const PANEL_SPACING: f64 = 8.0;
    const SUB_PANEL_RATIO: f64 = 0.20;
    const TICK_COUNT: usize = 5;
    const X_TICK_COUNT: usize = 6;

    pub fn calculate(
        data: &KlineData,
        config: &ChartConfig,
        sub_panel_count: usize,
    ) -> ChartLayout {
        let sub_panel_count = sub_panel_count.min(3);
        let total_width = config.width as f64;
        let total_height = config.height as f64;
        let total_rect = Rect::new(0.0, 0.0, total_width, total_height);

        let has_title = !config.title.is_empty();
        let has_legend = config.show_legend;

        let title_height = if has_title { Self::TITLE_HEIGHT } else { 0.0 };
        let legend_height = if has_legend { Self::LEGEND_HEIGHT } else { 0.0 };

        let title_rect = if has_title {
            Some(Rect::new(
                Self::LEFT_MARGIN,
                Self::TOP_MARGIN,
                total_width - Self::LEFT_MARGIN - Self::RIGHT_MARGIN,
                title_height,
            ))
        } else {
            None
        };

        let legend_y = Self::TOP_MARGIN + title_height;
        let legend_rect = if has_legend {
            Some(Rect::new(
                Self::LEFT_MARGIN,
                legend_y,
                total_width - Self::LEFT_MARGIN - Self::RIGHT_MARGIN,
                legend_height,
            ))
        } else {
            None
        };

        let panels_top = Self::TOP_MARGIN + title_height + legend_height;
        let panels_bottom = total_height - Self::BOTTOM_MARGIN;
        let panels_height = (panels_bottom - panels_top).max(0.0);

        let total_spacing = sub_panel_count as f64 * Self::PANEL_SPACING;
        let available_height = (panels_height - total_spacing).max(0.0);
        let main_ratio = 1.0 - sub_panel_count as f64 * Self::SUB_PANEL_RATIO;
        let main_height = available_height * main_ratio;
        let sub_height = available_height * Self::SUB_PANEL_RATIO;

        let plot_x = Self::LEFT_MARGIN;
        let plot_width = (total_width - Self::LEFT_MARGIN - Self::RIGHT_MARGIN).max(0.0);

        let (main_data_min, main_data_max) = if !data.highs.is_empty() {
            let min = data.lows.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = data.highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (min, max)
        } else {
            (0.0, 100.0)
        };

        let main_y_scale = Scale::linear_scale(
            main_data_min,
            main_data_max,
            panels_top + main_height,
            panels_top,
        );

        let main_plot_area = Rect::new(plot_x, panels_top, plot_width, main_height);
        let main_full_area = Rect::new(
            0.0,
            panels_top,
            total_width - Self::RIGHT_MARGIN,
            main_height,
        );

        let main_panel = PanelRect {
            plot_area: main_plot_area,
            full_area: main_full_area,
            y_scale: main_y_scale,
        };

        let main_y_ticks = main_y_scale.nice_ticks(Self::TICK_COUNT);
        let main_y_labels: Vec<String> = main_y_ticks.iter().map(|&v| format_price(v)).collect();
        let main_y_axis = AxisInfo {
            ticks: main_y_ticks,
            labels: main_y_labels,
            min: main_data_min,
            max: main_data_max,
        };

        let mut sub_panels = Vec::new();
        let mut y_axes = vec![main_y_axis];

        for i in 0..sub_panel_count {
            let panel_top = panels_top
                + main_height
                + (i as f64 + 1.0) * Self::PANEL_SPACING
                + i as f64 * sub_height;

            let (sub_data_min, sub_data_max) = if i == 0 && !data.volumes.is_empty() {
                let max = data.volumes.iter().cloned().fold(0.0_f64, f64::max);
                (0.0, max)
            } else {
                (0.0, 100.0)
            };

            let sub_y_scale = Scale::linear_scale(
                sub_data_min,
                sub_data_max,
                panel_top + sub_height,
                panel_top,
            );

            let sub_plot_area = Rect::new(plot_x, panel_top, plot_width, sub_height);
            let sub_full_area =
                Rect::new(0.0, panel_top, total_width - Self::RIGHT_MARGIN, sub_height);

            let sub_panel = PanelRect {
                plot_area: sub_plot_area,
                full_area: sub_full_area,
                y_scale: sub_y_scale,
            };

            let sub_y_ticks = sub_y_scale.nice_ticks(Self::TICK_COUNT);
            let sub_y_labels: Vec<String> = if i == 0 {
                sub_y_ticks.iter().map(|&v| format_volume(v)).collect()
            } else {
                sub_y_ticks.iter().map(|&v| format_percentage(v)).collect()
            };

            let sub_y_axis = AxisInfo {
                ticks: sub_y_ticks,
                labels: sub_y_labels,
                min: sub_data_min,
                max: sub_data_max,
            };

            sub_panels.push(sub_panel);
            y_axes.push(sub_y_axis);
        }

        let data_len = data.dates.len();
        let (x_ticks, x_labels) = if data_len > 0 {
            let tick_count = Self::X_TICK_COUNT.min(data_len);
            let mut ticks = Vec::new();
            let mut labels = Vec::new();
            for i in 0..tick_count {
                let idx = if tick_count > 1 {
                    (i as f64 * (data_len - 1) as f64 / (tick_count - 1) as f64).round() as usize
                } else {
                    0
                };
                ticks.push(idx as f64);
                labels.push(format_date(&data.dates[idx]));
            }
            (ticks, labels)
        } else {
            (vec![], vec![])
        };

        let x_axis = AxisInfo {
            ticks: x_ticks,
            labels: x_labels,
            min: 0.0,
            max: if data_len > 0 {
                (data_len - 1) as f64
            } else {
                0.0
            },
        };

        ChartLayout {
            total_rect,
            main_panel,
            sub_panels,
            x_axis,
            y_axes,
            legend_rect,
            title_rect,
        }
    }
}

pub fn format_price(value: f64) -> String {
    format!("{:.2}", value)
}

pub fn format_volume(value: f64) -> String {
    if value < 0.0 {
        return format!("-{}", format_volume(-value));
    }
    if value >= 1_000_000_000.0 {
        format!("{:.2}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("{:.2}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.2}K", value / 1_000.0)
    } else {
        format!("{:.2}", value)
    }
}

pub fn format_percentage(value: f64) -> String {
    format!("{:.2}%", value)
}

pub fn format_date(date: &str) -> String {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() >= 3 {
        format!("{}-{}", parts[1], parts[2])
    } else if parts.len() == 2 {
        format!("{}-{}", parts[0], parts[1])
    } else {
        date.to_string()
    }
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
                "2024-01-09".to_string(),
                "2024-01-10".to_string(),
                "2024-01-11".to_string(),
                "2024-01-12".to_string(),
                "2024-01-15".to_string(),
            ],
            vec![
                100.0, 102.0, 101.0, 103.0, 105.0, 104.0, 106.0, 108.0, 107.0, 109.0,
            ],
            vec![
                105.0, 106.0, 104.0, 107.0, 108.0, 107.0, 109.0, 110.0, 109.0, 111.0,
            ],
            vec![
                98.0, 100.0, 99.0, 101.0, 103.0, 102.0, 104.0, 106.0, 105.0, 107.0,
            ],
            vec![
                103.0, 104.0, 100.0, 105.0, 107.0, 103.0, 108.0, 106.0, 108.0, 110.0,
            ],
            vec![
                1000.0, 1200.0, 800.0, 1500.0, 2000.0, 1100.0, 1800.0, 900.0, 1300.0, 1600.0,
            ],
        )
    }

    fn make_test_config() -> ChartConfig {
        ChartConfigBuilder::new().with_dimensions(1200, 800).build()
    }

    #[test]
    fn test_layout_no_sub_panels() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        assert_eq!(layout.sub_panels.len(), 0);
        assert_eq!(layout.y_axes.len(), 1);
        assert!(layout.main_panel.plot_area.height > 0.0);
        assert!(layout.main_panel.plot_area.width > 0.0);

        let expected_main_height = (800.0 - 10.0 - 25.0 - 30.0) * 1.0;
        assert!((layout.main_panel.plot_area.height - expected_main_height).abs() < 1e-10);
    }

    #[test]
    fn test_layout_one_sub_panel() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 1);

        assert_eq!(layout.sub_panels.len(), 1);
        assert_eq!(layout.y_axes.len(), 2);

        let available = 800.0 - 10.0 - 25.0 - 30.0 - 8.0;
        let expected_main = available * 0.8;
        let expected_sub = available * 0.2;

        assert!((layout.main_panel.plot_area.height - expected_main).abs() < 1e-10);
        assert!((layout.sub_panels[0].plot_area.height - expected_sub).abs() < 1e-10);
    }

    #[test]
    fn test_layout_two_sub_panels() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 2);

        assert_eq!(layout.sub_panels.len(), 2);
        assert_eq!(layout.y_axes.len(), 3);

        let available = 800.0 - 10.0 - 25.0 - 30.0 - 16.0;
        let expected_main = available * 0.6;
        let expected_sub = available * 0.2;

        assert!((layout.main_panel.plot_area.height - expected_main).abs() < 1e-10);
        assert!((layout.sub_panels[0].plot_area.height - expected_sub).abs() < 1e-10);
        assert!((layout.sub_panels[1].plot_area.height - expected_sub).abs() < 1e-10);
    }

    #[test]
    fn test_layout_three_sub_panels() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 3);

        assert_eq!(layout.sub_panels.len(), 3);
        assert_eq!(layout.y_axes.len(), 4);

        let available = 800.0 - 10.0 - 25.0 - 30.0 - 24.0;
        let expected_main = available * 0.4;
        let expected_sub = available * 0.2;

        assert!((layout.main_panel.plot_area.height - expected_main).abs() < 1e-10);
        for (i, sp) in layout.sub_panels.iter().enumerate() {
            assert!(
                (sp.plot_area.height - expected_sub).abs() < 1e-10,
                "sub panel {} height mismatch",
                i
            );
        }
    }

    #[test]
    fn test_layout_sub_panel_max_three() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 5);

        assert_eq!(layout.sub_panels.len(), 3);
        assert_eq!(layout.y_axes.len(), 4);
    }

    #[test]
    fn test_layout_title_area() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new()
            .with_title("Test Chart")
            .with_dimensions(1200, 800)
            .build();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        let title_rect = layout.title_rect.expect("finkit-visualization: unexpected None/Err in visualization/src/layout.rs (A5 governance)");
        assert!((title_rect.height - 30.0).abs() < 1e-10);
        assert!((title_rect.x - 80.0).abs() < 1e-10);
        assert!((title_rect.y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_layout_no_title() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().with_dimensions(1200, 800).build();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        assert!(layout.title_rect.is_none());
    }

    #[test]
    fn test_layout_legend_area() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new()
            .with_dimensions(1200, 800)
            .show_legend(true)
            .build();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        let legend_rect = layout.legend_rect.expect("finkit-visualization: unexpected None/Err in visualization/src/layout.rs (A5 governance)");
        assert!((legend_rect.height - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_layout_no_legend() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new()
            .show_legend(false)
            .with_dimensions(1200, 800)
            .build();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        assert!(layout.legend_rect.is_none());
    }

    #[test]
    fn test_layout_title_and_legend() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new()
            .with_title("Test")
            .show_legend(true)
            .with_dimensions(1200, 800)
            .build();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        assert!(layout.title_rect.is_some());
        assert!(layout.legend_rect.is_some());

        let title = layout.title_rect.expect("finkit-visualization: unexpected None/Err in visualization/src/layout.rs (A5 governance)");
        let legend = layout.legend_rect.expect("finkit-visualization: unexpected None/Err in visualization/src/layout.rs (A5 governance)");
        assert!((legend.y - (title.y + title.height)).abs() < 1e-10);
    }

    #[test]
    fn test_layout_panel_positions() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 2);

        assert!((layout.main_panel.plot_area.x - 80.0).abs() < 1e-10);
        assert!((layout.main_panel.full_area.x - 0.0).abs() < 1e-10);

        let main_bottom = layout.main_panel.plot_area.bottom();
        let sub0_top = layout.sub_panels[0].plot_area.y;
        assert!((sub0_top - main_bottom - 8.0).abs() < 1e-10);

        let sub0_bottom = layout.sub_panels[0].plot_area.bottom();
        let sub1_top = layout.sub_panels[1].plot_area.y;
        assert!((sub1_top - sub0_bottom - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_layout_y_scale_main() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        let scale = &layout.main_panel.y_scale;
        assert!((scale.data_min - 98.0).abs() < 1e-10);
        assert!((scale.data_max - 111.0).abs() < 1e-10);
        assert!((scale.pixel_max - layout.main_panel.plot_area.y).abs() < 1e-10);
        assert!((scale.pixel_min - layout.main_panel.plot_area.bottom()).abs() < 1e-10);
    }

    #[test]
    fn test_layout_y_axis_ticks() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        let y_axis = &layout.y_axes[0];
        assert!(!y_axis.ticks.is_empty());
        assert_eq!(y_axis.ticks.len(), y_axis.labels.len());
        assert!(*y_axis.ticks.first().expect("finkit-visualization: unexpected None/Err in visualization/src/layout.rs (A5 governance)") <= y_axis.min);
        assert!(*y_axis.ticks.last().expect("finkit-visualization: unexpected None/Err in visualization/src/layout.rs (A5 governance)") >= y_axis.max);
    }

    #[test]
    fn test_layout_y_axis_volume_sub_panel() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 1);

        let vol_axis = &layout.y_axes[1];
        assert!((vol_axis.min - 0.0).abs() < 1e-10);
        assert!(vol_axis.max > 0.0);
        assert!(!vol_axis.ticks.is_empty());
        for label in &vol_axis.labels {
            assert!(
                label.ends_with('K')
                    || label.ends_with('M')
                    || label.ends_with('B')
                    || label.parse::<f64>().is_ok(),
                "volume label '{}' should have K/M/B suffix or be a number",
                label
            );
        }
    }

    #[test]
    fn test_layout_x_axis() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        assert!(!layout.x_axis.ticks.is_empty());
        assert_eq!(layout.x_axis.ticks.len(), layout.x_axis.labels.len());
        assert!((layout.x_axis.min - 0.0).abs() < 1e-10);
        assert!((layout.x_axis.max - 9.0).abs() < 1e-10);

        assert_eq!(layout.x_axis.labels[0], "01-02");
        assert_eq!(*layout.x_axis.labels.last().expect("finkit-visualization: unexpected None/Err in visualization/src/layout.rs (A5 governance)"), "01-15");
    }

    #[test]
    fn test_layout_x_axis_single_data() {
        let data = KlineData::new(
            vec!["2024-01-01".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        );
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        assert_eq!(layout.x_axis.ticks.len(), 1);
        assert_eq!(layout.x_axis.labels[0], "01-01");
    }

    #[test]
    fn test_layout_empty_data() {
        let data = KlineData::new(vec![], vec![], vec![], vec![], vec![], vec![]);
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        assert!(layout.x_axis.ticks.is_empty());
        assert!(layout.x_axis.labels.is_empty());
        assert!((layout.y_axes[0].min - 0.0).abs() < 1e-10);
        assert!((layout.y_axes[0].max - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_layout_total_rect() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        assert!((layout.total_rect.width - 1200.0).abs() < 1e-10);
        assert!((layout.total_rect.height - 800.0).abs() < 1e-10);
    }

    #[test]
    fn test_layout_plot_area_margins() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        assert!((layout.main_panel.plot_area.x - 80.0).abs() < 1e-10);
        assert!((layout.main_panel.plot_area.right() - (1200.0 - 20.0)).abs() < 1e-10);
    }

    #[test]
    fn test_format_price() {
        assert_eq!(format_price(1234.567), "1234.57");
        assert_eq!(format_price(0.0), "0.00");
        assert_eq!(format_price(-100.5), "-100.50");
        assert_eq!(format_price(0.1), "0.10");
    }

    #[test]
    fn test_format_volume() {
        assert_eq!(format_volume(500.0), "500.00");
        assert_eq!(format_volume(1500.0), "1.50K");
        assert_eq!(format_volume(1500000.0), "1.50M");
        assert_eq!(format_volume(1500000000.0), "1.50B");
        assert_eq!(format_volume(0.0), "0.00");
        assert_eq!(format_volume(-1500.0), "-1.50K");
    }

    #[test]
    fn test_format_percentage() {
        assert_eq!(format_percentage(12.34), "12.34%");
        assert_eq!(format_percentage(0.0), "0.00%");
        assert_eq!(format_percentage(-5.67), "-5.67%");
    }

    #[test]
    fn test_format_date() {
        assert_eq!(format_date("2024-01-15"), "01-15");
        assert_eq!(format_date("2024-12-31"), "12-31");
        assert_eq!(format_date("2024-1-5"), "1-5");
    }

    #[test]
    fn test_format_date_short() {
        assert_eq!(format_date("01-01"), "01-01");
        assert_eq!(format_date("abc"), "abc");
    }

    #[test]
    fn test_panel_rect_full_area_includes_y_axis() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 0);

        assert!((layout.main_panel.full_area.x - 0.0).abs() < 1e-10);
        assert!(layout.main_panel.full_area.width > layout.main_panel.plot_area.width);
    }

    #[test]
    fn test_sub_panels_do_not_overlap() {
        let data = make_test_data();
        let config = make_test_config();
        let layout = LayoutCalculator::calculate(&data, &config, 3);

        let main = &layout.main_panel.plot_area;
        for (i, sp) in layout.sub_panels.iter().enumerate() {
            assert!(
                sp.plot_area.y >= main.bottom() + 8.0 - 1e-10,
                "sub panel {} overlaps main panel",
                i
            );
            if i > 0 {
                let prev = &layout.sub_panels[i - 1].plot_area;
                assert!(
                    sp.plot_area.y >= prev.bottom() + 8.0 - 1e-10,
                    "sub panel {} overlaps previous sub panel",
                    i
                );
            }
        }
    }
}
