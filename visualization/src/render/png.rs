use crate::config::ChartConfig;
use crate::error::{Result, VisualizationError};
use crate::primitive::{Color, DrawList, Primitive};

use super::Renderer;

pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let pixels = vec![255; (width * height * 4) as usize];
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8, a: u8) {
        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        self.pixels[idx] = r;
        self.pixels[idx + 1] = g;
        self.pixels[idx + 2] = b;
        self.pixels[idx + 3] = a;
    }

    pub fn blend_pixel(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let ux = x as u32;
        let uy = y as u32;
        let idx = ((uy * self.width + ux) * 4) as usize;
        let src_a = a as f32 / 255.0;
        let dst_a = self.pixels[idx + 3] as f32 / 255.0;
        let out_a = src_a + dst_a * (1.0 - src_a);
        if out_a == 0.0 {
            self.pixels[idx] = 0;
            self.pixels[idx + 1] = 0;
            self.pixels[idx + 2] = 0;
            self.pixels[idx + 3] = 0;
            return;
        }
        let out_r = (r as f32 * src_a + self.pixels[idx] as f32 * dst_a * (1.0 - src_a)) / out_a;
        let out_g =
            (g as f32 * src_a + self.pixels[idx + 1] as f32 * dst_a * (1.0 - src_a)) / out_a;
        let out_b =
            (b as f32 * src_a + self.pixels[idx + 2] as f32 * dst_a * (1.0 - src_a)) / out_a;
        self.pixels[idx] = out_r.round().min(255.0) as u8;
        self.pixels[idx + 1] = out_g.round().min(255.0) as u8;
        self.pixels[idx + 2] = out_b.round().min(255.0) as u8;
        self.pixels[idx + 3] = (out_a * 255.0).round().min(255.0) as u8;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, r: u8, g: u8, b: u8) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut cx = x0;
        let mut cy = y0;
        loop {
            self.set_pixel(cx as u32, cy as u32, r, g, b, 255);
            let e2 = 2 * err;
            if e2 >= dy {
                if cx == x1 {
                    break;
                }
                err += dy;
                cx += sx;
            }
            if e2 <= dx {
                if cy == y1 {
                    break;
                }
                err += dx;
                cy += sy;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_line_aa(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        r: u8,
        g: u8,
        b: u8,
        width: f64,
    ) {
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        let (mut x0, mut y0, mut x1, mut y1) = if steep {
            (y0, x0, y1, x1)
        } else {
            (x0, y0, x1, y1)
        };
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }
        let dx = x1 - x0;
        let dy = y1 - y0;
        let gradient = if dx.abs() < 1e-10 { 1.0 } else { dy / dx };

        let half_width = width / 2.0;

        let fpart = |x: f64| x - x.floor();
        let rfpart = |x: f64| 1.0 - fpart(x);

        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xgap = rfpart(x0 + 0.5);
        let xpxl1 = xend as i32;
        let ypxl1 = yend.floor() as i32;
        let fpart_y = fpart(yend);

        let alpha1 = (rfpart(yend) * xgap * (255.0 / half_width.max(1.0))).min(255.0) as u8;
        let alpha2 = (fpart_y * xgap * (255.0 / half_width.max(1.0))).min(255.0) as u8;

        if steep {
            self.blend_pixel(ypxl1, xpxl1, r, g, b, alpha1);
            self.blend_pixel(ypxl1 + 1, xpxl1, r, g, b, alpha2);
        } else {
            self.blend_pixel(xpxl1, ypxl1, r, g, b, alpha1);
            self.blend_pixel(xpxl1, ypxl1 + 1, r, g, b, alpha2);
        }
        let mut intery = yend + gradient;

        let xend2 = x1.round();
        let yend2 = y1 + gradient * (xend2 - x1);
        let xgap2 = fpart(x1 + 0.5);
        let xpxl2 = xend2 as i32;
        let ypxl2 = yend2.floor() as i32;
        let fpart_y2 = fpart(yend2);

        let alpha3 = (rfpart(yend2) * xgap2 * (255.0 / half_width.max(1.0))).min(255.0) as u8;
        let alpha4 = (fpart_y2 * xgap2 * (255.0 / half_width.max(1.0))).min(255.0) as u8;

        if steep {
            self.blend_pixel(ypxl2, xpxl2, r, g, b, alpha3);
            self.blend_pixel(ypxl2 + 1, xpxl2, r, g, b, alpha4);
        } else {
            self.blend_pixel(xpxl2, ypxl2, r, g, b, alpha3);
            self.blend_pixel(xpxl2, ypxl2 + 1, r, g, b, alpha4);
        }

        for x in (xpxl1 + 1)..xpxl2 {
            let fy = intery.floor() as i32;
            let fpart_val = fpart(intery);
            let a1 = (rfpart(intery) * (255.0 / half_width.max(1.0))).min(255.0) as u8;
            let a2 = (fpart_val * (255.0 / half_width.max(1.0))).min(255.0) as u8;
            if steep {
                self.blend_pixel(fy, x, r, g, b, a1);
                self.blend_pixel(fy + 1, x, r, g, b, a2);
            } else {
                self.blend_pixel(x, fy, r, g, b, a1);
                self.blend_pixel(x, fy + 1, r, g, b, a2);
            }
            intery += gradient;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8, a: u8) {
        for py in y..(y + h) {
            for px in x..(x + w) {
                self.blend_pixel(px, py, r, g, b, a);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stroke_rect(&mut self, x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8) {
        self.draw_line(x, y, x + w - 1, y, r, g, b);
        self.draw_line(x + w - 1, y, x + w - 1, y + h - 1, r, g, b);
        self.draw_line(x + w - 1, y + h - 1, x, y + h - 1, r, g, b);
        self.draw_line(x, y + h - 1, x, y, r, g, b);
    }
}

pub struct PngRenderer;

impl PngRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PngRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for PngRenderer {
    fn render(&self, draw_list: &DrawList, config: &ChartConfig) -> Result<String> {
        let mut fb = FrameBuffer::new(config.width, config.height);
        let bg = Color::from_hex(&config.theme_config.background_color);
        fb.clear(bg.r, bg.g, bg.b, 255);
        for prim in &draw_list.primitives {
            Self::render_primitive(prim, &mut fb);
        }
        let data = encode_png(&fb)?;
        Ok(base64_encode(&data))
    }
}

impl PngRenderer {
    fn render_primitive(prim: &Primitive, fb: &mut FrameBuffer) {
        match prim {
            Primitive::Line { p1, p2, style } => {
                let color = style.stroke_color.unwrap_or(Color::BLACK);
                let line_width = style.line_width as f64;
                fb.draw_line_aa(
                    p1.x, p1.y, p2.x, p2.y, color.r, color.g, color.b, line_width,
                );
            }
            Primitive::Rect { rect, style } => {
                let x = rect.x as i32;
                let y = rect.y as i32;
                let w = rect.width as i32;
                let h = rect.height as i32;
                if let Some(fill) = &style.fill_color {
                    fb.fill_rect(x, y, w, h, fill.r, fill.g, fill.b, fill.a);
                }
                if let Some(stroke) = &style.stroke_color {
                    fb.stroke_rect(x, y, w, h, stroke.r, stroke.g, stroke.b);
                }
            }
            Primitive::FilledRect { rect, fill, stroke } => {
                let x = rect.x as i32;
                let y = rect.y as i32;
                let w = rect.width as i32;
                let h = rect.height as i32;
                fb.fill_rect(x, y, w, h, fill.r, fill.g, fill.b, fill.a);
                if let Some(stroke_color) = stroke {
                    fb.stroke_rect(x, y, w, h, stroke_color.r, stroke_color.g, stroke_color.b);
                }
            }
            Primitive::Circle {
                center,
                radius,
                style,
            } => {
                let cx = center.x;
                let cy = center.y;
                let r = *radius;
                let color = style.stroke_color.unwrap_or(Color::BLACK);
                let mut x = 0_i32;
                let mut y = r.round() as i32;
                let mut d = 1 - y;
                while x <= y {
                    fb.set_pixel(
                        (cx.round() as i32 + x) as u32,
                        (cy.round() as i32 + y) as u32,
                        color.r,
                        color.g,
                        color.b,
                        color.a,
                    );
                    fb.set_pixel(
                        (cx.round() as i32 + y) as u32,
                        (cy.round() as i32 + x) as u32,
                        color.r,
                        color.g,
                        color.b,
                        color.a,
                    );
                    fb.set_pixel(
                        (cx.round() as i32 - x) as u32,
                        (cy.round() as i32 + y) as u32,
                        color.r,
                        color.g,
                        color.b,
                        color.a,
                    );
                    fb.set_pixel(
                        (cx.round() as i32 - y) as u32,
                        (cy.round() as i32 + x) as u32,
                        color.r,
                        color.g,
                        color.b,
                        color.a,
                    );
                    fb.set_pixel(
                        (cx.round() as i32 + x) as u32,
                        (cy.round() as i32 - y) as u32,
                        color.r,
                        color.g,
                        color.b,
                        color.a,
                    );
                    fb.set_pixel(
                        (cx.round() as i32 + y) as u32,
                        (cy.round() as i32 - x) as u32,
                        color.r,
                        color.g,
                        color.b,
                        color.a,
                    );
                    fb.set_pixel(
                        (cx.round() as i32 - x) as u32,
                        (cy.round() as i32 - y) as u32,
                        color.r,
                        color.g,
                        color.b,
                        color.a,
                    );
                    fb.set_pixel(
                        (cx.round() as i32 - y) as u32,
                        (cy.round() as i32 - x) as u32,
                        color.r,
                        color.g,
                        color.b,
                        color.a,
                    );
                    x += 1;
                    if d < 0 {
                        d += 2 * x + 1;
                    } else {
                        y -= 1;
                        d += 2 * (x - y) + 1;
                    }
                }
            }
            Primitive::Polygon { points, style } => {
                if points.len() < 2 {
                    return;
                }
                let color = style.stroke_color.unwrap_or(Color::BLACK);
                let line_width = style.line_width as f64;
                for i in 0..points.len() {
                    let j = (i + 1) % points.len();
                    fb.draw_line_aa(
                        points[i].x,
                        points[i].y,
                        points[j].x,
                        points[j].y,
                        color.r,
                        color.g,
                        color.b,
                        line_width,
                    );
                }
            }
            Primitive::Path { points, style, .. } => {
                if points.len() < 2 {
                    return;
                }
                let color = style.stroke_color.unwrap_or(Color::BLACK);
                let line_width = style.line_width as f64;
                for i in 0..points.len() - 1 {
                    fb.draw_line_aa(
                        points[i].x,
                        points[i].y,
                        points[i + 1].x,
                        points[i + 1].y,
                        color.r,
                        color.g,
                        color.b,
                        line_width,
                    );
                }
            }
            Primitive::Text {
                position, style, ..
            } => {
                let x = position.x as i32;
                let y = position.y as i32;
                let w = (style.font_size * 6.0) as i32;
                let h = style.font_size as i32;
                if let Some(fill) = &style.fill_color {
                    fb.fill_rect(x, y, w, h, fill.r, fill.g, fill.b, fill.a);
                }
            }
            Primitive::Group { primitives, .. } => {
                for child in primitives {
                    Self::render_primitive(child, fb);
                }
            }
        }
    }
}

fn encode_png(fb: &FrameBuffer) -> Result<Vec<u8>> {
    use std::io::Cursor;
    let mut buf = Cursor::new(Vec::new());
    {
        let mut encoder = png::Encoder::new(&mut buf, fb.width, fb.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer =
            encoder
                .write_header()
                .map_err(|e| VisualizationError::PngEncodeError {
                    message: e.to_string(),
                })?;
        writer
            .write_image_data(&fb.pixels)
            .map_err(|e| VisualizationError::PngEncodeError {
                message: e.to_string(),
            })?;
    }
    Ok(buf.into_inner())
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 2 < data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        result.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        result.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        result.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        result.push(TABLE[(n & 0x3F) as usize] as char);
        i += 3;
    }
    if data.len() - i == 2 {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
        result.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        result.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        result.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        result.push('=');
    } else if data.len() - i == 1 {
        let n = (data[i] as u32) << 16;
        result.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        result.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        result.push_str("==");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitive::Style;

    #[test]
    fn test_framebuffer_new() {
        let fb = FrameBuffer::new(10, 20);
        assert_eq!(fb.width, 10);
        assert_eq!(fb.height, 20);
        assert_eq!(fb.pixels.len(), 10 * 20 * 4);
        for chunk in fb.pixels.chunks_exact(4) {
            assert_eq!(chunk, &[255, 255, 255, 255]);
        }
    }

    #[test]
    fn test_framebuffer_clear() {
        let mut fb = FrameBuffer::new(5, 5);
        fb.clear(100, 150, 200, 255);
        for chunk in fb.pixels.chunks_exact(4) {
            assert_eq!(chunk, &[100, 150, 200, 255]);
        }
    }

    #[test]
    fn test_framebuffer_set_pixel() {
        let mut fb = FrameBuffer::new(10, 10);
        fb.set_pixel(3, 5, 255, 0, 0, 128);
        let idx = ((5 * 10 + 3) * 4) as usize;
        assert_eq!(fb.pixels[idx], 255);
        assert_eq!(fb.pixels[idx + 1], 0);
        assert_eq!(fb.pixels[idx + 2], 0);
        assert_eq!(fb.pixels[idx + 3], 128);
    }

    #[test]
    fn test_framebuffer_set_pixel_out_of_bounds() {
        let mut fb = FrameBuffer::new(10, 10);
        fb.set_pixel(20, 20, 255, 0, 0, 128);
    }

    #[test]
    fn test_framebuffer_blend_pixel() {
        let mut fb = FrameBuffer::new(10, 10);
        fb.clear(0, 0, 0, 255);
        fb.blend_pixel(5, 5, 255, 0, 0, 128);
        let idx = ((5 * 10 + 5) * 4) as usize;
        assert_eq!(fb.pixels[idx], 128);
        assert_eq!(fb.pixels[idx + 1], 0);
        assert_eq!(fb.pixels[idx + 2], 0);
        assert_eq!(fb.pixels[idx + 3], 255);
    }

    #[test]
    fn test_framebuffer_blend_pixel_out_of_bounds() {
        let mut fb = FrameBuffer::new(10, 10);
        fb.blend_pixel(-1, 5, 255, 0, 0, 128);
        fb.blend_pixel(5, -1, 255, 0, 0, 128);
        fb.blend_pixel(10, 5, 255, 0, 0, 128);
        fb.blend_pixel(5, 10, 255, 0, 0, 128);
    }

    #[test]
    fn test_framebuffer_blend_pixel_transparent() {
        let mut fb = FrameBuffer::new(10, 10);
        fb.clear(255, 0, 0, 255);
        fb.blend_pixel(5, 5, 0, 255, 0, 0);
        let idx = ((5 * 10 + 5) * 4) as usize;
        assert_eq!(fb.pixels[idx], 255);
        assert_eq!(fb.pixels[idx + 1], 0);
        assert_eq!(fb.pixels[idx + 2], 0);
    }

    #[test]
    fn test_draw_line_horizontal() {
        let mut fb = FrameBuffer::new(20, 20);
        fb.clear(0, 0, 0, 255);
        fb.draw_line(0, 5, 9, 5, 255, 255, 255);
        for x in 0..10u32 {
            let idx = ((5 * 20 + x) * 4) as usize;
            assert_eq!(fb.pixels[idx], 255);
        }
    }

    #[test]
    fn test_draw_line_vertical() {
        let mut fb = FrameBuffer::new(20, 20);
        fb.clear(0, 0, 0, 255);
        fb.draw_line(5, 0, 5, 9, 255, 255, 255);
        for y in 0..10u32 {
            let idx = ((y * 20 + 5) * 4) as usize;
            assert_eq!(fb.pixels[idx], 255);
        }
    }

    #[test]
    fn test_draw_line_diagonal() {
        let mut fb = FrameBuffer::new(20, 20);
        fb.clear(0, 0, 0, 255);
        fb.draw_line(0, 0, 9, 9, 255, 255, 255);
        for i in 0..10u32 {
            let idx = ((i * 20 + i) * 4) as usize;
            assert_eq!(fb.pixels[idx], 255);
        }
    }

    #[test]
    fn test_draw_line_aa() {
        let mut fb = FrameBuffer::new(100, 100);
        fb.clear(0, 0, 0, 255);
        fb.draw_line_aa(10.0, 10.0, 90.0, 90.0, 255, 255, 255, 1.0);
        let idx = ((10 * 100 + 10) * 4) as usize;
        assert!(fb.pixels[idx] > 0 || fb.pixels[idx + 1] > 0 || fb.pixels[idx + 2] > 0);
    }

    #[test]
    fn test_fill_rect() {
        let mut fb = FrameBuffer::new(20, 20);
        fb.clear(0, 0, 0, 255);
        fb.fill_rect(5, 5, 10, 10, 255, 0, 0, 255);
        for y in 5..15u32 {
            for x in 5..15u32 {
                let idx = ((y * 20 + x) * 4) as usize;
                assert_eq!(fb.pixels[idx], 255);
                assert_eq!(fb.pixels[idx + 1], 0);
                assert_eq!(fb.pixels[idx + 2], 0);
            }
        }
        let idx = 0;
        assert_eq!(fb.pixels[idx], 0);
    }

    #[test]
    fn test_stroke_rect() {
        let mut fb = FrameBuffer::new(20, 20);
        fb.clear(0, 0, 0, 255);
        fb.stroke_rect(5, 5, 10, 10, 255, 255, 255);
        let top_idx = ((5 * 20 + 7) * 4) as usize;
        assert_eq!(fb.pixels[top_idx], 255);
        let bottom_idx = ((14 * 20 + 7) * 4) as usize;
        assert_eq!(fb.pixels[bottom_idx], 255);
        let inside_idx = ((8 * 20 + 8) * 4) as usize;
        assert_eq!(fb.pixels[inside_idx], 0);
    }

    #[test]
    fn test_encode_png() {
        let fb = FrameBuffer::new(4, 4);
        let result = encode_png(&fb);
        assert!(result.is_ok());
        let data = result.expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/png.rs (A5 governance)");
        assert!(!data.is_empty());
        assert_eq!(&data[0..4], b"\x89PNG");
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_png_renderer_new() {
        let _renderer = PngRenderer::new();
    }

    #[test]
    fn test_png_renderer_default() {
        let _renderer = PngRenderer;
    }

    #[test]
    fn test_render_primitive_line() {
        use crate::geometry::Point;
        let mut fb = FrameBuffer::new(100, 100);
        fb.clear(0, 0, 0, 255);
        let prim = Primitive::Line {
            p1: Point::new(10.0, 10.0),
            p2: Point::new(90.0, 90.0),
            style: Style::default().with_stroke(Color::WHITE),
        };
        PngRenderer::render_primitive(&prim, &mut fb);
        let idx = ((10 * 100 + 10) * 4) as usize;
        assert!(fb.pixels[idx] > 0 || fb.pixels[idx + 1] > 0 || fb.pixels[idx + 2] > 0);
    }

    #[test]
    fn test_render_primitive_filled_rect() {
        use crate::geometry::Rect;
        let mut fb = FrameBuffer::new(100, 100);
        fb.clear(0, 0, 0, 255);
        let prim = Primitive::FilledRect {
            rect: Rect::new(10.0, 10.0, 30.0, 30.0),
            fill: Color::RED,
            stroke: Some(Color::BLACK),
        };
        PngRenderer::render_primitive(&prim, &mut fb);
        let idx = ((20 * 100 + 20) * 4) as usize;
        assert_eq!(fb.pixels[idx], 255);
        assert_eq!(fb.pixels[idx + 1], 0);
        assert_eq!(fb.pixels[idx + 2], 0);
    }

    #[test]
    fn test_render_primitive_circle() {
        use crate::geometry::Point;
        let mut fb = FrameBuffer::new(100, 100);
        fb.clear(0, 0, 0, 255);
        let prim = Primitive::Circle {
            center: Point::new(50.0, 50.0),
            radius: 20.0,
            style: Style::default().with_stroke(Color::WHITE),
        };
        PngRenderer::render_primitive(&prim, &mut fb);
        let idx = ((50 * 100 + 70) * 4) as usize;
        assert!(fb.pixels[idx] > 0 || fb.pixels[idx + 1] > 0 || fb.pixels[idx + 2] > 0);
    }

    #[test]
    fn test_render_primitive_polygon() {
        use crate::geometry::Point;
        let mut fb = FrameBuffer::new(100, 100);
        fb.clear(0, 0, 0, 255);
        let prim = Primitive::Polygon {
            points: vec![
                Point::new(10.0, 10.0),
                Point::new(90.0, 10.0),
                Point::new(50.0, 90.0),
            ],
            style: Style::default().with_stroke(Color::WHITE),
        };
        PngRenderer::render_primitive(&prim, &mut fb);
        let idx = ((10 * 100 + 50) * 4) as usize;
        assert!(fb.pixels[idx] > 0 || fb.pixels[idx + 1] > 0 || fb.pixels[idx + 2] > 0);
    }

    #[test]
    fn test_render_primitive_path() {
        use crate::geometry::Point;
        let mut fb = FrameBuffer::new(100, 100);
        fb.clear(0, 0, 0, 255);
        let prim = Primitive::Path {
            points: vec![
                Point::new(10.0, 10.0),
                Point::new(50.0, 50.0),
                Point::new(90.0, 10.0),
            ],
            style: Style::default().with_stroke(Color::WHITE),
            close: false,
        };
        PngRenderer::render_primitive(&prim, &mut fb);
        let idx = ((30 * 100 + 30) * 4) as usize;
        assert!(fb.pixels[idx] > 0 || fb.pixels[idx + 1] > 0 || fb.pixels[idx + 2] > 0);
    }

    #[test]
    fn test_render_primitive_text() {
        use crate::geometry::Point;
        let mut fb = FrameBuffer::new(100, 100);
        fb.clear(0, 0, 0, 255);
        let prim = Primitive::Text {
            position: Point::new(10.0, 10.0),
            content: "Hello".to_string(),
            style: Style::default().with_fill(Color::WHITE),
        };
        PngRenderer::render_primitive(&prim, &mut fb);
        let idx = ((10 * 100 + 10) * 4) as usize;
        assert_eq!(fb.pixels[idx], 255);
    }

    #[test]
    fn test_render_primitive_group() {
        use crate::geometry::Point;
        let mut fb = FrameBuffer::new(100, 100);
        fb.clear(0, 0, 0, 255);
        let prim = Primitive::Group {
            primitives: vec![Primitive::Line {
                p1: Point::new(0.0, 0.0),
                p2: Point::new(99.0, 99.0),
                style: Style::default().with_stroke(Color::WHITE),
            }],
            transform: None,
        };
        PngRenderer::render_primitive(&prim, &mut fb);
        let idx = ((50 * 100 + 50) * 4) as usize;
        assert!(fb.pixels[idx] > 0 || fb.pixels[idx + 1] > 0 || fb.pixels[idx + 2] > 0);
    }
}
