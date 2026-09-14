use finkit::calendar::{MarketCalendarPreset, TimeZoneSpec, TradingCalendar};
use finkit::chan::{ChanConfig, ChanVariant};
use finkit::chan_mtf::ChanMultiConfig;
use finkit_visualization::chart::{EventMarker, KlineChart};
use finkit_visualization::config::{
    ChartConfigBuilder, IndicatorConfig, IndicatorType, InteractionConfig,
};
use finkit_visualization::data::KlineData;
use finkit_visualization::interaction::ReplayState;
use finkit_visualization::language::Language;
use finkit_visualization::primitive::Color;
use finkit_visualization::viewport::{LodLevel, LodPolicy, Viewport};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

fn js_error(error: impl std::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

fn language(value: &str) -> Language {
    match value.to_ascii_lowercase().as_str() {
        "zh" | "zh-cn" | "zhcn" => Language::ZhCn,
        _ => Language::EnUs,
    }
}

fn variant(value: &str) -> ChanVariant {
    match value.to_ascii_lowercase().as_str() {
        "conservative" => ChanVariant::Conservative,
        "aggressive" => ChanVariant::Aggressive,
        _ => ChanVariant::Standard,
    }
}

#[derive(Debug, Clone, Serialize)]
struct ReplayRange {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Serialize)]
struct CalendarSessionWasm {
    session_day: i64,
    session_index: usize,
    open_timestamp: i64,
    close_timestamp: i64,
    source: Option<String>,
    revision: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarSessionWindowInput {
    open_seconds: u32,
    close_seconds: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarSessionOverrideInput {
    date: String,
    sessions: Vec<CalendarSessionWindowInput>,
}

/// Resolve a market session using the same calendar presets and overrides as
/// Rust, Python and Node. `sessions` and `special_sessions` are optional JS
/// arrays of `{openSeconds, closeSeconds}` and
/// `{date, sessions: [...]}` objects, respectively.
#[wasm_bindgen]
pub fn resolve_market_session(
    market: String,
    timestamp: i64,
    timezone: Option<String>,
    holidays: Option<Vec<String>>,
    sessions: Option<JsValue>,
    special_sessions: Option<JsValue>,
) -> Result<JsValue, JsError> {
    let preset = MarketCalendarPreset::parse(&market).map_err(js_error)?;
    let mut calendar = TradingCalendar::for_market(preset);
    if let Some(timezone) = timezone {
        calendar = calendar.with_timezone(TimeZoneSpec::parse(&timezone).map_err(js_error)?);
    }
    for holiday in holidays.unwrap_or_default() {
        calendar.add_holiday(&holiday).map_err(js_error)?;
    }
    if let Some(sessions) = sessions {
        let sessions: Vec<CalendarSessionWindowInput> =
            serde_wasm_bindgen::from_value(sessions).map_err(js_error)?;
        let sessions = sessions
            .into_iter()
            .map(|session| {
                finkit::calendar::SessionWindow::new(session.open_seconds, session.close_seconds)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(js_error)?;
        calendar = calendar.with_sessions(&sessions);
    }
    if let Some(special_sessions) = special_sessions {
        let special_sessions: Vec<CalendarSessionOverrideInput> =
            serde_wasm_bindgen::from_value(special_sessions).map_err(js_error)?;
        for override_item in special_sessions {
            let sessions = override_item
                .sessions
                .into_iter()
                .map(|session| {
                    finkit::calendar::SessionWindow::new(
                        session.open_seconds,
                        session.close_seconds,
                    )
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(js_error)?;
            calendar
                .set_special_sessions(&override_item.date, &sessions)
                .map_err(js_error)?;
        }
    }
    let session = calendar
        .session_for_timestamp_local(timestamp)
        .map_err(js_error)?;
    let value = session.map(|session| CalendarSessionWasm {
        session_day: session.session_day,
        session_index: session.session_index,
        open_timestamp: session.open_timestamp,
        close_timestamp: session.close_timestamp,
        source: calendar.source().map(str::to_string),
        revision: calendar.revision().map(str::to_string),
    });
    serde_wasm_bindgen::to_value(&value).map_err(js_error)
}

/// Resolve a session from a versioned JSON exchange-calendar definition.
#[wasm_bindgen]
pub fn resolve_market_session_config(
    config_json: String,
    timestamp: i64,
) -> Result<JsValue, JsError> {
    let calendar = TradingCalendar::from_json(&config_json).map_err(js_error)?;
    let session = calendar
        .session_for_timestamp_local(timestamp)
        .map_err(js_error)?;
    let value = session.map(|session| CalendarSessionWasm {
        session_day: session.session_day,
        session_index: session.session_index,
        open_timestamp: session.open_timestamp,
        close_timestamp: session.close_timestamp,
        source: calendar.source().map(str::to_string),
        revision: calendar.revision().map(str::to_string),
    });
    serde_wasm_bindgen::to_value(&value).map_err(js_error)
}

/// Resolve a session from an exchange-published annual CSV calendar.
#[wasm_bindgen]
pub fn resolve_market_session_csv(
    csv: String,
    market: String,
    timestamp: i64,
    timezone: Option<String>,
) -> Result<JsValue, JsError> {
    let calendar =
        TradingCalendar::from_csv(&csv, Some(&market), timezone.as_deref()).map_err(js_error)?;
    let session = calendar
        .session_for_timestamp_local(timestamp)
        .map_err(js_error)?;
    let value = session.map(|session| CalendarSessionWasm {
        session_day: session.session_day,
        session_index: session.session_index,
        open_timestamp: session.open_timestamp,
        close_timestamp: session.close_timestamp,
        source: calendar.source().map(str::to_string),
        revision: calendar.revision().map(str::to_string),
    });
    serde_wasm_bindgen::to_value(&value).map_err(js_error)
}

/// Browser-facing K-line chart facade backed by the same scene engine as
/// Rust, Python and Node. The exported strings are self-contained and can be
/// used without reimplementing source-index mapping in JavaScript.
#[wasm_bindgen]
pub struct WasmKlineChart {
    inner: KlineChart,
    data: KlineData,
    indicators: Vec<IndicatorConfig>,
    custom_indicators: Vec<(String, Vec<f64>)>,
    replay: ReplayState,
}

#[wasm_bindgen]
impl WasmKlineChart {
    #[wasm_bindgen(constructor)]
    pub fn new(
        dates: Vec<String>,
        opens: Vec<f64>,
        highs: Vec<f64>,
        lows: Vec<f64>,
        closes: Vec<f64>,
        volumes: Vec<f64>,
        language_name: String,
        title: String,
        width: u32,
        height: u32,
    ) -> Result<WasmKlineChart, JsError> {
        let data = KlineData::new(dates, opens, highs, lows, closes, volumes);
        if !data.validate_ohlcv() {
            return Err(js_error(data.validation_errors().join("; ")));
        }
        let config = ChartConfigBuilder::new()
            .with_title(&title)
            .with_language(language(&language_name))
            .with_dimensions(width, height)
            .build();
        let mut inner = KlineChart::new(config);
        inner.set_data(data.clone());
        let mut chart = WasmKlineChart {
            inner,
            data,
            indicators: Vec::new(),
            custom_indicators: Vec::new(),
            replay: ReplayState::new(0, 200),
        };
        chart.replay = ReplayState::new(chart.data.len(), 200);
        chart.rebuild()?;
        Ok(chart)
    }

    #[wasm_bindgen(js_name = setViewport)]
    pub fn set_viewport(
        &mut self,
        start: usize,
        end: usize,
        pixel_width: u32,
        pixel_height: u32,
        overscan_bars: usize,
        follow_latest: bool,
    ) -> Result<(), JsError> {
        let viewport = if end == 0 {
            Viewport::full()
        } else {
            Viewport::new(start, end)
                .with_pixels(pixel_width, pixel_height)
                .with_overscan(overscan_bars)
                .with_follow_latest(follow_latest)
        };
        self.inner.set_viewport(viewport);
        self.rebuild()
    }

    #[wasm_bindgen(js_name = setLodPolicy)]
    pub fn set_lod_policy(&mut self, value: String) -> Result<(), JsError> {
        self.inner
            .set_lod_policy(match value.to_ascii_lowercase().as_str() {
                "raw" => LodPolicy::Fixed(LodLevel::Raw),
                "balanced" => LodPolicy::Fixed(LodLevel::Balanced),
                "overview" => LodPolicy::Fixed(LodLevel::Overview),
                "auto" => LodPolicy::Auto,
                other => return Err(js_error(format!("unknown LOD policy: {other}"))),
            });
        self.rebuild()
    }

    #[wasm_bindgen(js_name = setInteraction)]
    pub fn set_interaction(
        &mut self,
        enabled: bool,
        show_crosshair: bool,
        show_data_window: bool,
        enable_pan_zoom: bool,
        enable_keyboard: bool,
    ) {
        self.inner.set_interaction_config(InteractionConfig {
            enabled,
            show_crosshair,
            show_data_window,
            enable_pan_zoom,
            enable_keyboard,
        });
    }

    #[wasm_bindgen(js_name = addMa)]
    pub fn add_ma(&mut self, periods: Vec<u32>) -> Result<(), JsError> {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::MA,
            periods.into_iter().map(f64::from).collect(),
        ));
        self.rebuild()
    }

    #[wasm_bindgen(js_name = addMacd)]
    pub fn add_macd(&mut self, fast: u32, slow: u32, signal: u32) -> Result<(), JsError> {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::MACD,
            vec![f64::from(fast), f64::from(slow), f64::from(signal)],
        ));
        self.rebuild()
    }

    #[wasm_bindgen(js_name = addRsi)]
    pub fn add_rsi(&mut self, period: u32) -> Result<(), JsError> {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::RSI,
            vec![f64::from(period)],
        ));
        self.rebuild()
    }

    #[wasm_bindgen(js_name = addCustomIndicator)]
    pub fn add_custom_indicator(&mut self, name: String, values: Vec<f64>) -> Result<(), JsError> {
        self.set_custom_indicator(name, values)
    }

    /// Replaces or registers a custom indicator series without duplicating the
    /// chart definition. This is the real-time recalculation path.
    #[wasm_bindgen(js_name = setCustomIndicator)]
    pub fn set_custom_indicator(&mut self, name: String, values: Vec<f64>) -> Result<(), JsError> {
        if name.trim().is_empty() || values.len() != self.data.len() {
            return Err(js_error(format!(
                "custom indicator '{}' has {} values, expected {}",
                name,
                values.len(),
                self.data.len()
            )));
        }
        if !self.indicators.iter().any(|indicator| {
            matches!(&indicator.indicator_type, IndicatorType::Custom(existing) if existing == &name)
        }) {
            self.indicators.push(IndicatorConfig::new(
                IndicatorType::Custom(name.clone()),
                Vec::new(),
            ));
        }
        self.custom_indicators.retain(|(key, _)| key != &name);
        self.custom_indicators.push((name.clone(), values.clone()));
        self.inner
            .set_custom_indicator_series(name, values)
            .map_err(js_error)?;
        self.rebuild()
    }

    #[wasm_bindgen(js_name = addEventMarker)]
    pub fn add_event_marker(
        &mut self,
        index: usize,
        label: String,
        value: Option<f64>,
        color: Option<String>,
    ) -> Result<(), JsError> {
        let mut marker = EventMarker::new(index, label)
            .with_color(Color::from_hex(color.as_deref().unwrap_or("#f59e0b")));
        if let Some(value) = value {
            marker = marker.with_value(value);
        }
        self.inner.add_event_marker(marker);
        self.rebuild()
    }

    #[wasm_bindgen(js_name = addChan)]
    pub fn add_chan(
        &mut self,
        min_stroke_bars: usize,
        variant_name: String,
        show_labels: bool,
        show_multi_timeframe_annotations: bool,
    ) -> Result<(), JsError> {
        let chan_variant = variant(&variant_name);
        let mut render_config = self.inner.config().chan.clone();
        render_config.enabled = true;
        render_config.min_stroke_bars = min_stroke_bars.max(1);
        render_config.variant = variant_name;
        render_config.show_labels = show_labels;
        render_config.show_multi_timeframe_annotations = show_multi_timeframe_annotations;
        self.inner.set_chan_render_config(render_config);
        self.inner
            .analyze_and_set_chan(
                ChanConfig {
                    min_stroke_bars: min_stroke_bars.max(1),
                    ..ChanConfig::default()
                }
                .with_variant(chan_variant),
            )
            .map_err(js_error)?;
        self.rebuild()
    }

    #[wasm_bindgen(js_name = addChanMulti)]
    pub fn add_chan_multi(
        &mut self,
        factors: Vec<u32>,
        variant_name: String,
    ) -> Result<(), JsError> {
        let chan_variant = variant(&variant_name);
        let mut render_config = self.inner.config().chan.clone();
        render_config.enabled = true;
        render_config.variant = variant_name;
        render_config.show_multi_timeframe_annotations = true;
        self.inner.set_chan_render_config(render_config);
        self.inner
            .analyze_and_set_chan_multi(ChanMultiConfig {
                factors: factors.into_iter().map(|factor| factor as usize).collect(),
                chan: ChanConfig::default().with_variant(chan_variant),
                ..ChanMultiConfig::default()
            })
            .map_err(js_error)?;
        self.rebuild()
    }

    #[wasm_bindgen(js_name = setChanThresholds)]
    pub fn set_chan_thresholds(
        &mut self,
        min_stroke_change_ratio: f64,
        min_fractal_range_ratio: f64,
        signal_min_strength: f64,
        center_break_ratio: f64,
    ) -> Result<(), JsError> {
        let mut render_config = self.inner.config().chan.clone();
        render_config.min_stroke_change_ratio = min_stroke_change_ratio;
        render_config.min_fractal_range_ratio = min_fractal_range_ratio;
        render_config.signal_min_strength = signal_min_strength;
        render_config.center_break_ratio = center_break_ratio;
        self.inner.set_chan_render_config(render_config);
        if self.inner.config().chan.enabled {
            let analysis = finkit_visualization::chart::chan::analyze_configured(
                &self.data,
                self.inner.config(),
            )
            .map_err(js_error)?;
            self.inner.set_chan_analysis(analysis);
        }
        self.rebuild()
    }

    #[wasm_bindgen(js_name = upsertKline)]
    pub fn upsert_kline(
        &mut self,
        date: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Result<String, JsError> {
        let update = self
            .inner
            .upsert_kline(&date, open, high, low, close, volume)
            .map_err(js_error)?;
        self.data = self
            .inner
            .data()
            .cloned()
            .ok_or_else(|| js_error("chart has no data"))?;
        if matches!(
            update.kind,
            finkit_visualization::chart::KlineUpdateKind::Appended
        ) {
            for (_, values) in &mut self.custom_indicators {
                values.push(f64::NAN);
            }
        }
        self.rebuild()?;
        Ok(match update.kind {
            finkit_visualization::chart::KlineUpdateKind::Appended => "appended",
            finkit_visualization::chart::KlineUpdateKind::Updated => "updated",
        }
        .to_string())
    }

    /// Apply a live quote while preserving the exchange timestamp used by
    /// calendar-aware axes and floating data windows.
    #[wasm_bindgen(js_name = upsertKlineTimestamped)]
    pub fn upsert_kline_timestamped(
        &mut self,
        timestamp: i64,
        date: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Result<String, JsError> {
        let update = self
            .inner
            .upsert_kline_with_timestamp(timestamp, &date, open, high, low, close, volume)
            .map_err(js_error)?;
        self.data = self
            .inner
            .data()
            .cloned()
            .ok_or_else(|| js_error("chart has no data"))?;
        if matches!(
            update.kind,
            finkit_visualization::chart::KlineUpdateKind::Appended
        ) {
            for (_, values) in &mut self.custom_indicators {
                values.push(f64::NAN);
            }
        }
        self.rebuild()?;
        Ok(match update.kind {
            finkit_visualization::chart::KlineUpdateKind::Appended => "appended",
            finkit_visualization::chart::KlineUpdateKind::Updated => "updated",
        }
        .to_string())
    }

    #[wasm_bindgen(js_name = upsertKlines)]
    pub fn upsert_klines(
        &mut self,
        dates: Vec<String>,
        opens: Vec<f64>,
        highs: Vec<f64>,
        lows: Vec<f64>,
        closes: Vec<f64>,
        volumes: Vec<f64>,
    ) -> Result<Vec<String>, JsError> {
        let lengths = [
            dates.len(),
            opens.len(),
            highs.len(),
            lows.len(),
            closes.len(),
            volumes.len(),
        ];
        if lengths.windows(2).any(|pair| pair[0] != pair[1]) {
            return Err(js_error("batch quote columns must have equal lengths"));
        }
        let bars: Vec<finkit_visualization::data::KlineBar> = dates
            .into_iter()
            .zip(opens)
            .zip(highs)
            .zip(lows)
            .zip(closes)
            .zip(volumes)
            .map(|(((((date, open), high), low), close), volume)| {
                finkit_visualization::data::KlineBar::new(date, open, high, low, close, volume)
            })
            .collect();
        let updates = self.inner.upsert_klines(&bars).map_err(js_error)?;
        self.data = self
            .inner
            .data()
            .cloned()
            .ok_or_else(|| js_error("chart has no data"))?;
        for update in &updates {
            if matches!(
                update.kind,
                finkit_visualization::chart::KlineUpdateKind::Appended
            ) {
                for (_, values) in &mut self.custom_indicators {
                    values.push(f64::NAN);
                }
            }
        }
        self.rebuild()?;
        Ok(updates
            .into_iter()
            .map(|update| match update.kind {
                finkit_visualization::chart::KlineUpdateKind::Appended => "appended".to_string(),
                finkit_visualization::chart::KlineUpdateKind::Updated => "updated".to_string(),
            })
            .collect())
    }

    #[wasm_bindgen(js_name = setReplayWindow)]
    pub fn set_replay_window(
        &mut self,
        window: usize,
        cursor: Option<usize>,
    ) -> Result<JsValue, JsError> {
        self.replay.window = window.max(1);
        if let Some(cursor) = cursor {
            self.replay.seek(cursor, self.data.len());
        } else {
            self.replay.reset(self.data.len());
        }
        self.apply_replay_view()?;
        self.range_value()
    }

    #[wasm_bindgen(js_name = replayNext)]
    pub fn replay_next(&mut self) -> Result<JsValue, JsError> {
        if self.replay.advance(self.data.len()).is_none() {
            return Ok(JsValue::NULL);
        }
        self.apply_replay_view()?;
        self.range_value()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&mut self) -> Result<String, JsError> {
        self.rebuild()?;
        self.inner.to_json_string().map_err(js_error)
    }

    #[wasm_bindgen(js_name = toHtml)]
    pub fn to_html(&mut self) -> Result<String, JsError> {
        self.rebuild()?;
        self.inner.to_html_string().map_err(js_error)
    }

    #[wasm_bindgen(js_name = toCanvasHtml)]
    pub fn to_canvas_html(&mut self) -> Result<String, JsError> {
        self.rebuild()?;
        self.inner.to_canvas_html_string().map_err(js_error)
    }

    #[wasm_bindgen(js_name = toWebglHtml)]
    pub fn to_webgl_html(&mut self) -> Result<String, JsError> {
        self.rebuild()?;
        self.inner.to_webgl_html_string().map_err(js_error)
    }

    #[wasm_bindgen(js_name = toWebgpuHtml)]
    pub fn to_webgpu_html(&mut self) -> Result<String, JsError> {
        self.rebuild()?;
        self.inner.to_webgpu_html_string().map_err(js_error)
    }

    #[wasm_bindgen(js_name = renderStats)]
    pub fn render_stats(&mut self) -> Result<JsValue, JsError> {
        self.rebuild()?;
        let stats = self.inner.render_stats();
        serde_wasm_bindgen::to_value(&RenderStats {
            source_bars: stats.source_bars,
            rendered_bars: stats.rendered_bars,
            primitive_count: stats.primitive_count,
            panel_count: stats.panel_count,
            hit_region_count: stats.hit_region_count,
        })
        .map_err(js_error)
    }
}

#[derive(Debug, Clone, Serialize)]
struct RenderStats {
    source_bars: usize,
    rendered_bars: usize,
    primitive_count: usize,
    panel_count: usize,
    hit_region_count: usize,
}

impl WasmKlineChart {
    fn rebuild(&mut self) -> Result<(), JsError> {
        for (name, values) in &self.custom_indicators {
            self.inner
                .set_custom_indicator_series(name.clone(), values.clone())
                .map_err(js_error)?;
        }
        self.inner
            .build_draw_list(&self.data, &self.indicators)
            .map_err(js_error)
    }

    fn apply_replay_view(&mut self) -> Result<(), JsError> {
        let (start, end) = self.replay.visible_range(self.data.len());
        self.inner.set_viewport(Viewport::new(start, end));
        self.rebuild()
    }

    fn range_value(&self) -> Result<JsValue, JsError> {
        let (start, end) = self.replay.visible_range(self.data.len());
        serde_wasm_bindgen::to_value(&ReplayRange { start, end }).map_err(js_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_chart() -> WasmKlineChart {
        WasmKlineChart::new(
            (0..12)
                .map(|index| format!("2024-01-{:02}", index + 1))
                .collect(),
            (0..12).map(|index| 100.0 + index as f64).collect(),
            (0..12).map(|index| 102.0 + index as f64).collect(),
            (0..12).map(|index| 99.0 + index as f64).collect(),
            (0..12).map(|index| 101.0 + index as f64).collect(),
            (0..12).map(|index| 1_000.0 + index as f64).collect(),
            "zh-CN".to_string(),
            "WASM smoke".to_string(),
            960,
            540,
        )
        .expect("sample chart should be valid")
    }

    #[test]
    fn chart_facade_exports_interactive_backends_and_updates() {
        let mut chart = sample_chart();
        chart.add_ma(vec![3, 5]).unwrap();
        chart.add_macd(3, 6, 2).unwrap();
        chart.add_rsi(5).unwrap();
        chart
            .add_custom_indicator("score".to_string(), vec![0.5; 12])
            .unwrap();
        chart
            .set_custom_indicator("score".to_string(), vec![0.75; 12])
            .unwrap();
        chart
            .add_event_marker(4, "买入候选".to_string(), Some(105.0), None)
            .unwrap();
        chart
            .add_chan(1, "standard".to_string(), true, true)
            .unwrap();
        chart.set_interaction(true, true, true, true, true);

        let json = chart.to_json().unwrap();
        assert!(json.contains("\"data\""));
        assert!(json.contains("score"));

        let html = chart.to_html().unwrap();
        assert!(html.contains("数据窗口"));
        assert!(html.contains("crosshair"));

        let canvas = chart.to_canvas_html().unwrap();
        assert!(canvas.contains("canvas2d"));
        assert!(canvas.contains("finkit-canvas-tooltip"));

        let webgl = chart.to_webgl_html().unwrap();
        assert!(webgl.contains("data-renderer=\"webgl2\""));
        assert!(webgl.contains("drawArraysInstanced"));
        assert!(webgl.contains("navigator.gpu"));
        assert!(webgl.contains("indicatorRows"));
        assert!(webgl.contains("score"));
        assert!(webgl.contains("买入候选"));
        assert!(webgl.contains("enhancedTooltip"));

        #[cfg(target_arch = "wasm32")]
        {
            assert!(!chart.render_stats().unwrap().is_null());
            assert!(!chart.set_replay_window(4, None).unwrap().is_null());
            assert!(!chart.replay_next().unwrap().is_null());
        }
        assert_eq!(
            chart
                .upsert_kline(
                    "2024-01-12".to_string(),
                    110.0,
                    113.0,
                    109.0,
                    112.0,
                    2_000.0
                )
                .unwrap(),
            "updated"
        );
        assert_eq!(
            chart
                .upsert_kline(
                    "2024-01-13".to_string(),
                    112.0,
                    114.0,
                    111.0,
                    113.0,
                    2_100.0
                )
                .unwrap(),
            "appended"
        );
        assert_eq!(
            chart
                .upsert_klines(
                    vec!["2024-01-13".to_string(), "2024-01-14".to_string()],
                    vec![112.0, 113.0],
                    vec![115.0, 116.0],
                    vec![111.0, 112.0],
                    vec![114.0, 115.0],
                    vec![2_200.0, 2_300.0],
                )
                .unwrap(),
            vec!["updated", "appended"]
        );
    }
}
