//! `#[pyclass]` chart objects and their streaming helpers.

use super::*;

#[pyclass(name = "KlineData")]
#[derive(Clone)]
pub struct PyKlineData {
    inner: KlineData,
}

#[pymethods]
impl PyKlineData {
    #[new]
    #[pyo3(signature = (dates, opens, highs, lows, closes, volumes, timestamps=None))]
    fn new(
        dates: Vec<String>,
        opens: Vec<f64>,
        highs: Vec<f64>,
        lows: Vec<f64>,
        closes: Vec<f64>,
        volumes: Vec<f64>,
        timestamps: Option<Vec<i64>>,
    ) -> PyResult<Self> {
        let mut inner = KlineData::new(dates, opens, highs, lows, closes, volumes);
        if let Some(timestamps) = timestamps {
            if !inner.set_timestamps(timestamps) {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "timestamps must be strictly increasing and match data length",
                ));
            }
        }
        Ok(Self { inner })
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn validate(&self) -> bool {
        self.inner.validate()
    }

    fn validate_ohlcv(&self) -> bool {
        self.inner.validate_ohlcv()
    }

    fn validation_errors(&self) -> Vec<String> {
        self.inner.validation_errors()
    }

    fn set_timestamps(&mut self, timestamps: Vec<i64>) -> PyResult<()> {
        if self.inner.set_timestamps(timestamps) {
            Ok(())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "timestamps must be strictly increasing and match data length",
            ))
        }
    }

    fn push(&mut self, date: String, open: f64, high: f64, low: f64, close: f64, volume: f64) {
        self.inner.push(date, open, high, low, close, volume);
    }

    fn push_timestamped(
        &mut self,
        timestamp: i64,
        date: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> PyResult<()> {
        if self
            .inner
            .push_timestamped(timestamp, date, open, high, low, close, volume)
        {
            Ok(())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "timestamped push requires a complete, strictly increasing timestamp index",
            ))
        }
    }

    #[staticmethod]
    fn from_json(json_str: &str) -> PyResult<Self> {
        KlineData::from_json(json_str)
            .map(|d| Self { inner: d })
            .map_err(convert_vis_error)
    }

    #[staticmethod]
    fn from_csv(csv_str: &str) -> PyResult<Self> {
        KlineData::from_csv(csv_str)
            .map(|d| Self { inner: d })
            .map_err(convert_vis_error)
    }

    #[getter]
    fn dates(&self) -> Vec<String> {
        self.inner.dates().to_vec()
    }

    #[getter]
    fn opens(&self) -> Vec<f64> {
        self.inner.opens().to_vec()
    }

    #[getter]
    fn highs(&self) -> Vec<f64> {
        self.inner.highs().to_vec()
    }

    #[getter]
    fn lows(&self) -> Vec<f64> {
        self.inner.lows().to_vec()
    }

    #[getter]
    fn closes(&self) -> Vec<f64> {
        self.inner.closes().to_vec()
    }

    #[getter]
    fn volumes(&self) -> Vec<f64> {
        self.inner.volumes().to_vec()
    }

    #[getter]
    fn timestamps(&self) -> Vec<i64> {
        self.inner.timestamps.clone()
    }
}

pub fn validate_stream_bar(
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
) -> PyResult<()> {
    let candidate = KlineData::new(
        vec!["live".to_string()],
        vec![open],
        vec![high],
        vec![low],
        vec![close],
        vec![volume],
    );
    let errors = candidate.validation_errors();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            errors.join("; "),
        ))
    }
}

#[pyclass(name = "KlineChart")]
pub struct PyKlineChart {
    data: PyKlineData,
    indicators: Vec<IndicatorConfig>,
    custom_indicator_series: Vec<(String, Vec<f64>)>,
    event_markers: Vec<EventMarker>,
    config: ChartConfig,
    viewport: Viewport,
    replay: ReplayState,
    lod_policy: LodPolicy,
    hidden_layers: Vec<String>,
    chan_multi: Option<ChanMultiConfig>,
}

#[pymethods]
impl PyKlineChart {
    #[new]
    #[pyo3(signature = (data, language="zh", title="", width=1200, height=600))]
    fn new(
        data: PyKlineData,
        language: &str,
        title: &str,
        width: u32,
        height: u32,
    ) -> PyResult<Self> {
        let lang = match language {
            "en" => Language::EnUs,
            _ => Language::ZhCn,
        };
        let config = ChartConfigBuilder::new()
            .with_title(title)
            .with_language(lang)
            .with_dimensions(width, height)
            .build();
        let data_len = data.inner.len();
        Ok(Self {
            data,
            indicators: Vec::new(),
            custom_indicator_series: Vec::new(),
            event_markers: Vec::new(),
            config,
            viewport: Viewport::full(),
            replay: ReplayState::new(data_len, 200),
            lod_policy: LodPolicy::Auto,
            hidden_layers: Vec::new(),
            chan_multi: None,
        })
    }

    /// Restrict rendering to a source-index window. `end=0` restores full data.
    #[pyo3(signature = (start=0, end=0, pixel_width=1200, pixel_height=600, overscan_bars=0, follow_latest=false))]
    fn set_viewport(
        &mut self,
        start: usize,
        end: usize,
        pixel_width: u32,
        pixel_height: u32,
        overscan_bars: usize,
        follow_latest: bool,
    ) {
        self.viewport = if end == 0 {
            Viewport::full()
        } else {
            Viewport::new(start, end)
                .with_pixels(pixel_width, pixel_height)
                .with_overscan(overscan_bars)
                .with_follow_latest(follow_latest)
        };
    }

    /// Append a validated bar to the live chart source.
    fn append_kline(
        &mut self,
        date: &str,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> PyResult<()> {
        validate_stream_bar(open, high, low, close, volume)?;
        self.data
            .inner
            .push(date.to_string(), open, high, low, close, volume);
        for (_, values) in &mut self.custom_indicator_series {
            values.push(f64::NAN);
        }
        Ok(())
    }

    /// Replace the current live bar, expanding high/low when omitted.
    #[pyo3(signature = (close, high=None, low=None, volume=None))]
    fn update_last_kline(
        &mut self,
        close: f64,
        high: Option<f64>,
        low: Option<f64>,
        volume: Option<f64>,
    ) -> PyResult<()> {
        if self.data.inner.is_empty() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "cannot update an empty chart",
            ));
        }
        let index = self.data.inner.len() - 1;
        let next_high = high.unwrap_or(self.data.inner.highs[index].max(close));
        let next_low = low.unwrap_or(self.data.inner.lows[index].min(close));
        let next_volume = volume.unwrap_or(self.data.inner.volumes[index]);
        validate_stream_bar(
            self.data.inner.opens[index],
            next_high,
            next_low,
            close,
            next_volume,
        )?;
        self.data.inner.highs[index] = next_high;
        self.data.inner.lows[index] = next_low;
        self.data.inner.closes[index] = close;
        self.data.inner.volumes[index] = next_volume;
        self.data.inner.bump_revision();
        for (_, values) in &mut self.custom_indicator_series {
            if values.len() > index {
                values[index] = f64::NAN;
            }
        }
        Ok(())
    }

    /// Append a new bar or revise the current bar when the date is repeated.
    fn upsert_kline(
        &mut self,
        date: &str,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> PyResult<String> {
        let same_date = self
            .data
            .inner
            .dates
            .last()
            .is_some_and(|last| last == date);
        if same_date {
            validate_stream_bar(open, high, low, close, volume)?;
            let index = self.data.inner.len() - 1;
            self.data.inner.opens[index] = open;
            self.data.inner.highs[index] = high;
            self.data.inner.lows[index] = low;
            self.data.inner.closes[index] = close;
            self.data.inner.volumes[index] = volume;
            self.data.inner.bump_revision();
            for (_, values) in &mut self.custom_indicator_series {
                if values.len() > index {
                    values[index] = f64::NAN;
                }
            }
            Ok("updated".to_string())
        } else {
            self.append_kline(date, open, high, low, close, volume)?;
            Ok("appended".to_string())
        }
    }

    /// Timestamp-preserving variant for exchange-calendar-aware live feeds.
    fn upsert_kline_timestamped(
        &mut self,
        timestamp: i64,
        date: &str,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> PyResult<String> {
        validate_stream_bar(open, high, low, close, volume)?;
        if self.data.inner.timestamps.len() != self.data.inner.len()
            || self
                .data
                .inner
                .timestamps
                .last()
                .is_some_and(|last| timestamp < *last)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "timestamp must extend the existing timestamp index",
            ));
        }
        let same_date = self
            .data
            .inner
            .dates
            .last()
            .is_some_and(|last| last == date);
        if same_date {
            let index = self.data.inner.len() - 1;
            self.data.inner.opens[index] = open;
            self.data.inner.highs[index] = high;
            self.data.inner.lows[index] = low;
            self.data.inner.closes[index] = close;
            self.data.inner.volumes[index] = volume;
            self.data.inner.timestamps[index] = timestamp;
            self.data.inner.bump_revision();
            for (_, values) in &mut self.custom_indicator_series {
                if values.len() > index {
                    values[index] = f64::NAN;
                }
            }
            Ok("updated".to_string())
        } else if self.data.inner.push_timestamped(
            timestamp,
            date.to_string(),
            open,
            high,
            low,
            close,
            volume,
        ) {
            for (_, values) in &mut self.custom_indicator_series {
                values.push(f64::NAN);
            }
            Ok("appended".to_string())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "timestamped update must extend the existing timestamp index",
            ))
        }
    }

    /// Apply live quotes in one call and defer chart rendering until export.
    fn upsert_klines(
        &mut self,
        updates: Vec<(String, f64, f64, f64, f64, f64)>,
    ) -> PyResult<Vec<String>> {
        updates
            .into_iter()
            .map(|(date, open, high, low, close, volume)| {
                self.upsert_kline(&date, open, high, low, close, volume)
            })
            .collect()
    }

    /// Selects `auto`, `raw`, `balanced` or `overview` level of detail.
    #[pyo3(signature = (level="auto"))]
    fn set_lod_policy(&mut self, level: &str) -> PyResult<()> {
        self.lod_policy = match level.to_ascii_lowercase().as_str() {
            "auto" => LodPolicy::Auto,
            "raw" => LodPolicy::Fixed(LodLevel::Raw),
            "balanced" => LodPolicy::Fixed(LodLevel::Balanced),
            "overview" => LodPolicy::Fixed(LodLevel::Overview),
            value => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "unknown LOD policy: {value}"
                )))
            }
        };
        Ok(())
    }

    /// Set a deterministic replay window and return its resolved source range.
    #[pyo3(signature = (window=200, cursor=None, pixel_width=1200, pixel_height=600, overscan_bars=0))]
    fn set_replay_window(
        &mut self,
        window: usize,
        cursor: Option<usize>,
        pixel_width: u32,
        pixel_height: u32,
        overscan_bars: usize,
    ) -> (usize, usize) {
        self.replay.window = window.max(1);
        if let Some(cursor) = cursor {
            self.replay.seek(cursor, self.data.inner.len());
        } else {
            self.replay.reset(self.data.inner.len());
        }
        let (start, end) = self.replay.visible_range(self.data.inner.len());
        self.viewport = Viewport::new(start, end)
            .with_pixels(pixel_width, pixel_height)
            .with_overscan(overscan_bars);
        (start, end)
    }

    /// Advance the replay by one configured step and return its source range.
    fn replay_next(&mut self) -> Option<(usize, usize)> {
        self.replay.advance(self.data.inner.len())?;
        let range = self.replay.visible_range(self.data.inner.len());
        self.viewport = Viewport::new(range.0, range.1);
        Some(range)
    }

    /// Enables or disables a semantic chart layer.
    fn set_layer_visible(&mut self, layer: &str, visible: bool) {
        if visible {
            self.hidden_layers.retain(|value| value != layer);
        } else if !self.hidden_layers.iter().any(|value| value == layer) {
            self.hidden_layers.push(layer.to_string());
        }
    }

    fn add_ma(&mut self, periods: Vec<usize>) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::MA,
            periods.iter().map(|&p| p as f64).collect(),
        ));
    }

    fn add_ema(&mut self, periods: Vec<usize>) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::EMA,
            periods.iter().map(|&p| p as f64).collect(),
        ));
    }

    #[pyo3(signature = (period=20, nb_dev=2.0))]
    fn add_boll(&mut self, period: usize, nb_dev: f64) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::BOLL,
            vec![period as f64, nb_dev],
        ));
    }

    #[pyo3(signature = (fast=12, slow=26, signal=9))]
    fn add_macd(&mut self, fast: usize, slow: usize, signal: usize) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::MACD,
            vec![fast as f64, slow as f64, signal as f64],
        ));
    }

    #[pyo3(signature = (period=14))]
    fn add_rsi(&mut self, period: usize) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::RSI,
            vec![period as f64],
        ));
    }

    #[pyo3(signature = (fast_k=9, slow_k=3, slow_d=3))]
    fn add_kdj(&mut self, fast_k: usize, slow_k: usize, slow_d: usize) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::KDJ,
            vec![fast_k as f64, slow_k as f64, slow_d as f64],
        ));
    }

    #[pyo3(signature = (acceleration=0.02, maximum=0.2))]
    fn add_sar(&mut self, acceleration: f64, maximum: f64) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::Custom("SAR".to_string()),
            vec![acceleration, maximum],
        ));
    }

    /// Adds a user-supplied indicator line to the chart and the HTML data window.
    /// Values must align one-to-one with the source K-line rows.
    fn add_custom_indicator(&mut self, name: &str, values: Vec<f64>) -> PyResult<()> {
        self.set_custom_indicator_series(name, values)
    }

    /// Replaces or registers a user-supplied indicator line.
    ///
    /// This is the real-time update path: after an append/upsert, callers can
    /// submit the recalculated aligned series without creating duplicate
    /// indicator definitions.
    fn set_custom_indicator_series(&mut self, name: &str, values: Vec<f64>) -> PyResult<()> {
        if name.trim().is_empty() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "custom indicator name must not be empty",
            ));
        }
        if values.len() != self.data.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "custom indicator '{}' has {} values, expected {}",
                name,
                values.len(),
                self.data.inner.len()
            )));
        }
        if !self.indicators.iter().any(|indicator| {
            matches!(&indicator.indicator_type, IndicatorType::Custom(existing) if existing == name)
        }) {
            self.indicators.push(IndicatorConfig::new(
                IndicatorType::Custom(name.to_string()),
                Vec::new(),
            ));
        }
        self.custom_indicator_series
            .retain(|(existing, _)| existing != name);
        self.custom_indicator_series
            .push((name.to_string(), values));
        Ok(())
    }

    /// Adds a semantic event marker to the main chart panel.
    #[pyo3(signature = (index, label, value=None, color="#f59e0b", priority=30))]
    fn add_event_marker(
        &mut self,
        index: usize,
        label: &str,
        value: Option<f64>,
        color: &str,
        priority: i32,
    ) {
        let mut marker = EventMarker::new(index, label)
            .with_color(Color::from_hex(color))
            .with_priority(priority);
        if let Some(value) = value {
            marker = marker.with_value(value);
        }
        self.event_markers.push(marker);
    }

    /// Configures TDX-style crosshair, data-window, pan/zoom and keyboard input.
    #[pyo3(signature = (enabled=true, show_crosshair=true, show_data_window=true, enable_pan_zoom=true, enable_keyboard=true))]
    fn set_interaction(
        &mut self,
        enabled: bool,
        show_crosshair: bool,
        show_data_window: bool,
        enable_pan_zoom: bool,
        enable_keyboard: bool,
    ) {
        self.config.interaction.enabled = enabled;
        self.config.interaction.show_crosshair = show_crosshair;
        self.config.interaction.show_data_window = show_data_window;
        self.config.interaction.enable_pan_zoom = enable_pan_zoom;
        self.config.interaction.enable_keyboard = enable_keyboard;
    }

    /// Enables the Chanlun overlay on the main price panel.
    #[pyo3(signature = (min_stroke_bars=6, show_labels=false, variant="standard", stroke_policy="configurable", center_policy="dynamic", signal_min_strength=0.0, show_multi_timeframe_annotations=false))]
    fn add_chan(
        &mut self,
        min_stroke_bars: usize,
        show_labels: bool,
        variant: &str,
        stroke_policy: &str,
        center_policy: &str,
        signal_min_strength: f64,
        show_multi_timeframe_annotations: bool,
    ) {
        self.config.chan.enabled = true;
        self.config.chan.min_stroke_bars = min_stroke_bars;
        self.config.chan.show_labels = show_labels;
        self.config.chan.variant = variant.to_string();
        self.config.chan.stroke_policy = stroke_policy.to_string();
        self.config.chan.center_policy = center_policy.to_string();
        self.config.chan.signal_min_strength = signal_min_strength;
        self.config.chan.show_multi_timeframe_annotations = show_multi_timeframe_annotations;
        self.chan_multi = None;
    }

    /// Enable automatic or explicit multi-timeframe Chanlun overlays.
    #[pyo3(signature = (factors=None, variant="standard"))]
    fn add_chan_multi(&mut self, factors: Option<Vec<usize>>, variant: &str) -> PyResult<()> {
        self.config.chan.enabled = true;
        self.config.chan.variant = variant.to_string();
        self.config.chan.show_multi_timeframe_annotations = true;
        let mut chan = parse_chan_config(
            self.config.chan.min_stroke_bars,
            variant,
            &self.config.chan.fractal_policy,
            &self.config.chan.stroke_policy,
            &self.config.chan.center_policy,
        )?;
        chan.thresholds.min_stroke_change_ratio = self.config.chan.min_stroke_change_ratio;
        chan.thresholds.min_fractal_range_ratio = self.config.chan.min_fractal_range_ratio;
        chan.thresholds.signal_min_strength = self.config.chan.signal_min_strength;
        chan.thresholds.center_break_ratio = self.config.chan.center_break_ratio;
        self.chan_multi = Some(ChanMultiConfig {
            factors: factors.unwrap_or_default(),
            chan,
            ..ChanMultiConfig::default()
        });
        Ok(())
    }

    /// Configure numeric Chan thresholds without changing the selected variant.
    fn set_chan_thresholds(
        &mut self,
        min_stroke_change_ratio: f64,
        min_fractal_range_ratio: f64,
        signal_min_strength: f64,
        center_break_ratio: f64,
    ) {
        self.config.chan.min_stroke_change_ratio = min_stroke_change_ratio;
        self.config.chan.min_fractal_range_ratio = min_fractal_range_ratio;
        self.config.chan.signal_min_strength = signal_min_strength;
        self.config.chan.center_break_ratio = center_break_ratio;
        if let Some(config) = &mut self.chan_multi {
            config.chan.thresholds.min_stroke_change_ratio = min_stroke_change_ratio;
            config.chan.thresholds.min_fractal_range_ratio = min_fractal_range_ratio;
            config.chan.thresholds.signal_min_strength = signal_min_strength;
            config.chan.thresholds.center_break_ratio = center_break_ratio;
        }
    }

    fn save_as_svg(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        let svg = py.detach(|| -> PyResult<String> {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_svg_string().map_err(convert_vis_error)
        })?;
        std::fs::write(path, svg)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{}", e)))
    }

    fn save_as_html(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        let html = py.detach(|| -> PyResult<String> {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_html_string().map_err(convert_vis_error)
        })?;
        std::fs::write(path, html)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{}", e)))
    }

    /// Save a high-volume Canvas 2D command-stream HTML document.
    fn save_as_canvas_html(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        let html = py.detach(|| -> PyResult<String> {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_canvas_html_string().map_err(convert_vis_error)
        })?;
        std::fs::write(path, html)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{}", e)))
    }

    fn to_svg_string(&self, py: Python<'_>) -> PyResult<String> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        py.detach(|| {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_svg_string().map_err(convert_vis_error)
        })
    }

    /// Return a high-volume Canvas 2D command-stream HTML document.
    fn to_canvas_html(&self, py: Python<'_>) -> PyResult<String> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        py.detach(|| {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_canvas_html_string().map_err(convert_vis_error)
        })
    }

    /// Return an instanced WebGL2 HTML document with a Canvas overlay.
    fn to_webgl_html(&self, py: Python<'_>) -> PyResult<String> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        py.detach(|| {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_webgl_html_string().map_err(convert_vis_error)
        })
    }

    /// Save an instanced WebGL2 HTML document.
    fn save_as_webgl_html(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        let html = self.to_webgl_html(py)?;
        std::fs::write(path, html)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{e}")))
    }

    /// Return an explicitly WebGPU-preferred HTML document.
    fn to_webgpu_html(&self, py: Python<'_>) -> PyResult<String> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        py.detach(|| {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_webgpu_html_string().map_err(convert_vis_error)
        })
    }

    /// Save an explicitly WebGPU-preferred HTML document.
    fn save_as_webgpu_html(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        let html = self.to_webgpu_html(py)?;
        std::fs::write(path, html)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{e}")))
    }
}

impl PyKlineChart {
    fn configure_chart(&self, chart: &mut finkit_visualization::chart::KlineChart) {
        chart.set_data(self.data.inner.clone());
        chart.set_viewport(self.viewport);
        chart.set_lod_policy(self.lod_policy);
        for layer in &self.hidden_layers {
            chart.set_layer_visible(layer, false);
        }
        for (name, values) in &self.custom_indicator_series {
            let _ = chart.set_custom_indicator_series(name.clone(), values.clone());
        }
        for marker in &self.event_markers {
            chart.add_event_marker(marker.clone());
        }
        if let Some(config) = &self.chan_multi {
            let _ = chart.analyze_and_set_chan_multi(config.clone());
        }
    }
}
