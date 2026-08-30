pub mod area_chart;
pub mod candlestick;
pub mod indicators;
pub mod line_chart;
pub mod ohlc_bar;

use crate::config::{ChartConfig, ChartType, IndicatorConfig, IndicatorType};
use crate::data::KlineData;
use crate::error::{Result, VisualizationError};
use crate::geometry::Point;
use crate::layout::{ChartLayout, LayoutCalculator};
use crate::primitive::{Color, DrawList, Primitive, Style};
use crate::render::{Renderer, SvgRenderer};

pub struct RenderCache {
    background_draw_list: DrawList,
    kline_draw_list: DrawList,
    indicator_draw_list: DrawList,
    last_kline_count: usize,
    dirty: bool,
    bg_prim_count: usize,
    kline_prim_count: usize,
}

impl RenderCache {
    pub fn new() -> Self {
        Self {
            background_draw_list: DrawList::new(),
            kline_draw_list: DrawList::new(),
            indicator_draw_list: DrawList::new(),
            last_kline_count: 0,
            dirty: true,
            bg_prim_count: 0,
            kline_prim_count: 0,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn update_kline_count(&mut self, count: usize) -> bool {
        let changed = count != self.last_kline_count;
        self.last_kline_count = count;
        changed
    }

    pub fn split_draw_list(&mut self, full: DrawList) {
        let total = full.primitives.len();
        let bg_end = self.bg_prim_count.min(total);
        let kline_end = (self.bg_prim_count + self.kline_prim_count).min(total);

        self.background_draw_list = DrawList {
            primitives: full.primitives[..bg_end].to_vec(),
        };
        self.kline_draw_list = DrawList {
            primitives: full.primitives[bg_end..kline_end].to_vec(),
        };
        self.indicator_draw_list = DrawList {
            primitives: full.primitives[kline_end..].to_vec(),
        };
        self.dirty = false;
    }
}

impl Default for RenderCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "html")]
use crate::render::HtmlRenderer;

pub struct KlineChart {
    config: ChartConfig,
    layout: Option<ChartLayout>,
    draw_list: DrawList,
    data: Option<KlineData>,
    render_cache: RenderCache,
}

impl KlineChart {
    pub fn new(config: ChartConfig) -> Self {
        Self {
            config,
            layout: None,
            draw_list: DrawList::new(),
            data: None,
            render_cache: RenderCache::new(),
        }
    }

    pub fn config(&self) -> &ChartConfig {
        &self.config
    }

    pub fn layout(&self) -> Option<&ChartLayout> {
        self.layout.as_ref()
    }

    pub fn draw_list(&self) -> &DrawList {
        &self.draw_list
    }

    pub fn set_data(&mut self, data: KlineData) {
        self.data = Some(data);
        self.render_cache.mark_dirty();
    }

    pub fn data(&self) -> Option<&KlineData> {
        self.data.as_ref()
    }

    pub fn append_kline(
        &mut self,
        date: &str,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Result<()> {
        let data = self.data.as_mut().ok_or(VisualizationError::EmptyData)?;
        data.push(date.to_string(), open, high, low, close, volume);
        Ok(())
    }

    pub fn update_last_kline(
        &mut self,
        close: f64,
        high: Option<f64>,
        low: Option<f64>,
        volume: Option<f64>,
    ) -> Result<()> {
        let data = self.data.as_mut().ok_or(VisualizationError::EmptyData)?;
        if data.is_empty() {
            return Err(VisualizationError::EmptyData);
        }
        let last = data.len() - 1;
        data.closes[last] = close;
        if let Some(h) = high {
            data.highs[last] = h;
        }
        if let Some(l) = low {
            data.lows[last] = l;
        }
        if let Some(v) = volume {
            data.volumes[last] = v;
        }
        Ok(())
    }

    pub fn render_incremental(&mut self) -> Result<DrawList> {
        let data_clone;
        {
            let data = self.data.as_ref().ok_or(VisualizationError::EmptyData)?;
            if data.is_empty() {
                return Err(VisualizationError::EmptyData);
            }
            if !data.validate() {
                return Err(VisualizationError::ConversionError {
                    message: "Data arrays have inconsistent lengths".to_string(),
                });
            }
            data_clone = data.clone();
        }

        let needs_full = self.render_cache.is_dirty()
            || self.layout.is_none()
            || data_clone.len() != self.render_cache.last_kline_count;

        if needs_full {
            self.full_render_internal(&data_clone)?;
        } else {
            self.incremental_render_internal(&data_clone)?;
        }

        let mut result = DrawList::new();
        result.extend(self.render_cache.background_draw_list.clone());
        result.extend(self.render_cache.kline_draw_list.clone());
        result.extend(self.render_cache.indicator_draw_list.clone());
        self.draw_list = result.clone();
        Ok(result)
    }

    fn full_render_internal(&mut self, data: &KlineData) -> Result<()> {
        let sub_count = if self.config.show_volume { 1 } else { 0 };
        let layout = LayoutCalculator::calculate(data, &self.config, sub_count);
        self.layout = Some(layout);
        let layout = self.layout.as_ref().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        let mut bg = DrawList::new();
        indicators::render_grid(&mut bg, layout, &self.config);
        indicators::render_axes(&mut bg, layout, &self.config);

        let mut kline = DrawList::new();
        match self.config.chart_type {
            ChartType::Candlestick => {
                candlestick::render_candlestick(&mut kline, data, layout, &self.config)
            }
            ChartType::Bar => ohlc_bar::render_ohlc_bar(&mut kline, data, layout, &self.config),
            ChartType::Line => line_chart::render_line(&mut kline, data, layout, &self.config),
            ChartType::Area => area_chart::render_area(&mut kline, data, layout, &self.config),
        }

        let mut ind = DrawList::new();
        indicators::render_volume(&mut ind, data, layout, &self.config);
        indicators::render_title(&mut ind, layout, &self.config);
        indicators::render_legend(&mut ind, layout, &self.config, &[]);

        self.render_cache.bg_prim_count = bg.len();
        self.render_cache.kline_prim_count = kline.len();
        self.render_cache.background_draw_list = bg;
        self.render_cache.kline_draw_list = kline;
        self.render_cache.indicator_draw_list = ind;
        self.render_cache.update_kline_count(data.len());
        self.render_cache.dirty = false;

        Ok(())
    }

    fn incremental_render_internal(&mut self, data: &KlineData) -> Result<()> {
        let layout = self.layout.as_ref().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let last_idx = data.len() - 1;

        match self.config.chart_type {
            ChartType::Candlestick => {
                let prim_count = self.render_cache.kline_draw_list.len();
                if prim_count >= 2 {
                    self.render_cache
                        .kline_draw_list
                        .primitives
                        .truncate(prim_count - 2);
                }
                Self::render_candlestick_bar(
                    &mut self.render_cache.kline_draw_list,
                    data,
                    layout,
                    &self.config,
                    last_idx,
                    data.len(),
                );
            }
            ChartType::Bar => {
                let prim_count = self.render_cache.kline_draw_list.len();
                if prim_count >= 3 {
                    self.render_cache
                        .kline_draw_list
                        .primitives
                        .truncate(prim_count - 3);
                }
                Self::render_ohlc_bar_at(
                    &mut self.render_cache.kline_draw_list,
                    data,
                    layout,
                    &self.config,
                    last_idx,
                    data.len(),
                );
            }
            ChartType::Line | ChartType::Area => {
                let mut kline = DrawList::new();
                match self.config.chart_type {
                    ChartType::Line => {
                        line_chart::render_line(&mut kline, data, layout, &self.config)
                    }
                    ChartType::Area => {
                        area_chart::render_area(&mut kline, data, layout, &self.config)
                    }
                    _ => unreachable!(),
                }
                self.render_cache.kline_draw_list = kline;
            }
        }

        let mut ind = DrawList::new();
        indicators::render_volume(&mut ind, data, layout, &self.config);
        indicators::render_title(&mut ind, layout, &self.config);
        indicators::render_legend(&mut ind, layout, &self.config, &[]);
        self.render_cache.indicator_draw_list = ind;
        self.render_cache.update_kline_count(data.len());

        Ok(())
    }

    fn render_candlestick_bar(
        draw_list: &mut DrawList,
        data: &KlineData,
        layout: &ChartLayout,
        config: &ChartConfig,
        idx: usize,
        total: usize,
    ) {
        let plot_area = &layout.main_panel.plot_area;
        let y_scale = &layout.main_panel.y_scale;
        let plot_width = plot_area.width;
        let bar_width = plot_width / total as f64;
        let candle_width = (bar_width * 0.7).max(1.0);
        let gap = (bar_width - candle_width) / 2.0;

        let up_color = Color::from_hex(config.color_scheme.up_color());
        let down_color = Color::from_hex(config.color_scheme.down_color());

        let open = data.opens[idx];
        let high = data.highs[idx];
        let low = data.lows[idx];
        let close = data.closes[idx];

        let is_up = close >= open;
        let color = if is_up { up_color } else { down_color };

        let x_center = plot_area.x + idx as f64 * bar_width + bar_width / 2.0;
        let y_high = y_scale.data_to_pixel(high);
        let y_low = y_scale.data_to_pixel(low);
        let y_open = y_scale.data_to_pixel(open);
        let y_close = y_scale.data_to_pixel(close);

        let wick_style = Style::new()
            .with_stroke(color)
            .with_line_width(1.0)
            .with_fill(Color::TRANSPARENT);

        draw_list.push(Primitive::Line {
            p1: Point::new(x_center, y_high),
            p2: Point::new(x_center, y_low),
            style: wick_style,
        });

        let body_top = y_open.min(y_close);
        let body_bottom = y_open.max(y_close);
        let body_height = (body_bottom - body_top).max(1.0);
        let body_x = plot_area.x + idx as f64 * bar_width + gap;

        draw_list.push(Primitive::FilledRect {
            rect: crate::geometry::Rect::new(body_x, body_top, candle_width, body_height),
            fill: color,
            stroke: Some(color),
        });
    }

    fn render_ohlc_bar_at(
        draw_list: &mut DrawList,
        data: &KlineData,
        layout: &ChartLayout,
        config: &ChartConfig,
        idx: usize,
        total: usize,
    ) {
        let plot_area = &layout.main_panel.plot_area;
        let y_scale = &layout.main_panel.y_scale;
        let plot_width = plot_area.width;
        let bar_width = plot_width / total as f64;
        let tick_len = (bar_width / 3.0).max(2.0);

        let up_color = Color::from_hex(config.color_scheme.up_color());
        let down_color = Color::from_hex(config.color_scheme.down_color());

        let open = data.opens[idx];
        let high = data.highs[idx];
        let low = data.lows[idx];
        let close = data.closes[idx];

        let is_up = close >= open;
        let color = if is_up { up_color } else { down_color };

        let x_center = plot_area.x + idx as f64 * bar_width + bar_width / 2.0;
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

    pub fn build_draw_list(
        &mut self,
        data: &KlineData,
        indicators: &[IndicatorConfig],
    ) -> Result<()> {
        if data.is_empty() {
            return Err(VisualizationError::EmptyData);
        }
        if !data.validate() {
            return Err(VisualizationError::ConversionError {
                message: "Data arrays have inconsistent lengths".to_string(),
            });
        }

        let mut sub_count = if self.config.show_volume { 1 } else { 0 };

        let mut macd_count = 0usize;
        let mut rsi_count = 0usize;
        let mut kdj_count = 0usize;

        for ic in indicators {
            if !ic.visible {
                continue;
            }
            match ic.indicator_type {
                IndicatorType::MACD => macd_count += 1,
                IndicatorType::RSI => rsi_count += 1,
                IndicatorType::KDJ => kdj_count += 1,
                _ => {}
            }
        }

        sub_count += macd_count + rsi_count + kdj_count;

        self.layout = Some(LayoutCalculator::calculate(data, &self.config, sub_count));
        self.draw_list.clear();

        let layout = self.layout.as_ref().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        indicators::render_grid(&mut self.draw_list, layout, &self.config);
        indicators::render_axes(&mut self.draw_list, layout, &self.config);
        let bg_count = self.draw_list.len();

        match self.config.chart_type {
            ChartType::Candlestick => {
                candlestick::render_candlestick(&mut self.draw_list, data, layout, &self.config);
            }
            ChartType::Bar => {
                ohlc_bar::render_ohlc_bar(&mut self.draw_list, data, layout, &self.config);
            }
            ChartType::Line => {
                line_chart::render_line(&mut self.draw_list, data, layout, &self.config);
            }
            ChartType::Area => {
                area_chart::render_area(&mut self.draw_list, data, layout, &self.config);
            }
        }

        let kline_count = self.draw_list.len() - bg_count;

        let vol_offset = if self.config.show_volume { 1 } else { 0 };
        let mut macd_panel_idx = vol_offset;
        let mut rsi_panel_idx = macd_panel_idx + macd_count;
        let mut kdj_panel_idx = rsi_panel_idx + rsi_count;

        for ic in indicators {
            if !ic.visible {
                continue;
            }
            match ic.indicator_type {
                IndicatorType::MA => {
                    let periods: Vec<usize> = ic.params.iter().map(|&p| p as usize).collect();
                    indicators::render_ma(
                        &mut self.draw_list,
                        data,
                        layout,
                        &self.config,
                        &periods,
                    );
                }
                IndicatorType::EMA => {
                    let periods: Vec<usize> = ic.params.iter().map(|&p| p as usize).collect();
                    indicators::render_ema(
                        &mut self.draw_list,
                        data,
                        layout,
                        &self.config,
                        &periods,
                    );
                }
                IndicatorType::SMA => {
                    let periods: Vec<usize> = ic.params.iter().map(|&p| p as usize).collect();
                    indicators::render_ma(
                        &mut self.draw_list,
                        data,
                        layout,
                        &self.config,
                        &periods,
                    );
                }
                IndicatorType::BOLL => {
                    let period = ic.params.first().copied().unwrap_or(20.0) as usize;
                    let nb_dev = ic.params.get(1).copied().unwrap_or(2.0);
                    indicators::render_boll(
                        &mut self.draw_list,
                        data,
                        layout,
                        &self.config,
                        period,
                        nb_dev,
                    );
                }
                IndicatorType::MACD => {
                    let fast = ic.params.first().copied().unwrap_or(12.0) as usize;
                    let slow = ic.params.get(1).copied().unwrap_or(26.0) as usize;
                    let signal = ic.params.get(2).copied().unwrap_or(9.0) as usize;
                    indicators::render_macd(
                        &mut self.draw_list,
                        data,
                        layout,
                        &self.config,
                        fast,
                        slow,
                        signal,
                        macd_panel_idx,
                    );
                    macd_panel_idx += 1;
                }
                IndicatorType::RSI => {
                    let period = ic.params.first().copied().unwrap_or(14.0) as usize;
                    indicators::render_rsi(
                        &mut self.draw_list,
                        data,
                        layout,
                        &self.config,
                        period,
                        rsi_panel_idx,
                    );
                    rsi_panel_idx += 1;
                }
                IndicatorType::KDJ => {
                    let fast_k = ic.params.first().copied().unwrap_or(9.0) as usize;
                    let slow_k = ic.params.get(1).copied().unwrap_or(3.0) as usize;
                    let slow_d = ic.params.get(2).copied().unwrap_or(3.0) as usize;
                    indicators::render_kdj(
                        &mut self.draw_list,
                        data,
                        layout,
                        &self.config,
                        fast_k,
                        slow_k,
                        slow_d,
                        kdj_panel_idx,
                    );
                    kdj_panel_idx += 1;
                }
                IndicatorType::Custom(_) => {}
            }
        }

        indicators::render_volume(&mut self.draw_list, data, layout, &self.config);
        indicators::render_title(&mut self.draw_list, layout, &self.config);
        indicators::render_legend(&mut self.draw_list, layout, &self.config, indicators);

        self.render_cache.bg_prim_count = bg_count;
        self.render_cache.kline_prim_count = kline_count;
        self.render_cache.update_kline_count(data.len());
        self.render_cache.dirty = false;

        Ok(())
    }

    pub fn add_ma(&mut self, data: &KlineData, periods: &[usize]) {
        let layout = match self.layout.as_ref() {
            Some(l) => l,
            None => return,
        };
        indicators::render_ma(&mut self.draw_list, data, layout, &self.config, periods);
    }

    pub fn add_ema(&mut self, data: &KlineData, periods: &[usize]) {
        let layout = match self.layout.as_ref() {
            Some(l) => l,
            None => return,
        };
        indicators::render_ema(&mut self.draw_list, data, layout, &self.config, periods);
    }

    pub fn add_boll(&mut self, data: &KlineData, period: usize, nb_dev: f64) {
        let layout = match self.layout.as_ref() {
            Some(l) => l,
            None => return,
        };
        indicators::render_boll(
            &mut self.draw_list,
            data,
            layout,
            &self.config,
            period,
            nb_dev,
        );
    }

    pub fn add_macd(
        &mut self,
        data: &KlineData,
        fast: usize,
        slow: usize,
        signal: usize,
        panel_index: usize,
    ) {
        let layout = match self.layout.as_ref() {
            Some(l) => l,
            None => return,
        };
        indicators::render_macd(
            &mut self.draw_list,
            data,
            layout,
            &self.config,
            fast,
            slow,
            signal,
            panel_index,
        );
    }

    pub fn add_rsi(&mut self, data: &KlineData, period: usize, panel_index: usize) {
        let layout = match self.layout.as_ref() {
            Some(l) => l,
            None => return,
        };
        indicators::render_rsi(
            &mut self.draw_list,
            data,
            layout,
            &self.config,
            period,
            panel_index,
        );
    }

    pub fn add_kdj(
        &mut self,
        data: &KlineData,
        fast_k: usize,
        slow_k: usize,
        slow_d: usize,
        panel_index: usize,
    ) {
        let layout = match self.layout.as_ref() {
            Some(l) => l,
            None => return,
        };
        indicators::render_kdj(
            &mut self.draw_list,
            data,
            layout,
            &self.config,
            fast_k,
            slow_k,
            slow_d,
            panel_index,
        );
    }

    pub fn add_sar(&mut self, data: &KlineData, acceleration: f64, maximum: f64) {
        let layout = match self.layout.as_ref() {
            Some(l) => l,
            None => return,
        };
        indicators::render_sar(
            &mut self.draw_list,
            data,
            layout,
            &self.config,
            acceleration,
            maximum,
        );
    }

    pub fn save_as_svg(&self, path: &str) -> Result<()> {
        let svg_string = self.to_svg_string()?;
        std::fs::write(path, svg_string).map_err(|e| VisualizationError::RenderError {
            message: format!("Failed to write SVG file: {}", e),
        })
    }

    #[cfg(feature = "html")]
    pub fn save_as_html(&self, path: &str) -> Result<()> {
        let html_string = self.to_html_string()?;
        std::fs::write(path, html_string).map_err(|e| VisualizationError::RenderError {
            message: format!("Failed to write HTML file: {}", e),
        })
    }

    #[cfg(not(feature = "html"))]
    pub fn save_as_html(&self, _path: &str) -> Result<()> {
        Err(VisualizationError::RenderError {
            message: "HTML rendering is not enabled. Enable the 'html' feature.".to_string(),
        })
    }

    #[cfg(feature = "html")]
    pub fn to_html_string(&self) -> Result<String> {
        let _layout = self
            .layout
            .as_ref()
            .ok_or_else(|| VisualizationError::RenderError {
                message: "Layout not calculated. Call build_draw_list first.".to_string(),
            })?;

        let renderer = HtmlRenderer::new();
        renderer.render(&self.draw_list, &self.config)
    }

    pub fn to_svg_string(&self) -> Result<String> {
        let _layout = self
            .layout
            .as_ref()
            .ok_or_else(|| VisualizationError::RenderError {
                message: "Layout not calculated. Call build_draw_list first.".to_string(),
            })?;

        let renderer = SvgRenderer::new();
        renderer.render(&self.draw_list, &self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChartConfigBuilder, IndicatorType};

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

    #[test]
    fn test_kline_chart_new() {
        let config = ChartConfig::default();
        let chart = KlineChart::new(config);
        assert_eq!(chart.config().width, 1200);
    }

    #[test]
    fn test_kline_chart_build_empty() {
        let config = ChartConfig::default();
        let mut chart = KlineChart::new(config);
        let data = KlineData::new(vec![], vec![], vec![], vec![], vec![], vec![]);
        let result = chart.build_draw_list(&data, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_kline_chart_build_valid() {
        let config = ChartConfig::default();
        let mut chart = KlineChart::new(config);
        let data = KlineData::new(
            vec!["2024-01-01".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        );
        let indicators = vec![IndicatorConfig::new(IndicatorType::MA, vec![5.0])];
        let result = chart.build_draw_list(&data, &indicators);
        assert!(result.is_ok());
    }

    #[test]
    fn test_kline_chart_candlestick_draw_list() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Candlestick)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.build_draw_list(&data, &[]).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert!(!chart.draw_list().is_empty());
        assert!(chart.layout().is_some());
    }

    #[test]
    fn test_kline_chart_ohlc_bar_draw_list() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Bar)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.build_draw_list(&data, &[]).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert!(!chart.draw_list().is_empty());
    }

    #[test]
    fn test_kline_chart_line_draw_list() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Line)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.build_draw_list(&data, &[]).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert!(!chart.draw_list().is_empty());
    }

    #[test]
    fn test_kline_chart_area_draw_list() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Area)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.build_draw_list(&data, &[]).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert!(!chart.draw_list().is_empty());
    }

    #[test]
    fn test_kline_chart_svg_output() {
        let config = ChartConfigBuilder::new()
            .with_title("Test K线图")
            .with_chart_type(ChartType::Candlestick)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        let indicators = vec![IndicatorConfig::new(IndicatorType::MA, vec![5.0])];
        chart.build_draw_list(&data, &indicators).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let svg = chart.to_svg_string().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains(">Test K线图</text>"));
    }

    #[test]
    fn test_kline_chart_svg_without_build() {
        let config = ChartConfig::default();
        let chart = KlineChart::new(config);
        let result = chart.to_svg_string();
        assert!(result.is_err());
    }

    #[test]
    #[cfg(not(feature = "html"))]
    fn test_kline_chart_save_as_html_not_enabled() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Candlestick)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.build_draw_list(&data, &[]).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let result = chart.save_as_html("test.html");
        assert!(result.is_err());
    }

    #[test]
    fn test_kline_chart_with_volume() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Candlestick)
            .show_volume(true)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.build_draw_list(&data, &[]).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert!(!chart.layout().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").sub_panels.is_empty());
    }

    #[test]
    fn test_kline_chart_without_volume() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Candlestick)
            .show_volume(false)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.build_draw_list(&data, &[]).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert!(chart.layout().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").sub_panels.is_empty());
    }

    #[test]
    fn test_kline_chart_inconsistent_data() {
        let config = ChartConfig::default();
        let mut chart = KlineChart::new(config);
        let data = KlineData::new(
            vec!["2024-01-01".to_string(), "2024-01-02".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        );
        let result = chart.build_draw_list(&data, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_cache_new() {
        let cache = RenderCache::new();
        assert!(cache.is_dirty());
        assert_eq!(cache.last_kline_count, 0);
    }

    #[test]
    fn test_render_cache_mark_dirty() {
        let mut cache = RenderCache::new();
        cache.dirty = false;
        assert!(!cache.is_dirty());
        cache.mark_dirty();
        assert!(cache.is_dirty());
    }

    #[test]
    fn test_render_cache_update_kline_count() {
        let mut cache = RenderCache::new();
        assert!(cache.update_kline_count(5));
        assert!(!cache.update_kline_count(5));
        assert!(cache.update_kline_count(10));
    }

    #[test]
    fn test_render_cache_split_draw_list() {
        let mut cache = RenderCache::new();
        cache.bg_prim_count = 2;
        cache.kline_prim_count = 3;

        let mut full = DrawList::new();
        full.push(Primitive::Line {
            p1: Point::new(0.0, 0.0),
            p2: Point::new(1.0, 1.0),
            style: Style::default(),
        });
        full.push(Primitive::Line {
            p1: Point::new(1.0, 1.0),
            p2: Point::new(2.0, 2.0),
            style: Style::default(),
        });
        full.push(Primitive::Rect {
            rect: crate::geometry::Rect::new(0.0, 0.0, 10.0, 10.0),
            style: Style::default(),
        });
        full.push(Primitive::Rect {
            rect: crate::geometry::Rect::new(10.0, 10.0, 20.0, 20.0),
            style: Style::default(),
        });
        full.push(Primitive::Rect {
            rect: crate::geometry::Rect::new(20.0, 20.0, 30.0, 30.0),
            style: Style::default(),
        });
        full.push(Primitive::Circle {
            center: Point::new(5.0, 5.0),
            radius: 3.0,
            style: Style::default(),
        });

        cache.split_draw_list(full);
        assert!(!cache.is_dirty());
        assert_eq!(cache.background_draw_list.len(), 2);
        assert_eq!(cache.kline_draw_list.len(), 3);
        assert_eq!(cache.indicator_draw_list.len(), 1);
    }

    #[test]
    fn test_kline_chart_set_data() {
        let config = ChartConfig::default();
        let mut chart = KlineChart::new(config);
        assert!(chart.data().is_none());
        let data = KlineData::new(
            vec!["2024-01-01".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        );
        chart.set_data(data);
        assert!(chart.data().is_some());
        assert_eq!(chart.data().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").len(), 1);
    }

    #[test]
    fn test_kline_chart_append_kline() {
        let config = ChartConfig::default();
        let mut chart = KlineChart::new(config);
        let data = KlineData::new(
            vec!["2024-01-01".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        );
        chart.set_data(data);
        chart.render_incremental().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart
            .append_kline("2024-01-02", 103.0, 108.0, 101.0, 107.0, 1200.0)
            .expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert_eq!(chart.data().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").len(), 2);
    }

    #[test]
    fn test_kline_chart_append_kline_no_data() {
        let config = ChartConfig::default();
        let mut chart = KlineChart::new(config);
        let result = chart.append_kline("2024-01-01", 100.0, 105.0, 98.0, 103.0, 1000.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_kline_chart_update_last_kline() {
        let config = ChartConfig::default();
        let mut chart = KlineChart::new(config);
        let data = KlineData::new(
            vec!["2024-01-01".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        );
        chart.set_data(data);
        chart.render_incremental().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart
            .update_last_kline(106.0, Some(110.0), Some(97.0), Some(1500.0))
            .expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let d = chart.data().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert_eq!(d.closes[0], 106.0);
        assert_eq!(d.highs[0], 110.0);
        assert_eq!(d.lows[0], 97.0);
        assert_eq!(d.volumes[0], 1500.0);
    }

    #[test]
    fn test_kline_chart_update_last_kline_no_data() {
        let config = ChartConfig::default();
        let mut chart = KlineChart::new(config);
        let result = chart.update_last_kline(106.0, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_kline_chart_render_incremental_full() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Candlestick)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.set_data(data);
        let result = chart.render_incremental();
        assert!(result.is_ok());
        assert!(!result.expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").is_empty());
    }

    #[test]
    fn test_kline_chart_render_incremental_partial() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Candlestick)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.set_data(data);
        chart.render_incremental().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart
            .update_last_kline(112.0, Some(113.0), Some(108.0), Some(2000.0))
            .expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let result = chart.render_incremental();
        assert!(result.is_ok());
        assert!(!result.expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").is_empty());
    }

    #[test]
    fn test_kline_chart_render_incremental_no_data() {
        let config = ChartConfig::default();
        let mut chart = KlineChart::new(config);
        let result = chart.render_incremental();
        assert!(result.is_err());
    }

    #[test]
    fn test_kline_chart_render_incremental_append_triggers_full() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Candlestick)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.set_data(data);
        chart.render_incremental().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart
            .append_kline("2024-01-16", 110.0, 113.0, 108.0, 112.0, 1800.0)
            .expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let result = chart.render_incremental();
        assert!(result.is_ok());
        assert_eq!(chart.data().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").len(), 11);
    }

    #[test]
    fn test_kline_chart_render_incremental_ohlc_bar() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Bar)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.set_data(data);
        chart.render_incremental().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart.update_last_kline(112.0, None, None, None).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let result = chart.render_incremental();
        assert!(result.is_ok());
    }

    #[test]
    fn test_kline_chart_render_incremental_line() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Line)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.set_data(data);
        chart.render_incremental().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart.update_last_kline(112.0, None, None, None).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let result = chart.render_incremental();
        assert!(result.is_ok());
    }

    #[test]
    fn test_kline_chart_render_incremental_area() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Area)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.set_data(data);
        chart.render_incremental().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart.update_last_kline(112.0, None, None, None).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let result = chart.render_incremental();
        assert!(result.is_ok());
    }
}
