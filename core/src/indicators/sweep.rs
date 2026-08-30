use crate::error::{Result, TaError};
use crate::utils::validate_input;

/// Compute SMA for multiple periods in a single data pass.
///
/// For each period `p` in `periods`, the result contains a `Vec<f64>` of length `data.len()`,
/// with the first `p-1` values set to NaN (warm-up phase).
///
/// Uses a single forward scan over the input data with shared incremental sums
/// for all periods, improving cache locality compared to per-period iteration.
pub fn sma_sweep(data: &[f64], periods: &[usize]) -> Result<Vec<Vec<f64>>> {
    if periods.is_empty() {
        return Ok(vec![]);
    }
    let max_period = *periods.iter().max().unwrap();
    for (i, &p) in periods.iter().enumerate() {
        if p == 0 {
            return Err(TaError::InvalidParameter {
                name: format!("periods[{i}]"),
                constraint: "greater than 0".to_string(),
            });
        }
    }
    validate_input(data.len(), max_period)?;

    let len = data.len();
    let n = periods.len();

    let mut results: Vec<Vec<f64>> = vec![vec![f64::NAN; len]; n];
    let mut sums: Vec<f64> = vec![0.0; n];
    let mut inv_periods: Vec<f64> = Vec::with_capacity(n);

    for &p in periods {
        inv_periods.push(1.0 / p as f64);
    }

    // Single forward scan over data: outer loop over data points, inner over periods.
    // This keeps the data array in cache and updates all period running sums together.
    for i in 0..len {
        for j in 0..n {
            let p = periods[j];
            if i < p {
                // Accumulate initial window sum
                sums[j] += data[i];
            }
            if i + 1 == p {
                // First valid SMA value
                results[j][i] = sums[j] * inv_periods[j];
            } else if i + 1 > p {
                // Slide window: add new, subtract old
                sums[j] += data[i] - data[i - p];
                results[j][i] = sums[j] * inv_periods[j];
            }
        }
    }

    Ok(results)
}

/// Compute EMA for multiple periods in a single data pass.
///
/// For each period `p` in `periods`, the result contains a `Vec<f64>` of length `data.len()`,
/// with the first `p-1` values set to NaN.
///
/// Uses data-point-outer-loop order for better cache locality: the input data array
/// is scanned sequentially, and all period EMAs are updated for each data point.
pub fn ema_sweep(data: &[f64], periods: &[usize]) -> Result<Vec<Vec<f64>>> {
    if periods.is_empty() {
        return Ok(vec![]);
    }
    let max_period = *periods.iter().max().unwrap();
    for (i, &p) in periods.iter().enumerate() {
        if p == 0 {
            return Err(TaError::InvalidParameter {
                name: format!("periods[{}]", i),
                constraint: "greater than 0".to_string(),
            });
        }
    }
    validate_input(data.len(), max_period)?;

    let len = data.len();
    let n = periods.len();

    let mut results: Vec<Vec<f64>> = Vec::with_capacity(n);
    let mut multipliers: Vec<f64> = Vec::with_capacity(n);
    let mut one_minus_k: Vec<f64> = Vec::with_capacity(n);
    let mut prev_ema: Vec<f64> = Vec::with_capacity(n);
    let mut initialized: Vec<bool> = vec![false; n];

    // Pre-compute initial SMA for each period
    let mut initial_sums: Vec<f64> = vec![0.0; n];
    for &p in periods {
        let out = vec![f64::NAN; len];
        let k = 2.0 / (p as f64 + 1.0);
        multipliers.push(k);
        one_minus_k.push(1.0 - k);
        prev_ema.push(0.0);
        results.push(out);
    }

    // Single forward scan: outer loop over data points, inner over periods.
    // This keeps data[i] in cache and updates all period EMAs together.
    for i in 0..len {
        for j in 0..n {
            let p = periods[j];
            if i < p {
                // Accumulate initial SMA window
                initial_sums[j] += data[i];
            }
            if i + 1 == p {
                // Initialize EMA with SMA of first p values
                let initial_sma = initial_sums[j] / p as f64;
                results[j][i] = initial_sma;
                prev_ema[j] = initial_sma;
                initialized[j] = true;
            } else if initialized[j] {
                // Standard EMA update
                let new_ema = data[i] * multipliers[j] + prev_ema[j] * one_minus_k[j];
                results[j][i] = new_ema;
                prev_ema[j] = new_ema;
            }
        }
    }

    Ok(results)
}

/// Compute RSI for multiple periods in a single data pass.
///
/// For each period `p` in `periods`, the result contains a `Vec<f64>` of length `data.len()`,
/// with the first `p` values set to NaN. RSI values are in the range [0, 100].
pub fn rsi_sweep(data: &[f64], periods: &[usize]) -> Result<Vec<Vec<f64>>> {
    if periods.is_empty() {
        return Ok(vec![]);
    }
    let max_period = *periods.iter().max().unwrap();
    for (i, &p) in periods.iter().enumerate() {
        if p == 0 {
            return Err(TaError::InvalidParameter {
                name: format!("periods[{}]", i),
                constraint: "greater than 0".to_string(),
            });
        }
    }
    validate_input(data.len(), max_period + 1)?;

    let len = data.len();
    let n = periods.len();

    let mut results: Vec<Vec<f64>> = vec![vec![f64::NAN; len]; n];
    let mut avg_gains: Vec<f64> = vec![0.0; n];
    let mut avg_losses: Vec<f64> = vec![0.0; n];
    let mut inv_periods: Vec<f64> = Vec::with_capacity(n);
    let mut period_minus_1_ratio: Vec<f64> = Vec::with_capacity(n);
    let mut initialized: Vec<bool> = vec![false; n];

    for &p in periods {
        inv_periods.push(1.0 / p as f64);
        period_minus_1_ratio.push((p as f64 - 1.0) / p as f64);
    }

    // Precompute changes
    let mut changes = vec![0.0f64; len];
    for i in 1..len {
        changes[i] = data[i] - data[i - 1];
    }

    for i in 1..len {
        let change = changes[i];
        let gain = change.max(0.0);
        let loss = (-change).max(0.0);

        for j in 0..n {
            let p = periods[j];
            if !initialized[j] {
                avg_gains[j] += gain;
                avg_losses[j] += loss;

                if i == p {
                    avg_gains[j] *= inv_periods[j];
                    avg_losses[j] *= inv_periods[j];

                    if avg_losses[j].abs() < 1e-15 {
                        results[j][i] = 100.0;
                    } else {
                        results[j][i] =
                            100.0 - 100.0 / (1.0 + avg_gains[j] / avg_losses[j]);
                    }
                    initialized[j] = true;
                }
            } else {
                avg_gains[j] = avg_gains[j] * period_minus_1_ratio[j] + gain * inv_periods[j];
                avg_losses[j] = avg_losses[j] * period_minus_1_ratio[j] + loss * inv_periods[j];

                if avg_losses[j].abs() < 1e-15 {
                    results[j][i] = 100.0;
                } else {
                    results[j][i] = 100.0 - 100.0 / (1.0 + avg_gains[j] / avg_losses[j]);
                }
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::momentum::rsi;
    use crate::math::moving_avg;
    use proptest::prelude::*;

    fn generate_test_data(n: usize) -> Vec<f64> {
        let mut data = Vec::with_capacity(n);
        let mut price = 100.0;
        for i in 0..n {
            price += ((i as f64 * 0.1).sin() * 2.0) + 0.01;
            data.push(price);
        }
        data
    }

    #[test]
    fn test_sma_sweep_basic() {
        let data = generate_test_data(100);
        let periods = [5, 10, 20, 50];
        let results = sma_sweep(&data, &periods).unwrap();
        assert_eq!(results.len(), 4);

        for (j, &p) in periods.iter().enumerate() {
            assert_eq!(results[j].len(), data.len());
            for (i, val) in results[j].iter().enumerate().take(p - 1) {
                assert!(val.is_nan(), "period={}, i={} should be NaN", p, i);
            }
            assert!(!results[j][p - 1].is_nan(), "period={}, i={} should not be NaN", p, p - 1);
        }
    }

    #[test]
    fn test_sma_sweep_matches_individual() {
        let data = generate_test_data(200);
        let periods = [5, 10, 20, 50];
        let results = sma_sweep(&data, &periods).unwrap();

        for (j, &p) in periods.iter().enumerate() {
            let expected = moving_avg::sma(&data, p).unwrap();
            for i in 0..data.len() {
                if expected[i].is_nan() {
                    assert!(results[j][i].is_nan(), "SMA mismatch at period={}, i={}", p, i);
                } else {
                    assert!(
                        (results[j][i] - expected[i]).abs() < 1e-10,
                        "SMA mismatch at period={}, i={}: got {} expected {}",
                        p, i, results[j][i], expected[i]
                    );
                }
            }
        }
    }

    #[test]
    fn test_ema_sweep_matches_individual() {
        let data = generate_test_data(200);
        let periods = [5, 10, 20, 50];
        let results = ema_sweep(&data, &periods).unwrap();

        for (j, &p) in periods.iter().enumerate() {
            let expected = moving_avg::ema(&data, p).unwrap();
            for i in 0..data.len() {
                if expected[i].is_nan() {
                    assert!(results[j][i].is_nan(), "EMA mismatch at period={}, i={}", p, i);
                } else {
                    assert!(
                        (results[j][i] - expected[i]).abs() < 1e-10,
                        "EMA mismatch at period={}, i={}: got {} expected {}",
                        p, i, results[j][i], expected[i]
                    );
                }
            }
        }
    }

    #[test]
    fn test_rsi_sweep_matches_individual() {
        let data = generate_test_data(200);
        let periods = [5, 10, 14, 20];
        let results = rsi_sweep(&data, &periods).unwrap();

        for (j, &p) in periods.iter().enumerate() {
            let expected = rsi(&data, p).unwrap();
            for i in 0..data.len() {
                if expected[i].is_nan() {
                    assert!(results[j][i].is_nan(), "RSI mismatch at period={}, i={}", p, i);
                } else {
                    assert!(
                        (results[j][i] - expected[i]).abs() < 1e-10,
                        "RSI mismatch at period={}, i={}: got {} expected {}",
                        p, i, results[j][i], expected[i]
                    );
                }
            }
        }
    }

    #[test]
    fn test_sweep_empty_periods() {
        let data = generate_test_data(100);
        assert_eq!(sma_sweep(&data, &[]).unwrap().len(), 0);
        assert_eq!(ema_sweep(&data, &[]).unwrap().len(), 0);
        assert_eq!(rsi_sweep(&data, &[]).unwrap().len(), 0);
    }

    #[test]
    fn test_sweep_invalid_period() {
        let data = generate_test_data(100);
        assert!(sma_sweep(&data, &[0]).is_err());
        assert!(ema_sweep(&data, &[0]).is_err());
        assert!(rsi_sweep(&data, &[0]).is_err());
    }

    #[test]
    fn test_sweep_single_period() {
        let data = generate_test_data(100);
        let results = sma_sweep(&data, &[10]).unwrap();
        let expected = moving_avg::sma(&data, 10).unwrap();
        for i in 0..data.len() {
            if expected[i].is_nan() {
                assert!(results[0][i].is_nan());
            } else {
                assert!((results[0][i] - expected[i]).abs() < 1e-10);
            }
        }
    }

    proptest! {
        #[test]
        fn prop_sma_sweep_matches_individual(
            len in 60usize..200,
            seed in 0u64..1000,
        ) {
            let mut data = Vec::with_capacity(len);
            let mut price = 100.0;
            for i in 0..len {
                price += ((i as f64 * 0.1 + seed as f64).sin() * 2.0) + 0.01;
                data.push(price);
            }
            let periods = [5, 10, 20, 50];
            let results = sma_sweep(&data, &periods).unwrap();
            for (j, &p) in periods.iter().enumerate() {
                let expected = moving_avg::sma(&data, p).unwrap();
                for i in 0..data.len() {
                    if expected[i].is_nan() {
                        prop_assert!(results[j][i].is_nan());
                    } else {
                        prop_assert!(
                            (results[j][i] - expected[i]).abs() < 1e-10,
                            "SMA mismatch at period={}, i={}: got {} expected {}",
                            p, i, results[j][i], expected[i]
                        );
                    }
                }
            }
        }
    }
}
