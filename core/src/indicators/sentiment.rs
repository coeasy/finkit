use crate::error::{Result, TaError};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

/// Calculate VIX-like volatility index based on price volatility
///
/// Approximates the VIX by calculating the annualized standard deviation
/// of logarithmic returns over a rolling window.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Lookback period for rolling window
///
/// # Returns
/// Array of VIX-like volatility values (annualized percentage)
///
/// # Formula
/// Volatility = std(log_returns) * sqrt(252) * 100
pub fn vix_like_volatility(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(close.len(), period + 1)?;

    let len = close.len();
    let mut output = init_output(len);

    for i in period..len {
        let mut log_returns = Vec::with_capacity(period);
        for j in (i + 1 - period)..=i {
            if close[j - 1].abs() > 1e-15 && close[j] > 0.0 {
                log_returns.push((close[j] / close[j - 1]).ln());
            }
        }

        if log_returns.len() >= 2 {
            let mean: f64 = log_returns.iter().sum::<f64>() / log_returns.len() as f64;
            let variance: f64 = log_returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>()
                / (log_returns.len() - 1) as f64;
            let std_dev = variance.sqrt();
            output[i] = std_dev * (252.0_f64).sqrt() * 100.0;
        }
    }

    Ok(output)
}

/// Calculate Fear & Greed Index
///
/// A composite sentiment indicator that combines volatility, momentum, and market breadth
/// into a single index ranging from 0 (extreme fear) to 100 (extreme greed).
///
/// # Arguments
/// * `volatility` - Volatility data series (normalized, higher = more volatile)
/// * `momentum` - Momentum data series (normalized, higher = stronger uptrend)
/// * `breadth` - Market breadth data series (normalized, higher = broader participation)
///
/// # Returns
/// Array of Fear & Greed Index values (0-100)
///
/// # Interpretation
/// * < 25: Extreme Fear
/// * 25-45: Fear
/// * 45-55: Neutral
/// * 55-75: Greed
/// * > 75: Extreme Greed
///
/// # Formula
/// FGI = (volatility + momentum + breadth) / 3.0 * 100.0
pub fn fear_greed_index(
    volatility: &[f64],
    momentum: &[f64],
    breadth: &[f64],
) -> Result<Array1<f64>> {
    if volatility.len() != momentum.len() || volatility.len() != breadth.len() {
        return Err(TaError::InvalidParameter {
            name: "volatility, momentum, breadth".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(volatility.len(), 1)?;

    let len = volatility.len();
    let mut output = init_output(len);

    for i in 0..len {
        if !volatility[i].is_nan() && !momentum[i].is_nan() && !breadth[i].is_nan() {
            let raw_score = (volatility[i] + momentum[i] + breadth[i]) / 3.0;
            output[i] = raw_score.clamp(0.0, 1.0) * 100.0;
        }
    }

    Ok(output)
}

/// Calculate Put/Call Ratio
///
/// A sentiment indicator that compares the volume of put options to call options.
/// Values above 1 indicate bearish sentiment (more puts), while values below 1
/// indicate bullish sentiment (more calls).
///
/// # Arguments
/// * `put_volume` - Put option trading volume
/// * `call_volume` - Call option trading volume
///
/// # Returns
/// Array of Put/Call Ratio values
///
/// # Interpretation
/// * > 1: Bearish sentiment (more puts than calls)
/// * < 1: Bullish sentiment (more calls than puts)
/// * = 1: Neutral sentiment
///
/// # Formula
/// PCR = put_volume / call_volume
pub fn put_call_ratio(put_volume: &[f64], call_volume: &[f64]) -> Result<Array1<f64>> {
    if put_volume.len() != call_volume.len() {
        return Err(TaError::InvalidParameter {
            name: "put_volume, call_volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(put_volume.len(), 1)?;

    let len = put_volume.len();
    let mut output = init_output(len);

    for i in 0..len {
        if call_volume[i].abs() > 1e-15 {
            output[i] = put_volume[i] / call_volume[i];
        }
    }

    Ok(output)
}

/// Calculate Volatility Index using Parkinson Estimator
///
/// A volatility indicator that uses the Parkinson estimator, which incorporates
/// high and low prices for a more efficient volatility estimate than close-to-close methods.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Lookback period for rolling window
///
/// # Returns
/// Array of Parkinson volatility values (annualized percentage)
///
/// # Formula
/// Parkinson Vol = sqrt( (1 / (4 * ln(2))) * mean( (ln(H/L))^2 ) ) * sqrt(252) * 100
pub fn volatility_index(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period)?;

    let len = close.len();
    let mut output = init_output(len);

    let factor = 1.0 / (4.0 * 2.0_f64.ln());

    for i in period - 1..len {
        let mut sum_sq = 0.0;
        let mut count = 0;

        for j in (i + 1 - period)..=i {
            if low[j].abs() > 1e-15 && high[j] > 0.0 && high[j] > low[j] {
                let hl_ratio = (high[j] / low[j]).ln();
                sum_sq += hl_ratio.powi(2);
                count += 1;
            }
        }

        if count > 0 {
            let parkinson_vol = (factor * sum_sq / count as f64).sqrt();
            output[i] = parkinson_vol * (252.0_f64).sqrt() * 100.0;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_vix_like_volatility_basic() {
        let close: Vec<f64> = (100..=120).map(|x| x as f64).collect();
        let high: Vec<f64> = close.iter().map(|&x| x + 1.0).collect();
        let low: Vec<f64> = close.iter().map(|&x| x - 1.0).collect();
        let result = vix_like_volatility(&high, &low, &close, 10).unwrap();
        assert!(!result[10].is_nan());
        assert!(result[10] > 0.0);
    }

    #[test]
    fn test_vix_like_volatility_high_volatility() {
        let close = vec![
            100.0, 90.0, 110.0, 85.0, 115.0, 80.0, 120.0, 75.0, 125.0, 70.0, 130.0,
        ];
        let high: Vec<f64> = close.iter().map(|&x| x + 5.0).collect();
        let low: Vec<f64> = close.iter().map(|&x| x - 5.0).collect();
        let result = vix_like_volatility(&high, &low, &close, 5).unwrap();
        assert!(!result[5].is_nan());
        assert!(result[5] > 50.0);
    }

    #[test]
    fn test_fear_greed_index_extreme_fear() {
        let volatility = vec![0.1, 0.1, 0.1];
        let momentum = vec![0.05, 0.05, 0.05];
        let breadth = vec![0.1, 0.1, 0.1];
        let result = fear_greed_index(&volatility, &momentum, &breadth).unwrap();
        assert!(result[0] < 25.0);
    }

    #[test]
    fn test_fear_greed_index_extreme_greed() {
        let volatility = vec![0.9, 0.9, 0.9];
        let momentum = vec![0.85, 0.85, 0.85];
        let breadth = vec![0.9, 0.9, 0.9];
        let result = fear_greed_index(&volatility, &momentum, &breadth).unwrap();
        assert!(result[0] > 75.0);
    }

    #[test]
    fn test_fear_greed_index_neutral() {
        let volatility = vec![0.5, 0.5, 0.5];
        let momentum = vec![0.5, 0.5, 0.5];
        let breadth = vec![0.5, 0.5, 0.5];
        let result = fear_greed_index(&volatility, &momentum, &breadth).unwrap();
        assert_relative_eq!(result[0], 50.0, epsilon = 1e-10);
    }

    #[test]
    fn test_put_call_ratio_bearish() {
        let put_volume = vec![1500.0, 1600.0, 1700.0];
        let call_volume = vec![1000.0, 1100.0, 1200.0];
        let result = put_call_ratio(&put_volume, &call_volume).unwrap();
        assert!(result[0] > 1.0);
        assert_relative_eq!(result[0], 1.5, epsilon = 1e-10);
    }

    #[test]
    fn test_put_call_ratio_bullish() {
        let put_volume = vec![500.0, 600.0, 700.0];
        let call_volume = vec![1000.0, 1200.0, 1400.0];
        let result = put_call_ratio(&put_volume, &call_volume).unwrap();
        assert!(result[0] < 1.0);
        assert_relative_eq!(result[0], 0.5, epsilon = 1e-10);
    }

    #[test]
    fn test_volatility_index_parkinson_basic() {
        let high = vec![102.0, 104.0, 106.0, 108.0, 110.0, 112.0, 114.0];
        let low = vec![98.0, 96.0, 94.0, 92.0, 90.0, 88.0, 86.0];
        let close = vec![100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0];
        let result = volatility_index(&high, &low, &close, 5).unwrap();
        assert!(!result[4].is_nan());
        assert!(result[4] > 0.0);
    }

    #[test]
    fn test_vix_like_volatility_invalid_input() {
        let high = vec![100.0, 101.0];
        let low = vec![99.0];
        let close = vec![100.0, 100.5];
        let result = vix_like_volatility(&high, &low, &close, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_fear_greed_index_clamping() {
        let volatility = vec![2.0, 2.0, 2.0];
        let momentum = vec![1.5, 1.5, 1.5];
        let breadth = vec![3.0, 3.0, 3.0];
        let result = fear_greed_index(&volatility, &momentum, &breadth).unwrap();
        assert!(result[0] <= 100.0);
        assert_relative_eq!(result[0], 100.0, epsilon = 1e-10);
    }

    #[test]
    fn test_put_call_ratio_neutral() {
        let put_volume = vec![1000.0, 1100.0, 1200.0];
        let call_volume = vec![1000.0, 1100.0, 1200.0];
        let result = put_call_ratio(&put_volume, &call_volume).unwrap();
        assert_relative_eq!(result[0], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_volatility_index_parkinson_low_volatility() {
        let high = vec![100.1, 100.2, 100.1, 100.2, 100.1];
        let low = vec![99.9, 99.8, 99.9, 99.8, 99.9];
        let close = vec![100.0, 100.0, 100.0, 100.0, 100.0];
        let result = volatility_index(&high, &low, &close, 3).unwrap();
        assert!(!result[2].is_nan());
        assert!(result[2] < 20.0);
    }
}
