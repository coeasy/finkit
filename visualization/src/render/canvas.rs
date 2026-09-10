//! Canvas 2D renderer for high-volume browser charts.
//!
//! The renderer emits a compact command stream and a small replay runtime
//! instead of one DOM node per primitive.  The command format is deliberately
//! backend-neutral so a WASM or native frontend can consume the JSON command
//! stream without depending on SVG.

use crate::config::{ChartConfig, IndicatorConfig};
use crate::error::{Result, VisualizationError};
use crate::geometry::{Point, Rect, Transform};
use crate::primitive::{Color, DrawList, LineStyle, Primitive, Style};
use crate::scene::ChartScene;
use serde::Serialize;
use std::collections::HashMap;

use super::Renderer;

#[derive(Debug, Clone, Serialize)]
struct CanvasStyle {
    stroke: Option<String>,
    fill: Option<String>,
    line_width: f32,
    line_dash: Vec<f32>,
    font_size: f32,
    font_family: String,
    opacity: f32,
}

#[derive(Debug, Clone, Serialize)]
struct CanvasRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Serialize)]
struct CanvasBar {
    index: usize,
    source_end: usize,
    date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<i64>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    change: Option<f64>,
    change_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
struct CanvasHit {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    priority: i32,
    target: String,
    tooltip: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CanvasIndicator {
    name: String,
    values: Vec<Option<f64>>,
}

fn indicator_payload(
    data: &crate::data::KlineData,
    source_ranges: &[(usize, usize)],
    indicator_configs: &[IndicatorConfig],
    custom_series: &HashMap<String, Vec<f64>>,
) -> Vec<CanvasIndicator> {
    #[cfg(feature = "html")]
    {
        return crate::render::html::indicator_payload(
            data,
            source_ranges,
            indicator_configs,
            custom_series,
        )
        .into_iter()
        .map(|item| CanvasIndicator {
            name: item.name,
            values: item.values,
        })
        .collect();
    }

    #[cfg(not(feature = "html"))]
    {
        indicator_configs
            .iter()
            .filter_map(|config| {
                let crate::config::IndicatorType::Custom(name) = &config.indicator_type else {
                    return None;
                };
                let values = custom_series.get(name)?;
                let mapped = if values.len() == data.len() {
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
                        .collect()
                };
                Some(CanvasIndicator {
                    name: name.clone(),
                    values: mapped
                        .into_iter()
                        .map(|value| value.is_finite().then_some(value))
                        .collect(),
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum CanvasCommand {
    Line {
        p1: [f64; 2],
        p2: [f64; 2],
        style: CanvasStyle,
    },
    Rect {
        rect: CanvasRect,
        style: CanvasStyle,
    },
    FilledRect {
        rect: CanvasRect,
        fill: String,
        stroke: Option<String>,
    },
    Polygon {
        points: Vec<[f64; 2]>,
        style: CanvasStyle,
    },
    Path {
        points: Vec<[f64; 2]>,
        style: CanvasStyle,
        close: bool,
    },
    Circle {
        center: [f64; 2],
        radius: f64,
        style: CanvasStyle,
    },
    Text {
        position: [f64; 2],
        content: String,
        style: CanvasStyle,
    },
    Group {
        primitives: Vec<CanvasCommand>,
        transform: Option<[f64; 6]>,
    },
}

pub struct CanvasRenderer;

impl CanvasRenderer {
    pub fn new() -> Self {
        Self
    }

    fn style(style: &Style) -> CanvasStyle {
        let line_dash = match style.line_style {
            LineStyle::Solid => Vec::new(),
            LineStyle::Dashed => vec![8.0, 4.0],
            LineStyle::Dotted => vec![2.0, 2.0],
            LineStyle::DashDot => vec![8.0, 4.0, 2.0, 4.0],
        };
        CanvasStyle {
            stroke: style.stroke_color.map(|color| color.to_rgba_string()),
            fill: style.fill_color.map(|color| color.to_rgba_string()),
            line_width: style.line_width,
            line_dash,
            font_size: style.font_size,
            font_family: style.font_family.clone(),
            opacity: style.opacity,
        }
    }

    fn color(color: Color) -> String {
        color.to_rgba_string()
    }

    fn rect(rect: Rect) -> CanvasRect {
        CanvasRect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }
    }

    fn point(point: Point) -> [f64; 2] {
        [point.x, point.y]
    }

    fn transform(transform: Transform) -> [f64; 6] {
        transform.m
    }

    fn command(primitive: &Primitive) -> CanvasCommand {
        match primitive {
            Primitive::Line { p1, p2, style } => CanvasCommand::Line {
                p1: Self::point(*p1),
                p2: Self::point(*p2),
                style: Self::style(style),
            },
            Primitive::Rect { rect, style } => CanvasCommand::Rect {
                rect: Self::rect(*rect),
                style: Self::style(style),
            },
            Primitive::FilledRect { rect, fill, stroke } => CanvasCommand::FilledRect {
                rect: Self::rect(*rect),
                fill: Self::color(*fill),
                stroke: stroke.map(Self::color),
            },
            Primitive::Polygon { points, style } => CanvasCommand::Polygon {
                points: points.iter().copied().map(Self::point).collect(),
                style: Self::style(style),
            },
            Primitive::Path {
                points,
                style,
                close,
            } => CanvasCommand::Path {
                points: points.iter().copied().map(Self::point).collect(),
                style: Self::style(style),
                close: *close,
            },
            Primitive::Circle {
                center,
                radius,
                style,
            } => CanvasCommand::Circle {
                center: Self::point(*center),
                radius: *radius,
                style: Self::style(style),
            },
            Primitive::Text {
                position,
                content,
                style,
            } => CanvasCommand::Text {
                position: Self::point(*position),
                content: content.clone(),
                style: Self::style(style),
            },
            Primitive::Group {
                primitives,
                transform,
            } => CanvasCommand::Group {
                primitives: primitives.iter().map(Self::command).collect(),
                transform: transform.map(Self::transform),
            },
        }
    }

    fn commands(draw_list: &DrawList) -> Vec<CanvasCommand> {
        draw_list.iter().map(Self::command).collect()
    }

    pub(crate) fn escape_html(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }

    pub(crate) fn command_json(draw_list: &DrawList) -> Result<String> {
        serde_json::to_string(&Self::commands(draw_list)).map_err(|error| {
            VisualizationError::SerializationError {
                message: format!("Failed to serialize Canvas command stream: {error}"),
            }
        })
    }

    /// Add a single DOM tooltip and crosshair overlay while retaining Canvas
    /// for all chart primitives. This keeps the data-window interaction
    /// available without creating SVG nodes for every K line.
    pub fn render_with_data(
        &self,
        draw_list: &DrawList,
        config: &ChartConfig,
        data: &crate::data::KlineData,
        source_offset: usize,
        source_ranges: &[(usize, usize)],
        plot: Rect,
    ) -> Result<String> {
        self.render_with_data_and_context(
            draw_list,
            config,
            data,
            source_offset,
            source_ranges,
            plot,
            &ChartScene::default(),
            &[],
            &HashMap::new(),
        )
    }

    /// Render Canvas with the same semantic tooltip payload as the HTML
    /// backend: visible indicator rows plus event/Chan hit regions.
    pub fn render_with_data_and_context(
        &self,
        draw_list: &DrawList,
        config: &ChartConfig,
        data: &crate::data::KlineData,
        source_offset: usize,
        source_ranges: &[(usize, usize)],
        plot: Rect,
        scene: &ChartScene,
        indicator_configs: &[IndicatorConfig],
        custom_series: &HashMap<String, Vec<f64>>,
    ) -> Result<String> {
        let mut html = self.render(draw_list, config)?;
        let bars: Vec<CanvasBar> = (0..data.len())
            .map(|index| {
                let (source_start, source_end) = source_ranges
                    .get(index)
                    .copied()
                    .unwrap_or((source_offset + index, source_offset + index + 1));
                let previous = index.checked_sub(1).map(|previous| data.closes[previous]);
                let change = previous.map(|previous| data.closes[index] - previous);
                let change_pct = previous.and_then(|previous| {
                    (previous.abs() > f64::EPSILON)
                        .then_some((data.closes[index] - previous) / previous * 100.0)
                });
                CanvasBar {
                    index: source_start,
                    source_end,
                    date: data.dates[index].clone(),
                    timestamp: data.timestamps.get(index).copied(),
                    open: data.opens[index],
                    high: data.highs[index],
                    low: data.lows[index],
                    close: data.closes[index],
                    volume: data.volumes[index],
                    change,
                    change_pct,
                }
            })
            .collect();
        let bars_json = serde_json::to_string(&bars)
            .map_err(|error| VisualizationError::SerializationError {
                message: format!("Failed to serialize Canvas data window: {error}"),
            })?
            .replace("</", "<\\/");
        let plot_json =
            serde_json::to_string(&[plot.x, plot.y, plot.width, plot.height]).map_err(|error| {
                VisualizationError::SerializationError {
                    message: format!("Failed to serialize Canvas plot bounds: {error}"),
                }
            })?;
        let indicators_json = serde_json::to_string(&indicator_payload(
            data,
            source_ranges,
            indicator_configs,
            custom_series,
        ))
        .map_err(|error| VisualizationError::SerializationError {
            message: format!("Failed to serialize Canvas indicator data: {error}"),
        })?;
        let hits: Vec<CanvasHit> = scene
            .hit_regions
            .iter()
            .map(|region| CanvasHit {
                x: region.rect.x,
                y: region.rect.y,
                width: region.rect.width,
                height: region.rect.height,
                priority: region.priority,
                target: format!("{:?}", region.target),
                tooltip: region.tooltip.clone(),
            })
            .collect();
        let hits_json = serde_json::to_string(&hits).map_err(|error| {
            VisualizationError::SerializationError {
                message: format!("Failed to serialize Canvas hit data: {error}"),
            }
        })?;
        let overlay = format!(
            r#"<div class="finkit-canvas-tooltip" id="finkit-canvas-tooltip" style="display:none"></div><div class="finkit-canvas-crosshair" id="finkit-canvas-crosshair" style="display:none"></div>
<style>.finkit-canvas-tooltip{{position:fixed;z-index:20;pointer-events:none;min-width:210px;background:rgba(14,18,28,.94);color:#f3f4f6;border:1px solid rgba(148,163,184,.45);border-radius:3px;padding:8px 10px;font:12px/1.45 sans-serif;box-shadow:0 3px 12px rgba(0,0,0,.22)}}.finkit-canvas-tooltip b{{color:#fff;display:block;border-bottom:1px solid rgba(148,163,184,.3);padding-bottom:4px;margin-bottom:4px}}.finkit-canvas-crosshair{{position:fixed;z-index:19;width:1px;background:rgba(148,163,184,.8);pointer-events:none}}</style>
<script>(function(){{'use strict';var canvas=document.querySelector('.finkit-canvas');var tip=document.getElementById('finkit-canvas-tooltip');var line=document.getElementById('finkit-canvas-crosshair');var bars={bars};var plot={plot};var indicatorRows={indicators};var hits={hits};if(!canvas||!bars.length)return;function esc(v){{return String(v).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;').replace(/'/g,'&#39;')}}function fmt(v){{return v===null||v===undefined||!isFinite(Number(v))?'--':Number(v).toFixed(2)}}function hide(){{tip.style.display='none';line.style.display='none'}}canvas.addEventListener('pointermove',function(e){{var rect=canvas.getBoundingClientRect();var x=e.clientX-rect.left;var y=e.clientY-rect.top;if(x<plot[0]||x>plot[0]+plot[2]||y<plot[1]||y>plot[1]+plot[3]){{hide();return}}var index=Math.max(0,Math.min(bars.length-1,Math.floor((x-plot[0])/(plot[2]/bars.length))));var bar=bars[index];var change=bar.change===null?'--':(bar.change>=0?'+':'')+fmt(bar.change);var pct=bar.change_pct===null?'--':(bar.change_pct>=0?'+':'')+fmt(bar.change_pct)+'%';var content='<b>数据窗口 · '+esc(bar.date)+' #'+bar.index+(bar.source_end>bar.index+1?' ['+bar.index+','+(bar.source_end-1)+']':'')+'</b><div>开 '+fmt(bar.open)+'　高 '+fmt(bar.high)+'</div><div>低 '+fmt(bar.low)+'　收 '+fmt(bar.close)+'</div><div>涨跌 '+change+'　涨幅 '+pct+'</div><div>成交量 '+fmt(bar.volume)+'</div>';if(indicatorRows.length){{content+='<div style="margin-top:5px;padding-top:4px;border-top:1px solid rgba(148,163,184,.3)">指标</div>';indicatorRows.forEach(function(row){{content+='<div>'+esc(row.name)+'：'+fmt(row.values[index])+'</div>';}});}}var matched=hits.filter(function(hit){{return x>=hit.x&&x<=hit.x+hit.width&&y>=hit.y&&y<=hit.y+hit.height;}}).sort(function(a,b){{return b.priority-a.priority;}})[0];if(matched)content+='<div style="margin-top:5px;padding-top:4px;border-top:1px solid rgba(148,163,184,.3);color:#fbbf24">'+esc(matched.tooltip||matched.target)+'</div>';tip.innerHTML=content;tip.style.display='block';tip.style.left=Math.max(6,Math.min(window.innerWidth-tip.offsetWidth-6,e.clientX+14))+'px';tip.style.top=Math.max(6,Math.min(window.innerHeight-tip.offsetHeight-6,e.clientY+14))+'px';line.style.display='block';line.style.left=(rect.left+x)+'px';line.style.top=(rect.top+plot[1])+'px';line.style.height=plot[3]+'px'}});canvas.addEventListener('pointerleave',hide);}})();</script>"#,
            bars = bars_json,
            plot = plot_json,
            indicators = indicators_json.replace("</", "<\\/"),
            hits = hits_json.replace("</", "<\\/"),
        );
        html = html.replace("</body>", &format!("{}\n</body>", overlay));
        Ok(html)
    }
}

impl Default for CanvasRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for CanvasRenderer {
    fn render(&self, draw_list: &DrawList, config: &ChartConfig) -> Result<String> {
        let commands = Self::command_json(draw_list)?.replace("</", "<\\/");
        let title = Self::escape_html(&config.title);
        let background = config.theme_config.background_color.clone();
        Ok(format!(
            "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{title}</title>\n<style>*{{margin:0;padding:0;box-sizing:border-box}}body{{background:{background};display:flex;justify-content:center;align-items:center;min-height:100vh}}.finkit-canvas{{display:block;width:{width}px;height:{height}px;touch-action:none}}</style></head>\n<body><canvas class=\"finkit-canvas\" data-renderer=\"canvas2d\" width=\"{width}\" height=\"{height}\"></canvas>\n<script>\n(function(){{\n'use strict';\nvar canvas=document.querySelector('.finkit-canvas');\nif(!canvas)return;\nvar ctx=canvas.getContext('2d');\nvar commands={commands};\nvar dpr=Math.max(1,window.devicePixelRatio||1);\ncanvas.width={width}*dpr;canvas.height={height}*dpr;\nctx.setTransform(dpr,0,0,dpr,0,0);\nfunction applyStyle(s){{ctx.globalAlpha=s.opacity;ctx.strokeStyle=s.stroke||'transparent';ctx.fillStyle=s.fill||'transparent';ctx.lineWidth=s.lineWidth;ctx.setLineDash(s.lineDash||[]);ctx.font=s.fontSize+'px '+s.fontFamily;}}\nfunction strokeFill(s,fill){{if(fill&&s.fill)ctx.fill();if(s.stroke)ctx.stroke();}}\nfunction path(points,close){{if(!points.length)return;ctx.beginPath();ctx.moveTo(points[0][0],points[0][1]);for(var i=1;i<points.length;i++)ctx.lineTo(points[i][0],points[i][1]);if(close)ctx.closePath();}}\nfunction draw(list){{list.forEach(function(c){{var s,r;switch(c.type){{case'line':applyStyle(c.style);ctx.beginPath();ctx.moveTo(c.p1[0],c.p1[1]);ctx.lineTo(c.p2[0],c.p2[1]);if(c.style.stroke)ctx.stroke();break;case'rect':applyStyle(c.style);r=c.rect;if(c.style.fill)ctx.fillRect(r.x,r.y,r.width,r.height);if(c.style.stroke)ctx.strokeRect(r.x,r.y,r.width,r.height);break;case'filledRect':r=c.rect;ctx.globalAlpha=1;ctx.fillStyle=c.fill;ctx.fillRect(r.x,r.y,r.width,r.height);if(c.stroke){{ctx.strokeStyle=c.stroke;ctx.strokeRect(r.x,r.y,r.width,r.height);}}break;case'polygon':applyStyle(c.style);path(c.points,true);strokeFill(c.style,true);break;case'path':applyStyle(c.style);path(c.points,c.close);strokeFill(c.style,c.close);break;case'circle':applyStyle(c.style);ctx.beginPath();ctx.arc(c.center[0],c.center[1],c.radius,0,Math.PI*2);strokeFill(c.style,true);break;case'text':applyStyle(c.style);ctx.fillStyle=c.style.fill||c.style.stroke||'#000';ctx.fillText(c.content,c.position[0],c.position[1]);break;case'group':ctx.save();if(c.transform)ctx.transform(c.transform[0],c.transform[3],c.transform[1],c.transform[4],c.transform[2],c.transform[5]);draw(c.primitives);ctx.restore();break;}}}});}}\nctx.clearRect(0,0,{width},{height});ctx.fillStyle='{background}';ctx.fillRect(0,0,{width},{height});draw(commands);\n}})();\n</script></body></html>",
            title = title,
            background = background,
            width = config.width,
            height = config.height,
            commands = commands,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ChartConfig;

    #[test]
    fn renders_canvas_command_stream() {
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Line {
            p1: Point::new(1.0, 2.0),
            p2: Point::new(3.0, 4.0),
            style: Style::default(),
        });
        draw_list.push(Primitive::Text {
            position: Point::new(5.0, 6.0),
            content: "<safe>".to_string(),
            style: Style::default(),
        });
        let html = CanvasRenderer::new()
            .render(&draw_list, &ChartConfig::default())
            .expect("Canvas renderer should succeed");
        assert!(html.contains("data-renderer=\"canvas2d\""));
        assert!(html.contains("getContext('2d')"));
        assert!(html.contains("lineWidth"));
        assert!(html.contains("<safe>"));
    }

    #[test]
    fn preserves_group_transform_in_commands() {
        let draw_list = DrawList {
            primitives: vec![Primitive::Group {
                primitives: vec![Primitive::Circle {
                    center: Point::new(1.0, 1.0),
                    radius: 2.0,
                    style: Style::default(),
                }],
                transform: Some(Transform::translate(4.0, 5.0)),
            }],
        };
        let json = CanvasRenderer::command_json(&draw_list).expect("command JSON");
        assert!(json.contains("\"type\":\"group\""));
        assert!(json.contains("4.0"));
        assert!(json.contains("5.0"));
    }

    #[test]
    fn renders_canvas_data_window_overlay() {
        let data = crate::data::KlineData::new(
            vec!["2026-01-01".to_string(), "2026-01-02".to_string()],
            vec![10.0, 11.0],
            vec![11.0, 12.0],
            vec![9.0, 10.0],
            vec![10.5, 11.5],
            vec![100.0, 120.0],
        );
        let html = CanvasRenderer::new()
            .render_with_data(
                &DrawList::new(),
                &ChartConfig::default(),
                &data,
                0,
                &[(0, 1), (1, 2)],
                Rect::new(60.0, 40.0, 700.0, 300.0),
            )
            .expect("Canvas data window should render");
        assert!(html.contains("finkit-canvas-tooltip"));
        assert!(html.contains("pointermove"));
        assert!(html.contains("2026-01-01"));
    }
}
