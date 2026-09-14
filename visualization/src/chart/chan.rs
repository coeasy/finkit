//! Chanlun structure overlay rendering.

use crate::config::{ChanRenderConfig, ChartConfig};
use crate::data::KlineData;
use crate::geometry::{Point, Rect};
use crate::layout::ChartLayout;
use crate::primitive::{Color, DrawList, LineStyle, Primitive, Style};
use finkit::chan::{
    analyze, CenterPolicy, ChanAnalysis, ChanConfig, ChanDivergenceKind, ChanSignalKind,
    ChanVariant, FractalKind, FractalPolicy, StrokePolicy,
};
use finkit::chan_mtf::ChanMultiAnalysis;

/// Computes and renders the Chanlun overlay for a validated OHLCV series.
pub fn render_chan(
    draw_list: &mut DrawList,
    data: &KlineData,
    layout: &ChartLayout,
    config: &ChartConfig,
) -> crate::error::Result<()> {
    // A replay or follow-tail viewport can legitimately contain fewer than
    // three bars even though the source series is valid. Rendering should
    // leave the Chan layer empty for that window rather than turning a
    // temporary display slice into a user-visible analysis error.
    if data.len() < 3 {
        return Ok(());
    }
    let analysis = analyze_configured(data, config)?;
    render_analysis(draw_list, &analysis, data, layout, &config.chan);
    Ok(())
}

pub fn analyze_configured(
    data: &KlineData,
    config: &ChartConfig,
) -> crate::error::Result<ChanAnalysis> {
    let chan_config = parse_render_chan_config(config)?;
    analyze(
        &data.opens,
        &data.highs,
        &data.lows,
        &data.closes,
        &data.volumes,
        chan_config,
    )
    .map_err(|error| crate::error::VisualizationError::ConversionError {
        message: format!("Chanlun analysis failed: {error}"),
    })
}

/// Renders all configured timeframes onto the same main price panel.
pub fn render_multi_analysis(
    draw_list: &mut DrawList,
    analysis: &ChanMultiAnalysis,
    data: &KlineData,
    layout: &ChartLayout,
    config: &ChanRenderConfig,
) {
    render_multi_analysis_window(draw_list, analysis, data, layout, config, 0, &[]);
}

/// Renders cached multi-timeframe structure against a sliced or aggregated
/// window while keeping raw source indices traceable to visible bars.
pub fn render_multi_analysis_window(
    draw_list: &mut DrawList,
    analysis: &ChanMultiAnalysis,
    data: &KlineData,
    layout: &ChartLayout,
    config: &ChanRenderConfig,
    source_offset: usize,
    source_ranges: &[(usize, usize)],
) {
    for (frame_index, frame) in analysis.frames.iter().enumerate() {
        let mut frame_config = config.clone();
        let is_base = frame.timeframe.factor == 1 && frame.timeframe.seconds.is_none();
        let show_annotations = is_base || config.show_multi_timeframe_annotations;
        frame_config.show_fractals = show_annotations && config.show_fractals;
        frame_config.show_developing = show_annotations && config.show_developing;
        frame_config.show_signals = show_annotations && config.show_signals;
        frame_config.show_divergences = show_annotations && config.show_divergences;
        frame_config.show_labels = show_annotations && config.show_labels;
        frame_config.line_width = config.line_width + (frame_index as f32 * 0.25);
        render_analysis_window(
            draw_list,
            &frame.analysis,
            data,
            layout,
            &frame_config,
            source_offset,
            source_ranges,
        );
    }
}

fn parse_render_chan_config(config: &ChartConfig) -> crate::error::Result<ChanConfig> {
    let variant = match config.chan.variant.to_ascii_lowercase().as_str() {
        "conservative" | "strict" => ChanVariant::Conservative,
        "standard" | "default" => ChanVariant::Standard,
        "aggressive" | "loose" => ChanVariant::Aggressive,
        value => {
            return Err(crate::error::VisualizationError::ConversionError {
                message: format!("unknown Chan variant: {value}"),
            })
        }
    };
    let fractal_policy = match config.chan.fractal_policy.to_ascii_lowercase().as_str() {
        "strict" => FractalPolicy::Strict,
        "loose" => FractalPolicy::Loose,
        "right_confirmed" | "right-confirmed" => FractalPolicy::RightConfirmed,
        value => {
            return Err(crate::error::VisualizationError::ConversionError {
                message: format!("unknown Chan fractal policy: {value}"),
            })
        }
    };
    let stroke_policy = match config.chan.stroke_policy.to_ascii_lowercase().as_str() {
        "configurable" | "min_bars" => StrokePolicy::Configurable,
        "fixed5" | "5" => StrokePolicy::Fixed5,
        "fixed6" | "6" => StrokePolicy::Fixed6,
        "fixed7" | "7" => StrokePolicy::Fixed7,
        "threshold" => StrokePolicy::Threshold,
        value => {
            return Err(crate::error::VisualizationError::ConversionError {
                message: format!("unknown Chan stroke policy: {value}"),
            })
        }
    };
    let center_policy = match config.chan.center_policy.to_ascii_lowercase().as_str() {
        "three_stroke" | "three-stroke" => CenterPolicy::ThreeStroke,
        "dynamic" => CenterPolicy::Dynamic,
        "hierarchical" | "multi_level" | "multi-level" => CenterPolicy::Hierarchical,
        value => {
            return Err(crate::error::VisualizationError::ConversionError {
                message: format!("unknown Chan center policy: {value}"),
            })
        }
    };
    let mut result = ChanConfig::default().with_variant(variant);
    result.min_stroke_bars = config.chan.min_stroke_bars;
    result.fractal_policy = fractal_policy;
    result.stroke_policy = stroke_policy;
    result.center_policy = center_policy;
    result.thresholds.min_stroke_change_ratio = config.chan.min_stroke_change_ratio;
    result.thresholds.min_fractal_range_ratio = config.chan.min_fractal_range_ratio;
    result.thresholds.signal_min_strength = config.chan.signal_min_strength;
    result.thresholds.center_break_ratio = config.chan.center_break_ratio;
    Ok(result)
}

/// Renders a precomputed analysis, useful when the caller caches structure
/// extraction separately from chart generation.
pub fn render_analysis(
    draw_list: &mut DrawList,
    analysis: &ChanAnalysis,
    data: &KlineData,
    layout: &ChartLayout,
    config: &ChanRenderConfig,
) {
    render_analysis_window(draw_list, analysis, data, layout, config, 0, &[]);
}

/// Renders a cached analysis in a viewport. Raw indices are remapped to the
/// local display coordinates, including when several source bars are folded
/// into one OHLCV bucket.
pub fn render_analysis_window(
    draw_list: &mut DrawList,
    analysis: &ChanAnalysis,
    data: &KlineData,
    layout: &ChartLayout,
    config: &ChanRenderConfig,
    source_offset: usize,
    source_ranges: &[(usize, usize)],
) {
    if data.is_empty() {
        return;
    }
    let mapped = remap_analysis(analysis, data.len(), source_offset, source_ranges);
    render_analysis_local(draw_list, &mapped, data, layout, config);
}

fn render_analysis_local(
    draw_list: &mut DrawList,
    analysis: &ChanAnalysis,
    data: &KlineData,
    layout: &ChartLayout,
    config: &ChanRenderConfig,
) {
    let plot = &layout.main_panel.plot_area;
    let scale = &layout.main_panel.y_scale;
    let bar_width = plot.width / data.len() as f64;
    let x = |index: usize| plot.x + (index as f64 + 0.5) * bar_width;
    let point = |index: usize, price: f64| Point::new(x(index), scale.data_to_pixel(price));

    let stroke_style = Style::new()
        .with_stroke(Color::from_hex(&config.stroke_color))
        .with_line_width(config.line_width);
    let segment_style = Style::new()
        .with_stroke(Color::from_hex(&config.segment_color))
        .with_line_width(config.line_width + 0.5)
        .with_line_style(LineStyle::Dashed);
    let mut label_boxes: Vec<Rect> = Vec::new();

    if config.show_centers {
        let color = Color::from_hex(&config.center_color);
        for center in &analysis.centers {
            let left = x(center.start_index);
            let right = x(center.end_index).max(left + bar_width);
            let top = scale.data_to_pixel(center.upper);
            let bottom = scale.data_to_pixel(center.lower);
            draw_list.push(Primitive::FilledRect {
                rect: Rect::new(left, top.min(bottom), right - left, (bottom - top).abs()),
                fill: color.with_alpha(34),
                stroke: Some(color.with_alpha(150)),
            });
            draw_list.push(Primitive::Line {
                p1: Point::new(left, scale.data_to_pixel(center.middle)),
                p2: Point::new(right, scale.data_to_pixel(center.middle)),
                style: Style::new()
                    .with_stroke(color.with_alpha(170))
                    .with_line_width(1.0)
                    .with_line_style(LineStyle::Dotted),
            });
        }
    }

    if config.show_segments {
        for segment in &analysis.segments {
            draw_list.push(Primitive::Line {
                p1: point(
                    segment.start_index,
                    endpoint_price(analysis, segment.start_index),
                ),
                p2: point(
                    segment.end_index,
                    endpoint_price(analysis, segment.end_index),
                ),
                style: segment_style.clone(),
            });
        }
    }

    if config.show_strokes {
        for stroke in &analysis.strokes {
            draw_list.push(Primitive::Line {
                p1: point(stroke.start.index, stroke.start.value),
                p2: point(stroke.end.index, stroke.end.value),
                style: stroke_style.clone(),
            });
        }
    }

    if config.show_fractals {
        let color = Color::from_hex(&config.fractal_color);
        for fractal in &analysis.fractals {
            let center = point(fractal.index, fractal.value);
            let size = (bar_width * 0.28).clamp(3.0, 7.0);
            let points = match fractal.kind {
                FractalKind::Top => vec![
                    Point::new(center.x, center.y - size),
                    Point::new(center.x - size, center.y + size),
                    Point::new(center.x + size, center.y + size),
                ],
                FractalKind::Bottom => vec![
                    Point::new(center.x, center.y + size),
                    Point::new(center.x - size, center.y - size),
                    Point::new(center.x + size, center.y - size),
                ],
            };
            draw_list.push(Primitive::Polygon {
                points,
                style: Style::new().with_stroke(color).with_fill(color),
            });
            if config.show_labels {
                let label = match fractal.kind {
                    FractalKind::Top => "顶",
                    FractalKind::Bottom => "底",
                };
                let label_y = match fractal.kind {
                    FractalKind::Top => center.y - size - 3.0,
                    FractalKind::Bottom => center.y + size + 12.0,
                };
                let label_box = Rect::new(center.x - 7.0, label_y - 11.0, 14.0, 13.0);
                if reserve_label(&mut label_boxes, label_box, config.label_min_pixel_gap) {
                    draw_list.push(Primitive::Text {
                        position: Point::new(center.x - 5.0, label_y),
                        content: label.to_string(),
                        style: Style::new().with_stroke(color).with_font_size(11.0),
                    });
                }
            }
        }
    }

    if config.show_developing {
        if let Some(fractal) = analysis.developing_fractal {
            let center = point(fractal.index, fractal.value);
            let color = Color::from_hex(&config.fractal_color).with_alpha(150);
            draw_list.push(Primitive::Circle {
                center,
                radius: (bar_width * 0.32).clamp(3.0, 8.0),
                style: Style::new().with_stroke(color).with_line_width(1.0),
            });
            if config.show_labels {
                draw_list.push(Primitive::Text {
                    position: Point::new(center.x - 9.0, center.y - 12.0),
                    content: "候选".to_string(),
                    style: Style::new().with_stroke(color).with_font_size(10.0),
                });
            }
        }
    }

    if config.show_signals {
        for signal in &analysis.signals {
            let color = if signal.kind.is_buy() {
                Color::from_hex("#16a34a")
            } else {
                Color::from_hex("#dc2626")
            };
            let center = point(signal.index, signal.price);
            draw_list.push(Primitive::Circle {
                center,
                radius: 3.0,
                style: Style::new()
                    .with_stroke(color)
                    .with_fill(color.with_alpha(180)),
            });
            if config.show_labels {
                let label = match signal.kind {
                    ChanSignalKind::Buy1 => "B1",
                    ChanSignalKind::Buy2 => "B2",
                    ChanSignalKind::Buy3 => "B3",
                    ChanSignalKind::Sell1 => "S1",
                    ChanSignalKind::Sell2 => "S2",
                    ChanSignalKind::Sell3 => "S3",
                };
                let label_box = Rect::new(center.x - 9.0, center.y - 19.0, 18.0, 12.0);
                if reserve_label(&mut label_boxes, label_box, config.label_min_pixel_gap) {
                    draw_list.push(Primitive::Text {
                        position: Point::new(center.x - 7.0, center.y - 8.0),
                        content: label.to_string(),
                        style: Style::new().with_stroke(color).with_font_size(9.0),
                    });
                }
            }
        }
    }

    if config.show_divergences {
        for divergence in &analysis.divergences {
            let color = match divergence.kind {
                ChanDivergenceKind::Bullish => Color::from_hex("#0891b2"),
                ChanDivergenceKind::Bearish => Color::from_hex("#c026d3"),
            };
            let center = point(divergence.index, divergence.price);
            draw_list.push(Primitive::Circle {
                center,
                radius: (bar_width * 0.42).clamp(4.0, 9.0),
                style: Style::new()
                    .with_stroke(color)
                    .with_fill(color.with_alpha(35))
                    .with_line_width(1.5),
            });
            if config.show_labels {
                let label = match divergence.kind {
                    ChanDivergenceKind::Bullish => "底背驰",
                    ChanDivergenceKind::Bearish => "顶背驰",
                };
                let label_box = Rect::new(center.x - 18.0, center.y - 22.0, 36.0, 12.0);
                if reserve_label(&mut label_boxes, label_box, config.label_min_pixel_gap) {
                    draw_list.push(Primitive::Text {
                        position: Point::new(center.x - 15.0, center.y - 11.0),
                        content: label.to_string(),
                        style: Style::new().with_stroke(color).with_font_size(9.0),
                    });
                }
            }
        }
    }
}

fn reserve_label(occupied: &mut Vec<Rect>, candidate: Rect, min_gap: f64) -> bool {
    let padding = min_gap.max(0.0) * 0.5;
    let padded = candidate.inflate(padding, 1.0);
    if occupied.iter().any(|existing| existing.intersects(&padded)) {
        return false;
    }
    occupied.push(padded);
    true
}

fn remap_analysis(
    analysis: &ChanAnalysis,
    data_len: usize,
    source_offset: usize,
    source_ranges: &[(usize, usize)],
) -> ChanAnalysis {
    let map_index = |raw: usize| -> Option<usize> {
        if source_ranges.is_empty() {
            return raw
                .checked_sub(source_offset)
                .filter(|index| *index < data_len);
        }
        source_ranges
            .iter()
            .position(|(start, end)| raw >= *start && raw < *end)
    };
    let clamp_index = |raw: usize| -> Option<usize> {
        if let Some(index) = map_index(raw) {
            return Some(index);
        }
        if source_ranges.is_empty() {
            if raw < source_offset {
                return Some(0);
            }
            return (data_len > 0).then_some(data_len - 1);
        }
        if source_ranges.is_empty() {
            return None;
        }
        if raw < source_ranges[0].0 {
            Some(0)
        } else if raw >= source_ranges.last().map(|range| range.1).unwrap_or(0) {
            Some(source_ranges.len().saturating_sub(1))
        } else {
            None
        }
    };

    let mut mapped = analysis.clone();
    mapped.fractals.retain_mut(|fractal| {
        if let Some(index) = map_index(fractal.index) {
            fractal.index = index;
            true
        } else {
            false
        }
    });
    mapped.strokes.retain_mut(|stroke| {
        let start = clamp_index(stroke.start.index);
        let end = clamp_index(stroke.end.index);
        if let (Some(start), Some(end)) = (start, end) {
            stroke.start.index = start;
            stroke.end.index = end;
            true
        } else {
            false
        }
    });
    mapped.segments.retain_mut(|segment| {
        let start = clamp_index(segment.start_index);
        let end = clamp_index(segment.end_index);
        if let (Some(start), Some(end)) = (start, end) {
            segment.start_index = start;
            segment.end_index = end;
            true
        } else {
            false
        }
    });
    mapped.centers.retain_mut(|center| {
        let start = clamp_index(center.start_index);
        let end = clamp_index(center.end_index);
        if let (Some(start), Some(end)) = (start, end) {
            center.start_index = start;
            center.end_index = end;
            true
        } else {
            false
        }
    });
    if let Some(fractal) = mapped.developing_fractal.as_mut() {
        if let Some(index) = map_index(fractal.index) {
            fractal.index = index;
        } else {
            mapped.developing_fractal = None;
        }
    }
    mapped.signals.retain_mut(|signal| {
        if let Some(index) = map_index(signal.index) {
            signal.index = index;
            signal.evidence.retain_mut(|evidence| {
                if let Some(mapped) = map_index(*evidence) {
                    *evidence = mapped;
                    true
                } else {
                    false
                }
            });
            true
        } else {
            false
        }
    });
    mapped.divergences.retain_mut(|divergence| {
        if let Some(index) = map_index(divergence.index) {
            divergence.index = index;
            true
        } else {
            false
        }
    });
    mapped
}

fn endpoint_price(analysis: &ChanAnalysis, index: usize) -> f64 {
    analysis
        .fractals
        .iter()
        .find(|fractal| fractal.index == index)
        .map(|fractal| fractal.value)
        .unwrap_or_else(|| {
            analysis
                .strokes
                .iter()
                .flat_map(|stroke| [stroke.start, stroke.end])
                .min_by_key(|fractal| fractal.index.abs_diff(index))
                .map(|fractal| fractal.value)
                .unwrap_or(0.0)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ChartConfigBuilder;
    use crate::layout::LayoutCalculator;

    #[test]
    fn renders_chan_layers_on_main_panel() {
        let close = vec![
            10., 11., 14., 13., 12., 9., 8., 10., 13., 12., 11., 7., 6., 9., 12., 11., 10., 8.,
        ];
        let data = KlineData::new(
            (0..close.len()).map(|index| index.to_string()).collect(),
            close.iter().map(|value| value - 0.2).collect(),
            close.iter().map(|value| value + 0.5).collect(),
            close.iter().map(|value| value - 0.5).collect(),
            close.clone(),
            vec![1.0; close.len()],
        );
        let mut config = ChartConfigBuilder::new().show_volume(false).build();
        config.chan.min_stroke_bars = 2;
        let layout = LayoutCalculator::calculate(&data, &config, 0);
        let mut draw_list = DrawList::new();
        render_chan(&mut draw_list, &data, &layout, &config).expect("valid Chanlun sample");

        assert!(draw_list
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Polygon { .. })));
        assert!(draw_list
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Line { .. })));
    }
}
