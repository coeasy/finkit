#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn distance_to(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn lerp(&self, other: &Point, t: f64) -> Point {
        Point {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn left(&self) -> f64 {
        self.x
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn top(&self) -> f64 {
        self.y
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.x
            && point.x <= self.right()
            && point.y >= self.y
            && point.y <= self.bottom()
    }

    pub fn contains_point(&self, p: &Point) -> bool {
        self.contains(p)
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn inflate(&self, dx: f64, dy: f64) -> Rect {
        Rect {
            x: self.x - dx,
            y: self.y - dy,
            width: self.width + 2.0 * dx,
            height: self.height + 2.0 * dy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale {
    pub data_min: f64,
    pub data_max: f64,
    pub pixel_min: f64,
    pub pixel_max: f64,
    pub linear: bool,
}

impl Scale {
    pub fn new(data_min: f64, data_max: f64, pixel_min: f64, pixel_max: f64, linear: bool) -> Self {
        Self {
            data_min,
            data_max,
            pixel_min,
            pixel_max,
            linear,
        }
    }

    pub fn linear_scale(data_min: f64, data_max: f64, pixel_min: f64, pixel_max: f64) -> Self {
        Self {
            data_min,
            data_max,
            pixel_min,
            pixel_max,
            linear: true,
        }
    }

    pub fn log_scale(data_min: f64, data_max: f64, pixel_min: f64, pixel_max: f64) -> Self {
        Self {
            data_min,
            data_max,
            pixel_min,
            pixel_max,
            linear: false,
        }
    }

    pub fn data_to_pixel(&self, value: f64) -> f64 {
        if self.linear {
            let data_range = self.data_max - self.data_min;
            if data_range == 0.0 {
                return (self.pixel_min + self.pixel_max) / 2.0;
            }
            let t = (value - self.data_min) / data_range;
            self.pixel_min + t * (self.pixel_max - self.pixel_min)
        } else {
            let log_min = self.data_min.ln();
            let log_max = self.data_max.ln();
            let log_range = log_max - log_min;
            if log_range == 0.0 {
                return (self.pixel_min + self.pixel_max) / 2.0;
            }
            let log_val = value.ln();
            let t = (log_val - log_min) / log_range;
            self.pixel_min + t * (self.pixel_max - self.pixel_min)
        }
    }

    pub fn pixel_to_data(&self, pixel: f64) -> f64 {
        if self.linear {
            let pixel_range = self.pixel_max - self.pixel_min;
            if pixel_range == 0.0 {
                return (self.data_min + self.data_max) / 2.0;
            }
            let t = (pixel - self.pixel_min) / pixel_range;
            self.data_min + t * (self.data_max - self.data_min)
        } else {
            let pixel_range = self.pixel_max - self.pixel_min;
            if pixel_range == 0.0 {
                return (self.data_min + self.data_max) / 2.0;
            }
            let t = (pixel - self.pixel_min) / pixel_range;
            let log_min = self.data_min.ln();
            let log_max = self.data_max.ln();
            (log_min + t * (log_max - log_min)).exp()
        }
    }

    pub fn nice_ticks(&self, count: usize) -> Vec<f64> {
        if count == 0 {
            return Vec::new();
        }
        nice_ticks_impl(self.data_min, self.data_max, count, self.linear)
    }
}

fn nice_ticks_impl(data_min: f64, data_max: f64, count: usize, linear: bool) -> Vec<f64> {
    if linear {
        nice_ticks_linear(data_min, data_max, count)
    } else {
        nice_ticks_log(data_min, data_max, count)
    }
}

fn nice_ticks_linear(data_min: f64, data_max: f64, count: usize) -> Vec<f64> {
    if data_min == data_max {
        return vec![data_min];
    }

    let range = data_max - data_min;
    let rough_step = range / count as f64;
    let step = nice_step(rough_step);

    let nice_min = (data_min / step).floor() * step;
    let nice_max = (data_max / step).ceil() * step;

    let mut ticks = Vec::new();
    let mut val = nice_min;
    while val <= nice_max + step * 1e-10 {
        ticks.push(round_tick(val, step));
        val += step;
    }
    ticks
}

fn nice_step(rough_step: f64) -> f64 {
    let exponent = rough_step.log10().floor();
    let fraction = rough_step / 10_f64.powf(exponent);

    let nice_fraction = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };

    nice_fraction * 10_f64.powf(exponent)
}

fn round_tick(value: f64, step: f64) -> f64 {
    let precision = if step >= 1.0 {
        0
    } else {
        (-step.log10()).ceil() as i32
    };
    if precision <= 0 {
        (value / step).round() * step
    } else {
        let factor = 10_f64.powi(precision);
        (value * factor).round() / factor
    }
}

fn nice_ticks_log(data_min: f64, data_max: f64, count: usize) -> Vec<f64> {
    if data_min <= 0.0 || data_max <= 0.0 {
        return nice_ticks_linear(data_min, data_max, count);
    }

    let log_min = data_min.ln();
    let log_max = data_max.ln();
    let log_ticks = nice_ticks_linear(log_min, log_max, count);

    log_ticks.iter().map(|&v| v.exp()).collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub m: [f64; 6],
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        }
    }

    pub fn translate(dx: f64, dy: f64) -> Self {
        Self {
            m: [1.0, 0.0, dx, 0.0, 1.0, dy],
        }
    }

    pub fn scale(sx: f64, sy: f64) -> Self {
        Self {
            m: [sx, 0.0, 0.0, 0.0, sy, 0.0],
        }
    }

    pub fn then(&self, other: &Transform) -> Transform {
        Transform {
            m: [
                other.m[0] * self.m[0] + other.m[1] * self.m[3],
                other.m[0] * self.m[1] + other.m[1] * self.m[4],
                other.m[0] * self.m[2] + other.m[1] * self.m[5] + other.m[2],
                other.m[3] * self.m[0] + other.m[4] * self.m[3],
                other.m[3] * self.m[1] + other.m[4] * self.m[4],
                other.m[3] * self.m[2] + other.m[4] * self.m[5] + other.m[5],
            ],
        }
    }

    pub fn apply(&self, p: &Point) -> Point {
        Point {
            x: self.m[0] * p.x + self.m[1] * p.y + self.m[2],
            y: self.m[3] * p.x + self.m[4] * p.y + self.m[5],
        }
    }

    pub fn apply_rect(&self, r: &Rect) -> Rect {
        let p0 = self.apply(&Point::new(r.x, r.y));
        let p1 = self.apply(&Point::new(r.right(), r.y));
        let p2 = self.apply(&Point::new(r.x, r.bottom()));
        let p3 = self.apply(&Point::new(r.right(), r.bottom()));

        let min_x = p0.x.min(p1.x).min(p2.x).min(p3.x);
        let min_y = p0.y.min(p1.y).min(p2.y).min(p3.y);
        let max_x = p0.x.max(p1.x).max(p2.x).max(p3.x);
        let max_y = p0.y.max(p1.y).max(p2.y).max(p3.y);

        Rect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }
}

pub struct ClipRect;

impl ClipRect {
    const INSIDE: u8 = 0;
    const LEFT: u8 = 1;
    const RIGHT: u8 = 2;
    const BOTTOM: u8 = 4;
    const TOP: u8 = 8;

    fn outcode(p: &Point, rect: &Rect) -> u8 {
        let mut code = Self::INSIDE;
        if p.x < rect.x {
            code |= Self::LEFT;
        } else if p.x > rect.right() {
            code |= Self::RIGHT;
        }
        if p.y < rect.y {
            code |= Self::TOP;
        } else if p.y > rect.bottom() {
            code |= Self::BOTTOM;
        }
        code
    }

    pub fn clip_line(p1: &Point, p2: &Point, rect: &Rect) -> Option<(Point, Point)> {
        let mut x0 = p1.x;
        let mut y0 = p1.y;
        let mut x1 = p2.x;
        let mut y1 = p2.y;

        let mut outcode0 = Self::outcode(&Point::new(x0, y0), rect);
        let mut outcode1 = Self::outcode(&Point::new(x1, y1), rect);

        loop {
            if outcode0 | outcode1 == 0 {
                return Some((Point::new(x0, y0), Point::new(x1, y1)));
            }
            if outcode0 & outcode1 != 0 {
                return None;
            }

            let outcode_out = if outcode0 != 0 { outcode0 } else { outcode1 };

            let (x, y) = if outcode_out & Self::TOP != 0 {
                let x = x0 + (x1 - x0) * (rect.y - y0) / (y1 - y0);
                (x, rect.y)
            } else if outcode_out & Self::BOTTOM != 0 {
                let x = x0 + (x1 - x0) * (rect.bottom() - y0) / (y1 - y0);
                (x, rect.bottom())
            } else if outcode_out & Self::RIGHT != 0 {
                let y = y0 + (y1 - y0) * (rect.right() - x0) / (x1 - x0);
                (rect.right(), y)
            } else if outcode_out & Self::LEFT != 0 {
                let y = y0 + (y1 - y0) * (rect.x - x0) / (x1 - x0);
                (rect.x, y)
            } else {
                unreachable!()
            };

            if outcode_out == outcode0 {
                x0 = x;
                y0 = y;
                outcode0 = Self::outcode(&Point::new(x0, y0), rect);
            } else {
                x1 = x;
                y1 = y;
                outcode1 = Self::outcode(&Point::new(x1, y1), rect);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_new() {
        let p = Point::new(3.0, 4.0);
        assert_eq!(p.x, 3.0);
        assert_eq!(p.y, 4.0);
    }

    #[test]
    fn test_point_zero() {
        let p = Point::zero();
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, 0.0);
    }

    #[test]
    fn test_point_distance_to() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert!((p1.distance_to(&p2) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_point_distance_to_same() {
        let p = Point::new(5.0, 5.0);
        assert!((p.distance_to(&p) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_point_lerp() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(10.0, 20.0);
        let mid = p1.lerp(&p2, 0.5);
        assert!((mid.x - 5.0).abs() < 1e-10);
        assert!((mid.y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_point_lerp_start() {
        let p1 = Point::new(1.0, 2.0);
        let p2 = Point::new(10.0, 20.0);
        let result = p1.lerp(&p2, 0.0);
        assert!((result.x - 1.0).abs() < 1e-10);
        assert!((result.y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_point_lerp_end() {
        let p1 = Point::new(1.0, 2.0);
        let p2 = Point::new(10.0, 20.0);
        let result = p1.lerp(&p2, 1.0);
        assert!((result.x - 10.0).abs() < 1e-10);
        assert!((result.y - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_rect_new() {
        let r = Rect::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(r.x, 1.0);
        assert_eq!(r.y, 2.0);
        assert_eq!(r.width, 3.0);
        assert_eq!(r.height, 4.0);
    }

    #[test]
    fn test_rect_sides() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(r.left(), 10.0);
        assert_eq!(r.right(), 40.0);
        assert_eq!(r.top(), 20.0);
        assert_eq!(r.bottom(), 60.0);
    }

    #[test]
    fn test_rect_contains_inside() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(r.contains(&Point::new(5.0, 5.0)));
    }

    #[test]
    fn test_rect_contains_boundary() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(r.contains(&Point::new(0.0, 0.0)));
        assert!(r.contains(&Point::new(10.0, 10.0)));
    }

    #[test]
    fn test_rect_contains_outside() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(!r.contains(&Point::new(15.0, 5.0)));
    }

    #[test]
    fn test_rect_contains_point() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(r.contains_point(&Point::new(5.0, 5.0)));
        assert!(!r.contains_point(&Point::new(15.0, 5.0)));
    }

    #[test]
    fn test_rect_intersects_overlapping() {
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::new(5.0, 5.0, 10.0, 10.0);
        assert!(r1.intersects(&r2));
        assert!(r2.intersects(&r1));
    }

    #[test]
    fn test_rect_intersects_non_overlapping() {
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::new(20.0, 20.0, 10.0, 10.0);
        assert!(!r1.intersects(&r2));
    }

    #[test]
    fn test_rect_intersects_touching() {
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::new(10.0, 0.0, 10.0, 10.0);
        assert!(!r1.intersects(&r2));
    }

    #[test]
    fn test_rect_center() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0);
        let c = r.center();
        assert!((c.x - 25.0).abs() < 1e-10);
        assert!((c.y - 40.0).abs() < 1e-10);
    }

    #[test]
    fn test_rect_inflate() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0);
        let inflated = r.inflate(5.0, 5.0);
        assert!((inflated.x - 5.0).abs() < 1e-10);
        assert!((inflated.y - 15.0).abs() < 1e-10);
        assert!((inflated.width - 40.0).abs() < 1e-10);
        assert!((inflated.height - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_rect_inflate_negative() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0);
        let deflated = r.inflate(-5.0, -5.0);
        assert!((deflated.x - 15.0).abs() < 1e-10);
        assert!((deflated.y - 25.0).abs() < 1e-10);
        assert!((deflated.width - 20.0).abs() < 1e-10);
        assert!((deflated.height - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_scale_linear_data_to_pixel() {
        let s = Scale::linear_scale(0.0, 100.0, 0.0, 800.0);
        assert!((s.data_to_pixel(0.0) - 0.0).abs() < 1e-10);
        assert!((s.data_to_pixel(50.0) - 400.0).abs() < 1e-10);
        assert!((s.data_to_pixel(100.0) - 800.0).abs() < 1e-10);
    }

    #[test]
    fn test_scale_linear_pixel_to_data() {
        let s = Scale::linear_scale(0.0, 100.0, 0.0, 800.0);
        assert!((s.pixel_to_data(0.0) - 0.0).abs() < 1e-10);
        assert!((s.pixel_to_data(400.0) - 50.0).abs() < 1e-10);
        assert!((s.pixel_to_data(800.0) - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_scale_linear_roundtrip() {
        let s = Scale::linear_scale(0.0, 100.0, 0.0, 800.0);
        let value = 42.0;
        let pixel = s.data_to_pixel(value);
        let recovered = s.pixel_to_data(pixel);
        assert!((recovered - value).abs() < 1e-10);
    }

    #[test]
    fn test_scale_linear_zero_range() {
        let s = Scale::linear_scale(50.0, 50.0, 0.0, 800.0);
        assert!((s.data_to_pixel(50.0) - 400.0).abs() < 1e-10);
    }

    #[test]
    fn test_scale_linear_inverted_pixel() {
        let s = Scale::linear_scale(0.0, 100.0, 800.0, 0.0);
        assert!((s.data_to_pixel(0.0) - 800.0).abs() < 1e-10);
        assert!((s.data_to_pixel(100.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_scale_log_data_to_pixel() {
        let s = Scale::log_scale(1.0, 1000.0, 0.0, 800.0);
        assert!((s.data_to_pixel(1.0) - 0.0).abs() < 1e-10);
        assert!((s.data_to_pixel(1000.0) - 800.0).abs() < 1e-10);
    }

    #[test]
    fn test_scale_log_roundtrip() {
        let s = Scale::log_scale(1.0, 1000.0, 0.0, 800.0);
        let value = 100.0;
        let pixel = s.data_to_pixel(value);
        let recovered = s.pixel_to_data(pixel);
        assert!((recovered - value).abs() < 1e-8);
    }

    #[test]
    fn test_scale_log_midpoint() {
        let s = Scale::log_scale(1.0, 100.0, 0.0, 100.0);
        let mid_pixel = 50.0;
        let mid_val = s.pixel_to_data(mid_pixel);
        let expected = (1.0_f64.ln() + 100.0_f64.ln()) / 2.0;
        let expected_val = expected.exp();
        assert!((mid_val - expected_val).abs() < 1e-8);
    }

    #[test]
    fn test_nice_ticks_basic() {
        let s = Scale::linear_scale(0.0, 100.0, 0.0, 800.0);
        let ticks = s.nice_ticks(5);
        assert!(ticks.len() >= 5);
        assert!(*ticks.first().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)") <= 0.0);
        assert!(*ticks.last().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)") >= 100.0);
    }

    #[test]
    fn test_nice_ticks_small_range() {
        let s = Scale::linear_scale(0.0, 1.0, 0.0, 800.0);
        let ticks = s.nice_ticks(5);
        assert!(ticks.len() >= 5);
        assert!(*ticks.first().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)") <= 0.0);
        assert!(*ticks.last().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)") >= 1.0);
    }

    #[test]
    fn test_nice_ticks_zero_count() {
        let s = Scale::linear_scale(0.0, 100.0, 0.0, 800.0);
        let ticks = s.nice_ticks(0);
        assert!(ticks.is_empty());
    }

    #[test]
    fn test_nice_ticks_equal_range() {
        let s = Scale::linear_scale(50.0, 50.0, 0.0, 800.0);
        let ticks = s.nice_ticks(5);
        assert_eq!(ticks.len(), 1);
        assert!((ticks[0] - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_nice_ticks_step_values() {
        let s = Scale::linear_scale(0.0, 100.0, 0.0, 800.0);
        let ticks = s.nice_ticks(5);
        if ticks.len() >= 2 {
            let step = ticks[1] - ticks[0];
            for i in 1..ticks.len() {
                assert!((ticks[i] - ticks[i - 1] - step).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_nice_ticks_log() {
        let s = Scale::log_scale(1.0, 1000.0, 0.0, 800.0);
        let ticks = s.nice_ticks(5);
        assert!(ticks.len() >= 2);
        for &t in &ticks {
            assert!(t > 0.0);
        }
    }

    #[test]
    fn test_transform_identity() {
        let t = Transform::identity();
        let p = Point::new(5.0, 10.0);
        let result = t.apply(&p);
        assert!((result.x - 5.0).abs() < 1e-10);
        assert!((result.y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_transform_translate() {
        let t = Transform::translate(10.0, 20.0);
        let p = Point::new(5.0, 5.0);
        let result = t.apply(&p);
        assert!((result.x - 15.0).abs() < 1e-10);
        assert!((result.y - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_transform_scale() {
        let t = Transform::scale(2.0, 3.0);
        let p = Point::new(5.0, 5.0);
        let result = t.apply(&p);
        assert!((result.x - 10.0).abs() < 1e-10);
        assert!((result.y - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_transform_then() {
        let t1 = Transform::scale(2.0, 2.0);
        let t2 = Transform::translate(10.0, 20.0);
        let combined = t1.then(&t2);
        let p = Point::new(5.0, 5.0);
        let result = combined.apply(&p);
        assert!((result.x - 20.0).abs() < 1e-10);
        assert!((result.y - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_transform_apply_rect() {
        let t = Transform::translate(10.0, 20.0);
        let r = Rect::new(0.0, 0.0, 30.0, 40.0);
        let result = t.apply_rect(&r);
        assert!((result.x - 10.0).abs() < 1e-10);
        assert!((result.y - 20.0).abs() < 1e-10);
        assert!((result.width - 30.0).abs() < 1e-10);
        assert!((result.height - 40.0).abs() < 1e-10);
    }

    #[test]
    fn test_transform_apply_rect_scale() {
        let t = Transform::scale(2.0, 3.0);
        let r = Rect::new(10.0, 20.0, 30.0, 40.0);
        let result = t.apply_rect(&r);
        assert!((result.x - 20.0).abs() < 1e-10);
        assert!((result.y - 60.0).abs() < 1e-10);
        assert!((result.width - 60.0).abs() < 1e-10);
        assert!((result.height - 120.0).abs() < 1e-10);
    }

    #[test]
    fn test_clip_line_fully_inside() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let p1 = Point::new(2.0, 2.0);
        let p2 = Point::new(8.0, 8.0);
        let result = ClipRect::clip_line(&p1, &p2, &rect);
        assert!(result.is_some());
        let (r1, r2) = result.expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)");
        assert!((r1.x - 2.0).abs() < 1e-10);
        assert!((r1.y - 2.0).abs() < 1e-10);
        assert!((r2.x - 8.0).abs() < 1e-10);
        assert!((r2.y - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_clip_line_fully_outside() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let p1 = Point::new(20.0, 20.0);
        let p2 = Point::new(30.0, 30.0);
        let result = ClipRect::clip_line(&p1, &p2, &rect);
        assert!(result.is_none());
    }

    #[test]
    fn test_clip_line_partial_clip() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let p1 = Point::new(-5.0, 5.0);
        let p2 = Point::new(15.0, 5.0);
        let result = ClipRect::clip_line(&p1, &p2, &rect);
        assert!(result.is_some());
        let (r1, r2) = result.expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)");
        assert!((r1.x - 0.0).abs() < 1e-10);
        assert!((r1.y - 5.0).abs() < 1e-10);
        assert!((r2.x - 10.0).abs() < 1e-10);
        assert!((r2.y - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_clip_line_diagonal_clip() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let p1 = Point::new(-5.0, -5.0);
        let p2 = Point::new(15.0, 15.0);
        let result = ClipRect::clip_line(&p1, &p2, &rect);
        assert!(result.is_some());
        let (r1, r2) = result.expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)");
        assert!((r1.x - 0.0).abs() < 1e-10);
        assert!((r1.y - 0.0).abs() < 1e-10);
        assert!((r2.x - 10.0).abs() < 1e-10);
        assert!((r2.y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_clip_line_same_side_outside() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let p1 = Point::new(-5.0, 2.0);
        let p2 = Point::new(-2.0, 8.0);
        let result = ClipRect::clip_line(&p1, &p2, &rect);
        assert!(result.is_none());
    }

    #[test]
    fn test_clip_line_cross_two_edges() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let p1 = Point::new(-5.0, 2.0);
        let p2 = Point::new(15.0, 8.0);
        let result = ClipRect::clip_line(&p1, &p2, &rect);
        assert!(result.is_some());
        let (r1, r2) = result.expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)");
        assert!((r1.x - 0.0).abs() < 1e-10);
        assert!((r2.x - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_nice_step_values() {
        assert!((nice_step(7.0) - 10.0).abs() < 1e-10);
        assert!((nice_step(3.0) - 5.0).abs() < 1e-10);
        assert!((nice_step(1.5) - 2.0).abs() < 1e-10);
        assert!((nice_step(0.7) - 1.0).abs() < 1e-10);
        assert!((nice_step(0.3) - 0.5).abs() < 1e-10);
        assert!((nice_step(0.15) - 0.2).abs() < 1e-10);
        assert!((nice_step(0.07) - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_nice_ticks_negative_range() {
        let s = Scale::linear_scale(-50.0, 50.0, 0.0, 800.0);
        let ticks = s.nice_ticks(5);
        assert!(ticks.len() >= 5);
        assert!(*ticks.first().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)") <= -50.0);
        assert!(*ticks.last().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)") >= 50.0);
        assert!(ticks.contains(&0.0));
    }

    #[test]
    fn test_nice_ticks_decimal_range() {
        let s = Scale::linear_scale(0.0, 0.01, 0.0, 800.0);
        let ticks = s.nice_ticks(5);
        assert!(ticks.len() >= 5);
        assert!(*ticks.first().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)") <= 0.0);
        assert!(*ticks.last().expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)") >= 0.01);
    }

    #[test]
    fn test_scale_new() {
        let s = Scale::new(0.0, 100.0, 0.0, 800.0, true);
        assert_eq!(s.data_min, 0.0);
        assert_eq!(s.data_max, 100.0);
        assert_eq!(s.pixel_min, 0.0);
        assert_eq!(s.pixel_max, 800.0);
        assert!(s.linear);
    }

    #[test]
    fn test_transform_matrix_values() {
        let t = Transform::identity();
        assert_eq!(t.m, [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);

        let t = Transform::translate(5.0, 10.0);
        assert_eq!(t.m, [1.0, 0.0, 5.0, 0.0, 1.0, 10.0]);

        let t = Transform::scale(2.0, 3.0);
        assert_eq!(t.m, [2.0, 0.0, 0.0, 0.0, 3.0, 0.0]);
    }

    #[test]
    fn test_size_new() {
        let s = Size::new(100.0, 200.0);
        assert_eq!(s.width, 100.0);
        assert_eq!(s.height, 200.0);
    }

    #[test]
    fn test_size_zero() {
        let s = Size::zero();
        assert_eq!(s.width, 0.0);
        assert_eq!(s.height, 0.0);
    }

    #[test]
    fn test_clip_line_vertical() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let p1 = Point::new(5.0, -5.0);
        let p2 = Point::new(5.0, 15.0);
        let result = ClipRect::clip_line(&p1, &p2, &rect);
        assert!(result.is_some());
        let (r1, r2) = result.expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)");
        assert!((r1.x - 5.0).abs() < 1e-10);
        assert!((r1.y - 0.0).abs() < 1e-10);
        assert!((r2.x - 5.0).abs() < 1e-10);
        assert!((r2.y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_clip_line_horizontal() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let p1 = Point::new(-5.0, 5.0);
        let p2 = Point::new(15.0, 5.0);
        let result = ClipRect::clip_line(&p1, &p2, &rect);
        assert!(result.is_some());
        let (r1, r2) = result.expect("alpha-ta-visualization: unexpected None/Err in visualization/src/geometry.rs (A5 governance)");
        assert!((r1.x - 0.0).abs() < 1e-10);
        assert!((r1.y - 5.0).abs() < 1e-10);
        assert!((r2.x - 10.0).abs() < 1e-10);
        assert!((r2.y - 5.0).abs() < 1e-10);
    }
}
