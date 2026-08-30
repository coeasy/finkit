use crate::geometry::Point;
use crate::interaction::ViewState;

pub fn zoom_to_range(state: &mut ViewState, data_start: usize, data_end: usize, total: usize) {
    if total == 0 || data_start >= data_end {
        return;
    }
    let range_len = (data_end - data_start) as f64;
    let total_f = total as f64;
    state.scale_x = total_f / range_len;
    state.scale_x = state.scale_x.clamp(0.1, 100.0);
    state.offset_x = -(data_start as f64) * state.scale_x;
    state.visible_start = data_start;
    state.visible_end = data_end.min(total);
}

pub fn zoom_in(state: &mut ViewState, center: &Point) {
    let factor = 1.2;
    let new_scale_x = (state.scale_x * factor).clamp(0.1, 100.0);
    state.offset_x = center.x - (center.x - state.offset_x) * (new_scale_x / state.scale_x);
    state.scale_x = new_scale_x;
}

pub fn zoom_out(state: &mut ViewState, center: &Point) {
    let factor = 1.0 / 1.2;
    let new_scale_x = (state.scale_x * factor).clamp(0.1, 100.0);
    state.offset_x = center.x - (center.x - state.offset_x) * (new_scale_x / state.scale_x);
    state.scale_x = new_scale_x;
}

pub fn visible_range(state: &ViewState, viewport_width: f64, total: usize) -> (usize, usize) {
    if total == 0 || state.scale_x <= 0.0 {
        return (0, 0);
    }
    let start_f = (-state.offset_x / state.scale_x).max(0.0);
    let end_f = ((viewport_width - state.offset_x) / state.scale_x).min(total as f64);
    let start = start_f.floor() as usize;
    let end = end_f.ceil() as usize;
    let start = start.min(total);
    let end = end.max(start).min(total);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_to_range() {
        let mut state = ViewState::new();
        zoom_to_range(&mut state, 10, 50, 100);
        assert_eq!(state.visible_start, 10);
        assert_eq!(state.visible_end, 50);
        assert!(state.scale_x > 1.0);
    }

    #[test]
    fn test_zoom_to_range_empty() {
        let mut state = ViewState::new();
        zoom_to_range(&mut state, 0, 0, 100);
        assert_eq!(state.scale_x, 1.0);
    }

    #[test]
    fn test_zoom_to_range_inverted() {
        let mut state = ViewState::new();
        zoom_to_range(&mut state, 50, 10, 100);
        assert_eq!(state.scale_x, 1.0);
    }

    #[test]
    fn test_zoom_to_range_total_zero() {
        let mut state = ViewState::new();
        zoom_to_range(&mut state, 0, 10, 0);
        assert_eq!(state.scale_x, 1.0);
    }

    #[test]
    fn test_zoom_to_range_full() {
        let mut state = ViewState::new();
        zoom_to_range(&mut state, 0, 100, 100);
        assert!((state.scale_x - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_zoom_in() {
        let mut state = ViewState::new();
        let center = Point::new(100.0, 100.0);
        let old_scale = state.scale_x;
        zoom_in(&mut state, &center);
        assert!(state.scale_x > old_scale);
    }

    #[test]
    fn test_zoom_out() {
        let mut state = ViewState::new();
        let center = Point::new(100.0, 100.0);
        let old_scale = state.scale_x;
        zoom_out(&mut state, &center);
        assert!(state.scale_x < old_scale);
    }

    #[test]
    fn test_zoom_in_clamp_max() {
        let mut state = ViewState::new();
        state.scale_x = 99.0;
        let center = Point::new(100.0, 100.0);
        zoom_in(&mut state, &center);
        assert!((state.scale_x - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_zoom_out_clamp_min() {
        let mut state = ViewState::new();
        state.scale_x = 0.11;
        let center = Point::new(100.0, 100.0);
        zoom_out(&mut state, &center);
        assert!((state.scale_x - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_visible_range_default() {
        let state = ViewState::new();
        let (start, end) = visible_range(&state, 1200.0, 100);
        assert_eq!(start, 0);
        assert!(end > 0);
    }

    #[test]
    fn test_visible_range_zero_total() {
        let state = ViewState::new();
        let (start, end) = visible_range(&state, 1200.0, 0);
        assert_eq!(start, 0);
        assert_eq!(end, 0);
    }

    #[test]
    fn test_visible_range_zoomed() {
        let mut state = ViewState::new();
        zoom_to_range(&mut state, 20, 60, 100);
        let (start, end) = visible_range(&state, 1200.0, 100);
        assert!(start <= 20);
        assert!(end >= 60);
    }

    #[test]
    fn test_zoom_in_center_preserved() {
        let mut state = ViewState::new();
        state.offset_x = 50.0;
        let center = Point::new(200.0, 100.0);
        let data_index_at_center = (center.x - state.offset_x) / state.scale_x;
        zoom_in(&mut state, &center);
        let screen_pos_after = data_index_at_center * state.scale_x + state.offset_x;
        assert!((screen_pos_after - center.x).abs() < 1e-5);
    }
}
