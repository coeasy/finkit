//! Canonical rolling statistics kernel.
//!
//! Shared by variance, standard deviation and volatility indicators.

#[derive(Debug, Clone, Copy, Default)]
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

    #[inline]
    pub fn count(&self) -> usize { self.count }

    #[inline]
    pub fn mean(&self) -> f64 { self.mean }

    #[inline]
    pub fn variance(&self) -> f64 {
        if self.count <= 1 { 0.0 } else { self.m2 / self.count as f64 }
    }

    #[inline]
    pub fn stddev(&self) -> f64 { self.variance().sqrt() }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welford_matches_basic_variance() {
        let mut state = WelfordState::default();
        for value in [1.0, 2.0, 3.0, 4.0] { state.update(value); }
        assert!((state.mean() - 2.5).abs() < 1e-12);
        assert!((state.stddev() - 1.1180339887).abs() < 1e-8);
    }
}
