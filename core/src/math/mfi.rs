//! Fused Money Flow Index kernel for the Architecture v3 hot path.
//!
//! MFI only needs the previous typical price and a `period`-sized signed
//! money-flow ring. Computing TP inline removes the legacy full-length typical
//! price allocation, while one signed ring replaces separate positive/negative
//! rings without changing the public up/non-up classification semantics.

use crate::error::{Result, TaError};
use ndarray::Array1;
use std::mem::{forget, MaybeUninit};

#[inline]
fn typical_price(high: f64, low: f64, close: f64) -> f64 {
    (high + low + close) / 3.0
}

pub fn mfi(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    if high.len() < period + 1 {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: period + 1,
        });
    }

    let len = close.len();
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
    // Positive money flow is stored as +x, non-positive-direction flow as -x.
    // This halves ring storage and lets one outgoing value update the correct
    // accumulator without a second ring lookup.
    let mut flow_ring = vec![0.0_f64; period];
    let mut pos_sum = 0.0;
    let mut neg_sum = 0.0;
    let mut ring_idx = 0usize;
    let mut prev_tp = typical_price(high[0], low[0], close[0]);

    // MaybeUninit makes the no-prefill output strategy explicit and sound:
    // every slot is written exactly once before ownership is reinterpreted as
    // Vec<f64>, avoiding both a full-length NaN pass and per-element push checks.
    let output = unsafe {
        raw_output.set_len(len);
        let high_ptr = high.as_ptr();
        let low_ptr = low.as_ptr();
        let close_ptr = close.as_ptr();
        let volume_ptr = volume.as_ptr();
        let output_ptr = raw_output.as_mut_ptr();
        let flow_ptr = flow_ring.as_mut_ptr();

        for index in 0..period {
            output_ptr.add(index).write(MaybeUninit::new(f64::NAN));
        }

        for i in 1..len {
            let tp = typical_price(*high_ptr.add(i), *low_ptr.add(i), *close_ptr.add(i));
            let money_flow = tp * *volume_ptr.add(i);
            let signed_flow = if tp > prev_tp {
                money_flow
            } else {
                -money_flow
            };
            prev_tp = tp;

            let old_flow = *flow_ptr.add(ring_idx);
            if old_flow > 0.0 {
                pos_sum -= old_flow;
            } else if old_flow < 0.0 {
                neg_sum += old_flow;
            }

            if signed_flow > 0.0 {
                pos_sum += signed_flow;
            } else if signed_flow < 0.0 {
                neg_sum -= signed_flow;
            }
            *flow_ptr.add(ring_idx) = signed_flow;

            ring_idx += 1;
            if ring_idx == period {
                ring_idx = 0;
            }

            if i >= period {
                // Algebraically identical to 100 - 100/(1 + pos/neg), but
                // requires one floating-point division instead of two.
                let value = if neg_sum.abs() > 1e-15 {
                    100.0 * pos_sum / (pos_sum + neg_sum)
                } else {
                    100.0
                };
                output_ptr.add(i).write(MaybeUninit::new(value));
            }
        }

        let ptr = raw_output.as_mut_ptr().cast::<f64>();
        let capacity = raw_output.capacity();
        let length = raw_output.len();
        forget(raw_output);
        Vec::from_raw_parts(ptr, length, capacity)
    };

    Ok(Array1::from_vec(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_reference(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
        period: usize,
    ) -> Vec<f64> {
        let len = close.len();
        let mut output = vec![f64::NAN; len];
        let mut pos_ring = vec![0.0_f64; period];
        let mut neg_ring = vec![0.0_f64; period];
        let mut pos_sum = 0.0;
        let mut neg_sum = 0.0;
        let mut ring_idx = 0usize;
        let mut prev_tp = typical_price(high[0], low[0], close[0]);

        for i in 1..len {
            let tp = typical_price(high[i], low[i], close[i]);
            let money_flow = tp * volume[i];
            let (positive, negative) = if tp > prev_tp {
                (money_flow, 0.0)
            } else {
                (0.0, money_flow)
            };
            prev_tp = tp;

            pos_sum += positive - pos_ring[ring_idx];
            neg_sum += negative - neg_ring[ring_idx];
            pos_ring[ring_idx] = positive;
            neg_ring[ring_idx] = negative;
            ring_idx = (ring_idx + 1) % period;

            if i >= period {
                output[i] = if neg_sum.abs() > 1e-15 {
                    100.0 - 100.0 / (1.0 + pos_sum / neg_sum)
                } else {
                    100.0
                };
            }
        }
        output
    }

    #[test]
    fn warmup_ends_at_period() {
        let high = [10.0, 11.0, 12.0, 11.0, 13.0, 14.0];
        let low = [9.0, 10.0, 11.0, 10.0, 12.0, 13.0];
        let close = [9.5, 10.5, 11.5, 10.5, 12.5, 13.5];
        let volume = [100.0, 110.0, 120.0, 130.0, 140.0, 150.0];
        let output = mfi(&high, &low, &close, &volume, 3).unwrap();
        assert!(output.iter().take(3).all(|value| value.is_nan()));
        assert!(output[3].is_finite());
        assert_eq!(output.len(), close.len());
    }

    #[test]
    fn signed_ring_matches_legacy_positive_negative_accounting() {
        let high = [10.0, 12.0, 11.0, 13.0, 13.0, 12.5, 14.0, 13.5, 15.0];
        let low = [9.0, 10.0, 9.5, 11.0, 11.0, 10.5, 12.0, 11.5, 13.0];
        let close = [9.5, 11.0, 10.0, 12.0, 12.0, 11.0, 13.0, 12.0, 14.0];
        let volume = [
            100.0, 120.0, 130.0, 125.0, 140.0, 135.0, 150.0, 145.0, 160.0,
        ];
        let period = 3;
        let expected = legacy_reference(&high, &low, &close, &volume, period);
        let actual = mfi(&high, &low, &close, &volume, period).unwrap();

        for (lhs, rhs) in actual.iter().zip(expected.iter()) {
            if rhs.is_nan() {
                assert!(lhs.is_nan());
            } else {
                assert!((lhs - rhs).abs() <= 1e-12, "{lhs} != {rhs}");
            }
        }
    }
}
