//! Shared OHLC family state for Architecture v3.
//!
//! ATR/NATR and the directional family (+DM/-DM, +DI/-DI, DX/ADX) all start
//! from the same per-bar OHLC deltas. Historically each public indicator scans
//! the input independently. This state object computes those intermediates once
//! and exposes a numeric sample that batch, formula and streaming executors can
//! project into whichever outputs they requested.

/// Values produced by one update of [`OhlcFamilyState`].
#[derive(Debug, Clone, Copy)]
pub struct OhlcFamilySample {
    /// Raw true range for the current bar. Row zero is NaN.
    pub tr: f64,
    /// Wilder ATR. Valid from `period`.
    pub atr: f64,
    /// ATR normalized by close, in percent. Valid from `period`.
    pub natr: f64,
    /// Raw positive directional movement for the current bar.
    pub plus_dm: f64,
    /// Raw negative directional movement for the current bar.
    pub minus_dm: f64,
    /// Wilder-smoothed +DI. Valid from `period`.
    pub plus_di: f64,
    /// Wilder-smoothed -DI. Valid from `period`.
    pub minus_di: f64,
    /// Directional Index. Valid from `period`.
    pub dx: f64,
    /// Average Directional Index. Valid from `2 * period - 1`.
    pub adx: f64,
}

impl OhlcFamilySample {
    const fn warmup() -> Self {
        Self {
            tr: f64::NAN,
            atr: f64::NAN,
            natr: f64::NAN,
            plus_dm: f64::NAN,
            minus_dm: f64::NAN,
            plus_di: f64::NAN,
            minus_di: f64::NAN,
            dx: f64::NAN,
            adx: f64::NAN,
        }
    }
}

/// Persistent shared Wilder state for one OHLC stream and one period.
///
/// The object is intentionally scalar and allocation-free so it can live in a
/// [`crate::state_arena::StateSlot`]. All expensive string/function resolution
/// happens when the surrounding execution plan is compiled.
#[derive(Debug, Clone)]
pub struct OhlcFamilyState {
    period: usize,
    index: usize,
    previous_high: f64,
    previous_low: f64,
    previous_close: f64,
    atr_tr_sum: f64,
    atr: f64,
    smooth_tr: f64,
    smooth_plus_dm: f64,
    smooth_minus_dm: f64,
    dx_seed_sum: f64,
    dx_seed_count: usize,
    adx: f64,
}

impl OhlcFamilyState {
    /// Construct state for a positive Wilder period.
    pub fn new(period: usize) -> Option<Self> {
        (period > 0).then_some(Self {
            period,
            index: 0,
            previous_high: f64::NAN,
            previous_low: f64::NAN,
            previous_close: f64::NAN,
            atr_tr_sum: 0.0,
            atr: f64::NAN,
            smooth_tr: 0.0,
            smooth_plus_dm: 0.0,
            smooth_minus_dm: 0.0,
            dx_seed_sum: 0.0,
            dx_seed_count: 0,
            adx: f64::NAN,
        })
    }

    /// Wilder period used by this state.
    pub const fn period(&self) -> usize {
        self.period
    }

    /// Number of bars consumed so far.
    pub const fn len(&self) -> usize {
        self.index
    }

    /// Whether no bars have been consumed yet.
    pub const fn is_empty(&self) -> bool {
        self.index == 0
    }

    /// Consume one OHLC bar and compute all shared family intermediates once.
    #[inline]
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> OhlcFamilySample {
        if self.index == 0 {
            self.previous_high = high;
            self.previous_low = low;
            self.previous_close = close;
            self.index = 1;
            return OhlcFamilySample::warmup();
        }

        let logical_index = self.index;
        let previous_high = self.previous_high;
        let previous_low = self.previous_low;
        let previous_close = self.previous_close;

        let tr = (high - low)
            .max((high - previous_close).abs())
            .max((low - previous_close).abs());
        let up_move = high - previous_high;
        let down_move = previous_low - low;
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

        let mut sample = OhlcFamilySample {
            tr,
            plus_dm,
            minus_dm,
            ..OhlcFamilySample::warmup()
        };

        // ATR seeds from TR[1..=period], then uses the standard Wilder update.
        if logical_index <= self.period {
            self.atr_tr_sum += tr;
            if logical_index == self.period {
                self.atr = self.atr_tr_sum / self.period as f64;
            }
        } else {
            self.atr += (tr - self.atr) / self.period as f64;
        }
        if logical_index >= self.period {
            sample.atr = self.atr;
            sample.natr = if close.abs() > 1e-15 {
                self.atr / close * 100.0
            } else {
                0.0
            };
        }

        // TA-Lib directional seed accumulates period-1 bars, then applies one
        // Wilder recurrence on bar `period` before exposing the first DI/DX.
        if logical_index < self.period {
            self.smooth_tr += tr;
            self.smooth_plus_dm += plus_dm;
            self.smooth_minus_dm += minus_dm;
        } else {
            let p = self.period as f64;
            self.smooth_tr = self.smooth_tr - self.smooth_tr / p + tr;
            self.smooth_plus_dm = self.smooth_plus_dm - self.smooth_plus_dm / p + plus_dm;
            self.smooth_minus_dm = self.smooth_minus_dm - self.smooth_minus_dm / p + minus_dm;

            let (plus_di, minus_di) = if self.smooth_tr.abs() > 1e-15 {
                (
                    self.smooth_plus_dm / self.smooth_tr * 100.0,
                    self.smooth_minus_dm / self.smooth_tr * 100.0,
                )
            } else {
                (0.0, 0.0)
            };
            let di_sum = plus_di + minus_di;
            let dx = if di_sum.abs() > 1e-15 {
                (plus_di - minus_di).abs() / di_sum * 100.0
            } else {
                0.0
            };
            sample.plus_di = plus_di;
            sample.minus_di = minus_di;
            sample.dx = dx;

            if self.dx_seed_count < self.period {
                self.dx_seed_sum += dx;
                self.dx_seed_count += 1;
                if self.dx_seed_count == self.period {
                    self.adx = self.dx_seed_sum / self.period as f64;
                    sample.adx = self.adx;
                }
            } else {
                self.adx = (self.adx * (p - 1.0) + dx) / p;
                sample.adx = self.adx;
            }
        }

        self.previous_high = high;
        self.previous_low = low;
        self.previous_close = close;
        self.index += 1;
        sample
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::volatility::{atr, natr};
    use crate::math::directional::{minus_di, plus_di};

    fn sample_ohlc(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let close: Vec<_> = (0..len)
            .map(|i| 100.0 + i as f64 * 0.17 + ((i % 7) as f64 - 3.0) * 0.11)
            .collect();
        let high: Vec<_> = close
            .iter()
            .enumerate()
            .map(|(i, value)| value + 0.8 + (i % 3) as f64 * 0.03)
            .collect();
        let low: Vec<_> = close
            .iter()
            .enumerate()
            .map(|(i, value)| value - 0.7 - (i % 5) as f64 * 0.02)
            .collect();
        (high, low, close)
    }

    fn assert_same(left: f64, right: f64) {
        if left.is_nan() || right.is_nan() {
            assert!(left.is_nan() && right.is_nan());
        } else {
            assert!((left - right).abs() <= 1e-12, "{left} != {right}");
        }
    }

    #[test]
    fn shared_state_matches_existing_atr_natr_and_di_contracts() {
        let period = 14;
        let (high, low, close) = sample_ohlc(96);
        let expected_atr = atr(&high, &low, &close, period).unwrap();
        let expected_natr = natr(&high, &low, &close, period).unwrap();
        let expected_plus = plus_di(&high, &low, &close, period).unwrap();
        let expected_minus = minus_di(&high, &low, &close, period).unwrap();
        let mut state = OhlcFamilyState::new(period).unwrap();

        for index in 0..high.len() {
            let sample = state.update(high[index], low[index], close[index]);
            assert_same(sample.atr, expected_atr[index]);
            assert_same(sample.natr, expected_natr[index]);
            if index >= period {
                assert_same(sample.plus_di, expected_plus[index]);
                assert_same(sample.minus_di, expected_minus[index]);
            }
        }
    }

    #[test]
    fn raw_tr_dm_and_directional_outputs_share_one_update() {
        let mut state = OhlcFamilyState::new(3).unwrap();
        assert!(state.update(10.0, 8.0, 9.0).tr.is_nan());
        let second = state.update(12.0, 9.0, 11.0);
        assert_eq!(second.tr, 3.0);
        assert_eq!(second.plus_dm, 2.0);
        assert_eq!(second.minus_dm, 0.0);
        state.update(13.0, 10.0, 12.0);
        let fourth = state.update(14.0, 9.0, 10.0);
        assert!(fourth.atr.is_finite());
        assert!(fourth.plus_di.is_finite());
        assert!(fourth.minus_di.is_finite());
        assert!(fourth.dx.is_finite());
    }

    #[test]
    fn invalid_zero_period_is_rejected_at_compile_boundary() {
        assert!(OhlcFamilyState::new(0).is_none());
    }
}
