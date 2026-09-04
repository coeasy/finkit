//! Fused Money Flow Index kernel for the Architecture v3 hot path.
//!
//! The legacy implementation materialised a full typical-price array before
//! running the MFI recurrence.  MFI only needs the previous typical price and a
//! `period`-sized positive/negative money-flow ring, so computing TP inline
//! removes one full-length allocation and memory pass without changing the
//! floating-point operation order used by the public implementation.

use crate::error::{Result, TaError};
use ndarray::Array1;

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
    let mut output = vec![f64::NAN; len];
    let mut pos_ring = vec![0.0_f64; period];
    let mut neg_ring = vec![0.0_f64; period];
    let mut pos_sum = 0.0;
    let mut neg_sum = 0.0;
    let mut ring_idx = 0usize;
    let mut prev_tp = typical_price(high[0], low[0], close[0]);

    unsafe {
        let high_ptr = high.as_ptr();
        let low_ptr = low.as_ptr();
        let close_ptr = close.as_ptr();
        let volume_ptr = volume.as_ptr();
        let output_ptr = output.as_mut_ptr();
        let pos_ptr = pos_ring.as_mut_ptr();
        let neg_ptr = neg_ring.as_mut_ptr();

        for i in 1..len {
            let tp = typical_price(*high_ptr.add(i), *low_ptr.add(i), *close_ptr.add(i));
            let money_flow = tp * *volume_ptr.add(i);
            let (positive, negative) = if tp > prev_tp {
                (money_flow, 0.0)
            } else {
                (0.0, money_flow)
            };
            prev_tp = tp;

            let old_positive = *pos_ptr.add(ring_idx);
            let old_negative = *neg_ptr.add(ring_idx);
            pos_sum += positive - old_positive;
            neg_sum += negative - old_negative;
            *pos_ptr.add(ring_idx) = positive;
            *neg_ptr.add(ring_idx) = negative;
            ring_idx += 1;
            if ring_idx == period {
                ring_idx = 0;
            }

            if i >= period {
                *output_ptr.add(i) = if neg_sum.abs() > 1e-15 {
                    100.0 - 100.0 / (1.0 + pos_sum / neg_sum)
                } else {
                    100.0
                };
            }
        }
    }

    Ok(Array1::from_vec(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warmup_ends_at_period() {
        let high = [10.0, 11.0, 12.0, 11.0, 13.0, 14.0];
        let low = [9.0, 10.0, 11.0, 10.0, 12.0, 13.0];
        let close = [9.5, 10.5, 11.5, 10.5, 12.5, 13.5];
        let volume = [100.0, 110.0, 120.0, 130.0, 140.0, 150.0];
        let output = mfi(&high, &low, &close, &volume, 3).unwrap();
        assert!(output[..3].iter().all(|value| value.is_nan()));
        assert!(output[3].is_finite());
    }
}
