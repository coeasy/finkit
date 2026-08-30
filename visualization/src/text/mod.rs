use crate::geometry::Rect;

#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub font_size: f32,
    pub color: String,
    pub bounds: Rect,
}

impl TextLayout {
    pub fn new(text: String, x: f64, y: f64, font_size: f32, color: String) -> Self {
        let approx_width = text.len() as f64 * font_size as f64 * 0.6;
        let approx_height = font_size as f64 * 1.2;
        let bounds = Rect::new(x, y, approx_width, approx_height);
        Self {
            text,
            x,
            y,
            font_size,
            color,
            bounds,
        }
    }

    pub fn measure(&self) -> (f64, f64) {
        (self.bounds.width, self.bounds.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_layout_new() {
        let layout = TextLayout::new("Hello".to_string(), 10.0, 20.0, 14.0, "#333333".to_string());
        assert_eq!(layout.text, "Hello");
        assert_eq!(layout.x, 10.0);
        assert_eq!(layout.y, 20.0);
        assert_eq!(layout.font_size, 14.0);
        assert!(layout.bounds.width > 0.0);
        assert!(layout.bounds.height > 0.0);
    }

    #[test]
    fn test_text_layout_measure() {
        let layout = TextLayout::new("Test".to_string(), 0.0, 0.0, 12.0, "#000000".to_string());
        let (w, h) = layout.measure();
        assert!(w > 0.0);
        assert!(h > 0.0);
    }
}
