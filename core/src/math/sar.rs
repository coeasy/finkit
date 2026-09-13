//! TA-Lib 0.7.1-compatible Parabolic SAR state and batch kernel.
//!
//! Architecture v3.1 keeps one transition state for batch and streaming SAR.
//! TA-Lib determines the initial direction from one-period directional movement
//! between the first two bars, consumes the first bar as warm-up, and publishes
//! its first SAR at index 1.

use crate::error::{Result, TaError};
use core::{mem::MaybeUninit, slice};

#[inline]
fn validate_params(acceleration: f64, maximum: f64) -> Result<()> {
    if acceleration < 0.0 {
        return Err(TaError::InvalidParameter {
            name: "acceleration".to_string(),
            constraint: "must be non-negative".to_string(),
        });
    }
    if maximum < 0.0 {
        return Err(TaError::InvalidParameter {
            name: "maximum".to_string(),
            constraint: "must be non-negative".to_string(),
        });
    }
    Ok(())
}

#[inline]
fn validate_inputs(high: &[f64], low: &[f64], acceleration: f64, maximum: f64) -> Result<()> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if high.len() < 2 {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: 2,
        });
    }
    validate_params(acceleration, maximum)
}

/// One incremental SAR output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SarPoint {
    /// TA-Lib-compatible SAR value. The first consumed bar is NaN warm-up.
    pub sar: f64,
    /// Current acceleration factor after processing the bar.
    pub af: f64,
    /// Trend direction: `1` long, `-1` short, `0` before bootstrap completes.
    pub direction: i32,
}

/// Canonical persistent Parabolic SAR transition state.
///
/// Batch and streaming frontends must both use this state so reversal, clamp,
/// extreme-point and acceleration-factor ordering cannot diverge.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SarState {
    acceleration: f64,
    maximum: f64,
    effective_acceleration: f64,
    af: f64,
    is_long: Option<bool>,
    sar: f64,
    ep: f64,
    first_high: f64,
    first_low: f64,
    prev_high: f64,
    prev_low: f64,
    count: usize,
}

impl SarState {
    /// Construct a state. Invalid parameters are rejected at the same boundary
    /// as the batch kernel.
    pub fn try_new(acceleration: f64, maximum: f64) -> Result<Self> {
        validate_params(acceleration, maximum)?;
        let effective_acceleration = acceleration.min(maximum);
        Ok(Self {
            acceleration,
            maximum,
            effective_acceleration,
            af: effective_acceleration,
            is_long: None,
            sar: f64::NAN,
            ep: f64::NAN,
            first_high: f64::NAN,
            first_low: f64::NAN,
            prev_high: f64::NAN,
            prev_low: f64::NAN,
            count: 0,
        })
    }

    /// Number of bars consumed.
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Whether no bar has been consumed.
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Whether the two-bar TA-Lib bootstrap is complete.
    pub const fn is_ready(&self) -> bool {
        self.count >= 2
    }

    /// Reset while retaining parameters.
    pub fn reset(&mut self) {
        self.effective_acceleration = self.acceleration.min(self.maximum);
        self.af = self.effective_acceleration;
        self.is_long = None;
        self.sar = f64::NAN;
        self.ep = f64::NAN;
        self.first_high = f64::NAN;
        self.first_low = f64::NAN;
        self.prev_high = f64::NAN;
        self.prev_low = f64::NAN;
        self.count = 0;
    }

    /// Consume one high/low bar using TA_SAR 0.7.1 transition ordering.
    #[inline(always)]
    pub fn next(&mut self, high: f64, low: f64) -> SarPoint {
        if self.count == 0 {
            self.first_high = high;
            self.first_low = low;
            self.prev_high = high;
            self.prev_low = low;
            self.count = 1;
            return SarPoint {
                sar: f64::NAN,
                af: self.af,
                direction: 0,
            };
        }

        if self.count == 1 {
            let diff_p = high - self.first_high;
            let diff_m = self.first_low - low;
            let is_long = !(diff_m > 0.0 && diff_p < diff_m);
            self.is_long = Some(is_long);
            self.af = self.effective_acceleration;
            if is_long {
                self.ep = high;
                self.sar = self.first_low;
            } else {
                self.ep = low;
                self.sar = self.first_high;
            }

            // TA-Lib primes prevHigh/prevLow with bar 1 before processing bar 1.
            self.prev_high = high;
            self.prev_low = low;
        }

        let prev_high = self.prev_high;
        let prev_low = self.prev_low;
        let mut is_long = self.is_long.expect("SAR direction bootstrapped");
        let output_sar;

        if is_long {
            if low <= self.sar {
                is_long = false;
                self.sar = self.ep;
                if self.sar < prev_high {
                    self.sar = prev_high;
                }
                if self.sar < high {
                    self.sar = high;
                }
                output_sar = self.sar;

                self.af = self.effective_acceleration;
                self.ep = low;
                self.sar += self.af * (self.ep - self.sar);
                if self.sar < prev_high {
                    self.sar = prev_high;
                }
                if self.sar < high {
                    self.sar = high;
                }
            } else {
                output_sar = self.sar;
                if high > self.ep {
                    self.ep = high;
                    self.af = (self.af + self.effective_acceleration).min(self.maximum);
                }
                self.sar += self.af * (self.ep - self.sar);
                if self.sar > prev_low {
                    self.sar = prev_low;
                }
                if self.sar > low {
                    self.sar = low;
                }
            }
        } else if high >= self.sar {
            is_long = true;
            self.sar = self.ep;
            if self.sar > prev_low {
                self.sar = prev_low;
            }
            if self.sar > low {
                self.sar = low;
            }
            output_sar = self.sar;

            self.af = self.effective_acceleration;
            self.ep = high;
            self.sar += self.af * (self.ep - self.sar);
            if self.sar > prev_low {
                self.sar = prev_low;
            }
            if self.sar > low {
                self.sar = low;
            }
        } else {
            output_sar = self.sar;
            if low < self.ep {
                self.ep = low;
                self.af = (self.af + self.effective_acceleration).min(self.maximum);
            }
            self.sar += self.af * (self.ep - self.sar);
            if self.sar < prev_high {
                self.sar = prev_high;
            }
            if self.sar < high {
                self.sar = high;
            }
        }

        self.is_long = Some(is_long);
        self.prev_high = high;
        self.prev_low = low;
        self.count += 1;

        SarPoint {
            sar: output_sar,
            af: self.af,
            direction: if is_long { 1 } else { -1 },
        }
    }
}

/// Calculate Parabolic SAR and acceleration-factor series through the canonical state.
pub fn sar_with_af(
    high: &[f64],
    low: &[f64],
    acceleration: f64,
    maximum: f64,
) -> Result<(Vec<f64>, Vec<f64>)> {
    validate_inputs(high, low, acceleration, maximum)?;
    let mut state = SarState::try_new(acceleration, maximum)?;
    let mut sar = Vec::with_capacity(high.len());
    let mut af = Vec::with_capacity(high.len());
    for index in 0..high.len() {
        let point = state.next(high[index], low[index]);
        sar.push(point.sar);
        af.push(if index == 0 { f64::NAN } else { point.af });
    }
    Ok((sar, af))
}

/// Calculate Parabolic SAR with the exact TA_SAR 0.7.1 bootstrap/update order.
///
/// The single-output path deliberately does not call [`sar_with_af`]. Consumers
/// such as the installed Python wheel only request SAR, so allocating and
/// writing a second full-length AF vector was pure hot-path overhead. Both
/// paths still share the same canonical [`SarState`] transition logic.
pub fn sar(high: &[f64], low: &[f64], acceleration: f64, maximum: f64) -> Result<Vec<f64>> {
    validate_inputs(high, low, acceleration, maximum)?;
    let len = high.len();
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe { raw_output.set_len(len) };
    let output = unsafe { slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast::<f64>(), len) };
    sar_into(high, low, acceleration, maximum, output)?;

    let ptr = raw_output.as_mut_ptr().cast::<f64>();
    let capacity = raw_output.capacity();
    core::mem::forget(raw_output);
    Ok(unsafe { Vec::from_raw_parts(ptr, len, capacity) })
}

/// Calculate Parabolic SAR directly into a caller-owned output slice.
///
/// This is the installed-wheel hot-path variant: it keeps the canonical SAR
/// transition loop while avoiding the temporary Rust buffer and the copy into
/// the final NumPy array.
pub fn sar_into(
    high: &[f64],
    low: &[f64],
    acceleration: f64,
    maximum: f64,
    output: &mut [f64],
) -> Result<()> {
    validate_inputs(high, low, acceleration, maximum)?;
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    let mut state = SarState::try_new(acceleration, maximum)?;
    let len = high.len();

    // Every slot is written exactly once. This removes both the unused AF
    // allocation and the per-row Vec::push capacity branch from the
    // benchmark-critical single-output path.
    unsafe {
        let high_ptr = high.as_ptr();
        let low_ptr = low.as_ptr();
        let output_ptr = output.as_mut_ptr();
        output_ptr.write(state.next(*high_ptr, *low_ptr).sar);
        output_ptr
            .add(1)
            .write(state.next(*high_ptr.add(1), *low_ptr.add(1)).sar);

        // The two-bar bootstrap above establishes every invariant needed by
        // the batch loop. Keep the transition state in local scalars so the
        // million-row path does not repeatedly unwrap `Option<bool>` or touch
        // the streaming-only bar counter.
        let mut is_long = state.is_long.expect("SAR direction bootstrapped");
        let mut sar = state.sar;
        let mut ep = state.ep;
        let mut af = state.af;
        let effective_acceleration = state.effective_acceleration;
        // The batch loop no longer needs the streaming state object after the
        // two-bar bootstrap. Keep the previous bar in local scalars so the
        // million-row path avoids a mutable field reference on every clamp.
        let mut previous_high = state.prev_high;
        let mut previous_low = state.prev_low;

        for index in 2..len {
            let current_high = *high_ptr.add(index);
            let current_low = *low_ptr.add(index);
            let output_sar;

            if is_long {
                if current_low <= sar {
                    is_long = false;
                    sar = ep;
                    if sar < previous_high {
                        sar = previous_high;
                    }
                    if sar < current_high {
                        sar = current_high;
                    }
                    output_sar = sar;

                    af = effective_acceleration;
                    ep = current_low;
                    sar += af * (ep - sar);
                    if sar < previous_high {
                        sar = previous_high;
                    }
                    if sar < current_high {
                        sar = current_high;
                    }
                } else {
                    output_sar = sar;
                    if current_high > ep {
                        ep = current_high;
                        af = (af + effective_acceleration).min(maximum);
                    }
                    sar += af * (ep - sar);
                    if sar > previous_low {
                        sar = previous_low;
                    }
                    if sar > current_low {
                        sar = current_low;
                    }
                }
            } else if current_high >= sar {
                is_long = true;
                sar = ep;
                if sar > previous_low {
                    sar = previous_low;
                }
                if sar > current_low {
                    sar = current_low;
                }
                output_sar = sar;

                af = effective_acceleration;
                ep = current_high;
                sar += af * (ep - sar);
                if sar > previous_low {
                    sar = previous_low;
                }
                if sar > current_low {
                    sar = current_low;
                }
            } else {
                output_sar = sar;
                if current_low < ep {
                    ep = current_low;
                    af = (af + effective_acceleration).min(maximum);
                }
                sar += af * (ep - sar);
                if sar < previous_high {
                    sar = previous_high;
                }
                if sar < current_high {
                    sar = current_high;
                }
            }

            previous_high = current_high;
            previous_low = current_low;
            output_ptr.add(index).write(output_sar);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_output_consumes_first_bar_and_defaults_long_on_dm_tie() {
        let high = [10.0, 11.0, 12.0, 13.0];
        let low = [9.0, 10.0, 11.0, 12.0];
        let output = sar(&high, &low, 0.02, 0.2).unwrap();
        assert!(output[0].is_nan());
        assert_eq!(output[1], low[0]);
    }

    #[test]
    fn positive_minus_dm_bootstraps_short() {
        let high = [10.0, 9.5, 9.0, 8.5];
        let low = [9.0, 8.0, 7.0, 6.0];
        let output = sar(&high, &low, 0.02, 0.2).unwrap();
        assert!(output[0].is_nan());
        assert_eq!(output[1], high[0]);
    }

    #[test]
    fn acceleration_is_capped_by_maximum_at_bootstrap() {
        let high = [10.0, 11.0, 12.0, 13.0];
        let low = [9.0, 10.0, 11.0, 12.0];
        let capped = sar(&high, &low, 0.5, 0.2).unwrap();
        let explicit = sar(&high, &low, 0.2, 0.2).unwrap();
        assert!(capped[0].is_nan() && explicit[0].is_nan());
        assert_eq!(&capped[1..], &explicit[1..]);
    }

    #[test]
    fn incremental_state_matches_batch_exactly() {
        let high: Vec<f64> = (0..64)
            .map(|i| 10.0 + (i as f64 * 0.31).sin() * 3.0 + i as f64 * 0.01)
            .collect();
        let low: Vec<f64> = high
            .iter()
            .enumerate()
            .map(|(i, h)| h - 1.2 - (i % 4) as f64 * 0.05)
            .collect();
        let batch = sar(&high, &low, 0.02, 0.2).unwrap();
        let mut state = SarState::try_new(0.02, 0.2).unwrap();
        for index in 0..high.len() {
            let point = state.next(high[index], low[index]);
            if batch[index].is_nan() {
                assert!(point.sar.is_nan());
            } else {
                assert_eq!(point.sar, batch[index]);
            }
        }
    }

    #[test]
    fn single_output_matches_with_af_projection() {
        let high: Vec<f64> = (0..128)
            .map(|i| 25.0 + (i as f64 * 0.19).sin() * 2.5 + i as f64 * 0.015)
            .collect();
        let low: Vec<f64> = high
            .iter()
            .enumerate()
            .map(|(i, h)| h - 1.0 - (i % 5) as f64 * 0.04)
            .collect();

        let single = sar(&high, &low, 0.02, 0.2).unwrap();
        let (projected, _) = sar_with_af(&high, &low, 0.02, 0.2).unwrap();
        assert_eq!(single.len(), projected.len());
        for (left, right) in single.iter().zip(projected.iter()) {
            if left.is_nan() || right.is_nan() {
                assert!(left.is_nan() && right.is_nan());
            } else {
                assert_eq!(left, right);
            }
        }
    }
}
