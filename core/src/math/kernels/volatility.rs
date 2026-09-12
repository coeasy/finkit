//! Canonical volatility and directional-movement kernels.
//!
//! These state machines preserve TA-Lib-compatible Wilder seeding while
//! sharing True Range and Directional Movement intermediates across ATR, DI,
//! DX and ADX consumers.

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TrueRangeState {
    previous_close: Option<f64>,
}

impl TrueRangeState {
    #[inline]
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> f64 {
        let value = if let Some(previous_close) = self.previous_close {
            true_range(high, low, previous_close)
        } else {
            high - low
        };
        self.previous_close = Some(close);
        value
    }

    pub fn reset(&mut self) {
        self.previous_close = None;
    }
}

/// Incremental ATR with TA-Lib-compatible seed semantics.
#[derive(Debug, Clone, Copy)]
pub struct AtrState {
    period: usize,
    previous_close: Option<f64>,
    seed_sum: f64,
    seed_count: usize,
    atr: f64,
}

impl AtrState {
    #[must_use]
    pub fn new(period: usize) -> Self {
        assert!(period > 0);
        Self {
            period,
            previous_close: None,
            seed_sum: 0.0,
            seed_count: 0,
            atr: 0.0,
        }
    }

    #[inline]
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> Option<f64> {
        let previous_close = match self.previous_close.replace(close) {
            Some(previous_close) => previous_close,
            None => return None,
        };
        let tr = true_range(high, low, previous_close);

        if self.seed_count < self.period {
            self.seed_sum += tr;
            self.seed_count += 1;
            if self.seed_count < self.period {
                return None;
            }
            self.atr = self.seed_sum / self.period as f64;
        } else {
            self.atr += (tr - self.atr) / self.period as f64;
        }
        Some(self.atr)
    }

    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.seed_count >= self.period
    }

    pub fn reset(&mut self) {
        let period = self.period;
        *self = Self::new(period);
    }
}

/// Per-bar directional movement primitives shared by DMI/ADX.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DirectionalMovement {
    pub true_range: f64,
    pub plus_dm: f64,
    pub minus_dm: f64,
}

/// Incremental raw TR/+DM/-DM state.
#[derive(Debug, Clone, Copy, Default)]
pub struct DirectionalMovementState {
    previous_high: Option<f64>,
    previous_low: Option<f64>,
    previous_close: Option<f64>,
}

impl DirectionalMovementState {
    #[inline]
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> Option<DirectionalMovement> {
        let previous = match (
            self.previous_high.replace(high),
            self.previous_low.replace(low),
            self.previous_close.replace(close),
        ) {
            (Some(previous_high), Some(previous_low), Some(previous_close)) => {
                (previous_high, previous_low, previous_close)
            }
            _ => return None,
        };

        let up_move = high - previous.0;
        let down_move = previous.1 - low;
        let plus_dm = if up_move > down_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };
        let minus_dm = if down_move > up_move && down_move > 0.0 {
            down_move
        } else {
            0.0
        };

        Some(DirectionalMovement {
            true_range: true_range(high, low, previous.2),
            plus_dm,
            minus_dm,
        })
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// DMI/DX values after Wilder smoothing is initialized.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DirectionalIndex {
    pub atr: f64,
    pub plus_di: f64,
    pub minus_di: f64,
    pub dx: f64,
}

/// Shared Wilder state for ATR, +DI, -DI and DX.
#[derive(Debug, Clone, Copy)]
pub struct DmiState {
    period: usize,
    raw: DirectionalMovementState,
    seed_count: usize,
    tr_sum: f64,
    plus_dm_sum: f64,
    minus_dm_sum: f64,
    smoothed_tr: f64,
    smoothed_plus_dm: f64,
    smoothed_minus_dm: f64,
}

impl DmiState {
    #[must_use]
    pub fn new(period: usize) -> Self {
        assert!(period > 0);
        Self {
            period,
            raw: DirectionalMovementState::default(),
            seed_count: 0,
            tr_sum: 0.0,
            plus_dm_sum: 0.0,
            minus_dm_sum: 0.0,
            smoothed_tr: 0.0,
            smoothed_plus_dm: 0.0,
            smoothed_minus_dm: 0.0,
        }
    }

    #[inline]
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> Option<DirectionalIndex> {
        let movement = self.raw.update(high, low, close)?;
        let period = self.period as f64;

        if self.seed_count < self.period {
            self.seed_count += 1;
            self.tr_sum += movement.true_range;
            self.plus_dm_sum += movement.plus_dm;
            self.minus_dm_sum += movement.minus_dm;
            if self.seed_count < self.period {
                return None;
            }
            self.smoothed_tr = self.tr_sum;
            self.smoothed_plus_dm = self.plus_dm_sum;
            self.smoothed_minus_dm = self.minus_dm_sum;
        } else {
            self.smoothed_tr = self.smoothed_tr - self.smoothed_tr / period + movement.true_range;
            self.smoothed_plus_dm =
                self.smoothed_plus_dm - self.smoothed_plus_dm / period + movement.plus_dm;
            self.smoothed_minus_dm =
                self.smoothed_minus_dm - self.smoothed_minus_dm / period + movement.minus_dm;
        }

        Some(directional_index(
            self.smoothed_tr,
            self.smoothed_plus_dm,
            self.smoothed_minus_dm,
            period,
        ))
    }

    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.seed_count >= self.period
    }

    pub fn reset(&mut self) {
        let period = self.period;
        *self = Self::new(period);
    }
}

/// Complete ADX family streaming output.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AdxOutput {
    pub atr: f64,
    pub plus_di: f64,
    pub minus_di: f64,
    pub dx: f64,
    pub adx: Option<f64>,
}

/// Incremental ADX sharing the same DMI state used by ATR/DI/DX.
#[derive(Debug, Clone, Copy)]
pub struct AdxState {
    period: usize,
    dmi: DmiState,
    dx_seed_sum: f64,
    dx_count: usize,
    adx: f64,
}

impl AdxState {
    #[must_use]
    pub fn new(period: usize) -> Self {
        assert!(period > 0);
        Self {
            period,
            dmi: DmiState::new(period),
            dx_seed_sum: 0.0,
            dx_count: 0,
            adx: 0.0,
        }
    }

    #[inline]
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> Option<AdxOutput> {
        let dmi = self.dmi.update(high, low, close)?;
        let adx = if self.dx_count < self.period {
            self.dx_seed_sum += dmi.dx;
            self.dx_count += 1;
            if self.dx_count == self.period {
                self.adx = self.dx_seed_sum / self.period as f64;
                Some(self.adx)
            } else {
                None
            }
        } else {
            self.adx += (dmi.dx - self.adx) / self.period as f64;
            Some(self.adx)
        };

        Some(AdxOutput {
            atr: dmi.atr,
            plus_di: dmi.plus_di,
            minus_di: dmi.minus_di,
            dx: dmi.dx,
            adx,
        })
    }

    pub fn reset(&mut self) {
        let period = self.period;
        *self = Self::new(period);
    }
}

#[inline]
fn true_range(high: f64, low: f64, previous_close: f64) -> f64 {
    (high - low)
        .max((high - previous_close).abs())
        .max((low - previous_close).abs())
}

#[inline]
fn directional_index(
    smoothed_tr: f64,
    smoothed_plus_dm: f64,
    smoothed_minus_dm: f64,
    period: f64,
) -> DirectionalIndex {
    let atr = smoothed_tr / period;
    if smoothed_tr.abs() <= f64::EPSILON {
        return DirectionalIndex {
            atr,
            plus_di: 0.0,
            minus_di: 0.0,
            dx: 0.0,
        };
    }
    let plus_di = 100.0 * smoothed_plus_dm / smoothed_tr;
    let minus_di = 100.0 * smoothed_minus_dm / smoothed_tr;
    let denominator = plus_di + minus_di;
    let dx = if denominator.abs() <= f64::EPSILON {
        0.0
    } else {
        100.0 * (plus_di - minus_di).abs() / denominator
    };
    DirectionalIndex {
        atr,
        plus_di,
        minus_di,
        dx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn true_range_uses_previous_close() {
        let mut state = TrueRangeState::default();
        assert_eq!(state.update(12.0, 10.0, 11.0), 2.0);
        assert_eq!(state.update(15.0, 13.0, 14.0), 4.0);
    }

    #[test]
    fn atr_seeds_from_period_movements_not_first_bar() {
        let mut state = AtrState::new(3);
        assert_eq!(state.update(10.0, 8.0, 9.0), None);
        assert_eq!(state.update(12.0, 10.0, 11.0), None);
        assert_eq!(state.update(14.0, 12.0, 13.0), None);
        assert_eq!(state.update(16.0, 14.0, 15.0), Some(3.0));
        assert_eq!(state.update(18.0, 16.0, 17.0), Some(3.0));
    }

    #[test]
    fn directional_movement_obeys_wilder_exclusivity() {
        let mut state = DirectionalMovementState::default();
        assert_eq!(state.update(10.0, 8.0, 9.0), None);
        let up = state.update(12.0, 9.0, 11.0).unwrap();
        assert_eq!(up.plus_dm, 2.0);
        assert_eq!(up.minus_dm, 0.0);
        let down = state.update(11.0, 6.0, 7.0).unwrap();
        assert_eq!(down.plus_dm, 0.0);
        assert_eq!(down.minus_dm, 3.0);
    }

    #[test]
    fn adx_first_value_occurs_at_two_period_minus_one() {
        let period = 3;
        let mut state = AdxState::new(period);
        let bars = [
            (10.0, 8.0, 9.0),
            (11.0, 9.0, 10.0),
            (12.0, 10.0, 11.0),
            (13.0, 11.0, 12.0),
            (14.0, 12.0, 13.0),
            (15.0, 13.0, 14.0),
        ];
        let mut first_adx = None;
        for (index, (high, low, close)) in bars.into_iter().enumerate() {
            if let Some(output) = state.update(high, low, close) {
                if output.adx.is_some() && first_adx.is_none() {
                    first_adx = Some(index);
                }
            }
        }
        assert_eq!(first_adx, Some(2 * period - 1));
    }
}
