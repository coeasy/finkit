//! Canonical moving-average kernel shared by batch and streaming execution.
//!
//! Higher-level indicators should depend on this state machine instead of
//! owning duplicate rolling state. SMA and WMA are O(1) per update; EMA keeps
//! the recursive state in the same reusable object.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovingAverageKind {
    Sma,
    /// First-value seeded EMA recursion. Batch APIs with SMA seeding should
    /// warm the state explicitly before switching to the recursive phase.
    Ema { alpha: f64 },
    Wma,
}

#[derive(Debug, Clone)]
pub struct MovingAverageState {
    kind: MovingAverageKind,
    window: usize,
    buffer: Vec<f64>,
    cursor: usize,
    count: usize,
    sum: f64,
    weighted_sum: f64,
    ema: f64,
}

impl MovingAverageState {
    pub fn new(kind: MovingAverageKind, window: usize) -> Self {
        assert!(window > 0);
        if let MovingAverageKind::Ema { alpha } = kind {
            assert!(alpha.is_finite() && alpha > 0.0 && alpha <= 1.0);
        }
        Self {
            kind,
            window,
            buffer: vec![0.0; window],
            cursor: 0,
            count: 0,
            sum: 0.0,
            weighted_sum: 0.0,
            ema: 0.0,
        }
    }

    #[inline(always)]
    pub fn update(&mut self, value: f64) -> f64 {
        let full = self.count == self.window;
        let old = if full { self.buffer[self.cursor] } else { 0.0 };

        let output = match self.kind {
            MovingAverageKind::Sma => {
                if !full {
                    self.count += 1;
                }
                self.sum += value - old;
                self.sum / self.count as f64
            }
            MovingAverageKind::Ema { alpha } => {
                if self.count == 0 {
                    self.ema = value;
                } else {
                    self.ema = alpha.mul_add(value - self.ema, self.ema);
                }
                if !full {
                    self.count += 1;
                }
                self.sum += value - old;
                self.ema
            }
            MovingAverageKind::Wma => {
                if !full {
                    self.count += 1;
                    self.sum += value;
                    self.weighted_sum += self.count as f64 * value;
                } else {
                    // For weights 1..N from oldest to newest:
                    // W' = W - S + N*x, S' = S - oldest + x.
                    let previous_sum = self.sum;
                    self.weighted_sum =
                        self.weighted_sum - previous_sum + self.window as f64 * value;
                    self.sum = previous_sum - old + value;
                }
                let denominator = (self.count * (self.count + 1) / 2) as f64;
                self.weighted_sum / denominator
            }
        };

        self.buffer[self.cursor] = value;
        self.cursor += 1;
        if self.cursor == self.window {
            self.cursor = 0;
        }
        output
    }

    #[must_use]
    pub const fn window(&self) -> usize {
        self.window
    }

    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }

    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.count == self.window
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.cursor = 0;
        self.count = 0;
        self.sum = 0.0;
        self.weighted_sum = 0.0;
        self.ema = 0.0;
    }

    pub fn update_range(&mut self, input: &[f64], range: Range<usize>, output: &mut [f64]) {
        assert!(range.end <= input.len());
        assert!(output.len() >= range.len());
        for (dst, value) in output.iter_mut().zip(input[range].iter()) {
            *dst = self.update(*value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sma_matches_expected_values() {
        let mut state = MovingAverageState::new(MovingAverageKind::Sma, 3);
        assert_eq!(state.update(1.0), 1.0);
        assert_eq!(state.update(2.0), 1.5);
        assert_eq!(state.update(3.0), 2.0);
        assert_eq!(state.update(4.0), 3.0);
    }

    #[test]
    fn ema_streaming_is_stateful() {
        let mut state = MovingAverageState::new(MovingAverageKind::Ema { alpha: 0.5 }, 3);
        assert_eq!(state.update(2.0), 2.0);
        assert_eq!(state.update(4.0), 3.0);
    }

    #[test]
    fn wma_matches_naive_weighting_across_wraparound() {
        let mut state = MovingAverageState::new(MovingAverageKind::Wma, 3);
        let input = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let mut actual = Vec::new();
        for value in input {
            actual.push(state.update(value));
        }

        for end in 0..input.len() {
            let start = end.saturating_add(1).saturating_sub(3);
            let window = &input[start..=end];
            let denominator = (window.len() * (window.len() + 1) / 2) as f64;
            let expected = window
                .iter()
                .enumerate()
                .map(|(index, value)| (index + 1) as f64 * value)
                .sum::<f64>()
                / denominator;
            assert!((actual[end] - expected).abs() < 1e-12);
        }
    }

    #[test]
    fn reset_clears_all_rolling_state() {
        let mut state = MovingAverageState::new(MovingAverageKind::Wma, 3);
        state.update(1.0);
        state.update(2.0);
        state.reset();
        assert_eq!(state.count(), 0);
        assert_eq!(state.update(5.0), 5.0);
    }
}
