//! Fused Money Flow Index kernel for the Architecture v3 hot path.
//!
//! MFI only needs the previous typical price and period-sized money-flow rings.
//! Computing TP inline removes the legacy full-length typical-price allocation;
//! the hot period-14 path keeps both rings on the stack like TA-Lib.

use crate::error::{Result, TaError};
use ndarray::Array1;

#[inline(always)]
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
    if period < 2 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "between 2 and 100000".to_string(),
        });
    }
    if high.len() < period + 1 {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: period + 1,
        });
    }

    let mut output = Array1::<f64>::zeros(close.len());
    mfi_into(
        high,
        low,
        close,
        volume,
        period,
        output.as_slice_mut().unwrap(),
    )?;
    Ok(output)
}

/// Compute MFI directly into a caller-owned output buffer.
///
/// This is the binding hot path: it preserves the fused signed-flow ring and
/// the established arithmetic while avoiding the temporary `Array1` and raw
/// vector conversion at the FFI boundary.
pub fn mfi_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if period < 2 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "between 2 and 100000".to_string(),
        });
    }
    if high.len() < period + 1 {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: period + 1,
        });
    }
    if output.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as close".to_string(),
        });
    }

    output[..period].fill(f64::NAN);
    if period == 14 {
        return mfi_period14_into(high, low, close, volume, output);
    }

    let mut positive = vec![0.0_f64; period];
    let mut negative = vec![0.0_f64; period];
    mfi_kernel_into(
        high,
        low,
        close,
        volume,
        period,
        &mut positive,
        &mut negative,
        output,
    );
    Ok(())
}

#[inline(always)]
fn mfi_period14_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    output: &mut [f64],
) -> Result<()> {
    let mut positive_ring = [0.0_f64; 14];
    let mut negative_ring = [0.0_f64; 14];
    let mut pos_sum = 0.0;
    let mut neg_sum = 0.0;
    let mut ring_idx = 0usize;

    unsafe {
        let high_ptr = high.as_ptr();
        let low_ptr = low.as_ptr();
        let close_ptr = close.as_ptr();
        let volume_ptr = volume.as_ptr();
        let output_ptr = output.as_mut_ptr();
        let mut prev_tp = typical_price(*high_ptr, *low_ptr, *close_ptr);
        for i in 1..close.len() {
            let tp = typical_price(*high_ptr.add(i), *low_ptr.add(i), *close_ptr.add(i));
            let money_flow = tp * *volume_ptr.add(i);
            let is_positive = tp > prev_tp;
            prev_tp = tp;

            let positive = if is_positive { money_flow } else { 0.0 };
            let negative = if is_positive { 0.0 } else { money_flow };
            pos_sum += positive - *positive_ring.as_ptr().add(ring_idx);
            neg_sum += negative - *negative_ring.as_ptr().add(ring_idx);
            *positive_ring.as_mut_ptr().add(ring_idx) = positive;
            *negative_ring.as_mut_ptr().add(ring_idx) = negative;
            ring_idx += 1;
            if ring_idx == 14 {
                ring_idx = 0;
            }
            if i >= 14 {
                *output_ptr.add(i) = mfi_value(pos_sum, neg_sum);
            }
        }
    }
    Ok(())
}

#[inline(always)]
fn mfi_value(pos_sum: f64, neg_sum: f64) -> f64 {
    if neg_sum.abs() > 1.0e-15 {
        100.0 * pos_sum / (pos_sum + neg_sum)
    } else {
        100.0
    }
}

#[inline(always)]
fn mfi_kernel_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
    positive_ring: &mut [f64],
    negative_ring: &mut [f64],
    output: &mut [f64],
) {
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
        for i in 1..close.len() {
            let tp = typical_price(*high_ptr.add(i), *low_ptr.add(i), *close_ptr.add(i));
            let money_flow = tp * *volume_ptr.add(i);
            let positive = if tp > prev_tp { money_flow } else { 0.0 };
            let negative = if tp > prev_tp { 0.0 } else { money_flow };
            prev_tp = tp;

            pos_sum += positive - *positive_ring.as_ptr().add(ring_idx);
            neg_sum += negative - *negative_ring.as_ptr().add(ring_idx);
            *positive_ring.as_mut_ptr().add(ring_idx) = positive;
            *negative_ring.as_mut_ptr().add(ring_idx) = negative;

            ring_idx += 1;
            if ring_idx == period {
                ring_idx = 0;
            }
            if i >= period {
                *output_ptr.add(i) = mfi_value(pos_sum, neg_sum);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(
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
                output[i] = if neg_sum.abs() > 1.0e-15 {
                    100.0 * pos_sum / (pos_sum + neg_sum)
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
        let expected = reference(&high, &low, &close, &volume, period);
        let actual = mfi(&high, &low, &close, &volume, period).unwrap();

        for (lhs, rhs) in actual.iter().zip(expected.iter()) {
            if rhs.is_nan() {
                assert!(lhs.is_nan());
            } else {
                assert!((lhs - rhs).abs() <= 1e-12, "{lhs} != {rhs}");
            }
        }
    }

    #[test]
    fn into_matches_allocating_kernel() {
        let high = [10.0, 12.0, 11.0, 13.0, 13.0, 12.5, 14.0, 13.5, 15.0];
        let low = [9.0, 10.0, 9.5, 11.0, 11.0, 10.5, 12.0, 11.5, 13.0];
        let close = [9.5, 11.0, 10.0, 12.0, 12.0, 11.0, 13.0, 12.0, 14.0];
        let volume = [
            100.0, 120.0, 130.0, 125.0, 140.0, 135.0, 150.0, 145.0, 160.0,
        ];
        let period = 3;
        let expected = mfi(&high, &low, &close, &volume, period).unwrap();
        let mut actual = vec![0.0; close.len()];
        mfi_into(&high, &low, &close, &volume, period, &mut actual).unwrap();

        for (lhs, rhs) in actual.iter().zip(expected.iter()) {
            if rhs.is_nan() {
                assert!(lhs.is_nan());
            } else {
                assert!((lhs - rhs).abs() <= 1e-12, "{lhs} != {rhs}");
            }
        }
    }
}
