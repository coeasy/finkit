//! Canonical rolling statistics kernels.
//!
//! `WelfordState` provides numerically stable online moments. The rolling
//! wrapper adds an O(1) remove/add path so variance, standard deviation,
//! Bollinger-style bands and z-score consumers can share one canonical state.

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WelfordState {
    count: usize,
    mean: f64,
    m2: f64,
}

impl WelfordState {
    #[inline]
    pub fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    /// Remove one observation from the state in O(1).
    ///
    /// This is the inverse Welford update used by fixed-size rolling windows.
    #[inline]
    pub fn remove(&mut self, value: f64) {
        match self.count {
            0 => {}
            1 => self.reset(),
            count => {
                let next_count = count - 1;
                let next_mean = (self.mean * count as f64 - value) / next_count as f64;
                self.m2 -= (value - self.mean) * (value - next_mean);
                if self.m2 < 0.0 && self.m2 > -1e-12 {
                    self.m2 = 0.0;
                }
                self.count = next_count;
                self.mean = next_mean;
            }
        }
    }

    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    #[inline]
    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Population variance, matching TA-style rolling variance conventions.
    #[inline]
    pub fn variance(&self) -> f64 {
        if self.count <= 1 {
            0.0
        } else {
            self.m2 / self.count as f64
        }
    }

    #[inline]
    pub fn stddev(&self) -> f64 {
        self.variance().sqrt()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Snapshot returned by the rolling statistics kernel after each update.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RollingStatistics {
    pub count: usize,
    pub mean: f64,
    pub variance: f64,
    pub stddev: f64,
}

/// Fixed-size rolling Welford state with O(1) push/remove updates.
#[derive(Debug, Clone)]
pub struct RollingWelfordState {
    window: usize,
    buffer: Vec<f64>,
    cursor: usize,
    count: usize,
    state: WelfordState,
}

impl RollingWelfordState {
    #[must_use]
    pub fn new(window: usize) -> Self {
        assert!(window > 0);
        Self {
            window,
            buffer: vec![0.0; window],
            cursor: 0,
            count: 0,
            state: WelfordState::default(),
        }
    }

    #[inline]
    pub fn update(&mut self, value: f64) -> RollingStatistics {
        if self.count == self.window {
            let old = self.buffer[self.cursor];
            self.state.remove(old);
        } else {
            self.count += 1;
        }

        self.buffer[self.cursor] = value;
        self.cursor += 1;
        if self.cursor == self.window {
            self.cursor = 0;
        }
        self.state.update(value);
        self.snapshot()
    }

    #[must_use]
    pub fn snapshot(&self) -> RollingStatistics {
        RollingStatistics {
            count: self.state.count(),
            mean: self.state.mean(),
            variance: self.state.variance(),
            stddev: self.state.stddev(),
        }
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
        self.buffer.fill(0.0);
        self.cursor = 0;
        self.count = 0;
        self.state.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welford_matches_basic_variance() {
        let mut state = WelfordState::default();
        for value in [1.0, 2.0, 3.0, 4.0] {
            state.update(value);
        }
        assert!((state.mean() - 2.5).abs() < 1e-12);
        assert!((state.stddev() - 1.1180339887).abs() < 1e-8);
    }

    #[test]
    fn remove_is_inverse_of_update_for_window_membership() {
        let mut state = WelfordState::default();
        for value in [1.0, 2.0, 3.0, 4.0] {
            state.update(value);
        }
        state.remove(1.0);
        assert_eq!(state.count(), 3);
        assert!((state.mean() - 3.0).abs() < 1e-12);
        assert!((state.variance() - (2.0 / 3.0)).abs() < 1e-12);
    }

    #[test]
    fn rolling_state_matches_naive_population_statistics() {
        let input = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
        let mut rolling = RollingWelfordState::new(3);
        for (end, value) in input.iter().copied().enumerate() {
            let actual = rolling.update(value);
            let start = end.saturating_add(1).saturating_sub(3);
            let window = &input[start..=end];
            let mean = window.iter().sum::<f64>() / window.len() as f64;
            let variance = window
                .iter()
                .map(|item| {
                    let delta = item - mean;
                    delta * delta
                })
                .sum::<f64>()
                / window.len() as f64;
            assert!((actual.mean - mean).abs() < 1e-12);
            assert!((actual.variance - variance).abs() < 1e-10);
        }
    }
}
