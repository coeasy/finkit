//! Canonical volatility state primitives.

#[derive(Debug, Clone, Copy, Default)]
pub struct TrueRangeState {
    previous_close: Option<f64>,
}

impl TrueRangeState {
    #[inline]
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> f64 {
        let value = match self.previous_close {
            Some(previous) => {
                let a = high - low;
                let b = (high - previous).abs();
                let c = (low - previous).abs();
                a.max(b).max(c)
            }
            None => high - low,
        };
        self.previous_close = Some(close);
        value
    }

    pub fn reset(&mut self) {
        self.previous_close = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn true_range_uses_previous_close() {
        let mut state = TrueRangeState::default();
        assert_eq!(state.update(10.0, 8.0, 9.0), 2.0);
        assert_eq!(state.update(12.0, 11.0, 11.5), 3.0);
    }
}
