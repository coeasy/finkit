//! Sliding extrema kernel foundation.
//!
//! The deque based implementation provides O(1) amortized min/max updates.

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct MonotonicExtrema {
    window: usize,
    values: VecDeque<(usize, f64)>,
    descending: bool,
}

impl MonotonicExtrema {
    pub fn new(window: usize, max: bool) -> Self {
        Self { window, values: VecDeque::new(), descending: max }
    }

    pub fn update(&mut self, index: usize, value: f64) -> f64 {
        while let Some((_, tail)) = self.values.back() {
            let remove = if self.descending { *tail <= value } else { *tail >= value };
            if remove { self.values.pop_back(); } else { break; }
        }
        self.values.push_back((index, value));

        while let Some((position, _)) = self.values.front() {
            if *position + self.window <= index { self.values.pop_front(); } else { break; }
        }

        self.values.front().map(|(_, v)| *v).unwrap_or(value)
    }

    pub fn reset(&mut self) {
        self.values.clear();
    }
}
