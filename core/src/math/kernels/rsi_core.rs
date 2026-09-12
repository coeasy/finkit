//! Canonical Wilder RSI streaming state.

/// Incremental Relative Strength Index state using Wilder smoothing.
#[derive(Debug, Clone, Copy)]
pub struct RsiState {
    period: usize,
    previous: Option<f64>,
    seed_gain: f64,
    seed_loss: f64,
    avg_gain: f64,
    avg_loss: f64,
    changes: usize,
}

impl RsiState {
    /// Create RSI state for a positive lookback period.
    #[must_use]
    pub fn new(period: usize) -> Self {
        assert!(period > 0);
        Self {
            period,
            previous: None,
            seed_gain: 0.0,
            seed_loss: 0.0,
            avg_gain: 0.0,
            avg_loss: 0.0,
            changes: 0,
        }
    }

    /// Push one price. Returns `None` until `period` price changes have been
    /// observed, then emits one RSI value per update.
    #[inline]
    pub fn update(&mut self, value: f64) -> Option<f64> {
        let previous = match self.previous.replace(value) {
            Some(previous) => previous,
            None => return None,
        };
        let change = value - previous;
        let gain = change.max(0.0);
        let loss = (-change).max(0.0);

        if self.changes < self.period {
            self.seed_gain += gain;
            self.seed_loss += loss;
            self.changes += 1;
            if self.changes < self.period {
                return None;
            }
            self.avg_gain = self.seed_gain / self.period as f64;
            self.avg_loss = self.seed_loss / self.period as f64;
        } else {
            let period = self.period as f64;
            self.avg_gain = (self.avg_gain * (period - 1.0) + gain) / period;
            self.avg_loss = (self.avg_loss * (period - 1.0) + loss) / period;
        }

        Some(rsi_from_averages(self.avg_gain, self.avg_loss))
    }

    /// Whether the Wilder seed window has completed.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.changes >= self.period
    }

    /// Reset the state while retaining the configured period.
    pub fn reset(&mut self) {
        let period = self.period;
        *self = Self::new(period);
    }
}

#[inline]
fn rsi_from_averages(avg_gain: f64, avg_loss: f64) -> f64 {
    if avg_loss == 0.0 {
        if avg_gain == 0.0 {
            0.0
        } else {
            100.0
        }
    } else {
        100.0 - 100.0 / (1.0 + avg_gain / avg_loss)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsi_warms_up_on_price_changes() {
        let mut state = RsiState::new(3);
        assert_eq!(state.update(1.0), None);
        assert_eq!(state.update(2.0), None);
        assert_eq!(state.update(3.0), None);
        assert_eq!(state.update(4.0), Some(100.0));
        assert!(state.is_ready());
    }

    #[test]
    fn flat_series_has_zero_rsi_by_contract() {
        let mut state = RsiState::new(2);
        state.update(5.0);
        state.update(5.0);
        assert_eq!(state.update(5.0), Some(0.0));
    }
}
