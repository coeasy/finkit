pub mod crosshair;
pub mod pan;
pub mod zoom;

use crate::data::KlineData;
use crate::geometry::Point;
use crate::language::LanguageResource;
use crate::layout::ChartLayout;
use serde::{Deserialize, Serialize};

pub use crosshair::{CrosshairDataWindow, CrosshairInfo};

/// Pointer button used by the shared interaction state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Auxiliary,
}

/// Keyboard commands understood by chart frontends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionKey {
    Previous,
    Next,
    First,
    Last,
    Escape,
}

/// Normalized input event. Frontends translate browser/native events into
/// this small vocabulary before handing them to the shared controller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractionEvent {
    PointerMove { point: Point },
    PointerDown { point: Point, button: PointerButton },
    PointerUp { point: Point, button: PointerButton },
    Wheel { point: Point, delta_y: f64 },
    Key(InteractionKey),
    Leave,
}

/// Result of reducing one normalized input event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractionAction {
    None,
    Crosshair { point: Point },
    Pan { dx: f64, dy: f64 },
    Zoom { center: Point, factor: f64 },
    Select { index: usize },
    HideCrosshair,
}

/// Shared state machine for browser, WASM and native chart frontends.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionController {
    pub state: ViewState,
    pub selected_index: Option<usize>,
    dragging: bool,
    last_pointer: Option<Point>,
}

impl Default for InteractionController {
    fn default() -> Self {
        Self {
            state: ViewState::default(),
            selected_index: None,
            dragging: false,
            last_pointer: None,
        }
    }
}

impl InteractionController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reduce one input event and update the shared view/selection state.
    pub fn handle(&mut self, event: InteractionEvent, data_len: usize) -> InteractionAction {
        match event {
            InteractionEvent::PointerDown {
                point,
                button: PointerButton::Primary,
            } => {
                self.dragging = true;
                self.last_pointer = Some(point);
                InteractionAction::None
            }
            InteractionEvent::PointerDown { .. } => InteractionAction::None,
            InteractionEvent::PointerMove { point } => {
                if self.dragging {
                    if let Some(previous) = self.last_pointer.replace(point) {
                        let dx = point.x - previous.x;
                        let dy = point.y - previous.y;
                        self.state.pan(dx, dy);
                        return InteractionAction::Pan { dx, dy };
                    }
                }
                self.state.set_cursor(Some(point));
                InteractionAction::Crosshair { point }
            }
            InteractionEvent::PointerUp {
                point,
                button: PointerButton::Primary,
            } => {
                self.dragging = false;
                self.last_pointer = Some(point);
                InteractionAction::None
            }
            InteractionEvent::PointerUp { .. } => InteractionAction::None,
            InteractionEvent::Wheel { point, delta_y } => {
                let factor = if delta_y < 0.0 { 1.1 } else { 0.9 };
                self.state.zoom(factor, &point);
                InteractionAction::Zoom {
                    center: point,
                    factor,
                }
            }
            InteractionEvent::Key(key) => {
                let index = match key {
                    InteractionKey::Previous => self.selected_index.unwrap_or(0).saturating_sub(1),
                    InteractionKey::Next => self
                        .selected_index
                        .unwrap_or(0)
                        .saturating_add(1)
                        .min(data_len.saturating_sub(1)),
                    InteractionKey::First => 0,
                    InteractionKey::Last => data_len.saturating_sub(1),
                    InteractionKey::Escape => {
                        self.selected_index = None;
                        self.state.set_cursor(None);
                        return InteractionAction::HideCrosshair;
                    }
                };
                if data_len == 0 {
                    self.selected_index = None;
                    return InteractionAction::None;
                }
                self.selected_index = Some(index.min(data_len - 1));
                InteractionAction::Select {
                    index: self.selected_index.unwrap_or(0),
                }
            }
            InteractionEvent::Leave => {
                self.dragging = false;
                self.last_pointer = None;
                self.state.set_cursor(None);
                InteractionAction::HideCrosshair
            }
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }
}

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

/// Deterministic source-index replay state shared by chart frontends.
///
/// `cursor` is the last source bar available to the replay. The visible
/// window is always `[start, end)` and therefore can be fed directly into
/// `Viewport::new(start, end)` without renderer-specific off-by-one rules.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ReplayState {
    pub cursor: usize,
    pub window: usize,
    pub step: usize,
    pub speed: f32,
    pub playing: bool,
}

impl ReplayState {
    pub fn new(total: usize, window: usize) -> Self {
        let window = window.max(1);
        Self {
            cursor: total.min(window).saturating_sub(1),
            window,
            step: 1,
            speed: 1.0,
            playing: false,
        }
    }

    pub fn with_step(mut self, step: usize) -> Self {
        self.step = step.max(1);
        self
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.01);
        self
    }

    pub fn reset(&mut self, total: usize) {
        self.cursor = total.min(self.window).saturating_sub(1);
        self.playing = false;
    }

    pub fn seek(&mut self, index: usize, total: usize) -> usize {
        self.cursor = index.min(total.saturating_sub(1));
        self.cursor
    }

    /// Advance by `step`, returning the new cursor. Playback stops at the
    /// final source bar and remains deterministic for repeated calls.
    pub fn advance(&mut self, total: usize) -> Option<usize> {
        if total == 0 {
            self.playing = false;
            return None;
        }
        let next = self.cursor.saturating_add(self.step);
        self.cursor = next.min(total - 1);
        if self.cursor == total - 1 {
            self.playing = false;
        }
        Some(self.cursor)
    }

    pub fn visible_range(&self, total: usize) -> (usize, usize) {
        if total == 0 {
            return (0, 0);
        }
        let end = self.cursor.saturating_add(1).min(total);
        let start = end.saturating_sub(self.window.max(1));
        (start, end)
    }
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

    /// Find the nearest source bar in the current visible range.
    pub fn find_nearest_kline_in_range(
        &self,
        cursor_x: f64,
        layout: &ChartLayout,
        data_len: usize,
        visible_start: usize,
        visible_end: usize,
    ) -> usize {
        crosshair::find_nearest_kline_in_range(
            cursor_x,
            layout,
            data_len,
            visible_start,
            visible_end,
        )
    }

    /// Build the unified floating data window for one source bar.
    pub fn data_window(&self, index: usize, data: &KlineData) -> Option<CrosshairDataWindow> {
        crosshair::data_window(index, data)
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
        let (start, end) = if self.visible_end > self.visible_start {
            (self.visible_start, self.visible_end)
        } else {
            (0, data.len())
        };
        let index =
            crosshair::find_nearest_kline_in_range(cursor_x, layout, data.len(), start, end);
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

    #[test]
    fn test_interaction_controller_pan_and_crosshair() {
        let mut controller = InteractionController::new();
        assert_eq!(
            controller.handle(
                InteractionEvent::PointerDown {
                    point: Point::new(10.0, 20.0),
                    button: PointerButton::Primary,
                },
                100,
            ),
            InteractionAction::None
        );
        assert!(controller.is_dragging());
        assert_eq!(
            controller.handle(
                InteractionEvent::PointerMove {
                    point: Point::new(16.0, 24.0),
                },
                100,
            ),
            InteractionAction::Pan { dx: 6.0, dy: 4.0 }
        );
        assert_eq!(controller.state.offset_x, 6.0);
        assert_eq!(controller.state.offset_y, 4.0);
        controller.handle(
            InteractionEvent::PointerUp {
                point: Point::new(16.0, 24.0),
                button: PointerButton::Primary,
            },
            100,
        );
        assert!(!controller.is_dragging());
        assert!(matches!(
            controller.handle(
                InteractionEvent::PointerMove {
                    point: Point::new(30.0, 40.0),
                },
                100,
            ),
            InteractionAction::Crosshair { .. }
        ));
    }

    #[test]
    fn test_interaction_controller_keyboard_selection_and_escape() {
        let mut controller = InteractionController::new();
        assert_eq!(
            controller.handle(InteractionEvent::Key(InteractionKey::Last), 5),
            InteractionAction::Select { index: 4 }
        );
        assert_eq!(
            controller.handle(InteractionEvent::Key(InteractionKey::Previous), 5),
            InteractionAction::Select { index: 3 }
        );
        assert_eq!(
            controller.handle(InteractionEvent::Key(InteractionKey::Escape), 5),
            InteractionAction::HideCrosshair
        );
        assert_eq!(controller.selected_index, None);
    }

    #[test]
    fn test_interaction_controller_zoom_and_empty_selection() {
        let mut controller = InteractionController::new();
        let action = controller.handle(
            InteractionEvent::Wheel {
                point: Point::new(100.0, 100.0),
                delta_y: -1.0,
            },
            0,
        );
        assert!(matches!(
            action,
            InteractionAction::Zoom { factor: 1.1, .. }
        ));
        assert_eq!(
            controller.handle(InteractionEvent::Key(InteractionKey::Next), 0),
            InteractionAction::None
        );
    }

    #[test]
    fn test_replay_state_window_seek_and_end() {
        let mut replay = ReplayState::new(100, 20).with_step(5).with_speed(2.0);
        assert_eq!(replay.visible_range(100), (0, 20));
        assert_eq!(replay.advance(100), Some(24));
        assert_eq!(replay.visible_range(100), (5, 25));
        assert_eq!(replay.seek(97, 100), 97);
        assert_eq!(replay.visible_range(100), (78, 98));
        assert_eq!(replay.advance(100), Some(99));
        assert!(!replay.playing);
    }

    #[test]
    fn test_replay_state_empty_and_reset() {
        let mut replay = ReplayState::new(0, 20);
        assert_eq!(replay.visible_range(0), (0, 0));
        assert_eq!(replay.advance(0), None);
        replay.reset(8);
        assert_eq!(replay.visible_range(8), (0, 8));
    }
}
