pub mod crosshair;
pub mod pan;
pub mod zoom;

use crate::data::KlineData;
use crate::geometry::Point;
use crate::language::LanguageResource;
use crate::layout::ChartLayout;

pub use crosshair::CrosshairInfo;

#[derive(Debug, Clone, PartialEq)]
pub struct ViewState {
    pub offset_x: f64,
    pub offset_y: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub visible_start: usize,
    pub visible_end: usize,
    pub cursor: Option<Point>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            visible_start: 0,
            visible_end: 0,
            cursor: None,
        }
    }
}

impl ViewState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_visible_range(mut self, start: usize, end: usize) -> Self {
        self.visible_start = start;
        self.visible_end = end;
        self
    }

    pub fn zoom(&mut self, factor: f64, center: &Point) {
        let new_scale_x = (self.scale_x * factor).clamp(0.1, 100.0);
        let new_scale_y = (self.scale_y * factor).clamp(0.1, 100.0);

        self.offset_x = center.x - (center.x - self.offset_x) * (new_scale_x / self.scale_x);
        self.offset_y = center.y - (center.y - self.offset_y) * (new_scale_y / self.scale_y);

        self.scale_x = new_scale_x;
        self.scale_y = new_scale_y;
    }

    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.offset_x += dx;
        self.offset_y += dy;
    }

    pub fn set_cursor(&mut self, point: Option<Point>) {
        self.cursor = point;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn zoom_to_range(&mut self, data_start: usize, data_end: usize, total: usize) {
        zoom::zoom_to_range(self, data_start, data_end, total);
    }

    pub fn zoom_in(&mut self, center: &Point) {
        zoom::zoom_in(self, center);
    }

    pub fn zoom_out(&mut self, center: &Point) {
        zoom::zoom_out(self, center);
    }

    pub fn visible_data_range(&self, viewport_width: f64, total: usize) -> (usize, usize) {
        zoom::visible_range(self, viewport_width, total)
    }

    pub fn pan_by(&mut self, dx: f64, dy: f64, total: usize) {
        pan::pan_by(self, dx, dy, total);
    }

    pub fn pan_to(&mut self, index: usize, total: usize) {
        pan::pan_to(self, index, total);
    }

    pub fn find_nearest_kline(
        &self,
        cursor_x: f64,
        layout: &ChartLayout,
        data_len: usize,
    ) -> usize {
        crosshair::find_nearest_kline(cursor_x, layout, data_len)
    }

    pub fn format_tooltip(
        &self,
        index: usize,
        data: &KlineData,
        resource: &LanguageResource,
    ) -> String {
        crosshair::format_tooltip(index, data, resource)
    }

    pub fn crosshair_info(
        &self,
        cursor_x: f64,
        cursor_y: f64,
        data: &KlineData,
        layout: &ChartLayout,
    ) -> Option<CrosshairInfo> {
        let index = crosshair::find_nearest_kline(cursor_x, layout, data.len());
        crosshair::create_crosshair_info(index, cursor_x, cursor_y, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_state_default() {
        let state = ViewState::default();
        assert_eq!(state.scale_x, 1.0);
        assert_eq!(state.scale_y, 1.0);
        assert_eq!(state.offset_x, 0.0);
        assert!(state.cursor.is_none());
    }

    #[test]
    fn test_view_state_zoom() {
        let mut state = ViewState::new();
        let center = Point::new(100.0, 100.0);
        state.zoom(2.0, &center);
        assert_eq!(state.scale_x, 2.0);
        assert_eq!(state.scale_y, 2.0);
    }

    #[test]
    fn test_view_state_pan() {
        let mut state = ViewState::new();
        state.pan(10.0, -5.0);
        assert_eq!(state.offset_x, 10.0);
        assert_eq!(state.offset_y, -5.0);
    }

    #[test]
    fn test_view_state_reset() {
        let mut state = ViewState::new();
        state.pan(100.0, 100.0);
        state.zoom(5.0, &Point::new(0.0, 0.0));
        state.reset();
        assert_eq!(state.offset_x, 0.0);
        assert_eq!(state.scale_x, 1.0);
    }

    #[test]
    fn test_view_state_cursor() {
        let mut state = ViewState::new();
        state.set_cursor(Some(Point::new(50.0, 50.0)));
        assert!(state.cursor.is_some());
        assert_eq!(state.cursor.expect("finkit-visualization: unexpected None/Err in visualization/src/interaction/mod.rs (A5 governance)").x, 50.0);
    }

    #[test]
    fn test_convenience_zoom_to_range() {
        let mut state = ViewState::new();
        state.zoom_to_range(10, 50, 100);
        assert_eq!(state.visible_start, 10);
        assert_eq!(state.visible_end, 50);
    }

    #[test]
    fn test_convenience_zoom_in() {
        let mut state = ViewState::new();
        let center = Point::new(100.0, 100.0);
        let old_scale = state.scale_x;
        state.zoom_in(&center);
        assert!(state.scale_x > old_scale);
    }

    #[test]
    fn test_convenience_zoom_out() {
        let mut state = ViewState::new();
        let center = Point::new(100.0, 100.0);
        let old_scale = state.scale_x;
        state.zoom_out(&center);
        assert!(state.scale_x < old_scale);
    }

    #[test]
    fn test_convenience_pan_by() {
        let mut state = ViewState::new();
        state.pan_by(-50.0, 0.0, 100);
        assert!((state.offset_x - (-50.0)).abs() < 1e-10);
    }

    #[test]
    fn test_convenience_pan_to() {
        let mut state = ViewState::new();
        state.pan_to(50, 100);
        assert!((state.offset_x - (-50.0)).abs() < 1e-10);
    }

    #[test]
    fn test_convenience_visible_data_range() {
        let state = ViewState::new();
        let (start, end) = state.visible_data_range(1200.0, 100);
        assert_eq!(start, 0);
        assert!(end > 0);
    }
}
