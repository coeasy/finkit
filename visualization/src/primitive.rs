use crate::geometry::{Point, Rect, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const RED: Color = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const GREEN: Color = Color {
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    };
    pub const BLUE: Color = Color {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };
    pub const BLACK: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const TRANSPARENT: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }

    pub fn from_hex(hex: &str) -> Color {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                Color { r, g, b, a: 255 }
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
                Color { r, g, b, a }
            }
            _ => Color::BLACK,
        }
    }

    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    pub fn to_rgba_string(&self) -> String {
        let alpha = self.a as f32 / 255.0;
        format!("rgba({},{},{},{})", self.r, self.g, self.b, alpha)
    }

    pub fn with_alpha(&self, alpha: u8) -> Color {
        Color {
            r: self.r,
            g: self.g,
            b: self.b,
            a: alpha,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::BLACK
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
    DashDot,
}

impl LineStyle {
    pub fn to_svg_dash_array(&self) -> Option<String> {
        match self {
            LineStyle::Solid => None,
            LineStyle::Dashed => Some("8,4".to_string()),
            LineStyle::Dotted => Some("2,2".to_string()),
            LineStyle::DashDot => Some("8,4,2,4".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub stroke_color: Option<Color>,
    pub fill_color: Option<Color>,
    pub line_width: f32,
    pub line_style: LineStyle,
    pub font_size: f32,
    pub font_family: String,
    pub opacity: f32,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            stroke_color: Some(Color::BLACK),
            fill_color: None,
            line_width: 1.0,
            line_style: LineStyle::Solid,
            font_size: 14.0,
            font_family: "sans-serif".to_string(),
            opacity: 1.0,
        }
    }
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_stroke(mut self, color: Color) -> Self {
        self.stroke_color = Some(color);
        self
    }

    pub fn with_fill(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }

    pub fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    pub fn with_line_style(mut self, style: LineStyle) -> Self {
        self.line_style = style;
        self
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn with_font_family(mut self, family: &str) -> Self {
        self.font_family = family.to_string();
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    Line {
        p1: Point,
        p2: Point,
        style: Style,
    },
    Rect {
        rect: Rect,
        style: Style,
    },
    FilledRect {
        rect: Rect,
        fill: Color,
        stroke: Option<Color>,
    },
    Polygon {
        points: Vec<Point>,
        style: Style,
    },
    Path {
        points: Vec<Point>,
        style: Style,
        close: bool,
    },
    Circle {
        center: Point,
        radius: f64,
        style: Style,
    },
    Text {
        position: Point,
        content: String,
        style: Style,
    },
    Group {
        primitives: Vec<Primitive>,
        transform: Option<Transform>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawList {
    pub primitives: Vec<Primitive>,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
        }
    }

    pub fn push(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    pub fn extend(&mut self, other: DrawList) {
        self.primitives.extend(other.primitives);
    }

    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Primitive> {
        self.primitives.iter()
    }

    pub fn clear(&mut self) {
        self.primitives.clear();
    }

    pub fn with_transform(self, transform: Transform) -> DrawList {
        DrawList {
            primitives: vec![Primitive::Group {
                primitives: self.primitives,
                transform: Some(transform),
            }],
        }
    }
}

impl Default for DrawList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_rgba() {
        let c = Color::from_rgba(255, 128, 0, 200);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 0);
        assert_eq!(c.a, 200);
    }

    #[test]
    fn test_color_from_hex_6() {
        let c = Color::from_hex("#ff0000");
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_color_from_hex_8() {
        let c = Color::from_hex("#ff000080");
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
        assert_eq!(c.a, 128);
    }

    #[test]
    fn test_color_from_hex_invalid() {
        let c = Color::from_hex("#xyz");
        assert_eq!(c, Color::BLACK);
    }

    #[test]
    fn test_color_from_hex_short() {
        let c = Color::from_hex("#fff");
        assert_eq!(c, Color::BLACK);
    }

    #[test]
    fn test_color_to_hex_opaque() {
        let c = Color::from_rgba(255, 0, 0, 255);
        assert_eq!(c.to_hex(), "#ff0000");
    }

    #[test]
    fn test_color_to_hex_with_alpha() {
        let c = Color::from_rgba(255, 0, 0, 128);
        assert_eq!(c.to_hex(), "#ff000080");
    }

    #[test]
    fn test_color_to_rgba_string() {
        let c = Color::from_rgba(255, 0, 0, 128);
        assert_eq!(c.to_rgba_string(), "rgba(255,0,0,0.5019608)");
    }

    #[test]
    fn test_color_to_rgba_string_opaque() {
        let c = Color::from_rgba(0, 128, 255, 255);
        assert_eq!(c.to_rgba_string(), "rgba(0,128,255,1)");
    }

    #[test]
    fn test_color_with_alpha() {
        let c = Color::RED.with_alpha(128);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
        assert_eq!(c.a, 128);
    }

    #[test]
    fn test_color_constants() {
        assert_eq!(Color::RED, Color::from_rgba(255, 0, 0, 255));
        assert_eq!(Color::GREEN, Color::from_rgba(0, 255, 0, 255));
        assert_eq!(Color::BLUE, Color::from_rgba(0, 0, 255, 255));
        assert_eq!(Color::BLACK, Color::from_rgba(0, 0, 0, 255));
        assert_eq!(Color::WHITE, Color::from_rgba(255, 255, 255, 255));
        assert_eq!(Color::TRANSPARENT, Color::from_rgba(0, 0, 0, 0));
    }

    #[test]
    fn test_color_default() {
        assert_eq!(Color::default(), Color::BLACK);
    }

    #[test]
    fn test_line_style_solid() {
        assert_eq!(LineStyle::Solid.to_svg_dash_array(), None);
    }

    #[test]
    fn test_line_style_dashed() {
        assert_eq!(
            LineStyle::Dashed.to_svg_dash_array(),
            Some("8,4".to_string())
        );
    }

    #[test]
    fn test_line_style_dotted() {
        assert_eq!(
            LineStyle::Dotted.to_svg_dash_array(),
            Some("2,2".to_string())
        );
    }

    #[test]
    fn test_line_style_dashdot() {
        assert_eq!(
            LineStyle::DashDot.to_svg_dash_array(),
            Some("8,4,2,4".to_string())
        );
    }

    #[test]
    fn test_line_style_default() {
        assert_eq!(LineStyle::default(), LineStyle::Solid);
    }

    #[test]
    fn test_style_default() {
        let s = Style::default();
        assert_eq!(s.stroke_color, Some(Color::BLACK));
        assert_eq!(s.fill_color, None);
        assert!((s.line_width - 1.0).abs() < f32::EPSILON);
        assert_eq!(s.line_style, LineStyle::Solid);
        assert!((s.font_size - 14.0).abs() < f32::EPSILON);
        assert_eq!(s.font_family, "sans-serif");
        assert!((s.opacity - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_style_builder() {
        let s = Style::new()
            .with_stroke(Color::RED)
            .with_fill(Color::BLUE)
            .with_line_width(2.5)
            .with_line_style(LineStyle::Dashed)
            .with_font_size(18.0)
            .with_font_family("monospace")
            .with_opacity(0.5);
        assert_eq!(s.stroke_color, Some(Color::RED));
        assert_eq!(s.fill_color, Some(Color::BLUE));
        assert!((s.line_width - 2.5).abs() < f32::EPSILON);
        assert_eq!(s.line_style, LineStyle::Dashed);
        assert!((s.font_size - 18.0).abs() < f32::EPSILON);
        assert_eq!(s.font_family, "monospace");
        assert!((s.opacity - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_primitive_line() {
        let p = Primitive::Line {
            p1: Point::new(0.0, 0.0),
            p2: Point::new(100.0, 100.0),
            style: Style::default(),
        };
        if let Primitive::Line { p1, p2, .. } = &p {
            assert_eq!(*p1, Point::new(0.0, 0.0));
            assert_eq!(*p2, Point::new(100.0, 100.0));
        } else {
            panic!("Expected Line variant");
        }
    }

    #[test]
    fn test_primitive_rect() {
        let p = Primitive::Rect {
            rect: Rect::new(10.0, 20.0, 100.0, 50.0),
            style: Style::default(),
        };
        if let Primitive::Rect { rect, .. } = &p {
            assert_eq!(rect.x, 10.0);
            assert_eq!(rect.y, 20.0);
            assert_eq!(rect.width, 100.0);
            assert_eq!(rect.height, 50.0);
        } else {
            panic!("Expected Rect variant");
        }
    }

    #[test]
    fn test_primitive_filled_rect() {
        let p = Primitive::FilledRect {
            rect: Rect::new(0.0, 0.0, 50.0, 50.0),
            fill: Color::RED,
            stroke: Some(Color::BLACK),
        };
        if let Primitive::FilledRect { fill, stroke, .. } = &p {
            assert_eq!(*fill, Color::RED);
            assert_eq!(*stroke, Some(Color::BLACK));
        } else {
            panic!("Expected FilledRect variant");
        }
    }

    #[test]
    fn test_primitive_polygon() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(50.0, 100.0),
        ];
        let p = Primitive::Polygon {
            points: points.clone(),
            style: Style::default(),
        };
        if let Primitive::Polygon { points: pts, .. } = &p {
            assert_eq!(pts.len(), 3);
        } else {
            panic!("Expected Polygon variant");
        }
    }

    #[test]
    fn test_primitive_path() {
        let points = vec![Point::new(0.0, 0.0), Point::new(50.0, 50.0)];
        let p = Primitive::Path {
            points: points.clone(),
            style: Style::default(),
            close: true,
        };
        if let Primitive::Path { close, .. } = &p {
            assert!(*close);
        } else {
            panic!("Expected Path variant");
        }
    }

    #[test]
    fn test_primitive_circle() {
        let p = Primitive::Circle {
            center: Point::new(50.0, 50.0),
            radius: 25.0,
            style: Style::default(),
        };
        if let Primitive::Circle { center, radius, .. } = &p {
            assert_eq!(*center, Point::new(50.0, 50.0));
            assert!((radius - 25.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected Circle variant");
        }
    }

    #[test]
    fn test_primitive_text() {
        let p = Primitive::Text {
            position: Point::new(10.0, 20.0),
            content: "Hello".to_string(),
            style: Style::default(),
        };
        if let Primitive::Text { content, .. } = &p {
            assert_eq!(content, "Hello");
        } else {
            panic!("Expected Text variant");
        }
    }

    #[test]
    fn test_primitive_group() {
        let inner = Primitive::Line {
            p1: Point::new(0.0, 0.0),
            p2: Point::new(10.0, 10.0),
            style: Style::default(),
        };
        let p = Primitive::Group {
            primitives: vec![inner],
            transform: Some(Transform::translate(5.0, 5.0)),
        };
        if let Primitive::Group {
            primitives,
            transform,
        } = &p
        {
            assert_eq!(primitives.len(), 1);
            assert_eq!(*transform, Some(Transform::translate(5.0, 5.0)));
        } else {
            panic!("Expected Group variant");
        }
    }

    #[test]
    fn test_draw_list_new() {
        let dl = DrawList::new();
        assert!(dl.is_empty());
        assert_eq!(dl.len(), 0);
    }

    #[test]
    fn test_draw_list_push() {
        let mut dl = DrawList::new();
        dl.push(Primitive::Line {
            p1: Point::new(0.0, 0.0),
            p2: Point::new(10.0, 10.0),
            style: Style::default(),
        });
        assert_eq!(dl.len(), 1);
        assert!(!dl.is_empty());
    }

    #[test]
    fn test_draw_list_extend() {
        let mut dl1 = DrawList::new();
        dl1.push(Primitive::Line {
            p1: Point::new(0.0, 0.0),
            p2: Point::new(10.0, 10.0),
            style: Style::default(),
        });
        let mut dl2 = DrawList::new();
        dl2.push(Primitive::Circle {
            center: Point::new(5.0, 5.0),
            radius: 3.0,
            style: Style::default(),
        });
        dl1.extend(dl2);
        assert_eq!(dl1.len(), 2);
    }

    #[test]
    fn test_draw_list_iter() {
        let mut dl = DrawList::new();
        dl.push(Primitive::Line {
            p1: Point::new(0.0, 0.0),
            p2: Point::new(10.0, 10.0),
            style: Style::default(),
        });
        dl.push(Primitive::Rect {
            rect: Rect::new(0.0, 0.0, 50.0, 50.0),
            style: Style::default(),
        });
        let count = dl.iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_draw_list_clear() {
        let mut dl = DrawList::new();
        dl.push(Primitive::Line {
            p1: Point::new(0.0, 0.0),
            p2: Point::new(10.0, 10.0),
            style: Style::default(),
        });
        dl.clear();
        assert!(dl.is_empty());
    }

    #[test]
    fn test_draw_list_with_transform() {
        let mut dl = DrawList::new();
        dl.push(Primitive::Line {
            p1: Point::new(0.0, 0.0),
            p2: Point::new(10.0, 10.0),
            style: Style::default(),
        });
        let transformed = dl.with_transform(Transform::translate(5.0, 5.0));
        assert_eq!(transformed.len(), 1);
        if let Primitive::Group {
            primitives,
            transform,
        } = &transformed.primitives[0]
        {
            assert_eq!(primitives.len(), 1);
            assert_eq!(*transform, Some(Transform::translate(5.0, 5.0)));
        } else {
            panic!("Expected Group variant");
        }
    }

    #[test]
    fn test_draw_list_default() {
        let dl = DrawList::default();
        assert!(dl.is_empty());
    }

    #[test]
    fn test_color_roundtrip_hex() {
        let colors = vec![
            Color::from_rgba(0, 0, 0, 255),
            Color::from_rgba(255, 255, 255, 255),
            Color::from_rgba(255, 0, 0, 255),
            Color::from_rgba(0, 255, 0, 255),
            Color::from_rgba(0, 0, 255, 255),
            Color::from_rgba(128, 64, 32, 255),
        ];
        for c in colors {
            let hex = c.to_hex();
            let parsed = Color::from_hex(&hex);
            assert_eq!(c, parsed);
        }
    }

    #[test]
    fn test_color_roundtrip_hex_with_alpha() {
        let c = Color::from_rgba(128, 64, 32, 200);
        let hex = c.to_hex();
        let parsed = Color::from_hex(&hex);
        assert_eq!(c, parsed);
    }
}
