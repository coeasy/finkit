use crate::config::ChartConfig;
use crate::error::{Result, VisualizationError};
use crate::geometry::Transform;
use crate::primitive::{Color, DrawList, LineStyle, Primitive, Style};
use serde_json::{json, Value};

pub struct JsonRenderer;

impl JsonRenderer {
    pub fn new() -> Self {
        Self
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
