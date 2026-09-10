use crate::config::ChartConfig;
use crate::data::KlineData;
use crate::error::{Result, VisualizationError};
use crate::geometry::Transform;
use crate::primitive::{Color, DrawList, LineStyle, Primitive, Style};
use crate::scene::{ChartScene, HitTarget, PanelId};
use serde_json::{json, Value};

pub struct JsonRenderer;

impl JsonRenderer {
    pub fn new() -> Self {
        Self
    }

    /// Export draw primitives together with the semantic scene schema.
    pub fn render_scene(
        &self,
        draw_list: &DrawList,
        config: &ChartConfig,
        scene: &ChartScene,
    ) -> Result<String> {
        let config_value =
            serde_json::to_value(config).map_err(|e| VisualizationError::SerializationError {
                message: e.to_string(),
            })?;
        let primitives: Vec<Value> = draw_list.primitives.iter().map(primitive_to_json).collect();
        let panels = scene
            .panels
            .iter()
            .map(|panel| {
                json!({
                    "id": panel_id_to_json(panel.id),
                    "rect": rect_to_json(&panel.rect),
                    "visible": panel.visible,
                })
            })
            .collect::<Vec<_>>();
        let layers = scene
            .layers
            .iter()
            .map(|layer| {
                json!({
                    "id": layer.id,
                    "panel": panel_id_to_json(layer.panel),
                    "z_index": layer.z_index,
                    "visible": layer.visible,
                    "opacity": layer.opacity,
                })
            })
            .collect::<Vec<_>>();
        let hit_regions = scene
            .hit_regions
            .iter()
            .map(|region| {
                json!({
                    "rect": rect_to_json(&region.rect),
                    "target": hit_target_to_json(&region.target),
                    "priority": region.priority,
                    "tooltip": region.tooltip,
                })
            })
            .collect::<Vec<_>>();
        let result = json!({
            "schema_version": 1,
            "config": config_value,
            "primitives": primitives,
            "scene": {
                "panels": panels,
                "layers": layers,
                "hit_regions": hit_regions,
                "metadata": {
                    "data_revision": scene.metadata.data_revision,
                    "source_start": scene.metadata.source_start,
                    "source_end": scene.metadata.source_end,
                    "timeframe_labels": scene.metadata.timeframe_labels,
                },
            },
        });
        serde_json::to_string(&result).map_err(|e| VisualizationError::SerializationError {
            message: e.to_string(),
        })
    }

    /// Export the scene plus the exact display-window OHLCV rows used by the
    /// renderer. `source_ranges` keeps aggregated rows traceable to source
    /// bars for native and WASM data windows.
    pub fn render_scene_with_data(
        &self,
        draw_list: &DrawList,
        config: &ChartConfig,
        scene: &ChartScene,
        data: &KlineData,
        source_offset: usize,
        source_ranges: &[(usize, usize)],
    ) -> Result<String> {
        let mut result: Value = serde_json::from_str(&self.render_scene(draw_list, config, scene)?)
            .map_err(|error| VisualizationError::SerializationError {
                message: format!("Failed to compose JSON chart payload: {error}"),
            })?;
        result["data"] = json!({
            "revision": data.revision(),
            "source_offset": source_offset,
            "source_ranges": source_ranges,
            "dates": &data.dates,
            "timestamps": &data.timestamps,
            "opens": &data.opens,
            "highs": &data.highs,
            "lows": &data.lows,
            "closes": &data.closes,
            "volumes": &data.volumes,
        });
        serde_json::to_string(&result).map_err(|error| VisualizationError::SerializationError {
            message: format!("Failed to serialize JSON chart payload: {error}"),
        })
    }
}

fn panel_id_to_json(panel: PanelId) -> Value {
    match panel {
        PanelId::Main => json!("main"),
        PanelId::Volume => json!("volume"),
        PanelId::Indicator(index) => json!({"indicator": index}),
    }
}

fn hit_target_to_json(target: &HitTarget) -> Value {
    match target {
        HitTarget::Kline { index } => json!({"type":"kline", "index":index}),
        HitTarget::Volume { index } => json!({"type":"volume", "index":index}),
        HitTarget::Indicator { name, index } => {
            json!({"type":"indicator", "name":name, "index":index})
        }
        HitTarget::ChanFractal { index } => json!({"type":"chan_fractal", "index":index}),
        HitTarget::ChanStroke {
            start_index,
            end_index,
        } => {
            json!({"type":"chan_stroke", "start_index":start_index, "end_index":end_index})
        }
        HitTarget::ChanSegment {
            start_index,
            end_index,
        } => {
            json!({"type":"chan_segment", "start_index":start_index, "end_index":end_index})
        }
        HitTarget::ChanCenter {
            start_index,
            end_index,
            level,
        } => {
            json!({"type":"chan_center", "start_index":start_index, "end_index":end_index, "level":level})
        }
        HitTarget::ChanSignal { index, kind } => {
            json!({"type":"chan_signal", "index":index, "kind":kind})
        }
        HitTarget::ChanDivergence { index, kind } => {
            json!({"type":"chan_divergence", "index":index, "kind":kind})
        }
        HitTarget::Event { index, kind } => {
            json!({"type":"event", "index":index, "kind":kind})
        }
    }
}

impl Default for JsonRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Renderer for JsonRenderer {
    fn render(&self, draw_list: &DrawList, config: &ChartConfig) -> Result<String> {
        let config_value =
            serde_json::to_value(config).map_err(|e| VisualizationError::SerializationError {
                message: e.to_string(),
            })?;

        let primitives: Vec<Value> = draw_list.primitives.iter().map(primitive_to_json).collect();

        let result = json!({
            "config": config_value,
            "primitives": primitives,
        });

        serde_json::to_string(&result).map_err(|e| VisualizationError::SerializationError {
            message: e.to_string(),
        })
    }
}

fn primitive_to_json(prim: &Primitive) -> Value {
    match prim {
        Primitive::Line { p1, p2, style } => json!({
            "type": "Line",
            "p1": point_to_json(p1),
            "p2": point_to_json(p2),
            "style": style_to_json(style),
        }),
        Primitive::Rect { rect, style } => json!({
            "type": "Rect",
            "rect": rect_to_json(rect),
            "style": style_to_json(style),
        }),
        Primitive::FilledRect { rect, fill, stroke } => {
            let mut obj = json!({
                "type": "FilledRect",
                "rect": rect_to_json(rect),
                "fill": color_to_json(fill),
            });
            if let Some(s) = stroke {
                obj["stroke"] = color_to_json(s);
            }
            obj
        }
        Primitive::Polygon { points, style } => json!({
            "type": "Polygon",
            "points": points.iter().map(point_to_json).collect::<Vec<_>>(),
            "style": style_to_json(style),
        }),
        Primitive::Path {
            points,
            style,
            close,
        } => json!({
            "type": "Path",
            "points": points.iter().map(point_to_json).collect::<Vec<_>>(),
            "style": style_to_json(style),
            "close": close,
        }),
        Primitive::Circle {
            center,
            radius,
            style,
        } => json!({
            "type": "Circle",
            "center": point_to_json(center),
            "radius": radius,
            "style": style_to_json(style),
        }),
        Primitive::Text {
            position,
            content,
            style,
        } => json!({
            "type": "Text",
            "position": point_to_json(position),
            "content": content,
            "style": style_to_json(style),
        }),
        Primitive::Group {
            primitives,
            transform,
        } => {
            let children: Vec<Value> = primitives.iter().map(primitive_to_json).collect();
            let mut obj = json!({
                "type": "Group",
                "primitives": children,
            });
            if let Some(t) = transform {
                obj["transform"] = transform_to_json(t);
            }
            obj
        }
    }
}

fn point_to_json(p: &crate::geometry::Point) -> Value {
    json!({"x": p.x, "y": p.y})
}

fn rect_to_json(r: &crate::geometry::Rect) -> Value {
    json!({"x": r.x, "y": r.y, "width": r.width, "height": r.height})
}

fn color_to_json(c: &Color) -> Value {
    json!({"r": c.r, "g": c.g, "b": c.b, "a": c.a})
}

fn style_to_json(style: &Style) -> Value {
    let mut obj = json!({
        "line_width": style.line_width,
        "line_style": line_style_to_str(&style.line_style),
        "font_size": style.font_size,
        "font_family": style.font_family,
        "opacity": style.opacity,
    });
    if let Some(c) = &style.stroke_color {
        obj["stroke_color"] = color_to_json(c);
    }
    if let Some(c) = &style.fill_color {
        obj["fill_color"] = color_to_json(c);
    }
    obj
}

fn line_style_to_str(style: &LineStyle) -> &'static str {
    match style {
        LineStyle::Solid => "Solid",
        LineStyle::Dashed => "Dashed",
        LineStyle::Dotted => "Dotted",
        LineStyle::DashDot => "DashDot",
    }
}

fn transform_to_json(t: &Transform) -> Value {
    json!({"m": t.m})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ChartConfig;
    use crate::geometry::{Point, Rect};
    use crate::render::Renderer;

    #[test]
    fn test_json_renderer_empty() {
        let renderer = JsonRenderer::new();
        let draw_list = DrawList::new();
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
        let json_str = result.expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let parsed: Value = serde_json::from_str(&json_str).expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        assert!(parsed["config"].is_object());
        assert!(parsed["primitives"].is_array());
        assert_eq!(parsed["primitives"].as_array().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)").len(), 0);
    }

    #[test]
    fn test_json_renderer_scene_schema() {
        let renderer = JsonRenderer::new();
        let mut scene = ChartScene::default();
        scene.add_panel(crate::scene::PanelDescriptor {
            id: PanelId::Main,
            rect: Rect::new(0.0, 0.0, 100.0, 80.0),
            visible: true,
        });
        scene.add_layer(crate::scene::LayerDescriptor::new(
            "price",
            PanelId::Main,
            1,
        ));
        scene.add_hit_region(crate::scene::HitRegion {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            target: HitTarget::Kline { index: 4 },
            priority: 1,
            tooltip: Some("bar".into()),
        });
        let json_str = renderer
            .render_scene(&DrawList::new(), &ChartConfig::default(), &scene)
            .expect("scene schema should serialize");
        let parsed: Value = serde_json::from_str(&json_str).expect("valid scene JSON");
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(
            parsed["scene"]["panels"].as_array().expect("panels").len(),
            1
        );
        assert_eq!(parsed["scene"]["hit_regions"][0]["target"]["index"], 4);
    }

    #[test]
    fn test_json_renderer_data_window_payload() {
        let renderer = JsonRenderer::new();
        let data = KlineData::new(
            vec!["2024-01-01".into(), "2024-01-02".into()],
            vec![10.0, 11.0],
            vec![12.0, 13.0],
            vec![9.0, 10.0],
            vec![11.0, 12.0],
            vec![100.0, 120.0],
        );
        let json_str = renderer
            .render_scene_with_data(
                &DrawList::new(),
                &ChartConfig::default(),
                &ChartScene::default(),
                &data,
                4,
                &[(4, 5), (5, 6)],
            )
            .expect("data payload should serialize");
        let parsed: Value = serde_json::from_str(&json_str).expect("valid data JSON");
        assert_eq!(parsed["data"]["revision"], 0);
        assert_eq!(parsed["data"]["source_offset"], 4);
        assert_eq!(parsed["data"]["closes"][1], 12.0);
    }

    #[test]
    fn test_json_renderer_with_line() {
        let renderer = JsonRenderer::new();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Line {
            p1: Point::new(0.0, 0.0),
            p2: Point::new(100.0, 100.0),
            style: Style::default(),
        });
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
        let json_str = result.expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let parsed: Value = serde_json::from_str(&json_str).expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let prims = parsed["primitives"].as_array().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0]["type"], "Line");
        assert_eq!(prims[0]["p1"]["x"], 0.0);
        assert_eq!(prims[0]["p2"]["x"], 100.0);
    }

    #[test]
    fn test_json_renderer_with_filled_rect() {
        let renderer = JsonRenderer::new();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::FilledRect {
            rect: Rect::new(10.0, 20.0, 30.0, 40.0),
            fill: Color::RED,
            stroke: Some(Color::BLACK),
        });
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
        let json_str = result.expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let parsed: Value = serde_json::from_str(&json_str).expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let prims = parsed["primitives"].as_array().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0]["type"], "FilledRect");
        assert_eq!(prims[0]["fill"]["r"], 255);
        assert_eq!(prims[0]["stroke"]["r"], 0);
    }

    #[test]
    fn test_json_renderer_with_circle() {
        let renderer = JsonRenderer::new();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Circle {
            center: Point::new(50.0, 50.0),
            radius: 25.0,
            style: Style::default(),
        });
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
        let json_str = result.expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let parsed: Value = serde_json::from_str(&json_str).expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let prims = parsed["primitives"].as_array().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        assert_eq!(prims[0]["type"], "Circle");
        assert_eq!(prims[0]["radius"], 25.0);
    }

    #[test]
    fn test_json_renderer_with_text() {
        let renderer = JsonRenderer::new();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Text {
            position: Point::new(10.0, 20.0),
            content: "Hello".to_string(),
            style: Style::default(),
        });
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
        let json_str = result.expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let parsed: Value = serde_json::from_str(&json_str).expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let prims = parsed["primitives"].as_array().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        assert_eq!(prims[0]["type"], "Text");
        assert_eq!(prims[0]["content"], "Hello");
    }

    #[test]
    fn test_json_renderer_with_group() {
        let renderer = JsonRenderer::new();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Group {
            primitives: vec![Primitive::Line {
                p1: Point::new(0.0, 0.0),
                p2: Point::new(10.0, 10.0),
                style: Style::default(),
            }],
            transform: Some(Transform::translate(5.0, 5.0)),
        });
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
        let json_str = result.expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let parsed: Value = serde_json::from_str(&json_str).expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let prims = parsed["primitives"].as_array().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        assert_eq!(prims[0]["type"], "Group");
        assert!(prims[0]["transform"].is_object());
        assert_eq!(prims[0]["primitives"].as_array().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)").len(), 1);
    }

    #[test]
    fn test_json_renderer_config_included() {
        let renderer = JsonRenderer::new();
        let draw_list = DrawList::new();
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
        let json_str = result.expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let parsed: Value = serde_json::from_str(&json_str).expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        assert_eq!(parsed["config"]["width"], 1200);
        assert_eq!(parsed["config"]["height"], 600);
    }

    #[test]
    fn test_json_renderer_multiple_primitives() {
        let renderer = JsonRenderer::new();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Line {
            p1: Point::new(0.0, 0.0),
            p2: Point::new(100.0, 100.0),
            style: Style::default(),
        });
        draw_list.push(Primitive::Circle {
            center: Point::new(50.0, 50.0),
            radius: 25.0,
            style: Style::default(),
        });
        draw_list.push(Primitive::Text {
            position: Point::new(10.0, 20.0),
            content: "Test".to_string(),
            style: Style::default(),
        });
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
        let json_str = result.expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let parsed: Value = serde_json::from_str(&json_str).expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let prims = parsed["primitives"].as_array().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        assert_eq!(prims.len(), 3);
    }

    #[test]
    fn test_json_renderer_default_impl() {
        let renderer = JsonRenderer;
        let draw_list = DrawList::new();
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_renderer_polygon() {
        let renderer = JsonRenderer::new();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Polygon {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(5.0, 10.0),
            ],
            style: Style::default(),
        });
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
        let json_str = result.expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let parsed: Value = serde_json::from_str(&json_str).expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let prims = parsed["primitives"].as_array().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        assert_eq!(prims[0]["type"], "Polygon");
        assert_eq!(prims[0]["points"].as_array().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)").len(), 3);
    }

    #[test]
    fn test_json_renderer_path() {
        let renderer = JsonRenderer::new();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Path {
            points: vec![Point::new(0.0, 0.0), Point::new(50.0, 50.0)],
            style: Style::default(),
            close: true,
        });
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
        let json_str = result.expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let parsed: Value = serde_json::from_str(&json_str).expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let prims = parsed["primitives"].as_array().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        assert_eq!(prims[0]["type"], "Path");
        assert_eq!(prims[0]["close"], true);
    }

    #[test]
    fn test_json_renderer_rect() {
        let renderer = JsonRenderer::new();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Rect {
            rect: Rect::new(10.0, 20.0, 30.0, 40.0),
            style: Style::default(),
        });
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
        let json_str = result.expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let parsed: Value = serde_json::from_str(&json_str).expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let prims = parsed["primitives"].as_array().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        assert_eq!(prims[0]["type"], "Rect");
        assert_eq!(prims[0]["rect"]["x"], 10.0);
        assert_eq!(prims[0]["rect"]["width"], 30.0);
    }

    #[test]
    fn test_json_renderer_style_fields() {
        let renderer = JsonRenderer::new();
        let mut draw_list = DrawList::new();
        let style = Style::new()
            .with_stroke(Color::RED)
            .with_fill(Color::BLUE)
            .with_line_width(2.5)
            .with_line_style(LineStyle::Dashed)
            .with_opacity(0.8);
        draw_list.push(Primitive::Line {
            p1: Point::new(0.0, 0.0),
            p2: Point::new(100.0, 100.0),
            style,
        });
        let config = ChartConfig::default();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
        let json_str = result.expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let parsed: Value = serde_json::from_str(&json_str).expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        let style_val = &parsed["primitives"][0]["style"];
        assert_eq!(style_val["line_width"], 2.5);
        assert_eq!(style_val["line_style"], "Dashed");
        let opacity = style_val["opacity"].as_f64().expect("finkit-visualization: unexpected None/Err in visualization/src/render/json.rs (A5 governance)");
        assert!((opacity - 0.8).abs() < 0.01);
        assert_eq!(style_val["stroke_color"]["r"], 255);
        assert_eq!(style_val["fill_color"]["b"], 255);
    }
}
