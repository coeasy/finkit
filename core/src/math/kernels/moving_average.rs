//! Canonical moving-average kernel shared by batch and streaming execution.
//!
//! This is the first Architecture V4 kernel migration. Higher level
//! indicators should depend on this state machine instead of owning duplicate
//! rolling state.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovingAverageKind {
    Sma,
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
    ema: f64,
}

impl MovingAverageState {
    pub fn new(kind: MovingAverageKind, window: usize) -> Self {
        assert!(window > 0);
        Self {
            kind,
            window,
            buffer: vec![0.0; window],
            cursor: 0,
            count: 0,
            sum: 0.0,
            ema: 0.0,
        }
    }

    #[inline(always)]
    pub fn update(&mut self, value: f64) -> f64 {
        let old = self.buffer[self.cursor];
        self.buffer[self.cursor] = value;
        self.cursor = (self.cursor + 1) % self.window;

        if self.count < self.window {
            self.count += 1;
        }

        self.sum += value - old;

        match self.kind {
            MovingAverageKind::Sma => self.sum / self.count as f64,
            MovingAverageKind::Ema { alpha } => {
                if self.count == 1 {
                    self.ema = value;
                } else {
                    self.ema = alpha * value + (1.0 - alpha) * self.ema;
                }
                self.ema
            }
            MovingAverageKind::Wma => {
                let mut total = 0.0;
                let mut weight_sum = 0.0;
                for offset in 0..self.count {
                    let index = (self.cursor + self.window - self.count + offset) % self.window;
                    let weight = (offset + 1) as f64;
                    total += self.buffer[index] * weight;
                    weight_sum += weight;
                }
                total / weight_sum
            }
        }
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.cursor = 0;
        self.count = 0;
        self.sum = 0.0;
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
}
