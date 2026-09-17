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
        assert!(window > 0);
        Self {
            window,
            values: VecDeque::new(),
            descending: max,
        }
    }

    pub fn update(&mut self, index: usize, value: f64) -> f64 {
        while let Some((_, tail)) = self.values.back() {
            let remove = if self.descending {
                *tail <= value
            } else {
                *tail >= value
            };
            if remove {
                self.values.pop_back();
            } else {
                break;
            }
        }
        self.values.push_back((index, value));

        while let Some((position, _)) = self.values.front() {
            if *position + self.window <= index {
                self.values.pop_front();
            } else {
                break;
            }
        }

        self.values
            .front()
            .map(|(_, value)| *value)
            .unwrap_or(value)
    }

    pub fn reset(&mut self) {
        self.values.clear();
    }
}

/// Paired rolling high/low extrema state for MIDPRICE, WILLR and STOCH-style
/// consumers. Both deques advance under one shared index.
#[derive(Debug, Clone)]
pub struct RollingExtremaPair {
    maximum: MonotonicExtrema,
    minimum: MonotonicExtrema,
    next_index: usize,
    count: usize,
    window: usize,
}

impl RollingExtremaPair {
    #[must_use]
    pub fn new(window: usize) -> Self {
        assert!(window > 0);
        Self {
            maximum: MonotonicExtrema::new(window, true),
            minimum: MonotonicExtrema::new(window, false),
            next_index: 0,
            count: 0,
            window,
        }
    }

    /// Push one high/low pair and return `(highest, lowest)` for the current
    /// trailing window, including warm-up prefixes.
    #[inline]
    pub fn update(&mut self, high: f64, low: f64) -> (f64, f64) {
        let index = self.next_index;
        self.next_index = self.next_index.saturating_add(1);
        self.count = self.count.saturating_add(1).min(self.window);
        (
            self.maximum.update(index, high),
            self.minimum.update(index, low),
        )
    }

    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.count == self.window
    }

    #[must_use]
    pub const fn window(&self) -> usize {
        self.window
    }

    pub fn reset(&mut self) {
        self.maximum.reset();
        self.minimum.reset();
        self.next_index = 0;
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_extrema_expires_both_sides_in_lockstep() {
        let mut state = RollingExtremaPair::new(3);
        assert_eq!(state.update(5.0, 2.0), (5.0, 2.0));
        assert_eq!(state.update(7.0, 3.0), (7.0, 2.0));
        assert_eq!(state.update(6.0, 1.0), (7.0, 1.0));
        assert_eq!(state.update(4.0, 2.5), (7.0, 1.0));
        assert_eq!(state.update(3.0, 2.0), (6.0, 1.0));
        assert_eq!(state.update(2.0, 1.5), (4.0, 1.5));
    }
}
