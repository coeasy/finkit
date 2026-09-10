//! Automatic multi-timeframe Chanlun analysis.
//!
//! The module aggregates aligned base bars into deterministic factor-based
//! frames.  A factor of `5` means five base bars per higher-timeframe bar.
//! When no factors are supplied, conservative factors are selected from the
//! input length and requested level count.  Structural indices are mapped back
//! to the base-bar coordinate system for cross-frame comparison.

use crate::calendar::TradingCalendar;
use crate::chan::{analyze, ChanAnalysis, ChanConfig, ChanTrend};
use crate::error::{IndicatorError, Result};

/// A named factor-based timeframe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChanTimeframe {
    /// Number of base bars per aggregated bar.
    pub factor: usize,
    /// Stable display label, for example `base`, `x5`, or `x20`.
    pub label: String,
    /// Optional wall-clock bucket size in seconds for timestamp-driven frames.
    pub seconds: Option<i64>,
}

/// Timeframe selection for timestamp-aware multi-period analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChanTimeframeSpec {
    /// Aggregate a fixed number of source bars.
    Bars(usize),
    /// Aggregate timestamps into fixed-width Unix-second buckets.
    Seconds(i64),
}

/// Configuration for multi-timeframe Chanlun analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct ChanMultiConfig {
    /// Explicit factors. Empty means automatic selection.
    pub factors: Vec<usize>,
    /// Number of automatic levels when [`Self::factors`] is empty.
    pub auto_levels: usize,
    /// Minimum aggregated bars required for an automatically selected frame.
    pub min_frame_bars: usize,
    /// Rule configuration shared by every frame.
    pub chan: ChanConfig,
}

impl Default for ChanMultiConfig {
    fn default() -> Self {
        Self {
            factors: Vec::new(),
            auto_levels: 3,
            min_frame_bars: 20,
            chan: ChanConfig::default(),
        }
    }
}

/// One analyzed timeframe, with structure indices mapped to base bars.
#[derive(Debug, Clone, PartialEq)]
pub struct ChanFrame {
    /// Timeframe metadata.
    pub timeframe: ChanTimeframe,
    /// Analysis result for this timeframe.
    pub analysis: ChanAnalysis,
    /// Raw source range represented by each analyzed frame bar.
    pub source_ranges: Vec<(usize, usize)>,
}

/// Complete multi-timeframe analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct ChanMultiAnalysis {
    /// Results ordered from the base frame to higher frames.
    pub frames: Vec<ChanFrame>,
}

/// Cross-timeframe directional agreement summary.
#[derive(Debug, Clone, PartialEq)]
pub struct ChanResonance {
    pub direction: ChanResonanceDirection,
    /// Absolute agreement ratio in `[0, 1]`.
    pub score: f64,
    pub aligned_frames: Vec<String>,
    pub conflicting_frames: Vec<String>,
}

/// Direction inferred from the latest state of all analyzed frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChanResonanceDirection {
    Bullish,
    Bearish,
    Mixed,
    Unknown,
}

impl ChanMultiAnalysis {
    /// Return the base frame, if one was produced.
    pub fn base(&self) -> Option<&ChanFrame> {
        self.frames
            .iter()
            .find(|frame| frame.timeframe.factor == 1 && frame.timeframe.seconds.is_none())
    }

    /// Return a frame by aggregation factor.
    pub fn by_factor(&self, factor: usize) -> Option<&ChanFrame> {
        self.frames
            .iter()
            .find(|frame| frame.timeframe.factor == factor && frame.timeframe.seconds.is_none())
    }

    /// Return a frame by its stable display label.
    pub fn by_label(&self, label: &str) -> Option<&ChanFrame> {
        self.frames
            .iter()
            .find(|frame| frame.timeframe.label == label)
    }

    /// Return a timestamp-driven frame by its bucket duration.
    pub fn by_seconds(&self, seconds: i64) -> Option<&ChanFrame> {
        self.frames
            .iter()
            .find(|frame| frame.timeframe.seconds == Some(seconds))
    }

    /// Summarize whether the available timeframes agree on direction.
    pub fn resonance(&self) -> ChanResonance {
        let mut bullish = Vec::new();
        let mut bearish = Vec::new();
        for frame in &self.frames {
            match frame.analysis.trend {
                ChanTrend::Bullish => bullish.push(frame.timeframe.label.clone()),
                ChanTrend::Bearish => bearish.push(frame.timeframe.label.clone()),
                ChanTrend::Range | ChanTrend::Unknown => {}
            }
        }
        let active = bullish.len() + bearish.len();
        let direction = if active == 0 {
            ChanResonanceDirection::Unknown
        } else if bullish.len() == bearish.len() {
            ChanResonanceDirection::Mixed
        } else if bullish.len() > bearish.len() {
            ChanResonanceDirection::Bullish
        } else {
            ChanResonanceDirection::Bearish
        };
        let score = if active == 0 {
            0.0
        } else {
            bullish.len().max(bearish.len()) as f64 / active as f64
        };
        let aligned_frames = match direction {
            ChanResonanceDirection::Bullish => bullish.clone(),
            ChanResonanceDirection::Bearish => bearish.clone(),
            _ => Vec::new(),
        };
        let conflicting_frames = match direction {
            ChanResonanceDirection::Bullish => bearish,
            ChanResonanceDirection::Bearish => bullish,
            _ => Vec::new(),
        };
        ChanResonance {
            direction,
            score,
            aligned_frames,
            conflicting_frames,
        }
    }
}

/// Analyze one OHLCV series at explicit or automatically selected factors.
pub fn analyze_multi(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    config: &ChanMultiConfig,
) -> Result<ChanMultiAnalysis> {
    validate_lengths(open, high, low, close, volume)?;
    let factors = if config.factors.is_empty() {
        auto_factors(close.len(), config.auto_levels, config.min_frame_bars)
    } else {
        let mut factors = config.factors.clone();
        factors.sort_unstable();
        factors.dedup();
        if !factors.contains(&1) {
            factors.insert(0, 1);
        }
        if factors.iter().any(|factor| *factor == 0) {
            return Err(IndicatorError::InvalidParameter {
                param: "factors".to_string(),
                reason: "timeframe factors must be greater than zero".to_string(),
            }
            .into());
        }
        factors
    };

    let mut frames = Vec::with_capacity(factors.len());
    for factor in factors {
        let (frame_open, frame_high, frame_low, frame_close, frame_volume, source_ranges) =
            aggregate_ohlcv(open, high, low, close, volume, factor);
        if frame_close.len() < 3 {
            continue;
        }
        let analysis = analyze(
            &frame_open,
            &frame_high,
            &frame_low,
            &frame_close,
            &frame_volume,
            config.chan,
        )?;
        frames.push(ChanFrame {
            timeframe: ChanTimeframe {
                factor,
                label: if factor == 1 {
                    "base".to_string()
                } else {
                    format!("x{factor}")
                },
                seconds: None,
            },
            analysis: remap_analysis_ranges(analysis, &source_ranges),
            source_ranges,
        });
    }
    Ok(ChanMultiAnalysis { frames })
}

/// Analyze timestamped OHLCV data using fixed wall-clock buckets.
///
/// Timestamps are Unix seconds and must be non-decreasing. The base frame is
/// always included; each requested duration is deduplicated and mapped back to
/// the exact source `[start, end)` ranges represented by its bars.
pub fn analyze_multi_timestamps(
    timestamps: &[i64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    durations_seconds: &[i64],
    config: &ChanMultiConfig,
) -> Result<ChanMultiAnalysis> {
    analyze_multi_timestamps_with_origin(
        timestamps,
        open,
        high,
        low,
        close,
        volume,
        durations_seconds,
        config,
        0,
    )
}

/// Analyze timestamped data with an exchange/session-aligned bucket origin.
///
/// The origin is expressed in Unix seconds. An origin of `0` preserves the
/// epoch-aligned behaviour of [`analyze_multi_timestamps`]. When the duration
/// list is empty, automatic wall-clock levels are selected from the observed
/// source-bar spacing.
pub fn analyze_multi_timestamps_with_origin(
    timestamps: &[i64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    durations_seconds: &[i64],
    config: &ChanMultiConfig,
    origin_seconds: i64,
) -> Result<ChanMultiAnalysis> {
    validate_lengths(open, high, low, close, volume)?;
    if timestamps.len() != close.len() {
        return Err(IndicatorError::InvalidParameter {
            param: "timestamps".to_string(),
            reason: "timestamps must have the same length as OHLCV".to_string(),
        }
        .into());
    }
    if timestamps.windows(2).any(|pair| pair[0] > pair[1]) {
        return Err(IndicatorError::InvalidParameter {
            param: "timestamps".to_string(),
            reason: "timestamps must be non-decreasing".to_string(),
        }
        .into());
    }

    let selected_durations = if durations_seconds.is_empty() {
        auto_durations_seconds(timestamps, config.auto_levels, config.min_frame_bars)
    } else {
        durations_seconds.to_vec()
    };
    let mut specs = vec![ChanTimeframeSpec::Bars(1)];
    let mut seen = std::collections::BTreeSet::new();
    for seconds in selected_durations.into_iter().filter(|value| *value > 0) {
        if seen.insert(seconds) {
            specs.push(ChanTimeframeSpec::Seconds(seconds));
        }
    }
    let mut frames = Vec::with_capacity(specs.len());
    for spec in specs {
        let (
            frame_open,
            frame_high,
            frame_low,
            frame_close,
            frame_volume,
            ranges,
            factor,
            label,
            seconds,
        ) = match spec {
            ChanTimeframeSpec::Bars(factor) => {
                let (o, h, l, c, v, ranges) =
                    aggregate_ohlcv(open, high, low, close, volume, factor);
                (
                    o,
                    h,
                    l,
                    c,
                    v,
                    ranges,
                    factor.max(1),
                    if factor == 1 {
                        "base".to_string()
                    } else {
                        format!("x{factor}")
                    },
                    None,
                )
            }
            ChanTimeframeSpec::Seconds(seconds) => {
                let (o, h, l, c, v, ranges) = aggregate_by_timestamp(
                    timestamps,
                    open,
                    high,
                    low,
                    close,
                    volume,
                    seconds,
                    origin_seconds,
                );
                (
                    o,
                    h,
                    l,
                    c,
                    v,
                    ranges,
                    1,
                    format!("{}s", seconds),
                    Some(seconds),
                )
            }
        };
        if frame_close.len() < 3 {
            continue;
        }
        let analysis = analyze(
            &frame_open,
            &frame_high,
            &frame_low,
            &frame_close,
            &frame_volume,
            config.chan,
        )?;
        frames.push(ChanFrame {
            timeframe: ChanTimeframe {
                factor,
                label,
                seconds,
            },
            analysis: remap_analysis_ranges(analysis, &ranges),
            source_ranges: ranges,
        });
    }
    Ok(ChanMultiAnalysis { frames })
}

/// Analyze timestamped OHLCV data with exchange-calendar-aware buckets.
///
/// Rows outside a configured trading session are excluded from every frame.
/// Higher-timeframe buckets start at each matched session's open, so a 30m
/// frame does not drift across a lunch break or a cross-midnight session.
/// `utc_offset_seconds` converts Unix timestamps into the exchange's fixed
/// local offset; daylight-saving transitions should be handled by the caller's
/// feed adapter when a fixed offset is insufficient.
pub fn analyze_multi_timestamps_with_calendar(
    timestamps: &[i64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    durations_seconds: &[i64],
    config: &ChanMultiConfig,
    calendar: &TradingCalendar,
    utc_offset_seconds: i32,
) -> Result<ChanMultiAnalysis> {
    validate_lengths(open, high, low, close, volume)?;
    if timestamps.len() != close.len() {
        return Err(IndicatorError::InvalidParameter {
            param: "timestamps".to_string(),
            reason: "timestamps must have the same length as OHLCV".to_string(),
        }
        .into());
    }
    if timestamps.windows(2).any(|pair| pair[0] > pair[1]) {
        return Err(IndicatorError::InvalidParameter {
            param: "timestamps".to_string(),
            reason: "timestamps must be non-decreasing".to_string(),
        }
        .into());
    }

    let mut source_indices = Vec::new();
    for (index, timestamp) in timestamps.iter().copied().enumerate() {
        if calendar
            .session_for_timestamp_for_analysis(timestamp, utc_offset_seconds)
            .map_err(calendar_error)?
            .is_some()
        {
            source_indices.push(index);
        }
    }
    if source_indices.is_empty() {
        return Ok(ChanMultiAnalysis { frames: Vec::new() });
    }

    let filtered_timestamps: Vec<i64> = source_indices.iter().map(|&i| timestamps[i]).collect();
    let filtered_open: Vec<f64> = source_indices.iter().map(|&i| open[i]).collect();
    let filtered_high: Vec<f64> = source_indices.iter().map(|&i| high[i]).collect();
    let filtered_low: Vec<f64> = source_indices.iter().map(|&i| low[i]).collect();
    let filtered_close: Vec<f64> = source_indices.iter().map(|&i| close[i]).collect();
    let filtered_volume: Vec<f64> = source_indices.iter().map(|&i| volume[i]).collect();

    let selected_durations = if durations_seconds.is_empty() {
        auto_durations_seconds(
            &filtered_timestamps,
            config.auto_levels,
            config.min_frame_bars,
        )
    } else {
        durations_seconds.to_vec()
    };
    let mut frames = Vec::with_capacity(selected_durations.len() + 1);
    let (base_open, base_high, base_low, base_close, base_volume, base_ranges) = aggregate_ohlcv(
        &filtered_open,
        &filtered_high,
        &filtered_low,
        &filtered_close,
        &filtered_volume,
        1,
    );
    let base_ranges = remap_ranges_to_source(base_ranges, &source_indices);
    if base_close.len() >= 3 {
        let analysis = analyze(
            &base_open,
            &base_high,
            &base_low,
            &base_close,
            &base_volume,
            config.chan,
        )?;
        frames.push(ChanFrame {
            timeframe: ChanTimeframe {
                factor: 1,
                label: "base".to_string(),
                seconds: None,
            },
            analysis: remap_analysis_ranges(analysis, &base_ranges),
            source_ranges: base_ranges,
        });
    }

    let mut seen = std::collections::BTreeSet::new();
    for seconds in selected_durations.into_iter().filter(|value| *value > 0) {
        if !seen.insert(seconds) {
            continue;
        }
        let (frame_open, frame_high, frame_low, frame_close, frame_volume, ranges) =
            aggregate_by_calendar_timestamp(
                &filtered_timestamps,
                &filtered_open,
                &filtered_high,
                &filtered_low,
                &filtered_close,
                &filtered_volume,
                seconds,
                calendar,
                utc_offset_seconds,
                &source_indices,
            )?;
        if frame_close.len() < 3 {
            continue;
        }
        let analysis = analyze(
            &frame_open,
            &frame_high,
            &frame_low,
            &frame_close,
            &frame_volume,
            config.chan,
        )?;
        frames.push(ChanFrame {
            timeframe: ChanTimeframe {
                factor: 1,
                label: format!("{}s-calendar", seconds),
                seconds: Some(seconds),
            },
            analysis: remap_analysis_ranges(analysis, &ranges),
            source_ranges: ranges,
        });
    }
    Ok(ChanMultiAnalysis { frames })
}

/// Select automatic wall-clock levels from median source-bar spacing.
pub fn auto_durations_seconds(
    timestamps: &[i64],
    levels: usize,
    min_frame_bars: usize,
) -> Vec<i64> {
    if timestamps.len() < 2 || levels <= 1 {
        return Vec::new();
    }
    let mut gaps: Vec<i64> = timestamps
        .windows(2)
        .filter_map(|pair| pair[1].checked_sub(pair[0]))
        .filter(|gap| *gap > 0)
        .collect();
    if gaps.is_empty() {
        return Vec::new();
    }
    gaps.sort_unstable();
    let base = gaps[gaps.len() / 2].max(1);
    let min_frame_bars = min_frame_bars.max(3) as i64;
    let span = timestamps
        .last()
        .copied()
        .unwrap_or_default()
        .saturating_sub(timestamps[0]);
    [5_i64, 20, 60, 240, 1200]
        .into_iter()
        .map(|factor| base.saturating_mul(factor))
        .filter(|duration| *duration > 0 && (span / *duration).saturating_add(1) >= min_frame_bars)
        .take(levels - 1)
        .collect()
}

/// Select stable factors from data length when the caller does not specify them.
pub fn auto_factors(length: usize, levels: usize, min_frame_bars: usize) -> Vec<usize> {
    let levels = levels.max(1);
    let min_frame_bars = min_frame_bars.max(3);
    let candidates = [1usize, 5, 20, 60, 240, 1200];
    let mut factors: Vec<usize> = candidates
        .into_iter()
        .filter(|factor| length / factor >= min_frame_bars || *factor == 1)
        .take(levels)
        .collect();
    if factors.is_empty() {
        factors.push(1);
    }
    factors
}

fn validate_lengths(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
) -> Result<()> {
    let length = close.len();
    if [open.len(), high.len(), low.len(), volume.len()]
        .iter()
        .any(|value| *value != length)
    {
        return Err(IndicatorError::InvalidParameter {
            param: "ohlcv".to_string(),
            reason: "all OHLCV arrays must have equal lengths".to_string(),
        }
        .into());
    }
    Ok(())
}

fn aggregate_ohlcv(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    factor: usize,
) -> (
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<(usize, usize)>,
) {
    let factor = factor.max(1);
    let groups = (close.len() + factor - 1) / factor;
    let mut result = (
        Vec::with_capacity(groups),
        Vec::with_capacity(groups),
        Vec::with_capacity(groups),
        Vec::with_capacity(groups),
        Vec::with_capacity(groups),
        Vec::with_capacity(groups),
    );
    for start in (0..close.len()).step_by(factor) {
        let end = (start + factor).min(close.len());
        result.0.push(open[start]);
        result.1.push(
            high[start..end]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max),
        );
        result.2.push(
            low[start..end]
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min),
        );
        result.3.push(close[end - 1]);
        result.4.push(volume[start..end].iter().sum());
        result.5.push((start, end));
    }
    result
}

fn aggregate_by_timestamp(
    timestamps: &[i64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    seconds: i64,
    origin_seconds: i64,
) -> (
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<(usize, usize)>,
) {
    let seconds = seconds.max(1);
    let mut result = (
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let mut start = 0usize;
    while start < close.len() {
        let bucket = timestamps[start]
            .saturating_sub(origin_seconds)
            .div_euclid(seconds);
        let mut end = start + 1;
        while end < close.len()
            && timestamps[end]
                .saturating_sub(origin_seconds)
                .div_euclid(seconds)
                == bucket
        {
            end += 1;
        }
        result.0.push(open[start]);
        result.1.push(
            high[start..end]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max),
        );
        result.2.push(
            low[start..end]
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min),
        );
        result.3.push(close[end - 1]);
        result.4.push(volume[start..end].iter().sum());
        result.5.push((start, end));
        start = end;
    }
    result
}

fn aggregate_by_calendar_timestamp(
    timestamps: &[i64],
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    seconds: i64,
    calendar: &TradingCalendar,
    utc_offset_seconds: i32,
    source_indices: &[usize],
) -> Result<(
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<(usize, usize)>,
)> {
    let seconds = seconds.max(1);
    let mut result = (
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let mut start = 0usize;
    while start < close.len() {
        let current = calendar
            .session_for_timestamp_for_analysis(timestamps[start], utc_offset_seconds)
            .map_err(calendar_error)?
            .ok_or_else(|| IndicatorError::InvalidParameter {
                param: "timestamps".to_string(),
                reason: "calendar aggregation received a non-trading timestamp".to_string(),
            })?;
        let bucket = (timestamps[start] - current.open_timestamp).div_euclid(seconds);
        let mut end = start + 1;
        while end < close.len() {
            let Some(next) = calendar
                .session_for_timestamp_for_analysis(timestamps[end], utc_offset_seconds)
                .map_err(calendar_error)?
            else {
                break;
            };
            let next_bucket = (timestamps[end] - next.open_timestamp).div_euclid(seconds);
            if next.session_day != current.session_day
                || next.session_index != current.session_index
                || next_bucket != bucket
            {
                break;
            }
            end += 1;
        }
        result.0.push(open[start]);
        result.1.push(
            high[start..end]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max),
        );
        result.2.push(
            low[start..end]
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min),
        );
        result.3.push(close[end - 1]);
        result.4.push(volume[start..end].iter().sum());
        result
            .5
            .push((source_indices[start], source_indices[end - 1] + 1));
        start = end;
    }
    Ok(result)
}

fn calendar_error(error: crate::calendar::CalendarError) -> IndicatorError {
    IndicatorError::InvalidParameter {
        param: "calendar".to_string(),
        reason: error.to_string(),
    }
}

fn remap_ranges_to_source(
    ranges: Vec<(usize, usize)>,
    source_indices: &[usize],
) -> Vec<(usize, usize)> {
    ranges
        .into_iter()
        .map(|(start, end)| {
            let first = source_indices.get(start).copied().unwrap_or(start);
            let last = source_indices
                .get(end.saturating_sub(1))
                .copied()
                .unwrap_or(first);
            (first, last + 1)
        })
        .collect()
}

fn remap_analysis_ranges(mut analysis: ChanAnalysis, ranges: &[(usize, usize)]) -> ChanAnalysis {
    let map = |index: usize| {
        ranges
            .get(index)
            .map(|(start, _)| *start)
            .unwrap_or_else(|| ranges.last().map(|(start, _)| *start).unwrap_or(index))
    };
    for bar in &mut analysis.bars {
        bar.start_index = map(bar.start_index);
        bar.end_index = map(bar.end_index);
        bar.high_index = map(bar.high_index);
        bar.low_index = map(bar.low_index);
    }
    for fractal in &mut analysis.fractals {
        fractal.index = map(fractal.index);
    }
    if let Some(fractal) = &mut analysis.developing_fractal {
        fractal.index = map(fractal.index);
    }
    for stroke in &mut analysis.strokes {
        stroke.start.index = map(stroke.start.index);
        stroke.end.index = map(stroke.end.index);
        stroke.bars = stroke.bars.max(1);
    }
    for segment in &mut analysis.segments {
        segment.start_index = map(segment.start_index);
        segment.end_index = map(segment.end_index);
    }
    for center in &mut analysis.centers {
        center.start_index = map(center.start_index);
        center.end_index = map(center.end_index);
    }
    for signal in &mut analysis.signals {
        signal.index = map(signal.index);
        for evidence in &mut signal.evidence {
            *evidence = map(*evidence);
        }
    }
    for divergence in &mut analysis.divergences {
        divergence.index = map(divergence.index);
    }
    analysis
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_factors_are_length_aware() {
        assert_eq!(auto_factors(30, 3, 20), vec![1]);
        assert_eq!(auto_factors(120, 3, 20), vec![1, 5]);
        assert_eq!(auto_factors(600, 3, 20), vec![1, 5, 20]);
    }

    #[test]
    fn aggregates_and_maps_indices_to_base_coordinates() {
        let close: Vec<f64> = (0..120).map(|value| value as f64 + 1.0).collect();
        let open = close.clone();
        let high: Vec<f64> = close.iter().map(|value| value + 1.0).collect();
        let low: Vec<f64> = close.iter().map(|value| value - 1.0).collect();
        let volume = vec![1.0; close.len()];
        let result = analyze_multi(
            &open,
            &high,
            &low,
            &close,
            &volume,
            &ChanMultiConfig {
                factors: vec![1, 5],
                ..ChanMultiConfig::default()
            },
        )
        .expect("valid multi-timeframe data");
        assert_eq!(result.frames.len(), 2);
        assert_eq!(result.frames[1].timeframe.factor, 5);
        assert!(result.frames[1]
            .analysis
            .bars
            .iter()
            .all(|bar| bar.start_index % 5 == 0));
        assert_eq!(result.frames[1].source_ranges[1], (5, 10));
    }

    #[test]
    fn timestamp_frames_preserve_irregular_source_ranges() {
        let timestamps = vec![0, 5, 10, 15, 30, 35, 40, 45, 60, 65, 70, 75];
        let close: Vec<f64> = (0..timestamps.len())
            .map(|index| 100.0 + (index as f64 * 0.7).sin())
            .collect();
        let open = close.clone();
        let high: Vec<f64> = close.iter().map(|value| value + 0.5).collect();
        let low: Vec<f64> = close.iter().map(|value| value - 0.5).collect();
        let volume = vec![1.0; close.len()];
        let result = analyze_multi_timestamps(
            &timestamps,
            &open,
            &high,
            &low,
            &close,
            &volume,
            &[20],
            &ChanMultiConfig::default(),
        )
        .expect("valid timestamped data");
        let frame = result
            .frames
            .iter()
            .find(|frame| frame.timeframe.seconds == Some(20))
            .expect("timestamp frame");
        assert_eq!(frame.timeframe.label, "20s");
        assert_eq!(frame.source_ranges[0], (0, 4));
        assert_eq!(frame.source_ranges[1], (4, 6));
    }

    #[test]
    fn automatic_timestamp_levels_use_median_spacing() {
        let timestamps: Vec<i64> = (0..160).map(|index| index as i64 * 60).collect();
        let durations = auto_durations_seconds(&timestamps, 3, 20);
        assert_eq!(durations, vec![300]);
    }

    #[test]
    fn timestamp_origin_changes_session_bucket_boundaries() {
        let timestamps = vec![0, 5, 10, 15, 20, 25];
        let close = vec![10.0; 6];
        let result = analyze_multi_timestamps_with_origin(
            &timestamps,
            &close,
            &close,
            &close,
            &close,
            &close,
            &[20],
            &ChanMultiConfig::default(),
            5,
        )
        .expect("timestamp origin should be accepted");
        let frame = result.by_seconds(20).expect("20 second frame");
        assert_eq!(frame.source_ranges, vec![(0, 1), (1, 5), (5, 6)]);
    }

    #[test]
    fn calendar_timestamp_frames_skip_weekends_and_align_to_sessions() {
        let mut calendar = TradingCalendar::weekdays();
        calendar.add_session(
            crate::calendar::SessionWindow::new(9 * 3600 + 30 * 60, 11 * 3600 + 30 * 60).unwrap(),
        );
        calendar.add_session(crate::calendar::SessionWindow::new(13 * 3600, 15 * 3600).unwrap());
        let timestamps = vec![
            1_704_447_000_i64, // Friday 09:30
            1_704_448_800,     // Friday 10:00
            1_704_452_400,     // Friday 11:00
            1_704_459_600,     // Friday 13:00
            1_704_463_200,     // Friday 14:00
            1_704_706_200,     // Monday 09:30
        ];
        let close: Vec<f64> = (0..timestamps.len())
            .map(|index| 100.0 + index as f64)
            .collect();
        let result = analyze_multi_timestamps_with_calendar(
            &timestamps,
            &close,
            &close,
            &close,
            &close,
            &vec![1.0; close.len()],
            &[3600],
            &ChanMultiConfig::default(),
            &calendar,
            0,
        )
        .expect("calendar-aware timestamp analysis should succeed");
        let frame = result
            .by_seconds(3600)
            .expect("calendar timeframe should be present");
        assert_eq!(frame.timeframe.label, "3600s-calendar");
        assert_eq!(
            frame.source_ranges,
            vec![(0, 2), (2, 3), (3, 4), (4, 5), (5, 6)]
        );
        assert_eq!(result.base().unwrap().source_ranges.len(), 6);
    }
}
