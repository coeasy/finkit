pub mod area_chart;
pub mod candlestick;
pub mod chan;
pub mod indicators;
pub mod line_chart;
pub mod ohlc_bar;

use crate::config::{
    ChanRenderConfig, ChartConfig, ChartType, DecimateStrategy, IndicatorConfig, IndicatorType,
    InteractionConfig,
};
use crate::data::{KlineBar, KlineData, KlineFrame};
use crate::error::{Result, VisualizationError};
use crate::geometry::Point;
use crate::layout::{ChartLayout, LayoutCalculator};
use crate::primitive::{Color, DrawList, Primitive, Style};
#[cfg(feature = "html")]
use crate::render::WebGlRenderer;
use crate::render::{CanvasRenderer, JsonRenderer, Renderer, SvgRenderer};
use crate::scene::{
    ChartMetadata, ChartScene, HitRegion, HitTarget, LayerDescriptor, PanelDescriptor, PanelId,
};
use crate::viewport::{LodLevel, LodPolicy, Viewport};
use finkit::chan::{ChanAnalysis, ChanConfig};
use finkit::chan_mtf::{analyze_multi, ChanMultiAnalysis, ChanMultiConfig};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

pub struct RenderCache {
    background_draw_list: DrawList,
    kline_draw_list: DrawList,
    kline_overlay_draw_list: DrawList,
    indicator_draw_list: DrawList,
    indicator_overlay_draw_list: DrawList,
    volume_draw_list: DrawList,
    last_kline_count: usize,
    last_data_revision: u64,
    dirty: bool,
    bg_prim_count: usize,
    kline_prim_count: usize,
}

/// Lightweight counters for profiling a chart render without inspecting
/// backend-specific primitives.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChartRenderStats {
    pub source_bars: usize,
    pub rendered_bars: usize,
    pub primitive_count: usize,
    pub panel_count: usize,
    pub hit_region_count: usize,
}

/// A user-defined event marker that is rendered on the main price panel and
/// participates in semantic hit testing and HTML data-window tooltips.
#[derive(Debug, Clone, PartialEq)]
pub struct EventMarker {
    pub index: usize,
    pub label: String,
    pub value: Option<f64>,
    pub color: Color,
    pub priority: i32,
}

/// Result of applying one real-time OHLCV update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KlineUpdateKind {
    Appended,
    Updated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KlineUpdate {
    pub index: usize,
    pub kind: KlineUpdateKind,
    pub revision: u64,
}

impl EventMarker {
    pub fn new(index: usize, label: impl Into<String>) -> Self {
        Self {
            index,
            label: label.into(),
            value: None,
            color: Color::from_hex("#f59e0b"),
            priority: 30,
        }
    }

    pub fn with_value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

impl RenderCache {
    pub fn new() -> Self {
        Self {
            background_draw_list: DrawList::new(),
            kline_draw_list: DrawList::new(),
            kline_overlay_draw_list: DrawList::new(),
            indicator_draw_list: DrawList::new(),
            indicator_overlay_draw_list: DrawList::new(),
            volume_draw_list: DrawList::new(),
            last_kline_count: 0,
            last_data_revision: 0,
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

    pub fn update_data_revision(&mut self, revision: u64) -> bool {
        let changed = revision != self.last_data_revision;
        self.last_data_revision = revision;
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
        self.kline_overlay_draw_list = DrawList::new();
        self.indicator_overlay_draw_list = DrawList::new();
        self.volume_draw_list = DrawList::new();
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
    frame: Option<KlineFrame>,
    indicator_configs: Vec<IndicatorConfig>,
    custom_indicator_series: HashMap<String, Vec<f64>>,
    event_markers: Vec<EventMarker>,
    render_cache: RenderCache,
    viewport: Viewport,
    scene: ChartScene,
    hidden_layers: HashSet<String>,
    chan_analysis: Option<ChanAnalysis>,
    chan_multi_analysis: Option<ChanMultiAnalysis>,
}

struct RenderWindow<'a> {
    data: Cow<'a, KlineData>,
    source_offset: usize,
    source_ranges: Vec<(usize, usize)>,
}

impl KlineChart {
    pub fn new(config: ChartConfig) -> Self {
        Self {
            config,
            layout: None,
            draw_list: DrawList::new(),
            data: None,
            frame: None,
            indicator_configs: Vec::new(),
            custom_indicator_series: HashMap::new(),
            event_markers: Vec::new(),
            render_cache: RenderCache::new(),
            viewport: Viewport::full(),
            scene: ChartScene::default(),
            hidden_layers: HashSet::new(),
            chan_analysis: None,
            chan_multi_analysis: None,
        }
    }

    pub fn config(&self) -> &ChartConfig {
        &self.config
    }

    pub fn interaction_config(&self) -> &InteractionConfig {
        &self.config.interaction
    }

    /// Update interaction behavior without rebuilding the chart object.
    pub fn set_interaction_config(&mut self, interaction: InteractionConfig) {
        self.config.interaction = interaction;
    }

    /// Configure the visual Chanlun layers independently from the analysis
    /// result. Bindings can use this to expose the same Chan settings without
    /// copying the complete `ChartConfig` type.
    pub fn set_chan_render_config(&mut self, chan: ChanRenderConfig) {
        self.config.chan = chan;
        self.render_cache.mark_dirty();
    }

    /// Return backend-independent render counters for telemetry and tuning.
    pub fn render_stats(&self) -> ChartRenderStats {
        let source_bars = self.data.as_ref().map(KlineData::len).unwrap_or(0);
        let rendered_bars = self
            .scene
            .hit_regions
            .iter()
            .filter_map(|region| match &region.target {
                HitTarget::Kline { .. } => Some(()),
                _ => None,
            })
            .count();
        ChartRenderStats {
            source_bars,
            rendered_bars,
            primitive_count: self.draw_list.primitives.len(),
            panel_count: self.scene.panels.len(),
            hit_region_count: self.scene.hit_regions.len(),
        }
    }

    pub fn layout(&self) -> Option<&ChartLayout> {
        self.layout.as_ref()
    }

    pub fn draw_list(&self) -> &DrawList {
        &self.draw_list
    }

    pub fn viewport(&self) -> Viewport {
        self.viewport
    }

    pub fn scene(&self) -> &ChartScene {
        &self.scene
    }

    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.viewport = viewport;
        self.render_cache.mark_dirty();
    }

    pub fn reset_viewport(&mut self) {
        self.set_viewport(Viewport::full());
    }

    pub fn set_lod_policy(&mut self, policy: LodPolicy) {
        self.config.lod_policy = policy;
        self.render_cache.mark_dirty();
    }

    pub fn set_layer_visible(&mut self, id: &str, visible: bool) -> bool {
        if visible {
            self.hidden_layers.remove(id);
        } else {
            self.hidden_layers.insert(id.to_string());
        }
        let known = matches!(
            id,
            "background.grid"
                | "background.axes"
                | "price"
                | "chan"
                | "chan.centers"
                | "chan.segments"
                | "chan.strokes"
                | "chan.fractals"
                | "chan.developing"
                | "chan.signals"
                | "chan.divergences"
                | "chan.labels"
                | "volume"
                | "indicators"
        );
        let changed = self.scene.set_layer_visible(id, visible) || known;
        if changed {
            self.render_cache.mark_dirty();
        }
        changed
    }

    /// Register a computed custom series for `IndicatorType::Custom`.
    pub fn set_custom_indicator_series(
        &mut self,
        name: impl Into<String>,
        values: Vec<f64>,
    ) -> Result<()> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(VisualizationError::ConversionError {
                message: "custom indicator name must not be empty".to_string(),
            });
        }
        let data_len = self
            .data
            .as_ref()
            .map(KlineData::len)
            .unwrap_or(values.len());
        if values.len() != data_len {
            return Err(VisualizationError::ConversionError {
                message: format!(
                    "custom indicator length {} does not match data length {}",
                    values.len(),
                    data_len
                ),
            });
        }
        self.custom_indicator_series.insert(name, values);
        self.render_cache.mark_dirty();
        Ok(())
    }

    /// Remove a registered custom chart series.
    pub fn clear_custom_indicator_series(&mut self, name: &str) -> bool {
        let removed = self.custom_indicator_series.remove(name).is_some();
        if removed {
            self.render_cache.mark_dirty();
        }
        removed
    }

    /// Add an indicator definition that will be included on the next render.
    /// This is useful for bindings that build a chart incrementally.
    pub fn add_indicator_config(&mut self, indicator: IndicatorConfig) {
        self.indicator_configs.push(indicator);
        self.render_cache.mark_dirty();
    }

    pub fn add_event_marker(&mut self, marker: EventMarker) {
        self.event_markers.push(marker);
        self.render_cache.mark_dirty();
    }

    pub fn clear_event_markers(&mut self) {
        if !self.event_markers.is_empty() {
            self.event_markers.clear();
            self.render_cache.mark_dirty();
        }
    }

    pub fn event_markers(&self) -> &[EventMarker] {
        &self.event_markers
    }

    pub fn hit_test(&self, point: Point) -> Option<&HitRegion> {
        self.scene.hit_test(point)
    }

    pub fn set_chan_analysis(&mut self, analysis: ChanAnalysis) {
        self.chan_analysis = Some(analysis);
        self.chan_multi_analysis = None;
        self.render_cache.mark_dirty();
    }

    pub fn clear_chan_analysis(&mut self) {
        self.chan_analysis = None;
        self.chan_multi_analysis = None;
        self.render_cache.mark_dirty();
    }

    pub fn set_chan_multi_analysis(&mut self, analysis: ChanMultiAnalysis) {
        self.chan_multi_analysis = Some(analysis);
        self.chan_analysis = None;
        self.render_cache.mark_dirty();
    }

    pub fn chan_analysis(&self) -> Option<&ChanAnalysis> {
        self.chan_analysis.as_ref()
    }

    pub fn chan_multi_analysis(&self) -> Option<&ChanMultiAnalysis> {
        self.chan_multi_analysis.as_ref()
    }

    pub fn analyze_and_set_chan(&mut self, config: ChanConfig) -> Result<()> {
        let data = self.data.as_ref().ok_or(VisualizationError::EmptyData)?;
        let analysis = finkit::chan::analyze(
            &data.opens,
            &data.highs,
            &data.lows,
            &data.closes,
            &data.volumes,
            config,
        )
        .map_err(|error| VisualizationError::ConversionError {
            message: format!("Chanlun analysis failed: {error}"),
        })?;
        self.set_chan_analysis(analysis);
        Ok(())
    }

    pub fn analyze_and_set_chan_multi(&mut self, config: ChanMultiConfig) -> Result<()> {
        let data = self.data.as_ref().ok_or(VisualizationError::EmptyData)?;
        let analysis = analyze_multi(
            &data.opens,
            &data.highs,
            &data.lows,
            &data.closes,
            &data.volumes,
            &config,
        )
        .map_err(|error| VisualizationError::ConversionError {
            message: format!("multi-timeframe Chanlun analysis failed: {error}"),
        })?;
        self.set_chan_multi_analysis(analysis);
        Ok(())
    }

    pub fn set_data(&mut self, data: KlineData) {
        self.data = Some(data);
        self.frame = None;
        self.custom_indicator_series.clear();
        self.event_markers.clear();
        self.chan_analysis = None;
        self.chan_multi_analysis = None;
        self.render_cache.mark_dirty();
    }

    pub fn data(&self) -> Option<&KlineData> {
        self.data.as_ref()
    }

    pub fn frame(&self) -> Option<&KlineFrame> {
        self.frame.as_ref()
    }

    pub fn set_frame(&mut self, frame: KlineFrame) {
        self.data = Some(frame.to_kline_data());
        self.frame = Some(frame);
        self.custom_indicator_series.clear();
        self.event_markers.clear();
        self.chan_analysis = None;
        self.chan_multi_analysis = None;
        self.render_cache.mark_dirty();
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
        Self::validate_ohlcv_bar(open, high, low, close, volume)?;
        let data = self.data.as_mut().ok_or(VisualizationError::EmptyData)?;
        data.push(date.to_string(), open, high, low, close, volume);
        for values in self.custom_indicator_series.values_mut() {
            values.push(f64::NAN);
        }
        self.frame = None;
        self.chan_analysis = None;
        self.chan_multi_analysis = None;
        self.render_cache.mark_dirty();
        Ok(())
    }

    /// Append a new bar or replace the current bar when the feed sends the
    /// same date again. This is the preferred API for live quote feeds where
    /// the last interval is revised repeatedly before it closes.
    pub fn upsert_kline(
        &mut self,
        date: &str,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Result<KlineUpdate> {
        self.upsert_kline_inner(date, open, high, low, close, volume, None)
    }

    /// Timestamp-preserving live update for feeds whose bar identity comes
    /// from an exchange timestamp rather than a display date string.
    pub fn upsert_kline_with_timestamp(
        &mut self,
        timestamp: i64,
        date: &str,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Result<KlineUpdate> {
        self.upsert_kline_inner(date, open, high, low, close, volume, Some(timestamp))
    }

    fn upsert_kline_inner(
        &mut self,
        date: &str,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        timestamp: Option<i64>,
    ) -> Result<KlineUpdate> {
        Self::validate_ohlcv_bar(open, high, low, close, volume)?;
        if let Some(timestamp) = timestamp {
            if let Some(data) = self.data.as_ref() {
                if data.timestamps.len() != data.len() {
                    return Err(VisualizationError::InvalidConfig {
                        field: "timestamp".to_string(),
                        reason: "timestamped live updates require a complete timestamp index"
                            .to_string(),
                    });
                }
                if data.timestamps.last().is_some_and(|last| timestamp < *last) {
                    return Err(VisualizationError::InvalidConfig {
                        field: "timestamp".to_string(),
                        reason: format!(
                            "live timestamp {timestamp} is older than the current last timestamp {}",
                            data.timestamps.last().copied().unwrap_or_default()
                        ),
                    });
                }
            }
        }
        if let Some(last_date) = self.data.as_ref().and_then(|data| data.dates.last()) {
            if date < last_date {
                return Err(VisualizationError::InvalidConfig {
                    field: "date".to_string(),
                    reason: format!(
                        "live bar date {date} is older than the current last bar {last_date}"
                    ),
                });
            }
        }
        let is_update = self
            .data
            .as_ref()
            .and_then(|data| data.dates.last())
            .is_some_and(|last_date| last_date == date);
        if !is_update {
            if let Some(timestamp) = timestamp {
                let data = self.data.as_mut().ok_or(VisualizationError::EmptyData)?;
                if !data.push_timestamped(
                    timestamp,
                    date.to_string(),
                    open,
                    high,
                    low,
                    close,
                    volume,
                ) {
                    return Err(VisualizationError::InvalidConfig {
                        field: "timestamp".to_string(),
                        reason: "timestamped live update must extend the existing timestamp index"
                            .to_string(),
                    });
                }
                for values in self.custom_indicator_series.values_mut() {
                    values.push(f64::NAN);
                }
                self.frame = None;
                self.chan_analysis = None;
                self.chan_multi_analysis = None;
                self.render_cache.mark_dirty();
            } else {
                self.append_kline(date, open, high, low, close, volume)?;
            }
            let data = self.data.as_ref().ok_or(VisualizationError::EmptyData)?;
            return Ok(KlineUpdate {
                index: data.len() - 1,
                kind: KlineUpdateKind::Appended,
                revision: data.revision(),
            });
        }

        let (index, revision) = {
            let data = self.data.as_mut().ok_or(VisualizationError::EmptyData)?;
            let index = data.len() - 1;
            data.opens[index] = open;
            data.highs[index] = high;
            data.lows[index] = low;
            data.closes[index] = close;
            data.volumes[index] = volume;
            if let Some(timestamp) = timestamp {
                data.timestamps[index] = timestamp;
            }
            data.bump_revision();
            (index, data.revision())
        };
        self.invalidate_after_data_change(true);
        Ok(KlineUpdate {
            index,
            kind: KlineUpdateKind::Updated,
            revision,
        })
    }

    /// Apply a live quote batch without rendering between rows.
    ///
    /// Every row still receives normal OHLCV validation and update/append
    /// semantics. The caller can render once after this method returns, which
    /// avoids rebuilding indicators, Chan overlays and layout for every tick.
    pub fn upsert_klines(&mut self, updates: &[KlineBar]) -> Result<Vec<KlineUpdate>> {
        let mut result = Vec::with_capacity(updates.len());
        for update in updates {
            result.push(self.upsert_kline_inner(
                &update.date,
                update.open,
                update.high,
                update.low,
                update.close,
                update.volume,
                update.timestamp,
            )?);
        }
        Ok(result)
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
        let next_high = high.unwrap_or(data.highs[last].max(close));
        let next_low = low.unwrap_or(data.lows[last].min(close));
        Self::validate_ohlcv_bar(
            data.opens[last],
            next_high,
            next_low,
            close,
            volume.unwrap_or(data.volumes[last]),
        )?;
        data.closes[last] = close;
        data.highs[last] = next_high;
        data.lows[last] = next_low;
        data.volumes[last] = volume.unwrap_or(data.volumes[last]);
        data.bump_revision();
        self.invalidate_after_data_change(true);
        Ok(())
    }

    fn validate_ohlcv_bar(open: f64, high: f64, low: f64, close: f64, volume: f64) -> Result<()> {
        let values = [
            ("open", open),
            ("high", high),
            ("low", low),
            ("close", close),
            ("volume", volume),
        ];
        if let Some((name, value)) = values.iter().find(|(_, value)| !value.is_finite()) {
            return Err(VisualizationError::ConversionError {
                message: format!("{name} must be finite, got {value}"),
            });
        }
        if high < open || high < close || low > open || low > close || high < low {
            return Err(VisualizationError::ConversionError {
                message: "OHLC range is invalid".to_string(),
            });
        }
        if volume < 0.0 {
            return Err(VisualizationError::ConversionError {
                message: "volume must not be negative".to_string(),
            });
        }
        Ok(())
    }

    fn invalidate_after_data_change(&mut self, invalidate_custom_tail: bool) {
        if invalidate_custom_tail {
            if let Some(data) = self.data.as_ref() {
                if let Some(last) = data.len().checked_sub(1) {
                    for values in self.custom_indicator_series.values_mut() {
                        if values.len() > last {
                            values[last] = f64::NAN;
                        }
                    }
                }
            }
        }
        self.frame = None;
        self.chan_analysis = None;
        self.chan_multi_analysis = None;
        self.render_cache.mark_dirty();
    }

    pub fn render_incremental(&mut self) -> Result<DrawList> {
        let data_len = {
            let data = self.data.as_ref().ok_or(VisualizationError::EmptyData)?;
            if data.is_empty() {
                return Err(VisualizationError::EmptyData);
            }
            if !data.validate() {
                return Err(VisualizationError::ConversionError {
                    message: "Data arrays have inconsistent lengths".to_string(),
                });
            }
            data.len()
        };
        let data_revision = self
            .data
            .as_ref()
            .map(KlineData::revision)
            .unwrap_or_default();

        // Temporarily move the owned series out so rendering can mutate the
        // chart cache without cloning every OHLCV vector.
        let data = self.data.take().ok_or(VisualizationError::EmptyData)?;

        let needs_full = self.render_cache.is_dirty()
            || self.layout.is_none()
            || data_len != self.render_cache.last_kline_count
            || data_revision != self.render_cache.last_data_revision
            || !self.viewport.is_full();

        let render_result = if needs_full {
            self.full_render_internal(&data)
        } else {
            self.incremental_render_internal(&data)
        };
        self.data = Some(data);
        render_result?;

        let mut result = DrawList::new();
        result.extend(self.render_cache.background_draw_list.clone());
        result.extend(self.render_cache.kline_draw_list.clone());
        result.extend(self.render_cache.indicator_draw_list.clone());
        self.draw_list = result.clone();
        Ok(result)
    }

    fn full_render_internal(&mut self, data: &KlineData) -> Result<()> {
        let render_window = self.render_window(data);
        let render_data = &render_window.data;
        let source_offset = render_window.source_offset;
        let layout = LayoutCalculator::calculate_with_sub_panel_kinds(
            &render_data,
            &self.config,
            &self.sub_panel_kinds(),
        );
        self.layout = Some(layout);
        let layout = self.layout.as_ref().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        let mut bg = DrawList::new();
        if self.layer_visible("background.grid") {
            indicators::render_grid(&mut bg, layout, &self.config);
        }
        if self.layer_visible("background.axes") {
            indicators::render_axes(&mut bg, layout, &self.config);
        }

        let mut kline = DrawList::new();
        let render_indices = self.render_indices(&render_data);
        if self.layer_visible("price") {
            match self.config.chart_type {
                ChartType::Candlestick => candlestick::render_candlestick_indices(
                    &mut kline,
                    &render_data,
                    layout,
                    &self.config,
                    &render_indices,
                ),
                ChartType::Bar => ohlc_bar::render_ohlc_bar_indices(
                    &mut kline,
                    &render_data,
                    layout,
                    &self.config,
                    &render_indices,
                ),
                ChartType::Line => line_chart::render_line_indices(
                    &mut kline,
                    &render_data,
                    layout,
                    &self.config,
                    &render_indices,
                ),
                ChartType::Area => area_chart::render_area_indices(
                    &mut kline,
                    &render_data,
                    layout,
                    &self.config,
                    &render_indices,
                ),
            }
        }
        let price_prim_count = kline.len();
        if self.config.chan.enabled && self.layer_visible("chan") {
            let chan_render_config = self.effective_chan_config();
            if let Some(analysis) = &self.chan_multi_analysis {
                chan::render_multi_analysis_window(
                    &mut kline,
                    analysis,
                    &render_data,
                    layout,
                    &chan_render_config,
                    source_offset,
                    &render_window.source_ranges,
                );
            } else if let Some(analysis) = &self.chan_analysis {
                chan::render_analysis_window(
                    &mut kline,
                    analysis,
                    &render_data,
                    layout,
                    &chan_render_config,
                    source_offset,
                    &render_window.source_ranges,
                );
            } else {
                let mut chan_config = self.config.clone();
                chan_config.chan = chan_render_config;
                chan::render_chan(&mut kline, &render_data, layout, &chan_config)?;
            }
        }
        Self::render_event_markers(
            &self.event_markers,
            &mut kline,
            &render_data,
            layout,
            &render_window.source_ranges,
            self.config.chan.label_min_pixel_gap,
        );

        let mut ind = DrawList::new();
        if self.layer_visible("volume") {
            indicators::render_volume_indices(
                &mut ind,
                &render_data,
                layout,
                &self.config,
                &render_indices,
            );
        }
        let volume_prim_count = ind.len();
        if self.layer_visible("indicators") {
            Self::render_indicator_configs(
                &mut ind,
                &render_data,
                layout,
                &self.config,
                &self.indicator_configs,
                &self.custom_indicator_series,
                &render_window.source_ranges,
            );
            indicators::render_title(&mut ind, layout, &self.config);
            indicators::render_legend(&mut ind, layout, &self.config, &[]);
        }

        self.render_cache.bg_prim_count = bg.len();
        self.render_cache.kline_prim_count = kline.len();
        self.render_cache.background_draw_list = bg;
        self.render_cache.kline_draw_list = kline;
        self.render_cache.kline_overlay_draw_list = DrawList {
            primitives: self.render_cache.kline_draw_list.primitives[price_prim_count..].to_vec(),
        };
        self.render_cache.indicator_draw_list = ind;
        self.render_cache.indicator_overlay_draw_list = DrawList {
            primitives: self.render_cache.indicator_draw_list.primitives[volume_prim_count..]
                .to_vec(),
        };
        self.render_cache.volume_draw_list = DrawList {
            primitives: self.render_cache.indicator_draw_list.primitives[..volume_prim_count]
                .to_vec(),
        };
        self.render_cache.update_kline_count(data.len());
        self.render_cache.update_data_revision(data.revision());
        self.render_cache.dirty = false;
        let plot_area = layout.main_panel.plot_area;
        let sub_panel_rects = layout
            .sub_panels
            .iter()
            .map(|panel| panel.plot_area)
            .collect::<Vec<_>>();
        self.rebuild_scene(
            render_data.len(),
            source_offset,
            plot_area,
            &render_window.source_ranges,
            &sub_panel_rects,
            layout.main_panel.y_scale,
            data.revision(),
        );

        Ok(())
    }

    fn incremental_render_internal(&mut self, data: &KlineData) -> Result<()> {
        let layout = self.layout.as_ref().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
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
        let render_indices = self.render_indices(data);
        if self.layer_visible("indicators") {
            Self::render_indicator_configs(
                &mut ind,
                data,
                layout,
                &self.config,
                &self.indicator_configs,
                &self.custom_indicator_series,
                &(0..data.len())
                    .map(|index| (index, index + 1))
                    .collect::<Vec<_>>(),
            );
            indicators::render_title(&mut ind, layout, &self.config);
            indicators::render_legend(&mut ind, layout, &self.config, &self.indicator_configs);
        }
        if self.layer_visible("volume") {
            indicators::render_volume_indices(
                &mut ind,
                data,
                layout,
                &self.config,
                &render_indices,
            );
        }
        self.render_cache.indicator_draw_list = ind;
        self.render_cache.update_kline_count(data.len());
        self.render_cache.update_data_revision(data.revision());

        Ok(())
    }

    fn sub_panel_kinds(&self) -> Vec<crate::layout::SubPanelKind> {
        let mut kinds = Vec::new();
        if self.config.show_volume {
            kinds.push(crate::layout::SubPanelKind::Volume);
        }
        if self.layer_visible("indicators") {
            for indicator in self.indicator_configs.iter().filter(|item| item.visible) {
                match indicator.indicator_type {
                    IndicatorType::MACD => kinds.push(crate::layout::SubPanelKind::Value),
                    IndicatorType::RSI | IndicatorType::KDJ => {
                        kinds.push(crate::layout::SubPanelKind::Oscillator)
                    }
                    _ => {}
                }
            }
        }
        kinds
    }

    fn render_indicator_configs(
        draw_list: &mut DrawList,
        data: &KlineData,
        layout: &ChartLayout,
        config: &ChartConfig,
        indicators_config: &[IndicatorConfig],
        custom_series: &HashMap<String, Vec<f64>>,
        source_ranges: &[(usize, usize)],
    ) {
        let visible: Vec<&IndicatorConfig> = indicators_config
            .iter()
            .filter(|indicator| indicator.visible)
            .collect();
        let volume_offset = usize::from(config.show_volume);
        let mut macd_panel = volume_offset;
        let mut rsi_panel = macd_panel
            + visible
                .iter()
                .filter(|indicator| indicator.indicator_type == IndicatorType::MACD)
                .count();
        let mut kdj_panel = rsi_panel
            + visible
                .iter()
                .filter(|indicator| indicator.indicator_type == IndicatorType::RSI)
                .count();

        for indicator in visible {
            match &indicator.indicator_type {
                IndicatorType::MA | IndicatorType::SMA => {
                    let periods: Vec<usize> = indicator
                        .params
                        .iter()
                        .map(|value| (*value).max(0.0) as usize)
                        .collect();
                    indicators::render_ma(draw_list, data, layout, config, &periods);
                }
                IndicatorType::EMA => {
                    let periods: Vec<usize> = indicator
                        .params
                        .iter()
                        .map(|value| (*value).max(0.0) as usize)
                        .collect();
                    indicators::render_ema(draw_list, data, layout, config, &periods);
                }
                IndicatorType::BOLL => {
                    let period =
                        indicator.params.first().copied().unwrap_or(20.0).max(0.0) as usize;
                    let deviation = indicator.params.get(1).copied().unwrap_or(2.0);
                    indicators::render_boll(draw_list, data, layout, config, period, deviation);
                }
                IndicatorType::MACD => {
                    let fast = indicator.params.first().copied().unwrap_or(12.0).max(0.0) as usize;
                    let slow = indicator.params.get(1).copied().unwrap_or(26.0).max(0.0) as usize;
                    let signal = indicator.params.get(2).copied().unwrap_or(9.0).max(0.0) as usize;
                    indicators::render_macd(
                        draw_list, data, layout, config, fast, slow, signal, macd_panel,
                    );
                    macd_panel += 1;
                }
                IndicatorType::RSI => {
                    let period =
                        indicator.params.first().copied().unwrap_or(14.0).max(0.0) as usize;
                    indicators::render_rsi(draw_list, data, layout, config, period, rsi_panel);
                    rsi_panel += 1;
                }
                IndicatorType::KDJ => {
                    let fast_k = indicator.params.first().copied().unwrap_or(9.0).max(0.0) as usize;
                    let slow_k = indicator.params.get(1).copied().unwrap_or(3.0).max(0.0) as usize;
                    let slow_d = indicator.params.get(2).copied().unwrap_or(3.0).max(0.0) as usize;
                    indicators::render_kdj(
                        draw_list, data, layout, config, fast_k, slow_k, slow_d, kdj_panel,
                    );
                    kdj_panel += 1;
                }
                IndicatorType::Custom(name) => {
                    if name.eq_ignore_ascii_case("sar") {
                        let acceleration = indicator.params.first().copied().unwrap_or(0.02);
                        let maximum = indicator.params.get(1).copied().unwrap_or(0.2);
                        indicators::render_sar(
                            draw_list,
                            data,
                            layout,
                            config,
                            acceleration,
                            maximum,
                        );
                    } else if let Some(values) = custom_series.get(name) {
                        let rendered = if values.len() == data.len() {
                            values.clone()
                        } else {
                            source_ranges
                                .iter()
                                .map(|(_, end)| {
                                    values
                                        .get(end.saturating_sub(1))
                                        .copied()
                                        .unwrap_or(f64::NAN)
                                })
                                .collect::<Vec<_>>()
                        };
                        indicators::render_custom(
                            draw_list,
                            data,
                            layout,
                            config,
                            &rendered,
                            &indicator.color,
                            indicator.line_width,
                        );
                    }
                }
            }
        }
    }

    fn render_indices(&self, data: &KlineData) -> Vec<usize> {
        if matches!(self.config.lod_policy, LodPolicy::Fixed(LodLevel::Raw)) {
            return (0..data.len()).collect();
        }
        let strategy = if !matches!(self.config.decimate_strategy, DecimateStrategy::Auto) {
            self.config.decimate_strategy
        } else {
            match self.config.lod_policy {
                LodPolicy::Strategy(strategy) => strategy,
                LodPolicy::Fixed(LodLevel::Overview) => DecimateStrategy::EveryNth,
                LodPolicy::Fixed(LodLevel::Balanced) | LodPolicy::Auto => {
                    match self.config.chart_type {
                        ChartType::Candlestick | ChartType::Bar => DecimateStrategy::MinMax,
                        ChartType::Line | ChartType::Area => DecimateStrategy::LTTB,
                    }
                }
                LodPolicy::Fixed(LodLevel::Raw) => unreachable!(),
            }
        };
        let pixel_width = if self.viewport.is_full() {
            self.config.width
        } else {
            self.viewport.pixel_width.max(1)
        };
        crate::decimate::select_indices(data, &strategy, pixel_width)
    }

    fn render_window<'a>(&self, data: &'a KlineData) -> RenderWindow<'a> {
        let (start, end) = if self.viewport.is_full() {
            (0, data.len())
        } else {
            self.viewport
                .warmup_range(data.len(), self.indicator_lookback())
        };
        let span = end.saturating_sub(start);
        let level = self
            .config
            .lod_policy
            .level(span, self.viewport.pixel_width);
        let should_aggregate = level == LodLevel::Overview && span > 1;

        if should_aggregate {
            let target = (self.viewport.pixel_width as usize)
                .saturating_mul(2)
                .max(1);
            let bucket = span.div_ceil(target).max(2);
            let aggregated = data.aggregate(start, end, bucket);
            return RenderWindow {
                data: Cow::Owned(aggregated.data),
                source_offset: start,
                source_ranges: aggregated.source_ranges,
            };
        }

        let source_ranges = (start..end).map(|index| (index, index + 1)).collect();
        let rendered = if start == 0 && end == data.len() {
            Cow::Borrowed(data)
        } else {
            Cow::Owned(data.slice(start, end))
        };
        RenderWindow {
            data: rendered,
            source_offset: start,
            source_ranges,
        }
    }

    /// Maximum historical lookback needed to stabilize the configured rolling
    /// indicators before the first requested viewport bar.
    pub fn indicator_lookback(&self) -> usize {
        self.indicator_configs
            .iter()
            .filter(|indicator| indicator.visible)
            .map(|indicator| match indicator.indicator_type {
                IndicatorType::MA
                | IndicatorType::SMA
                | IndicatorType::EMA
                | IndicatorType::BOLL => {
                    indicator.params.first().copied().unwrap_or(20.0).max(0.0) as usize
                }
                IndicatorType::MACD => {
                    let slow = indicator.params.get(1).copied().unwrap_or(26.0).max(0.0) as usize;
                    let signal = indicator.params.get(2).copied().unwrap_or(9.0).max(0.0) as usize;
                    slow.saturating_add(signal)
                }
                IndicatorType::RSI => {
                    indicator.params.first().copied().unwrap_or(14.0).max(0.0) as usize
                }
                IndicatorType::KDJ => {
                    let fast = indicator.params.first().copied().unwrap_or(9.0).max(0.0) as usize;
                    let slow = indicator.params.get(1).copied().unwrap_or(3.0).max(0.0) as usize;
                    let signal = indicator.params.get(2).copied().unwrap_or(3.0).max(0.0) as usize;
                    fast.saturating_add(slow).saturating_add(signal)
                }
                IndicatorType::Custom(_) => 0,
            })
            .max()
            .unwrap_or(0)
    }

    fn layer_visible(&self, id: &str) -> bool {
        !self.hidden_layers.contains(id)
    }

    fn effective_chan_config(&self) -> ChanRenderConfig {
        let mut config = self.config.chan.clone();
        config.show_centers &= self.layer_visible("chan.centers");
        config.show_segments &= self.layer_visible("chan.segments");
        config.show_strokes &= self.layer_visible("chan.strokes");
        config.show_fractals &= self.layer_visible("chan.fractals");
        config.show_developing &= self.layer_visible("chan.developing");
        config.show_signals &= self.layer_visible("chan.signals");
        config.show_divergences &= self.layer_visible("chan.divergences");
        config.show_labels &= self.layer_visible("chan.labels");
        config
    }

    fn render_event_markers(
        event_markers: &[EventMarker],
        draw_list: &mut DrawList,
        data: &KlineData,
        layout: &ChartLayout,
        source_ranges: &[(usize, usize)],
        label_min_pixel_gap: f64,
    ) {
        if data.is_empty() {
            return;
        }
        let plot = layout.main_panel.plot_area;
        let bar_width = plot.width / data.len() as f64;
        let mut occupied_labels = Vec::new();
        for marker in event_markers {
            let Some(index) = source_ranges
                .iter()
                .position(|(start, end)| marker.index >= *start && marker.index < *end)
                .or_else(|| (marker.index < data.len()).then_some(marker.index))
            else {
                continue;
            };
            if index >= data.len() {
                continue;
            }
            let x = plot.x + (index as f64 + 0.5) * bar_width;
            let y = marker
                .value
                .map(|value| layout.main_panel.y_scale.data_to_pixel(value))
                .unwrap_or(plot.y + 12.0)
                .clamp(plot.y + 4.0, plot.y + plot.height - 4.0);
            let style = Style::default()
                .with_stroke(marker.color)
                .with_fill(marker.color)
                .with_line_width(1.0);
            draw_list.push(Primitive::Circle {
                center: Point::new(x, y),
                radius: 4.0,
                style: style.clone(),
            });
            let label_box = crate::geometry::Rect::new(
                x + 4.0,
                y - 8.0,
                10.0 + marker.label.len() as f64 * 7.0,
                13.0,
            );
            let padded = label_box.inflate(label_min_pixel_gap.max(0.0) * 0.5, 1.0);
            if !occupied_labels
                .iter()
                .any(|rect: &crate::geometry::Rect| rect.intersects(&padded))
            {
                occupied_labels.push(padded);
                draw_list.push(Primitive::Text {
                    position: Point::new(x + 6.0, y + 3.0),
                    content: marker.label.clone(),
                    style: style.with_font_size(10.0),
                });
            }
        }
    }

    fn rebuild_scene(
        &mut self,
        data_len: usize,
        source_offset: usize,
        plot: crate::geometry::Rect,
        source_ranges: &[(usize, usize)],
        sub_panel_rects: &[crate::geometry::Rect],
        main_y_scale: crate::geometry::Scale,
        data_revision: u64,
    ) {
        self.scene.clear();
        self.scene.add_panel(PanelDescriptor {
            id: PanelId::Main,
            rect: plot,
            visible: true,
        });
        for (index, rect) in sub_panel_rects.iter().copied().enumerate() {
            let is_volume = self.config.show_volume && index == 0;
            self.scene.add_panel(PanelDescriptor {
                id: if is_volume {
                    PanelId::Volume
                } else {
                    PanelId::Indicator(if self.config.show_volume {
                        index - 1
                    } else {
                        index
                    })
                },
                rect,
                visible: true,
            });
        }
        let source_start = source_ranges
            .first()
            .map(|range| range.0)
            .unwrap_or(source_offset);
        let source_end = source_ranges
            .last()
            .map(|range| range.1)
            .unwrap_or(source_start);
        let timeframe_labels = self
            .chan_multi_analysis
            .as_ref()
            .map(|analysis| {
                analysis
                    .frames
                    .iter()
                    .map(|frame| frame.timeframe.label.clone())
                    .collect()
            })
            .unwrap_or_default();
        self.scene.metadata = ChartMetadata {
            data_revision,
            source_start,
            source_end,
            timeframe_labels,
        };
        self.scene
            .add_layer(LayerDescriptor::new("background.grid", PanelId::Main, 0));
        self.scene
            .add_layer(LayerDescriptor::new("background.axes", PanelId::Main, 1));
        self.scene
            .add_layer(LayerDescriptor::new("price", PanelId::Main, 10));
        self.scene
            .add_layer(LayerDescriptor::new("chan", PanelId::Main, 20));
        self.scene
            .add_layer(LayerDescriptor::new("chan.centers", PanelId::Main, 18));
        self.scene
            .add_layer(LayerDescriptor::new("chan.segments", PanelId::Main, 19));
        self.scene
            .add_layer(LayerDescriptor::new("chan.strokes", PanelId::Main, 20));
        self.scene
            .add_layer(LayerDescriptor::new("chan.fractals", PanelId::Main, 21));
        self.scene
            .add_layer(LayerDescriptor::new("chan.developing", PanelId::Main, 22));
        self.scene
            .add_layer(LayerDescriptor::new("chan.signals", PanelId::Main, 23));
        self.scene
            .add_layer(LayerDescriptor::new("chan.divergences", PanelId::Main, 24));
        self.scene
            .add_layer(LayerDescriptor::new("chan.labels", PanelId::Main, 25));
        self.scene
            .add_layer(LayerDescriptor::new("volume", PanelId::Volume, 30));
        self.scene.add_layer(LayerDescriptor::new(
            "indicators",
            PanelId::Indicator(0),
            40,
        ));
        for layer in &mut self.scene.layers {
            layer.visible = !self.hidden_layers.contains(&layer.id);
        }

        if data_len == 0 {
            return;
        }
        let bar_width = plot.width / data_len as f64;
        for index in 0..data_len {
            let (source_start, source_end) = source_ranges
                .get(index)
                .copied()
                .unwrap_or((source_offset + index, source_offset + index + 1));
            self.scene.add_hit_region(HitRegion {
                rect: crate::geometry::Rect::new(
                    plot.x + index as f64 * bar_width,
                    plot.y,
                    bar_width,
                    plot.height,
                ),
                target: HitTarget::Kline {
                    index: source_start,
                },
                priority: 1,
                tooltip: Some(format!(
                    "K线 {}-{}",
                    source_start,
                    source_end.saturating_sub(1)
                )),
            });
        }

        for marker in &self.event_markers {
            let Some(index) = source_ranges
                .iter()
                .position(|(start, end)| marker.index >= *start && marker.index < *end)
                .or_else(|| {
                    (source_ranges.is_empty()
                        && marker.index >= source_offset
                        && marker.index < source_offset.saturating_add(data_len))
                    .then_some(marker.index.saturating_sub(source_offset))
                })
            else {
                continue;
            };
            if index >= data_len {
                continue;
            }
            let x = plot.x + (index as f64 + 0.5) * bar_width;
            let y = marker
                .value
                .map(|value| main_y_scale.data_to_pixel(value))
                .unwrap_or(plot.y + 12.0)
                .clamp(plot.y + 4.0, plot.y + plot.height - 4.0);
            self.scene.add_hit_region(HitRegion {
                rect: crate::geometry::Rect::new(x - 7.0, y - 7.0, 14.0, 14.0),
                target: HitTarget::Event {
                    index: marker.index,
                    kind: marker.label.clone(),
                },
                priority: marker.priority,
                tooltip: Some(marker.label.clone()),
            });
        }

        let map_index = |raw: usize| -> Option<usize> {
            source_ranges
                .iter()
                .position(|(start, end)| raw >= *start && raw < *end)
                .or_else(|| {
                    raw.checked_sub(source_offset)
                        .filter(|index| *index < data_len)
                })
        };
        let map_clamped = |raw: usize| -> Option<usize> {
            map_index(raw).or_else(|| {
                if source_ranges.first().is_some_and(|range| raw < range.0) {
                    Some(0)
                } else if source_ranges.last().is_some_and(|range| raw >= range.1) {
                    Some(data_len.saturating_sub(1))
                } else {
                    None
                }
            })
        };
        let mut structure_regions = Vec::new();
        let mut add_analysis_regions = |analysis: &ChanAnalysis| {
            for fractal in &analysis.fractals {
                if let Some(index) = map_index(fractal.index) {
                    let x = plot.x + (index as f64 + 0.5) * bar_width;
                    structure_regions.push(HitRegion {
                        rect: crate::geometry::Rect::new(x - 5.0, plot.y, 10.0, plot.height),
                        target: HitTarget::ChanFractal {
                            index: fractal.index,
                        },
                        priority: 10,
                        tooltip: Some(format!("Chan fractal {:?}", fractal.kind)),
                    });
                }
            }
            for stroke in &analysis.strokes {
                if let (Some(start), Some(end)) = (
                    map_clamped(stroke.start.index),
                    map_clamped(stroke.end.index),
                ) {
                    let left = start.min(end);
                    let right = start.max(end);
                    structure_regions.push(HitRegion {
                        rect: crate::geometry::Rect::new(
                            plot.x + left as f64 * bar_width,
                            plot.y,
                            (right - left + 1) as f64 * bar_width,
                            plot.height,
                        ),
                        target: HitTarget::ChanStroke {
                            start_index: stroke.start.index,
                            end_index: stroke.end.index,
                        },
                        priority: 7,
                        tooltip: Some(format!("Chan stroke {:?}", stroke.direction)),
                    });
                }
            }
            for segment in &analysis.segments {
                if let (Some(start), Some(end)) = (
                    map_clamped(segment.start_index),
                    map_clamped(segment.end_index),
                ) {
                    let left = start.min(end);
                    let right = start.max(end);
                    structure_regions.push(HitRegion {
                        rect: crate::geometry::Rect::new(
                            plot.x + left as f64 * bar_width,
                            plot.y,
                            (right - left + 1) as f64 * bar_width,
                            plot.height,
                        ),
                        target: HitTarget::ChanSegment {
                            start_index: segment.start_index,
                            end_index: segment.end_index,
                        },
                        priority: 8,
                        tooltip: Some(format!("Chan segment {:?}", segment.direction)),
                    });
                }
            }
            for center in &analysis.centers {
                if let (Some(start), Some(end)) = (
                    map_clamped(center.start_index),
                    map_clamped(center.end_index),
                ) {
                    let left = start.min(end);
                    let right = start.max(end);
                    structure_regions.push(HitRegion {
                        rect: crate::geometry::Rect::new(
                            plot.x + left as f64 * bar_width,
                            plot.y,
                            (right - left + 1) as f64 * bar_width,
                            plot.height,
                        ),
                        target: HitTarget::ChanCenter {
                            start_index: center.start_index,
                            end_index: center.end_index,
                            level: center.level,
                        },
                        priority: 5,
                        tooltip: Some(format!(
                            "Chan center L{} [{:.2}, {:.2}]",
                            center.level, center.lower, center.upper
                        )),
                    });
                }
            }
            for signal in &analysis.signals {
                if let Some(index) = map_index(signal.index) {
                    let x = plot.x + (index as f64 + 0.5) * bar_width;
                    structure_regions.push(HitRegion {
                        rect: crate::geometry::Rect::new(x - 7.0, plot.y, 14.0, plot.height),
                        target: HitTarget::ChanSignal {
                            index: signal.index,
                            kind: format!("{:?}", signal.kind),
                        },
                        priority: 20,
                        tooltip: Some(format!("{:?} strength {:.2}", signal.kind, signal.strength)),
                    });
                }
            }
            for divergence in &analysis.divergences {
                if let Some(index) = map_index(divergence.index) {
                    let x = plot.x + (index as f64 + 0.5) * bar_width;
                    let kind = match divergence.kind {
                        finkit::chan::ChanDivergenceKind::Bullish => "bullish",
                        finkit::chan::ChanDivergenceKind::Bearish => "bearish",
                    };
                    structure_regions.push(HitRegion {
                        rect: crate::geometry::Rect::new(x - 8.0, plot.y, 16.0, plot.height),
                        target: HitTarget::ChanDivergence {
                            index: divergence.index,
                            kind: kind.to_string(),
                        },
                        priority: 21,
                        tooltip: Some(format!(
                            "Chan divergence {} strength {:.2}",
                            kind, divergence.strength
                        )),
                    });
                }
            }
        };
        if let Some(analysis) = &self.chan_analysis {
            add_analysis_regions(analysis);
        }
        if let Some(analysis) = &self.chan_multi_analysis {
            for frame in &analysis.frames {
                add_analysis_regions(&frame.analysis);
            }
        }
        for region in structure_regions {
            self.scene.add_hit_region(region);
        }
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
        if !data.validate_timestamps() {
            return Err(VisualizationError::ConversionError {
                message:
                    "Timestamp array must be empty or strictly increasing with one value per bar"
                        .to_string(),
            });
        }
        // Keep the source frame for interactive HTML export and hit-test
        // metadata. The render path still consumes the caller's reference,
        // so this copy is not used for drawing or indicator calculation.
        self.data = Some(data.clone());
        self.indicator_configs = indicators.to_vec();

        let macd_count = indicators
            .iter()
            .filter(|indicator| {
                indicator.visible && indicator.indicator_type == IndicatorType::MACD
            })
            .count();
        let rsi_count = indicators
            .iter()
            .filter(|indicator| indicator.visible && indicator.indicator_type == IndicatorType::RSI)
            .count();

        let render_window = self.render_window(data);
        let render_data = &render_window.data;
        let source_offset = render_window.source_offset;
        self.layout = Some(LayoutCalculator::calculate_with_sub_panel_kinds(
            &render_data,
            &self.config,
            &self.sub_panel_kinds(),
        ));
        self.draw_list.clear();

        let layout = self.layout.as_ref().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        let render_indices = self.render_indices(&render_data);

        if self.layer_visible("background.grid") {
            indicators::render_grid(&mut self.draw_list, layout, &self.config);
        }
        if self.layer_visible("background.axes") {
            indicators::render_axes(&mut self.draw_list, layout, &self.config);
        }
        let bg_count = self.draw_list.len();

        if self.layer_visible("price") {
            match self.config.chart_type {
                ChartType::Candlestick => {
                    candlestick::render_candlestick_indices(
                        &mut self.draw_list,
                        &render_data,
                        layout,
                        &self.config,
                        &render_indices,
                    );
                }
                ChartType::Bar => {
                    ohlc_bar::render_ohlc_bar_indices(
                        &mut self.draw_list,
                        &render_data,
                        layout,
                        &self.config,
                        &render_indices,
                    );
                }
                ChartType::Line => {
                    line_chart::render_line_indices(
                        &mut self.draw_list,
                        &render_data,
                        layout,
                        &self.config,
                        &render_indices,
                    );
                }
                ChartType::Area => {
                    area_chart::render_area_indices(
                        &mut self.draw_list,
                        &render_data,
                        layout,
                        &self.config,
                        &render_indices,
                    );
                }
            }
        }

        let price_prim_count = self.draw_list.len() - bg_count;

        if self.config.chan.enabled && self.layer_visible("chan") {
            let chan_render_config = self.effective_chan_config();
            if let Some(analysis) = &self.chan_multi_analysis {
                chan::render_multi_analysis_window(
                    &mut self.draw_list,
                    analysis,
                    &render_data,
                    layout,
                    &chan_render_config,
                    source_offset,
                    &render_window.source_ranges,
                );
            } else if let Some(analysis) = &self.chan_analysis {
                chan::render_analysis_window(
                    &mut self.draw_list,
                    analysis,
                    &render_data,
                    layout,
                    &chan_render_config,
                    source_offset,
                    &render_window.source_ranges,
                );
            } else {
                let mut chan_config = self.config.clone();
                chan_config.chan = chan_render_config;
                chan::render_chan(&mut self.draw_list, &render_data, layout, &chan_config)?;
            }
        }

        Self::render_event_markers(
            &self.event_markers,
            &mut self.draw_list,
            &render_data,
            layout,
            &render_window.source_ranges,
            self.config.chan.label_min_pixel_gap,
        );

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
                        &render_data,
                        layout,
                        &self.config,
                        &periods,
                    );
                }
                IndicatorType::EMA => {
                    let periods: Vec<usize> = ic.params.iter().map(|&p| p as usize).collect();
                    indicators::render_ema(
                        &mut self.draw_list,
                        &render_data,
                        layout,
                        &self.config,
                        &periods,
                    );
                }
                IndicatorType::SMA => {
                    let periods: Vec<usize> = ic.params.iter().map(|&p| p as usize).collect();
                    indicators::render_ma(
                        &mut self.draw_list,
                        &render_data,
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
                        &render_data,
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
                        &render_data,
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
                        &render_data,
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
                        &render_data,
                        layout,
                        &self.config,
                        fast_k,
                        slow_k,
                        slow_d,
                        kdj_panel_idx,
                    );
                    kdj_panel_idx += 1;
                }
                IndicatorType::Custom(ref name) => {
                    if name.eq_ignore_ascii_case("sar") {
                        let acceleration = ic.params.first().copied().unwrap_or(0.02);
                        let maximum = ic.params.get(1).copied().unwrap_or(0.2);
                        indicators::render_sar(
                            &mut self.draw_list,
                            &render_data,
                            layout,
                            &self.config,
                            acceleration,
                            maximum,
                        );
                    } else if let Some(values) = self.custom_indicator_series.get(name) {
                        let rendered = if values.len() == render_data.len() {
                            values.clone()
                        } else {
                            render_window
                                .source_ranges
                                .iter()
                                .map(|(_, end)| {
                                    values
                                        .get(end.saturating_sub(1))
                                        .copied()
                                        .unwrap_or(f64::NAN)
                                })
                                .collect::<Vec<_>>()
                        };
                        indicators::render_custom(
                            &mut self.draw_list,
                            &render_data,
                            layout,
                            &self.config,
                            &rendered,
                            &ic.color,
                            ic.line_width,
                        );
                    }
                }
            }
        }

        let indicator_overlay_before_volume = self.draw_list.len();
        if self.layer_visible("volume") {
            indicators::render_volume_indices(
                &mut self.draw_list,
                &render_data,
                layout,
                &self.config,
                &render_indices,
            );
        }
        let volume_end = self.draw_list.len();
        if self.layer_visible("indicators") {
            indicators::render_title(&mut self.draw_list, layout, &self.config);
            indicators::render_legend(&mut self.draw_list, layout, &self.config, indicators);
        }

        let indicator_overlay_end = self.draw_list.len();

        self.render_cache.bg_prim_count = bg_count;
        self.render_cache.kline_prim_count = kline_count;
        self.render_cache.update_kline_count(data.len());
        self.render_cache.update_data_revision(data.revision());
        self.render_cache.split_draw_list(self.draw_list.clone());
        let annotation_start = bg_count + price_prim_count;
        let annotation_end = bg_count + kline_count;
        self.render_cache.kline_overlay_draw_list = DrawList {
            primitives: self.draw_list.primitives[annotation_start..annotation_end].to_vec(),
        };
        self.render_cache.indicator_overlay_draw_list = DrawList {
            primitives: self.draw_list.primitives
                [bg_count + kline_count..indicator_overlay_before_volume]
                .iter()
                .chain(self.draw_list.primitives[volume_end..indicator_overlay_end].iter())
                .cloned()
                .collect(),
        };
        self.render_cache.volume_draw_list = DrawList {
            primitives: self.draw_list.primitives[indicator_overlay_before_volume..volume_end]
                .to_vec(),
        };
        let plot_area = layout.main_panel.plot_area;
        let sub_panel_rects = layout
            .sub_panels
            .iter()
            .map(|panel| panel.plot_area)
            .collect::<Vec<_>>();
        self.rebuild_scene(
            render_data.len(),
            source_offset,
            plot_area,
            &render_window.source_ranges,
            &sub_panel_rects,
            layout.main_panel.y_scale,
            data.revision(),
        );

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

    /// Save a Canvas 2D command-stream HTML document.
    ///
    /// Canvas avoids creating one browser DOM node per primitive and is the
    /// preferred static browser output for dense charts. The existing HTML
    /// renderer remains available when DOM/SVG interaction is required.
    pub fn save_as_canvas_html(&self, path: &str) -> Result<()> {
        let html_string = self.to_canvas_html_string()?;
        std::fs::write(path, html_string).map_err(|e| VisualizationError::RenderError {
            message: format!("Failed to write Canvas HTML file: {}", e),
        })
    }

    /// Render the current draw list using the Canvas 2D command backend.
    pub fn to_canvas_html_string(&self) -> Result<String> {
        let layout = self
            .layout
            .as_ref()
            .ok_or_else(|| VisualizationError::RenderError {
                message: "Layout not calculated. Call build_draw_list first.".to_string(),
            })?;
        let renderer = CanvasRenderer::new();
        if let Some(data) = self.data.as_ref() {
            let render_window = self.render_window(data);
            renderer.render_with_data_and_context(
                &self.draw_list,
                &self.config,
                &render_window.data,
                render_window.source_offset,
                &render_window.source_ranges,
                layout.main_panel.plot_area,
                &self.scene,
                &self.indicator_configs,
                &self.custom_indicator_series,
            )
        } else {
            renderer.render(&self.draw_list, &self.config)
        }
    }

    /// Save a WebGL2-instanced HTML document for dense price/volume charts.
    #[cfg(feature = "html")]
    pub fn save_as_webgl_html(&self, path: &str) -> Result<()> {
        let html_string = self.to_webgl_html_string()?;
        std::fs::write(path, html_string).map_err(|e| VisualizationError::RenderError {
            message: format!("Failed to write WebGL HTML file: {e}"),
        })
    }

    /// Save an explicitly WebGPU-preferred HTML document for dense charts.
    #[cfg(feature = "html")]
    pub fn save_as_webgpu_html(&self, path: &str) -> Result<()> {
        let html_string = self.to_webgpu_html_string()?;
        std::fs::write(path, html_string).map_err(|e| VisualizationError::RenderError {
            message: format!("Failed to write WebGPU HTML file: {e}"),
        })
    }

    /// Render the current chart through the WebGL2 GPU fast path.
    ///
    /// Repeated OHLCV geometry is drawn with instancing; background, labels,
    /// indicators and Chan primitives remain on a transparent 2D overlay so
    /// the semantic scene and existing fallback renderers stay unchanged.
    #[cfg(feature = "html")]
    pub fn to_webgl_html_string(&self) -> Result<String> {
        let layout = self
            .layout
            .as_ref()
            .ok_or_else(|| VisualizationError::RenderError {
                message: "Layout not calculated. Call build_draw_list first.".to_string(),
            })?;
        let data = self.data.as_ref().ok_or(VisualizationError::EmptyData)?;
        let render_window = self.render_window(data);
        WebGlRenderer::new().render_with_layers_and_indicators(
            &self.config,
            &render_window.data,
            render_window.source_offset,
            &render_window.source_ranges,
            layout.main_panel.plot_area,
            layout
                .sub_panels
                .first()
                .filter(|_| self.config.show_volume && self.layer_visible("volume"))
                .map(|panel| panel.plot_area),
            &self.scene,
            &self.render_cache.background_draw_list,
            &self.render_cache.kline_overlay_draw_list,
            &self.render_cache.indicator_overlay_draw_list,
            &self.indicator_configs,
            &self.custom_indicator_series,
        )
    }

    /// Render an explicitly WebGPU-preferred HTML document with WebGL2 and
    /// Canvas 2D fallback. The default `to_webgl_html_string` never probes
    /// WebGPU.
    #[cfg(feature = "html")]
    pub fn to_webgpu_html_string(&self) -> Result<String> {
        let layout = self
            .layout
            .as_ref()
            .ok_or_else(|| VisualizationError::RenderError {
                message: "Layout not calculated. Call build_draw_list first.".to_string(),
            })?;
        let data = self.data.as_ref().ok_or(VisualizationError::EmptyData)?;
        let render_window = self.render_window(data);
        WebGlRenderer::new().render_with_layers_and_indicators_prefer_webgpu(
            &self.config,
            &render_window.data,
            render_window.source_offset,
            &render_window.source_ranges,
            layout.main_panel.plot_area,
            layout
                .sub_panels
                .first()
                .filter(|_| self.config.show_volume && self.layer_visible("volume"))
                .map(|panel| panel.plot_area),
            &self.scene,
            &self.render_cache.background_draw_list,
            &self.render_cache.kline_overlay_draw_list,
            &self.render_cache.indicator_overlay_draw_list,
            &self.indicator_configs,
            &self.custom_indicator_series,
            true,
        )
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
        if let Some(data) = self.data.as_ref() {
            let render_window = self.render_window(data);
            renderer.render_with_data_and_ranges_and_indicators(
                &self.draw_list,
                &self.config,
                &render_window.data,
                render_window.source_offset,
                &render_window.source_ranges,
                &self.scene,
                &self.indicator_configs,
                &self.custom_indicator_series,
            )
        } else {
            renderer.render(&self.draw_list, &self.config)
        }
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

    pub fn to_json_string(&self) -> Result<String> {
        let _layout = self
            .layout
            .as_ref()
            .ok_or_else(|| VisualizationError::RenderError {
                message: "Layout not calculated. Call build_draw_list first.".to_string(),
            })?;
        if let Some(data) = self.data.as_ref() {
            let render_window = self.render_window(data);
            JsonRenderer::new().render_scene_with_data(
                &self.draw_list,
                &self.config,
                &self.scene,
                &render_window.data,
                render_window.source_offset,
                &render_window.source_ranges,
            )
        } else {
            JsonRenderer::new().render_scene(&self.draw_list, &self.config, &self.scene)
        }
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
    fn test_overview_lod_aggregates_visible_window_and_preserves_source_ranges() {
        let data = KlineData::new(
            (0..100).map(|index| index.to_string()).collect(),
            (0..100).map(|index| index as f64).collect(),
            (0..100).map(|index| index as f64 + 2.0).collect(),
            (0..100).map(|index| index as f64 - 1.0).collect(),
            (0..100).map(|index| index as f64 + 1.0).collect(),
            vec![10.0; 100],
        );
        let mut chart = KlineChart::new(ChartConfig::default());
        chart.set_lod_policy(LodPolicy::Fixed(LodLevel::Overview));
        chart.set_viewport(Viewport::new(20, 80).with_pixels(20, 600));
        let window = chart.render_window(&data);
        assert!(window.data.len() < 60);
        assert_eq!(window.source_ranges.first(), Some(&(20, 22)));
        assert_eq!(window.source_ranges.last().map(|range| range.1), Some(80));
    }

    #[test]
    fn test_scene_hit_regions_keep_raw_indices_after_viewport_render() {
        let data = KlineData::new(
            (0..30).map(|index| index.to_string()).collect(),
            vec![10.0; 30],
            vec![11.0; 30],
            vec![9.0; 30],
            vec![10.5; 30],
            vec![100.0; 30],
        );
        let mut chart = KlineChart::new(ChartConfig::default());
        chart.set_viewport(Viewport::new(10, 20).with_pixels(1200, 600));
        chart
            .build_draw_list(&data, &[])
            .expect("viewport chart should render");
        let first = chart
            .scene()
            .hit_regions
            .iter()
            .find_map(|region| match &region.target {
                HitTarget::Kline { index } => Some(*index),
                _ => None,
            });
        assert_eq!(first, Some(10));
    }

    #[test]
    fn test_layer_visibility_can_be_set_before_first_render() {
        let mut chart = KlineChart::new(ChartConfig::default());
        assert!(chart.set_layer_visible("price", false));
        chart
            .build_draw_list(&make_test_data(), &[])
            .expect("hidden price layer should still render other layers");
        assert!(!chart.scene().is_layer_visible("price"));
    }

    #[test]
    fn test_viewport_rerender_keeps_configured_indicators() {
        let data = KlineData::new(
            (0..80).map(|index| index.to_string()).collect(),
            (0..80).map(|index| index as f64).collect(),
            (0..80).map(|index| index as f64 + 1.0).collect(),
            (0..80).map(|index| index as f64 - 1.0).collect(),
            (0..80).map(|index| index as f64 + 0.5).collect(),
            vec![100.0; 80],
        );
        let mut chart = KlineChart::new(ChartConfig::default());
        chart
            .build_draw_list(&data, &[IndicatorConfig::new(IndicatorType::MA, vec![5.0])])
            .expect("indicator chart should render");
        chart.set_viewport(Viewport::new(40, 80).with_pixels(800, 600));
        let rerendered = chart
            .render_incremental()
            .expect("viewport rerender should succeed");
        assert!(rerendered
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Path { .. })));
    }

    #[test]
    fn test_custom_indicator_series_renders_and_validates_alignment() {
        let data = make_test_data();
        let values: Vec<f64> = data.closes.iter().map(|value| value + 0.25).collect();
        let mut chart = KlineChart::new(ChartConfig::default());
        chart.set_data(data.clone());
        chart
            .set_custom_indicator_series("my_line", values.clone())
            .expect("aligned custom series");
        chart
            .build_draw_list(
                &data,
                &[IndicatorConfig::new(
                    IndicatorType::Custom("my_line".to_string()),
                    vec![],
                )],
            )
            .expect("custom series should render");
        assert!(chart
            .draw_list()
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Path { .. })));
        assert!(chart.set_custom_indicator_series("bad", vec![1.0]).is_err());
    }

    #[test]
    fn test_append_kline_keeps_custom_series_aligned() {
        let data = make_test_data();
        let mut chart = KlineChart::new(ChartConfig::default());
        chart.set_data(data.clone());
        chart
            .set_custom_indicator_series("my_line", data.closes.clone())
            .expect("aligned custom series");
        chart
            .append_kline("next", 100.0, 102.0, 99.0, 101.0, 2000.0)
            .expect("append should succeed");
        let appended = chart.data().expect("data after append").clone();
        chart
            .build_draw_list(
                &appended,
                &[IndicatorConfig::new(
                    IndicatorType::Custom("my_line".to_string()),
                    vec![],
                )],
            )
            .expect("NaN warmup value should not break custom series");
        assert!(chart
            .draw_list()
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Path { .. })));
    }

    #[test]
    fn test_batch_upsert_defers_render_and_preserves_update_kinds() {
        let data = make_test_data();
        let mut chart = KlineChart::new(ChartConfig::default());
        chart.set_data(data);
        let updates = chart
            .upsert_klines(&[
                KlineBar::new("2024-01-15", 109.0, 112.0, 108.0, 111.0, 2_000.0),
                KlineBar::new("2024-01-16", 111.0, 113.0, 110.0, 112.0, 2_200.0),
                KlineBar::new("2024-01-16", 111.0, 114.0, 110.0, 113.0, 2_400.0),
            ])
            .expect("batch upsert should validate");
        assert_eq!(updates.len(), 3);
        assert_eq!(updates[0].kind, KlineUpdateKind::Updated);
        assert_eq!(updates[1].kind, KlineUpdateKind::Appended);
        assert_eq!(updates[2].kind, KlineUpdateKind::Updated);
        assert_eq!(chart.data().expect("data after batch").len(), 11);
    }

    #[test]
    fn test_timestamped_upsert_preserves_exchange_time_index() {
        let mut data = make_test_data();
        assert!(data.set_timestamps(
            (0..data.len())
                .map(|index| 1_700_000_000 + index as i64 * 60)
                .collect()
        ));
        let mut chart = KlineChart::new(ChartConfig::default());
        chart.set_data(data);
        let appended = chart
            .upsert_kline_with_timestamp(1_700_000_600, "next", 100.0, 102.0, 99.0, 101.0, 2_000.0)
            .expect("timestamped append should succeed");
        assert_eq!(appended.kind, KlineUpdateKind::Appended);
        assert_eq!(
            chart.data().expect("data after append").timestamps.last(),
            Some(&1_700_000_600)
        );
        assert!(chart
            .upsert_kline_with_timestamp(1_700_000_599, "older", 100.0, 102.0, 99.0, 101.0, 2_000.0)
            .is_err());
    }

    #[test]
    fn test_render_stats_report_source_and_visible_bars() {
        let data = make_test_data();
        let mut chart = KlineChart::new(ChartConfig::default());
        chart
            .build_draw_list(&data, &[])
            .expect("chart should render");
        let stats = chart.render_stats();
        assert_eq!(stats.source_bars, data.len());
        assert!(stats.rendered_bars > 0);
        assert!(stats.primitive_count > 0);
        assert!(stats.panel_count >= 1);
    }

    #[test]
    fn test_event_marker_renders_and_is_hittable() {
        let data = make_test_data();
        let mut chart = KlineChart::new(ChartConfig::default());
        chart.add_event_marker(
            EventMarker::new(3, "突破")
                .with_value(data.highs[3])
                .with_color(Color::from_hex("#ff00aa")),
        );
        chart
            .build_draw_list(&data, &[])
            .expect("event marker chart should render");
        assert!(chart
            .draw_list()
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Circle { .. })));
        assert!(chart
            .scene()
            .hit_regions
            .iter()
            .any(|region| matches!(&region.target, HitTarget::Event { index: 3, .. })));
    }

    #[test]
    fn test_canvas_data_window_includes_indicators_and_semantic_hits() {
        let data = make_test_data();
        let mut chart = KlineChart::new(ChartConfig::default());
        chart.set_data(data.clone());
        chart
            .set_custom_indicator_series("score", data.closes.clone())
            .expect("custom series should align");
        chart.add_event_marker(EventMarker::new(3, "突破候选"));
        chart
            .build_draw_list(
                &data,
                &[IndicatorConfig::new(
                    IndicatorType::Custom("score".to_string()),
                    vec![],
                )],
            )
            .expect("canvas chart should render");
        let html = chart
            .to_canvas_html_string()
            .expect("canvas data window should render");
        assert!(html.contains("score"));
        assert!(html.contains("突破候选"));
        assert!(html.contains("indicatorRows"));
        assert!(html.contains("var hits="));
    }

    #[cfg(feature = "html")]
    #[test]
    fn test_timestamp_index_reaches_json_and_html_payloads() {
        let mut data = make_test_data();
        let timestamps: Vec<i64> = (0..data.len())
            .map(|index| 1_704_067_200 + index as i64 * 86_400)
            .collect();
        assert!(data.set_timestamps(timestamps));
        let mut chart = KlineChart::new(ChartConfig::default());
        chart
            .build_draw_list(&data, &[])
            .expect("timestamp chart should render");
        let json = chart
            .to_json_string()
            .expect("timestamp JSON should render");
        assert!(json.contains("\"timestamps\":[1704067200"));
        let html = chart
            .to_html_string()
            .expect("timestamp HTML should render");
        assert!(html.contains("\"timestamp\":1704067200"));
        assert!(html.contains("时间戳"));
        let canvas = chart
            .to_canvas_html_string()
            .expect("timestamp Canvas HTML should render");
        assert!(canvas.contains("\"timestamp\":1704067200"));
    }

    #[cfg(feature = "html")]
    #[test]
    fn test_webgl_html_uses_gpu_fast_path_with_semantic_overlay() {
        let data = make_test_data();
        let mut chart = KlineChart::new(ChartConfig::default());
        let indicators = [IndicatorConfig::new(IndicatorType::MA, vec![3.0])];
        chart
            .build_draw_list(&data, &indicators)
            .expect("WebGL chart should render");
        let html = chart
            .to_webgl_html_string()
            .expect("WebGL HTML should render");
        assert!(html.contains("data-renderer=\"webgl2\""));
        assert!(html.contains("if(false){webgpuDraw=await tryWebGpu()}"));
        assert!(html.contains("drawArraysInstanced"));
        assert!(html.contains("navigator.gpu"));
        assert!(html.contains("@vertex fn vsBody"));
        assert!(html.contains("panBy"));
        assert!(html.contains("zoomAt"));
        assert!(html.contains("overlayData.semantic"));
        assert!(html.contains("pointerdown"));
        assert!(html.contains("WebGL2 unavailable"));
        assert!(html.contains("finkit-webgl-tooltip"));
        assert!(html.contains("indicatorRows"));
        assert!(html.contains("MA3"));
        assert!(html.contains("enhancedTooltip"));
    }

    #[cfg(feature = "html")]
    #[test]
    fn test_webgl_keeps_chan_and_event_primitives_on_overlay() {
        let data = make_test_data();
        let mut config = ChartConfig::default();
        config.chan.enabled = true;
        config.chan.min_stroke_bars = 2;
        let mut chart = KlineChart::new(config);
        chart.add_event_marker(EventMarker::new(3, "GPU事件"));
        chart
            .build_draw_list(&data, &[])
            .expect("Chan GPU chart should render");
        let html = chart
            .to_webgpu_html_string()
            .expect("WebGPU chart should render");
        assert!(html.contains("GPU事件"));
        assert!(html.contains("finkit-webgl-overlay"));
        assert!(html.contains("if(true){webgpuDraw=await tryWebGpu()}"));
    }

    #[test]
    fn test_gpu_indicator_overlay_excludes_gpu_volume_primitives() {
        let data = make_test_data();
        let mut config = ChartConfig::default();
        config.show_volume = true;
        config.lod_policy = LodPolicy::Fixed(LodLevel::Raw);
        let mut chart = KlineChart::new(config);
        assert!(chart.set_layer_visible("volume", true));
        chart
            .build_draw_list(&data, &[])
            .expect("volume chart should render");
        assert!(!chart.render_cache.volume_draw_list.is_empty());
        assert_eq!(
            chart.render_cache.indicator_overlay_draw_list.primitives,
            chart.render_cache.indicator_draw_list.primitives
                [chart.render_cache.volume_draw_list.len()..]
        );
    }

    #[test]
    fn test_sar_custom_indicator_routes_to_overlay_renderer() {
        let data = make_test_data();
        let mut chart = KlineChart::new(ChartConfig::default());
        chart
            .build_draw_list(
                &data,
                &[IndicatorConfig::new(
                    IndicatorType::Custom("SAR".to_string()),
                    vec![0.02, 0.2],
                )],
            )
            .expect("SAR custom indicator should render");
        assert!(chart
            .draw_list()
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Circle { .. })));
    }

    #[test]
    fn test_kline_chart_candlestick_draw_list() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Candlestick)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.build_draw_list(&data, &[]).expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
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
        chart.build_draw_list(&data, &[]).expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert!(!chart.draw_list().is_empty());
    }

    #[test]
    fn test_kline_chart_line_draw_list() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Line)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.build_draw_list(&data, &[]).expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert!(!chart.draw_list().is_empty());
    }

    #[test]
    fn test_kline_chart_area_draw_list() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Area)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.build_draw_list(&data, &[]).expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
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
        chart.build_draw_list(&data, &indicators).expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let svg = chart.to_svg_string().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
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
        chart.build_draw_list(&data, &[]).expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
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
        chart.build_draw_list(&data, &[]).expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert!(!chart.layout().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").sub_panels.is_empty());
    }

    #[test]
    fn test_kline_chart_without_volume() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Candlestick)
            .show_volume(false)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.build_draw_list(&data, &[]).expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert!(chart.layout().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").sub_panels.is_empty());
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
        assert_eq!(chart.data().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").len(), 1);
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
        chart.render_incremental().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart
            .append_kline("2024-01-02", 103.0, 108.0, 101.0, 107.0, 1200.0)
            .expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        assert_eq!(chart.data().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").len(), 2);
    }

    #[test]
    fn test_kline_chart_append_kline_no_data() {
        let config = ChartConfig::default();
        let mut chart = KlineChart::new(config);
        let result = chart.append_kline("2024-01-01", 100.0, 105.0, 98.0, 103.0, 1000.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_kline_chart_upsert_revises_live_bar_and_appends_new_bar() {
        let mut chart = KlineChart::new(ChartConfig::default());
        chart.set_data(KlineData::new(
            vec!["2024-01-01".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        ));
        let updated = chart
            .upsert_kline("2024-01-01", 100.0, 108.0, 97.0, 106.0, 1200.0)
            .expect("live bar should update");
        assert_eq!(updated.kind, KlineUpdateKind::Updated);
        assert_eq!(updated.index, 0);
        assert_eq!(chart.data().expect("data").closes[0], 106.0);

        let appended = chart
            .upsert_kline("2024-01-02", 106.0, 109.0, 105.0, 108.0, 1300.0)
            .expect("new bar should append");
        assert_eq!(appended.kind, KlineUpdateKind::Appended);
        assert_eq!(appended.index, 1);
        assert_eq!(chart.data().expect("data").len(), 2);
    }

    #[test]
    fn test_kline_chart_realtime_update_rejects_invalid_ohlcv() {
        let mut chart = KlineChart::new(ChartConfig::default());
        chart.set_data(KlineData::new(
            vec!["2024-01-01".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        ));
        assert!(chart
            .upsert_kline("2024-01-01", 100.0, 99.0, 98.0, 103.0, 1000.0)
            .is_err());
        assert_eq!(chart.data().expect("data").closes[0], 103.0);
    }

    #[test]
    fn test_kline_chart_realtime_update_rejects_out_of_order_date() {
        let mut chart = KlineChart::new(ChartConfig::default());
        chart.set_data(KlineData::new(
            vec!["2024-01-02".to_string()],
            vec![100.0],
            vec![105.0],
            vec![98.0],
            vec![103.0],
            vec![1000.0],
        ));
        assert!(chart
            .upsert_kline("2024-01-01", 100.0, 105.0, 98.0, 103.0, 1000.0)
            .is_err());
        assert_eq!(chart.data().expect("data").len(), 1);
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
        chart.render_incremental().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart
            .update_last_kline(106.0, Some(110.0), Some(97.0), Some(1500.0))
            .expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let d = chart.data().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
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
        assert!(!result.expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").is_empty());
    }

    #[test]
    fn test_kline_chart_render_incremental_partial() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Candlestick)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.set_data(data);
        chart.render_incremental().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart
            .update_last_kline(112.0, Some(113.0), Some(108.0), Some(2000.0))
            .expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let result = chart.render_incremental();
        assert!(result.is_ok());
        assert!(!result.expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").is_empty());
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
        chart.render_incremental().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart
            .append_kline("2024-01-16", 110.0, 113.0, 108.0, 112.0, 1800.0)
            .expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let result = chart.render_incremental();
        assert!(result.is_ok());
        assert_eq!(chart.data().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)").len(), 11);
    }

    #[test]
    fn test_kline_chart_render_incremental_ohlc_bar() {
        let config = ChartConfigBuilder::new()
            .with_chart_type(ChartType::Bar)
            .build();
        let mut chart = KlineChart::new(config);
        let data = make_test_data();
        chart.set_data(data);
        chart.render_incremental().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart.update_last_kline(112.0, None, None, None).expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
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
        chart.render_incremental().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart.update_last_kline(112.0, None, None, None).expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
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
        chart.render_incremental().expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");

        chart.update_last_kline(112.0, None, None, None).expect("finkit-visualization: unexpected None/Err in visualization/src/chart/mod.rs (A5 governance)");
        let result = chart.render_incremental();
        assert!(result.is_ok());
    }

    #[test]
    fn test_large_candlestick_chart_bounds_rendered_bars() {
        let n = 10_000;
        let mut data = KlineData::new(
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
        );
        for i in 0..n {
            let close = 100.0 + (i as f64 * 0.01).sin();
            data.push(
                format!("2024-01-{}", i + 1),
                close - 0.5,
                close + 1.0,
                close - 1.0,
                close,
                1_000.0,
            );
        }

        let config = ChartConfigBuilder::new()
            .with_dimensions(200, 600)
            .show_volume(false)
            .build();
        let mut chart = KlineChart::new(config);
        chart
            .build_draw_list(&data, &[])
            .expect("large chart should render");

        // Auto selects MinMax for OHLC charts, so output remains bounded by
        // the pixel budget instead of producing 20,000 market primitives.
        assert!(chart.render_cache.kline_prim_count <= 2 * 202);
    }

    #[test]
    fn test_build_draw_list_populates_incremental_cache() {
        let data = make_test_data();
        let config = ChartConfigBuilder::new().show_volume(false).build();
        let mut chart = KlineChart::new(config);
        chart.set_data(data.clone());
        chart
            .build_draw_list(&data, &[])
            .expect("initial chart should render");

        let rendered = chart
            .render_incremental()
            .expect("cached chart should render incrementally");
        assert!(!rendered.is_empty());
        assert_eq!(rendered.len(), chart.draw_list().len());
    }
}
