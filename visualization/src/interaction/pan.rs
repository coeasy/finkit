use crate::interaction::ViewState;

pub fn pan_by(state: &mut ViewState, dx: f64, _dy: f64, total: usize) {
    if total == 0 {
        return;
    }
    let new_offset = state.offset_x + dx;
    let max_offset = 0.0;
    let min_offset = -((total as f64 - 1.0) * state.scale_x);
    state.offset_x = new_offset.clamp(min_offset, max_offset);
}

pub fn pan_to(state: &mut ViewState, index: usize, total: usize) {
    if total == 0 {
        return;
    }
    let index = index.min(total - 1);
    state.offset_x = -(index as f64 * state.scale_x);
    let max_offset = 0.0;
    let min_offset = -((total as f64 - 1.0) * state.scale_x);
    state.offset_x = state.offset_x.clamp(min_offset, max_offset);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pan_by_positive() {
        let mut state = ViewState::new();
        pan_by(&mut state, 50.0, 0.0, 100);
        assert!((state.offset_x - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_pan_by_negative() {
        let mut state = ViewState::new();
        pan_by(&mut state, -50.0, 0.0, 100);
        assert!((state.offset_x - (-50.0)).abs() < 1e-10);
    }

    #[test]
    fn test_pan_by_clamp_left() {
        let mut state = ViewState::new();
        state.offset_x = -10.0;
        pan_by(&mut state, 20.0, 0.0, 100);
        assert!((state.offset_x - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_pan_by_clamp_right() {
        let mut state = ViewState::new();
        pan_by(&mut state, -200.0, 0.0, 100);
        let min_offset = -(99.0);
        assert!((state.offset_x - min_offset).abs() < 1e-10);
    }

    #[test]
    fn test_pan_by_zero_total() {
        let mut state = ViewState::new();
        pan_by(&mut state, -50.0, 0.0, 0);
        assert!((state.offset_x - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_pan_to_start() {
        let mut state = ViewState::new();
        state.offset_x = -50.0;
        pan_to(&mut state, 0, 100);
        assert!((state.offset_x - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_pan_to_middle() {
        let mut state = ViewState::new();
        pan_to(&mut state, 50, 100);
        assert!((state.offset_x - (-50.0)).abs() < 1e-10);
    }

    #[test]
    fn test_pan_to_end() {
        let mut state = ViewState::new();
        pan_to(&mut state, 99, 100);
        assert!((state.offset_x - (-99.0)).abs() < 1e-10);
    }

    #[test]
    fn test_pan_to_beyond_end() {
        let mut state = ViewState::new();
        pan_to(&mut state, 200, 100);
        assert!((state.offset_x - (-99.0)).abs() < 1e-10);
    }

    #[test]
    fn test_pan_to_zero_total() {
        let mut state = ViewState::new();
        pan_to(&mut state, 0, 0);
        assert!((state.offset_x - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_pan_by_with_scaled_state() {
        let mut state = ViewState::new();
        state.scale_x = 2.0;
        pan_by(&mut state, -100.0, 0.0, 100);
        assert!((state.offset_x - (-100.0)).abs() < 1e-10);
    }

    #[test]
    fn test_pan_to_with_scaled_state() {
        let mut state = ViewState::new();
        state.scale_x = 2.0;
        pan_to(&mut state, 50, 100);
        assert!((state.offset_x - (-100.0)).abs() < 1e-10);
    }
}
