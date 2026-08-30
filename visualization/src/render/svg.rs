use crate::config::ChartConfig;
use crate::error::Result;
use crate::geometry::Transform;
use crate::primitive::{DrawList, Primitive, Style};

use super::Renderer;

pub struct SvgRenderer;

impl SvgRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SvgRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for SvgRenderer {
    fn render(&self, draw_list: &DrawList, config: &ChartConfig) -> Result<String> {
        let estimated_size = draw_list.primitives.len() * 256 + 1024;
        let mut svg = String::with_capacity(estimated_size);
        svg.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
            config.width, config.height, config.width, config.height
        ));

        svg.push_str(&format!(
            "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>\n",
            config.theme_config.background_color
        ));

        for prim in &draw_list.primitives {
            svg.push_str(&Self::render_primitive(prim, 0));
        }

        svg.push_str("</svg>");
        Ok(svg)
    }
}

impl SvgRenderer {
    fn render_primitive(prim: &Primitive, indent: usize) -> String {
        let pad = "  ".repeat(indent + 1);
        match prim {
            Primitive::Line { p1, p2, style } => {
                let style_str = Self::style_to_svg(style);
                format!(
                    "{}<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" style=\"{}\"/>\n",
                    pad, p1.x, p1.y, p2.x, p2.y, style_str
                )
            }
            Primitive::Rect { rect, style } => {
                let style_str = Self::style_to_svg(style);
                format!(
                    "{}<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" style=\"{}\"/>\n",
                    pad, rect.x, rect.y, rect.width, rect.height, style_str
                )
            }
            Primitive::FilledRect { rect, fill, stroke } => {
                let fill_str = fill.to_hex();
                let stroke_str = match stroke {
                    Some(c) => format!(" stroke=\"{}\"", c.to_hex()),
                    None => String::new(),
                };
                format!(
                    "{}<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"{}/>\n",
                    pad, rect.x, rect.y, rect.width, rect.height, fill_str, stroke_str
                )
            }
            Primitive::Polygon { points, style } => {
                let points_str = points
                    .iter()
                    .map(|p| format!("{:.2},{:.2}", p.x, p.y))
                    .collect::<Vec<_>>()
                    .join(" ");
                let style_str = Self::style_to_svg(style);
                format!(
                    "{}<polygon points=\"{}\" style=\"{}\"/>\n",
                    pad, points_str, style_str
                )
            }
            Primitive::Path {
                points,
                style,
                close,
            } => {
                if points.is_empty() {
                    return String::new();
                }
                let mut d = format!("M{:.2} {:.2}", points[0].x, points[0].y);
                for p in &points[1..] {
                    d.push_str(&format!(" L{:.2} {:.2}", p.x, p.y));
                }
                if *close {
                    d.push_str(" Z");
                }
                let style_str = Self::style_to_svg(style);
                format!("{}<path d=\"{}\" style=\"{}\"/>\n", pad, d, style_str)
            }
            Primitive::Circle {
                center,
                radius,
                style,
            } => {
                let style_str = Self::style_to_svg(style);
                format!(
                    "{}<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" style=\"{}\"/>\n",
                    pad, center.x, center.y, radius, style_str
                )
            }
            Primitive::Text {
                position,
                content,
                style,
            } => {
                let style_str = Self::style_to_svg(style);
                let escaped = Self::escape_xml(content);
                format!(
                    "{}<text x=\"{:.2}\" y=\"{:.2}\" style=\"{}\">{}</text>\n",
                    pad, position.x, position.y, style_str, escaped
                )
            }
            Primitive::Group {
                primitives,
                transform,
            } => {
                let transform_attr = match transform {
                    Some(t) => format!(" transform=\"{}\"", Self::transform_to_svg(t)),
                    None => String::new(),
                };
                let mut s = format!("{}<g{}>\n", pad, transform_attr);
                for child in primitives {
                    s.push_str(&Self::render_primitive(child, indent + 1));
                }
                s.push_str(&format!("{}</g>\n", pad));
                s
            }
        }
    }

    fn style_to_svg(style: &Style) -> String {
        let mut parts: Vec<String> = Vec::new();

        match &style.stroke_color {
            Some(c) => parts.push(format!("stroke:{}", c.to_hex())),
            None => parts.push("stroke:none".to_string()),
        }

        match &style.fill_color {
            Some(c) => parts.push(format!("fill:{}", c.to_hex())),
            None => parts.push("fill:none".to_string()),
        }

        parts.push(format!("stroke-width:{}px", style.line_width));

        if let Some(dash) = style.line_style.to_svg_dash_array() {
            parts.push(format!("stroke-dasharray:{}", dash));
        }

        parts.push(format!("font-size:{}px", style.font_size));
        parts.push(format!("font-family:{}", style.font_family));

        if (style.opacity - 1.0).abs() > f32::EPSILON {
            parts.push(format!("opacity:{}", style.opacity));
        }

        parts.join(";")
    }

    fn transform_to_svg(t: &Transform) -> String {
        format!(
            "matrix({:.2},{:.2},{:.2},{:.2},{:.2},{:.2})",
            t.m[0], t.m[3], t.m[1], t.m[4], t.m[2], t.m[5]
        )
    }

    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point, Rect};
    use crate::primitive::{Color, LineStyle};

    fn default_config() -> ChartConfig {
        ChartConfig::default()
    }

    #[test]
    fn test_svg_header() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.starts_with("<svg"));
        assert!(result.contains("xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(result.contains(&format!("width=\"{}\"", config.width)));
        assert!(result.contains(&format!("height=\"{}\"", config.height)));
        assert!(result.contains(&format!(
            "viewBox=\"0 0 {} {}\"",
            config.width, config.height
        )));
        assert!(result.ends_with("</svg>"));
    }

    #[test]
    fn test_svg_background() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains(&format!(
            "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
            config.theme_config.background_color
        )));
    }

    #[test]
    fn test_render_line() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Line {
            p1: Point::new(10.0, 20.0),
            p2: Point::new(100.0, 200.0),
            style: Style::default(),
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<line"));
        assert!(result.contains("x1=\"10.00\""));
        assert!(result.contains("y1=\"20.00\""));
        assert!(result.contains("x2=\"100.00\""));
        assert!(result.contains("y2=\"200.00\""));
    }

    #[test]
    fn test_render_rect() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Rect {
            rect: Rect::new(10.0, 20.0, 100.0, 50.0),
            style: Style::default(),
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<rect"));
        assert!(result.contains("x=\"10.00\""));
        assert!(result.contains("y=\"20.00\""));
        assert!(result.contains("width=\"100.00\""));
        assert!(result.contains("height=\"50.00\""));
    }

    #[test]
    fn test_render_filled_rect() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::FilledRect {
            rect: Rect::new(10.0, 20.0, 100.0, 50.0),
            fill: Color::RED,
            stroke: Some(Color::BLACK),
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<rect"));
        assert!(result.contains("fill=\"#ff0000\""));
        assert!(result.contains("stroke=\"#000000\""));
    }

    #[test]
    fn test_render_filled_rect_no_stroke() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::FilledRect {
            rect: Rect::new(0.0, 0.0, 50.0, 50.0),
            fill: Color::GREEN,
            stroke: None,
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("fill=\"#00ff00\""));
        assert!(!result.contains("stroke="));
    }

    #[test]
    fn test_render_polygon() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Polygon {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(100.0, 0.0),
                Point::new(50.0, 100.0),
            ],
            style: Style::default(),
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<polygon"));
        assert!(result.contains("points=\"0.00,0.00 100.00,0.00 50.00,100.00\""));
    }

    #[test]
    fn test_render_path_open() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Path {
            points: vec![
                Point::new(10.0, 10.0),
                Point::new(50.0, 50.0),
                Point::new(100.0, 10.0),
            ],
            style: Style::default(),
            close: false,
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<path"));
        assert!(result.contains("d=\"M10.00 10.00 L50.00 50.00 L100.00 10.00\""));
        assert!(!result.contains(" Z"));
    }

    #[test]
    fn test_render_path_closed() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Path {
            points: vec![Point::new(10.0, 10.0), Point::new(50.0, 50.0)],
            style: Style::default(),
            close: true,
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains(" Z\""));
    }

    #[test]
    fn test_render_path_empty() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Path {
            points: vec![],
            style: Style::default(),
            close: false,
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(!result.contains("<path"));
    }

    #[test]
    fn test_render_circle() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Circle {
            center: Point::new(50.0, 50.0),
            radius: 25.0,
            style: Style::default(),
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<circle"));
        assert!(result.contains("cx=\"50.00\""));
        assert!(result.contains("cy=\"50.00\""));
        assert!(result.contains("r=\"25.00\""));
    }

    #[test]
    fn test_render_text() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Text {
            position: Point::new(10.0, 20.0),
            content: "Hello SVG".to_string(),
            style: Style::default(),
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<text"));
        assert!(result.contains("x=\"10.00\""));
        assert!(result.contains("y=\"20.00\""));
        assert!(result.contains(">Hello SVG</text>"));
    }

    #[test]
    fn test_render_text_xml_escape() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Text {
            position: Point::new(0.0, 0.0),
            content: "<script>alert('xss')&\"</script>".to_string(),
            style: Style::default(),
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("&lt;script&gt;alert(&apos;xss&apos;)&amp;&quot;&lt;/script&gt;"));
    }

    #[test]
    fn test_render_group_no_transform() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Group {
            primitives: vec![Primitive::Line {
                p1: Point::new(0.0, 0.0),
                p2: Point::new(10.0, 10.0),
                style: Style::default(),
            }],
            transform: None,
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<g>"));
        assert!(result.contains("</g>"));
        assert!(!result.contains("transform="));
    }

    #[test]
    fn test_render_group_with_transform() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Group {
            primitives: vec![Primitive::Line {
                p1: Point::new(0.0, 0.0),
                p2: Point::new(10.0, 10.0),
                style: Style::default(),
            }],
            transform: Some(Transform::translate(5.0, 10.0)),
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<g transform=\"matrix(1.00,0.00,0.00,1.00,5.00,10.00)\">"));
    }

    #[test]
    fn test_render_nested_group() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Group {
            primitives: vec![Primitive::Group {
                primitives: vec![Primitive::Circle {
                    center: Point::new(5.0, 5.0),
                    radius: 3.0,
                    style: Style::default(),
                }],
                transform: Some(Transform::scale(2.0, 2.0)),
            }],
            transform: Some(Transform::translate(10.0, 20.0)),
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("matrix(1.00,0.00,0.00,1.00,10.00,20.00)"));
        assert!(result.contains("matrix(2.00,0.00,0.00,2.00,0.00,0.00)"));
        assert!(result.contains("cx=\"5.00\""));
    }

    #[test]
    fn test_style_stroke_color() {
        let style = Style::new().with_stroke(Color::RED);
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("stroke:#ff0000"));
    }

    #[test]
    fn test_style_no_stroke() {
        let style = Style {
            stroke_color: None,
            ..Style::default()
        };
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("stroke:none"));
    }

    #[test]
    fn test_style_fill_color() {
        let style = Style::new().with_fill(Color::BLUE);
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("fill:#0000ff"));
    }

    #[test]
    fn test_style_no_fill() {
        let style = Style::default();
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("fill:none"));
    }

    #[test]
    fn test_style_line_width() {
        let style = Style::new().with_line_width(2.5);
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("stroke-width:2.5px"));
    }

    #[test]
    fn test_style_line_style_dashed() {
        let style = Style::new().with_line_style(LineStyle::Dashed);
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("stroke-dasharray:8,4"));
    }

    #[test]
    fn test_style_line_style_dotted() {
        let style = Style::new().with_line_style(LineStyle::Dotted);
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("stroke-dasharray:2,2"));
    }

    #[test]
    fn test_style_line_style_dashdot() {
        let style = Style::new().with_line_style(LineStyle::DashDot);
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("stroke-dasharray:8,4,2,4"));
    }

    #[test]
    fn test_style_line_style_solid_no_dasharray() {
        let style = Style::new().with_line_style(LineStyle::Solid);
        let result = SvgRenderer::style_to_svg(&style);
        assert!(!result.contains("stroke-dasharray"));
    }

    #[test]
    fn test_style_font_size() {
        let style = Style::new().with_font_size(18.0);
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("font-size:18px"));
    }

    #[test]
    fn test_style_font_family() {
        let style = Style::new().with_font_family("monospace");
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("font-family:monospace"));
    }

    #[test]
    fn test_style_opacity() {
        let style = Style::new().with_opacity(0.5);
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("opacity:0.5"));
    }

    #[test]
    fn test_style_full_opacity_omitted() {
        let style = Style::new().with_opacity(1.0);
        let result = SvgRenderer::style_to_svg(&style);
        assert!(!result.contains("opacity:"));
    }

    #[test]
    fn test_style_combined() {
        let style = Style::new()
            .with_stroke(Color::RED)
            .with_fill(Color::GREEN)
            .with_line_width(3.0)
            .with_line_style(LineStyle::Dashed)
            .with_font_size(16.0)
            .with_font_family("serif")
            .with_opacity(0.8);
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("stroke:#ff0000"));
        assert!(result.contains("fill:#00ff00"));
        assert!(result.contains("stroke-width:3px"));
        assert!(result.contains("stroke-dasharray:8,4"));
        assert!(result.contains("font-size:16px"));
        assert!(result.contains("font-family:serif"));
        assert!(result.contains("opacity:0.8"));
    }

    #[test]
    fn test_transform_to_svg_identity() {
        let t = Transform::identity();
        let result = SvgRenderer::transform_to_svg(&t);
        assert_eq!(result, "matrix(1.00,0.00,0.00,1.00,0.00,0.00)");
    }

    #[test]
    fn test_transform_to_svg_translate() {
        let t = Transform::translate(10.0, 20.0);
        let result = SvgRenderer::transform_to_svg(&t);
        assert_eq!(result, "matrix(1.00,0.00,0.00,1.00,10.00,20.00)");
    }

    #[test]
    fn test_transform_to_svg_scale() {
        let t = Transform::scale(2.0, 3.0);
        let result = SvgRenderer::transform_to_svg(&t);
        assert_eq!(result, "matrix(2.00,0.00,0.00,3.00,0.00,0.00)");
    }

    #[test]
    fn test_transform_to_svg_combined() {
        let t = Transform::scale(2.0, 2.0).then(&Transform::translate(10.0, 20.0));
        let result = SvgRenderer::transform_to_svg(&t);
        assert!(result.starts_with("matrix("));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(SvgRenderer::escape_xml("a&b"), "a&amp;b");
        assert_eq!(SvgRenderer::escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(SvgRenderer::escape_xml("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(SvgRenderer::escape_xml("it's"), "it&apos;s");
        assert_eq!(SvgRenderer::escape_xml("normal"), "normal");
    }

    #[test]
    fn test_color_to_hex_in_svg() {
        let style = Style::new().with_stroke(Color::from_rgba(255, 128, 0, 255));
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("stroke:#ff8000"));
    }

    #[test]
    fn test_color_with_alpha_in_svg() {
        let style = Style::new().with_stroke(Color::from_rgba(255, 0, 0, 128));
        let result = SvgRenderer::style_to_svg(&style);
        assert!(result.contains("stroke:#ff000080"));
    }

    #[test]
    fn test_empty_draw_list() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<svg"));
        assert!(result.contains("</svg>"));
        let body_start = result.find("</svg>").expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        let bg_end = result.find("<rect width=\"100%\"").expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(bg_end < body_start);
    }

    #[test]
    fn test_multiple_primitives() {
        let renderer = SvgRenderer::new();
        let config = default_config();
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
            position: Point::new(10.0, 10.0),
            content: "Test".to_string(),
            style: Style::default(),
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<line"));
        assert!(result.contains("<circle"));
        assert!(result.contains("<text"));
    }

    #[test]
    fn test_candlestick_svg() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();

        let up_color = Color::from_hex("#ef4444");
        let down_color = Color::from_hex("#22c55e");

        draw_list.push(Primitive::Line {
            p1: Point::new(50.0, 100.0),
            p2: Point::new(50.0, 300.0),
            style: Style::new().with_stroke(up_color).with_line_width(1.0),
        });
        draw_list.push(Primitive::FilledRect {
            rect: Rect::new(40.0, 150.0, 20.0, 80.0),
            fill: up_color,
            stroke: Some(up_color),
        });

        draw_list.push(Primitive::Line {
            p1: Point::new(100.0, 120.0),
            p2: Point::new(100.0, 280.0),
            style: Style::new().with_stroke(down_color).with_line_width(1.0),
        });
        draw_list.push(Primitive::FilledRect {
            rect: Rect::new(90.0, 160.0, 20.0, 70.0),
            fill: down_color,
            stroke: Some(down_color),
        });

        draw_list.push(Primitive::Path {
            points: vec![Point::new(50.0, 200.0), Point::new(100.0, 180.0)],
            style: Style::new()
                .with_stroke(Color::from_hex("#3b82f6"))
                .with_line_width(1.5),
            close: false,
        });

        draw_list.push(Primitive::Text {
            position: Point::new(10.0, 20.0),
            content: "K-Line Chart".to_string(),
            style: Style::new()
                .with_font_size(16.0)
                .with_stroke(Color::BLACK)
                .with_fill(Color::BLACK),
        });

        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<svg"));
        assert!(result.contains("fill=\"#ef4444\""));
        assert!(result.contains("fill=\"#22c55e\""));
        assert!(result.contains("stroke:#3b82f6"));
        assert!(result.contains(">K-Line Chart</text>"));
        assert!(result.contains("</svg>"));
    }

    #[test]
    fn test_svg_renderer_default() {
        let renderer = SvgRenderer;
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_single_point_path() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Path {
            points: vec![Point::new(10.0, 20.0)],
            style: Style::default(),
            close: false,
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("d=\"M10.00 20.00\""));
    }

    #[test]
    fn test_render_group_multiple_children() {
        let renderer = SvgRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Group {
            primitives: vec![
                Primitive::Rect {
                    rect: Rect::new(0.0, 0.0, 50.0, 50.0),
                    style: Style::default(),
                },
                Primitive::Circle {
                    center: Point::new(25.0, 25.0),
                    radius: 10.0,
                    style: Style::default(),
                },
            ],
            transform: None,
        });
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/svg.rs (A5 governance)");
        assert!(result.contains("<g>"));
        assert!(result.contains("<rect"));
        assert!(result.contains("<circle"));
        assert!(result.contains("</g>"));
    }
}
