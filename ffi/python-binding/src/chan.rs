//! Chan (缠论) analysis entry points.

use super::*;

#[pyfunction]
#[pyo3(signature = (open, high, low, close, volume, min_stroke_bars=6, variant="standard", fractal_policy="strict", stroke_policy="configurable", center_policy="dynamic", min_stroke_change_ratio=0.0, min_fractal_range_ratio=0.0, signal_min_strength=0.0, center_break_ratio=0.0))]
pub fn chan_analyze(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    min_stroke_bars: usize,
    variant: &str,
    fractal_policy: &str,
    stroke_policy: &str,
    center_policy: &str,
    min_stroke_change_ratio: f64,
    min_fractal_range_ratio: f64,
    signal_min_strength: f64,
    center_break_ratio: f64,
) -> PyResult<Py<PyAny>> {
    let open = open
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let high = high
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let low = low
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let close = close
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let volume = volume
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let mut chan_config = parse_chan_config(
        min_stroke_bars,
        variant,
        fractal_policy,
        stroke_policy,
        center_policy,
    )?;
    chan_config.thresholds.min_stroke_change_ratio = min_stroke_change_ratio;
    chan_config.thresholds.min_fractal_range_ratio = min_fractal_range_ratio;
    chan_config.thresholds.signal_min_strength = signal_min_strength;
    chan_config.thresholds.center_break_ratio = center_break_ratio;
    let analysis = py
        .detach(|| analyze_chan(open, high, low, close, volume, chan_config))
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;

    let result = pyo3::types::PyDict::new(py);
    result.set_item("bar_count", analysis.bars.len())?;
    result.set_item("fractal_count", analysis.fractals.len())?;
    result.set_item("stroke_count", analysis.strokes.len())?;
    result.set_item("segment_count", analysis.segments.len())?;
    result.set_item("center_count", analysis.centers.len())?;
    result.set_item(
        "trend",
        match analysis.trend {
            ::finkit::chan::ChanTrend::Unknown => "unknown",
            ::finkit::chan::ChanTrend::Bullish => "bullish",
            ::finkit::chan::ChanTrend::Bearish => "bearish",
            ::finkit::chan::ChanTrend::Range => "range",
        },
    )?;
    if let Some(fractal) = analysis.developing_fractal {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("index", fractal.index)?;
        item.set_item(
            "kind",
            match fractal.kind {
                FractalKind::Top => "top",
                FractalKind::Bottom => "bottom",
            },
        )?;
        item.set_item("high", fractal.high)?;
        item.set_item("low", fractal.low)?;
        item.set_item("value", fractal.value)?;
        result.set_item("developing_fractal", item)?;
    } else {
        result.set_item("developing_fractal", py.None())?;
    }

    let fractals = pyo3::types::PyList::empty(py);
    for fractal in analysis.fractals {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("index", fractal.index)?;
        item.set_item(
            "kind",
            match fractal.kind {
                FractalKind::Top => "top",
                FractalKind::Bottom => "bottom",
            },
        )?;
        item.set_item("high", fractal.high)?;
        item.set_item("low", fractal.low)?;
        item.set_item("value", fractal.value)?;
        fractals.append(item)?;
    }
    result.set_item("fractals", fractals)?;

    let strokes = pyo3::types::PyList::empty(py);
    for stroke in analysis.strokes {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("start_index", stroke.start.index)?;
        item.set_item("end_index", stroke.end.index)?;
        item.set_item(
            "direction",
            match stroke.direction {
                ::finkit::chan::ChanDirection::Up => "up",
                ::finkit::chan::ChanDirection::Down => "down",
            },
        )?;
        item.set_item("high", stroke.high)?;
        item.set_item("low", stroke.low)?;
        item.set_item("bars", stroke.bars)?;
        item.set_item("change", stroke.change)?;
        item.set_item("slope", stroke.slope)?;
        item.set_item("strength", stroke.strength)?;
        strokes.append(item)?;
    }
    result.set_item("strokes", strokes)?;

    let segments = pyo3::types::PyList::empty(py);
    for segment in analysis.segments {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("start_index", segment.start_index)?;
        item.set_item("end_index", segment.end_index)?;
        item.set_item("start_stroke", segment.start_stroke)?;
        item.set_item("end_stroke", segment.end_stroke)?;
        item.set_item("change", segment.change)?;
        segments.append(item)?;
    }
    result.set_item("segments", segments)?;

    let centers = pyo3::types::PyList::empty(py);
    for center in analysis.centers {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("start_index", center.start_index)?;
        item.set_item("end_index", center.end_index)?;
        item.set_item("upper", center.upper)?;
        item.set_item("lower", center.lower)?;
        item.set_item("middle", center.middle)?;
        item.set_item("level", center.level)?;
        item.set_item("start_stroke", center.start_stroke)?;
        item.set_item("end_stroke", center.end_stroke)?;
        centers.append(item)?;
    }
    result.set_item("centers", centers)?;

    let signals = pyo3::types::PyList::empty(py);
    for signal in analysis.signals {
        let item = pyo3::types::PyDict::new(py);
        item.set_item(
            "kind",
            match signal.kind {
                ::finkit::chan::ChanSignalKind::Buy1 => "buy1",
                ::finkit::chan::ChanSignalKind::Buy2 => "buy2",
                ::finkit::chan::ChanSignalKind::Buy3 => "buy3",
                ::finkit::chan::ChanSignalKind::Sell1 => "sell1",
                ::finkit::chan::ChanSignalKind::Sell2 => "sell2",
                ::finkit::chan::ChanSignalKind::Sell3 => "sell3",
            },
        )?;
        item.set_item("index", signal.index)?;
        item.set_item("price", signal.price)?;
        item.set_item("confirmed", signal.confirmed)?;
        item.set_item("strength", signal.strength)?;
        item.set_item("reason", signal.reason)?;
        item.set_item("evidence", signal.evidence.clone())?;
        signals.append(item)?;
    }
    result.set_item("signals", signals)?;

    let divergences = pyo3::types::PyList::empty(py);
    for divergence in analysis.divergences {
        let item = pyo3::types::PyDict::new(py);
        item.set_item(
            "kind",
            match divergence.kind {
                ::finkit::chan::ChanDivergenceKind::Bullish => "bullish",
                ::finkit::chan::ChanDivergenceKind::Bearish => "bearish",
            },
        )?;
        item.set_item("start_stroke", divergence.start_stroke)?;
        item.set_item("end_stroke", divergence.end_stroke)?;
        item.set_item("index", divergence.index)?;
        item.set_item("price", divergence.price)?;
        item.set_item("confirmed", divergence.confirmed)?;
        item.set_item("strength", divergence.strength)?;
        item.set_item("reason", divergence.reason)?;
        divergences.append(item)?;
    }
    result.set_item("divergences", divergences)?;

    Ok(result.into())
}

pub fn parse_chan_config(
    min_stroke_bars: usize,
    variant: &str,
    fractal_policy: &str,
    stroke_policy: &str,
    center_policy: &str,
) -> PyResult<ChanConfig> {
    let variant = match variant.to_ascii_lowercase().as_str() {
        "conservative" | "strict" => ChanVariant::Conservative,
        "standard" | "default" => ChanVariant::Standard,
        "aggressive" | "loose" => ChanVariant::Aggressive,
        value => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "unknown Chan variant: {value}"
            )))
        }
    };
    let fractal_policy = match fractal_policy.to_ascii_lowercase().as_str() {
        "strict" => FractalPolicy::Strict,
        "loose" => FractalPolicy::Loose,
        "right_confirmed" | "right-confirmed" => FractalPolicy::RightConfirmed,
        value => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "unknown fractal policy: {value}"
            )))
        }
    };
    let stroke_policy = match stroke_policy.to_ascii_lowercase().as_str() {
        "configurable" | "min_bars" => StrokePolicy::Configurable,
        "fixed5" | "5" => StrokePolicy::Fixed5,
        "fixed6" | "6" => StrokePolicy::Fixed6,
        "fixed7" | "7" => StrokePolicy::Fixed7,
        "threshold" => StrokePolicy::Threshold,
        value => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "unknown stroke policy: {value}"
            )))
        }
    };
    let center_policy = match center_policy.to_ascii_lowercase().as_str() {
        "three_stroke" | "three-stroke" => CenterPolicy::ThreeStroke,
        "dynamic" => CenterPolicy::Dynamic,
        "hierarchical" | "multi_level" | "multi-level" => CenterPolicy::Hierarchical,
        value => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "unknown center policy: {value}"
            )))
        }
    };
    let mut config = ChanConfig::default().with_variant(variant);
    config.min_stroke_bars = min_stroke_bars;
    config.fractal_policy = fractal_policy;
    config.stroke_policy = stroke_policy;
    config.center_policy = center_policy;
    Ok(config)
}

#[pyfunction]
#[pyo3(signature = (open, high, low, close, volume, factors=None, auto_levels=3, min_frame_bars=20, min_stroke_bars=6, variant="standard"))]
pub fn chan_analyze_multi(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    factors: Option<Vec<usize>>,
    auto_levels: usize,
    min_frame_bars: usize,
    min_stroke_bars: usize,
    variant: &str,
) -> PyResult<Py<PyAny>> {
    let open = open
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let high = high
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let low = low
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let close = close
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let volume = volume
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let config = ChanMultiConfig {
        factors: factors.unwrap_or_default(),
        auto_levels,
        min_frame_bars,
        chan: parse_chan_config(
            min_stroke_bars,
            variant,
            "strict",
            "configurable",
            "dynamic",
        )?,
    };
    let result = py
        .detach(|| analyze_chan_multi(&open, &high, &low, &close, &volume, &config))
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let resonance = result.resonance();
    let frames = pyo3::types::PyList::empty(py);
    for frame in result.frames {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("factor", frame.timeframe.factor)?;
        item.set_item("label", frame.timeframe.label)?;
        item.set_item("seconds", frame.timeframe.seconds)?;
        item.set_item("source_ranges", frame.source_ranges.clone())?;
        item.set_item("analysis", chan_analysis_summary(py, &frame.analysis)?)?;
        frames.append(item)?;
    }
    let output = pyo3::types::PyDict::new(py);
    output.set_item("frames", frames)?;
    output.set_item(
        "resonance_direction",
        match resonance.direction {
            ::finkit::chan_mtf::ChanResonanceDirection::Bullish => "bullish",
            ::finkit::chan_mtf::ChanResonanceDirection::Bearish => "bearish",
            ::finkit::chan_mtf::ChanResonanceDirection::Mixed => "mixed",
            ::finkit::chan_mtf::ChanResonanceDirection::Unknown => "unknown",
        },
    )?;
    output.set_item("resonance_score", resonance.score)?;
    output.set_item("aligned_frames", resonance.aligned_frames)?;
    output.set_item("conflicting_frames", resonance.conflicting_frames)?;
    Ok(output.into())
}

#[pyfunction(name = "chan_analyze_multi_timestamps")]
#[pyo3(signature = (timestamps, open, high, low, close, volume, durations_seconds=None, min_stroke_bars=6, variant="standard", origin_seconds=0))]
pub fn chan_analyze_multi_timestamps_py(
    py: Python<'_>,
    timestamps: PyReadonlyArray1<'_, i64>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    durations_seconds: Option<Vec<i64>>,
    min_stroke_bars: usize,
    variant: &str,
    origin_seconds: i64,
) -> PyResult<Py<PyAny>> {
    let to_vec = |array: &PyReadonlyArray1<'_, f64>| {
        array
            .as_slice()
            .map(|values| values.to_vec())
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))
    };
    let timestamps = timestamps
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let open = to_vec(&open)?;
    let high = to_vec(&high)?;
    let low = to_vec(&low)?;
    let close = to_vec(&close)?;
    let volume = to_vec(&volume)?;
    let config = ChanMultiConfig {
        chan: parse_chan_config(
            min_stroke_bars,
            variant,
            "strict",
            "configurable",
            "dynamic",
        )?,
        ..ChanMultiConfig::default()
    };
    let result = py
        .detach(|| {
            analyze_chan_multi_timestamps(
                &timestamps,
                &open,
                &high,
                &low,
                &close,
                &volume,
                durations_seconds.as_deref().unwrap_or_default(),
                &config,
                origin_seconds,
            )
        })
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let resonance = result.resonance();
    let frames = pyo3::types::PyList::empty(py);
    for frame in result.frames {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("factor", frame.timeframe.factor)?;
        item.set_item("label", frame.timeframe.label)?;
        item.set_item("seconds", frame.timeframe.seconds)?;
        item.set_item("source_ranges", frame.source_ranges)?;
        item.set_item("analysis", chan_analysis_summary(py, &frame.analysis)?)?;
        frames.append(item)?;
    }
    let output = pyo3::types::PyDict::new(py);
    output.set_item("frames", frames)?;
    output.set_item(
        "resonance_direction",
        match resonance.direction {
            ::finkit::chan_mtf::ChanResonanceDirection::Bullish => "bullish",
            ::finkit::chan_mtf::ChanResonanceDirection::Bearish => "bearish",
            ::finkit::chan_mtf::ChanResonanceDirection::Mixed => "mixed",
            ::finkit::chan_mtf::ChanResonanceDirection::Unknown => "unknown",
        },
    )?;
    output.set_item("resonance_score", resonance.score)?;
    output.set_item("aligned_frames", resonance.aligned_frames)?;
    output.set_item("conflicting_frames", resonance.conflicting_frames)?;
    Ok(output.into())
}

#[pyfunction(name = "chan_analyze_multi_timestamps_calendar")]
#[pyo3(signature = (timestamps, open, high, low, close, volume, market="a_share", durations_seconds=None, timezone=None, holidays=None, sessions=None, special_sessions=None, min_stroke_bars=6, variant="standard"))]
pub fn chan_analyze_multi_timestamps_calendar_py(
    py: Python<'_>,
    timestamps: PyReadonlyArray1<'_, i64>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    market: &str,
    durations_seconds: Option<Vec<i64>>,
    timezone: Option<&str>,
    holidays: Option<Vec<String>>,
    sessions: Option<Vec<(u32, u32)>>,
    special_sessions: Option<Vec<(String, Vec<(u32, u32)>)>>,
    min_stroke_bars: usize,
    variant: &str,
) -> PyResult<Py<PyAny>> {
    let to_vec = |array: &PyReadonlyArray1<'_, f64>| {
        array
            .as_slice()
            .map(|values| values.to_vec())
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))
    };
    let timestamps = timestamps
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let open = to_vec(&open)?;
    let high = to_vec(&high)?;
    let low = to_vec(&low)?;
    let close = to_vec(&close)?;
    let volume = to_vec(&volume)?;
    let preset = MarketCalendarPreset::parse(market)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let mut calendar = TradingCalendar::for_market(preset);
    if let Some(timezone) = timezone {
        calendar = calendar.with_timezone(TimeZoneSpec::parse(timezone).map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
        })?);
    }
    if let Some(holidays) = holidays {
        for holiday in holidays {
            calendar.add_holiday(&holiday).map_err(|error| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
            })?;
        }
    }
    if let Some(sessions) = sessions {
        let sessions: Vec<SessionWindow> = sessions
            .into_iter()
            .map(|(open, close)| SessionWindow::new(open, close))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
        calendar = calendar.with_sessions(&sessions);
    }
    if let Some(special_sessions) = special_sessions {
        for (date, sessions) in special_sessions {
            let sessions: Vec<SessionWindow> = sessions
                .into_iter()
                .map(|(open, close)| SessionWindow::new(open, close))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
                })?;
            calendar
                .set_special_sessions(&date, &sessions)
                .map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
                })?;
        }
    }
    let config = ChanMultiConfig {
        chan: parse_chan_config(
            min_stroke_bars,
            variant,
            "strict",
            "configurable",
            "dynamic",
        )?,
        ..ChanMultiConfig::default()
    };
    let result = py
        .detach(|| {
            analyze_chan_multi_timestamps_calendar(
                &timestamps,
                &open,
                &high,
                &low,
                &close,
                &volume,
                durations_seconds.as_deref().unwrap_or_default(),
                &config,
                &calendar,
                0,
            )
        })
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let resonance = result.resonance();
    let frames = pyo3::types::PyList::empty(py);
    for frame in result.frames {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("factor", frame.timeframe.factor)?;
        item.set_item("label", frame.timeframe.label)?;
        item.set_item("seconds", frame.timeframe.seconds)?;
        item.set_item("source_ranges", frame.source_ranges)?;
        item.set_item("analysis", chan_analysis_summary(py, &frame.analysis)?)?;
        frames.append(item)?;
    }
    let output = pyo3::types::PyDict::new(py);
    output.set_item("frames", frames)?;
    output.set_item(
        "resonance_direction",
        match resonance.direction {
            ::finkit::chan_mtf::ChanResonanceDirection::Bullish => "bullish",
            ::finkit::chan_mtf::ChanResonanceDirection::Bearish => "bearish",
            ::finkit::chan_mtf::ChanResonanceDirection::Mixed => "mixed",
            ::finkit::chan_mtf::ChanResonanceDirection::Unknown => "unknown",
        },
    )?;
    output.set_item("resonance_score", resonance.score)?;
    output.set_item("aligned_frames", resonance.aligned_frames)?;
    output.set_item("conflicting_frames", resonance.conflicting_frames)?;
    Ok(output.into())
}

pub fn chan_analysis_summary<'py>(
    py: Python<'py>,
    analysis: &::finkit::chan::ChanAnalysis,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let result = pyo3::types::PyDict::new(py);
    result.set_item("bar_count", analysis.bars.len())?;
    result.set_item("fractal_count", analysis.fractals.len())?;
    result.set_item("stroke_count", analysis.strokes.len())?;
    result.set_item("segment_count", analysis.segments.len())?;
    result.set_item("center_count", analysis.centers.len())?;
    result.set_item(
        "trend",
        match analysis.trend {
            ::finkit::chan::ChanTrend::Unknown => "unknown",
            ::finkit::chan::ChanTrend::Bullish => "bullish",
            ::finkit::chan::ChanTrend::Bearish => "bearish",
            ::finkit::chan::ChanTrend::Range => "range",
        },
    )?;
    let signals = pyo3::types::PyList::empty(py);
    for signal in &analysis.signals {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("index", signal.index)?;
        item.set_item("price", signal.price)?;
        item.set_item("strength", signal.strength)?;
        item.set_item("confirmed", signal.confirmed)?;
        item.set_item("reason", signal.reason.as_str())?;
        item.set_item("evidence", signal.evidence.clone())?;
        signals.append(item)?;
    }
    result.set_item("signals", signals)?;
    let divergences = pyo3::types::PyList::empty(py);
    for divergence in &analysis.divergences {
        let item = pyo3::types::PyDict::new(py);
        item.set_item(
            "kind",
            match divergence.kind {
                ::finkit::chan::ChanDivergenceKind::Bullish => "bullish",
                ::finkit::chan::ChanDivergenceKind::Bearish => "bearish",
            },
        )?;
        item.set_item("start_stroke", divergence.start_stroke)?;
        item.set_item("end_stroke", divergence.end_stroke)?;
        item.set_item("index", divergence.index)?;
        item.set_item("price", divergence.price)?;
        item.set_item("confirmed", divergence.confirmed)?;
        item.set_item("strength", divergence.strength)?;
        item.set_item("reason", divergence.reason.as_str())?;
        divergences.append(item)?;
    }
    result.set_item("divergences", divergences)?;
    Ok(result)
}
