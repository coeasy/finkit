//! TA-Lib 0.7.1-compatible Parabolic SAR kernel.
//!
//! SAR bootstrap is unusually sensitive to interpretation.  TA-Lib determines
//! the initial direction from one-period directional movement between the first
//! two bars, consumes the first bar, and publishes its first SAR at index 1.
//! This implementation mirrors that state transition order exactly.

use crate::error::{Result, TaError};

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

/// Calculate Parabolic SAR with the exact TA_SAR 0.7.1 bootstrap/update order.
pub fn sar(high: &[f64], low: &[f64], acceleration: f64, maximum: f64) -> Result<Vec<f64>> {
    validate_inputs(high, low, acceleration, maximum)?;

    let mut output = vec![f64::NAN; high.len()];
    let mut effective_acceleration = acceleration;
    let mut af = acceleration;
    if af > maximum {
        effective_acceleration = maximum;
        af = effective_acceleration;
    }

    // TA_MINUS_DM(period=1) between bar 0 and bar 1.  A positive -DM starts
    // short; every tie/default case starts long.
    let diff_p = high[1] - high[0];
    let diff_m = low[0] - low[1];
    let mut is_long = !(diff_m > 0.0 && diff_p < diff_m);

    let mut today_idx = 1usize;
    let mut new_high = high[today_idx - 1];
    let mut new_low = low[today_idx - 1];
    let (mut ep, mut sar) = if is_long {
        (high[today_idx], new_low)
    } else {
        (low[today_idx], new_high)
    };

    // TA-Lib deliberately primes these with today's bar, then reloads the same
    // bar in the first loop iteration so prevHigh/prevLow equal bar 1.
    new_low = low[today_idx];
    new_high = high[today_idx];

    while today_idx < high.len() {
        let prev_low = new_low;
        let prev_high = new_high;
        new_low = low[today_idx];
        new_high = high[today_idx];
        let output_idx = today_idx;
        today_idx += 1;

        if is_long {
            if new_low <= sar {
                is_long = false;
                sar = ep;
                if sar < prev_high {
                    sar = prev_high;
                }
                if sar < new_high {
                    sar = new_high;
                }
                output[output_idx] = sar;

                af = effective_acceleration;
                ep = new_low;
                sar = sar + af * (ep - sar);
                if sar < prev_high {
                    sar = prev_high;
                }
                if sar < new_high {
                    sar = new_high;
                }
            } else {
                output[output_idx] = sar;
                if new_high > ep {
                    ep = new_high;
                    af += effective_acceleration;
                    if af > maximum {
                        af = maximum;
                    }
                }
                sar = sar + af * (ep - sar);
                if sar > prev_low {
                    sar = prev_low;
                }
                if sar > new_low {
                    sar = new_low;
                }
            }
        } else if new_high >= sar {
            is_long = true;
            sar = ep;
            if sar > prev_low {
                sar = prev_low;
            }
            if sar > new_low {
                sar = new_low;
            }
            output[output_idx] = sar;

            af = effective_acceleration;
            ep = new_high;
            sar = sar + af * (ep - sar);
            if sar > prev_low {
                sar = prev_low;
            }
            if sar > new_low {
                sar = new_low;
            }
        } else {
            output[output_idx] = sar;
            if new_low < ep {
                ep = new_low;
                af += effective_acceleration;
                if af > maximum {
                    af = maximum;
                }
            }
            sar = sar + af * (ep - sar);
            if sar < prev_high {
                sar = prev_high;
            }
            if sar < new_high {
                sar = new_high;
            }
        }
    }

    Ok(output)
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
}
