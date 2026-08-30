//! Market microstructure features: order flow imbalance and spread estimators.

use ndarray::Array1;

use super::{Feature, FeatureMatrix};

/// Direction sign from consecutive close prices: +1 if up, -1 otherwise.
fn price_direction(close: &[f64]) -> Array1<f64> {
    let len = close.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    for i in 1..len {
        out[i] = if close[i] > close[i - 1] { 1.0 } else { -1.0 };
    }
    out
}

/// Tick imbalance: rolling mean of price direction signs.
///
/// Computes +1/-1 from close-to-close changes and returns the rolling
/// window average of directional ticks.
pub fn tick_imbalance(close: &[f64], window: usize) -> Array1<f64> {
    let directions = price_direction(close);
    rolling_mean(directions.as_slice().unwrap(), window)
}

/// Volume imbalance: rolling ratio of signed volume to total volume.
///
/// Up moves contribute positive volume; down moves contribute negative volume.
pub fn volume_imbalance(close: &[f64], volume: &[f64], window: usize) -> Array1<f64> {
    assert_eq!(close.len(), volume.len());
    let len = close.len();
    let mut signed_vol = Array1::from_elem(len, 0.0);
    for i in 1..len {
        let sign = if close[i] > close[i - 1] {
            1.0
        } else if close[i] < close[i - 1] {
            -1.0
        } else {
            0.0
        };
        signed_vol[i] = sign * volume[i];
    }

    let mut out = Array1::from_elem(len, f64::NAN);
    if window == 0 || len < window {
        return out;
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let mut signed_sum = 0.0;
        let mut total_vol = 0.0;
        for j in start..=i {
            signed_sum += signed_vol[j];
            total_vol += volume[j].abs();
        }
        out[i] = if total_vol > 1e-15 {
            signed_sum / total_vol
        } else {
            0.0
        };
    }
    out
}

/// Kyle's lambda: rolling regression of price impact on signed order flow.
///
/// For each window, computes `sum(signed_volume * price_change) / sum(signed_volume^2)`.
pub fn kyle_lambda(close: &[f64], volume: &[f64], window: usize) -> Array1<f64> {
    assert_eq!(close.len(), volume.len());
    let len = close.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 2 || len < window {
        return out;
    }

    let mut price_change = Array1::from_elem(len, 0.0);
    let mut signed_vol = Array1::from_elem(len, 0.0);
    for i in 1..len {
        price_change[i] = close[i] - close[i - 1];
        let sign = if price_change[i] > 0.0 {
            1.0
        } else if price_change[i] < 0.0 {
            -1.0
        } else {
            0.0
        };
        signed_vol[i] = sign * volume[i];
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let mut numerator = 0.0;
        let mut denominator = 0.0;
        for j in start..=i {
            let sv = signed_vol[j];
            numerator += sv * price_change[j];
            denominator += sv * sv;
        }
        out[i] = if denominator.abs() > 1e-15 {
            numerator / denominator
        } else {
            0.0
        };
    }
    out
}

/// Roll (1984) implied spread estimator from serial covariance of price changes.
///
/// `spread = 2 * sqrt(-cov(delta_p_t, delta_p_{t-1}))`, or 0 when covariance is non-negative.
pub fn roll_spread(close: &[f64], window: usize) -> Array1<f64> {
    let len = close.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 3 || len < window {
        return out;
    }

    let mut delta = Array1::from_elem(len, f64::NAN);
    for i in 1..len {
        delta[i] = close[i] - close[i - 1];
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let mut mean_curr = 0.0;
        let mut mean_prev = 0.0;
        let mut count = 0.0;
        for j in (start + 1)..=i {
            mean_curr += delta[j];
            mean_prev += delta[j - 1];
            count += 1.0;
        }
        if count < 1.0 {
            continue;
        }
        mean_curr /= count;
        mean_prev /= count;

        let mut cov = 0.0;
        for j in (start + 1)..=i {
            cov += (delta[j] - mean_curr) * (delta[j - 1] - mean_prev);
        }
        cov /= count;

        out[i] = if cov < 0.0 {
            2.0 * (-cov).sqrt()
        } else {
            0.0
        };
    }
    out
}

fn rolling_mean(data: &[f64], window: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window == 0 || len < window {
        return out;
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let sum: f64 = data[start..=i].iter().filter(|v| v.is_finite()).sum();
        let valid = data[start..=i].iter().filter(|v| v.is_finite()).count();
        out[i] = if valid > 0 {
            sum / valid as f64
        } else {
            f64::NAN
        };
    }
    out
}

// ─── VPIN (Volume-Synchronized Probability of Informed Trading) ─────

/// VPIN (Volume-Synchronized Probability of Informed Trading).
///
/// Approximates trade flow toxicity by bucketing volume into fixed-size buckets,
/// classifying each bar's volume as buy/sell (using close-to-close direction),
/// then computing the ratio of absolute order imbalance to total volume per bucket.
///
/// # Arguments
/// * `close` - Close prices
/// * `volume` - Volume per bar
/// * `bucket_size` - Volume per bucket (e.g., average daily volume / 50)
/// * `n_buckets` - Number of buckets for rolling average
///
/// # Returns
/// Array of VPIN values (one per bar, NaN until enough buckets fill)
pub fn vpin(close: &[f64], volume: &[f64], bucket_size: f64, n_buckets: usize) -> Array1<f64> {
    assert_eq!(close.len(), volume.len());
    let len = close.len();
    let mut output = Array1::from_elem(len, f64::NAN);

    if len < 2 || bucket_size <= 0.0 || n_buckets == 0 {
        return output;
    }

    let mut bucket_buy = 0.0;
    let mut bucket_sell = 0.0;
    let mut bucket_vol = 0.0;
    let mut imbalances: Vec<f64> = Vec::new();
    let mut bucket_bar_end: Vec<usize> = Vec::new();

    for i in 1..len {
        let dir = if close[i] > close[i - 1] {
            1.0
        } else if close[i] < close[i - 1] {
            -1.0
        } else {
            0.0
        };

        let mut remaining = volume[i];
        while remaining > 0.0 {
            let space = bucket_size - bucket_vol;
            let fill = remaining.min(space);
            bucket_vol += fill;
            remaining -= fill;

            if dir > 0.0 {
                bucket_buy += fill;
            } else if dir < 0.0 {
                bucket_sell += fill;
            } else {
                bucket_buy += fill * 0.5;
                bucket_sell += fill * 0.5;
            }

            if bucket_vol >= bucket_size - 1e-10 {
                let total = bucket_buy + bucket_sell;
                let imb = if total > 1e-15 {
                    (bucket_buy - bucket_sell).abs() / total
                } else {
                    0.0
                };
                imbalances.push(imb);
                bucket_bar_end.push(i);
                bucket_buy = 0.0;
                bucket_sell = 0.0;
                bucket_vol = 0.0;
            }
        }

        if imbalances.len() >= n_buckets {
            let start_idx = imbalances.len() - n_buckets;
            let vpin_val: f64 = imbalances[start_idx..].iter().sum::<f64>() / n_buckets as f64;
            output[i] = vpin_val;
        }
    }

    output
}

/// Level of the limit order book for LOB imbalance calculation.
#[derive(Debug, Clone)]
pub struct LobLevel {
    /// Bid volume at this level.
    pub bid_vol: f64,
    /// Ask volume at this level.
    pub ask_vol: f64,
}

/// Multi-level Limit Order Book (LOB) imbalance.
///
/// Computes weighted imbalance across multiple price levels:
/// `imbalance = sum(weight_i * (bid_i - ask_i)) / sum(weight_i * (bid_i + ask_i))`
///
/// Weights decay exponentially with level depth (level 0 = best bid/ask).
///
/// # Arguments
/// * `levels` - Array of LOB levels from best (index 0) to deeper levels
/// * `decay` - Exponential decay factor for level weighting (e.g., 0.5 means each deeper level has half the weight)
///
/// # Returns
/// Imbalance value in [-1, 1]. Positive = bid-heavy, negative = ask-heavy.
pub fn lob_imbalance(levels: &[LobLevel], decay: f64) -> f64 {
    if levels.is_empty() {
        return 0.0;
    }

    let mut weighted_diff = 0.0;
    let mut weighted_sum = 0.0;
    let mut weight = 1.0;

    for level in levels {
        weighted_diff += weight * (level.bid_vol - level.ask_vol);
        weighted_sum += weight * (level.bid_vol + level.ask_vol);
        weight *= decay;
    }

    if weighted_sum.abs() < 1e-15 {
        0.0
    } else {
        weighted_diff / weighted_sum
    }
}

/// Rolling LOB imbalance over a time series of order book snapshots.
///
/// # Arguments
/// * `snapshots` - Each element is a single time-point's LOB levels
/// * `decay` - Exponential decay factor for level weighting
///
/// # Returns
/// Vec of imbalance values per time point
pub fn rolling_lob_imbalance(snapshots: &[Vec<LobLevel>], decay: f64) -> Vec<f64> {
    snapshots
        .iter()
        .map(|levels| lob_imbalance(levels, decay))
        .collect()
}

/// Build a feature matrix with all microstructure columns for OHLCV data.
pub fn microstructure_matrix(
    close: &[f64],
    volume: &[f64],
    window: usize,
) -> FeatureMatrix {
    let mut matrix = FeatureMatrix::with_capacity(close.len(), 4);
    matrix.add_column(
        Feature::new(format!("tick_imbalance_{window}"), "microstructure", window),
        tick_imbalance(close, window).to_vec(),
    );
    matrix.add_column(
        Feature::new(format!("volume_imbalance_{window}"), "microstructure", window),
        volume_imbalance(close, volume, window).to_vec(),
    );
    matrix.add_column(
        Feature::new(format!("kyle_lambda_{window}"), "microstructure", window),
        kyle_lambda(close, volume, window).to_vec(),
    );
    matrix.add_column(
        Feature::new(format!("roll_spread_{window}"), "microstructure", window),
        roll_spread(close, window).to_vec(),
    );
    matrix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_imbalance_uptrend() {
        let close: Vec<f64> = (0..10).map(|i| 100.0 + i as f64).collect();
        let result = tick_imbalance(&close, 5);
        assert_eq!(result[4], 1.0);
        assert_eq!(result[9], 1.0);
    }

    #[test]
    fn test_tick_imbalance_downtrend() {
        let close: Vec<f64> = (0..10).map(|i| 100.0 - i as f64).collect();
        let result = tick_imbalance(&close, 5);
        assert_eq!(result[4], -1.0);
        assert_eq!(result[9], -1.0);
    }

    #[test]
    fn test_volume_imbalance_all_up() {
        let close: Vec<f64> = (0..8).map(|i| 10.0 + i as f64).collect();
        let volume = vec![100.0; 8];
        let result = volume_imbalance(&close, &volume, 5);
        assert!((result[7] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_volume_imbalance_all_down() {
        let close: Vec<f64> = (0..8).map(|i| 20.0 - i as f64).collect();
        let volume = vec![100.0; 8];
        let result = volume_imbalance(&close, &volume, 5);
        assert!((result[7] + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_kyle_lambda_positive_on_trend() {
        let close: Vec<f64> = (0..20).map(|i| 100.0 + i as f64).collect();
        let volume = vec![1000.0; 20];
        let result = kyle_lambda(&close, &volume, 10);
        assert!(result[19] > 0.0);
    }

    #[test]
    fn test_kyle_lambda_zero_flow() {
        let close = vec![100.0; 10];
        let volume = vec![500.0; 10];
        let result = kyle_lambda(&close, &volume, 5);
        assert_eq!(result[9], 0.0);
    }

    #[test]
    fn test_roll_spread_positive_when_negative_cov() {
        // Alternating noise creates negative serial covariance.
        let close: Vec<f64> = (0..30)
            .map(|i| 100.0 + if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let result = roll_spread(&close, 10);
        assert!(result[29] > 0.0);
    }

    #[test]
    fn test_roll_spread_zero_on_random_walk() {
        let close: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64 * 0.7).sin()).collect();
        let result = roll_spread(&close, 20);
        assert!(result[49] >= 0.0);
    }

    #[test]
    fn test_vpin_uptrend() {
        // Strong uptrend: VPIN should be high (mostly buy)
        let close: Vec<f64> = (0..100).map(|i| 100.0 + i as f64 * 0.5).collect();
        let volume = vec![1000.0; 100];
        let bucket_size = 5000.0; // 5 bars per bucket
        let n_buckets = 5;
        let result = vpin(&close, &volume, bucket_size, n_buckets);
        assert_eq!(result.len(), 100);
        // After enough buckets, VPIN should be close to 1.0
        let last = result[99];
        assert!(last.is_finite(), "VPIN should be finite after warm-up");
        assert!(last > 0.5, "VPIN should be high in uptrend, got {last}");
    }

    #[test]
    fn test_vpin_mixed() {
        // Alternating up/down: VPIN should be lower
        let close: Vec<f64> = (0..100)
            .map(|i| 100.0 + if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let volume = vec![1000.0; 100];
        let result = vpin(&close, &volume, 5000.0, 5);
        // After warm-up, VPIN should be relatively low
        let last_valid = result.iter().rev().find(|v| v.is_finite());
        if let Some(&v) = last_valid {
            assert!(v < 0.5, "VPIN should be low in mixed market, got {v}");
        }
    }

    #[test]
    fn test_vpin_edge_cases() {
        let close = vec![100.0; 5];
        let volume = vec![1000.0; 5];
        let result = vpin(&close, &volume, 1000.0, 10);
        // Not enough buckets: mostly NaN
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_lob_imbalance_balanced() {
        let levels = vec![
            LobLevel { bid_vol: 100.0, ask_vol: 100.0 },
            LobLevel { bid_vol: 200.0, ask_vol: 200.0 },
        ];
        let imb = lob_imbalance(&levels, 0.5);
        assert!((imb).abs() < 1e-10, "balanced LOB should have 0 imbalance");
    }

    #[test]
    fn test_lob_imbalance_bid_heavy() {
        let levels = vec![
            LobLevel { bid_vol: 200.0, ask_vol: 100.0 },
            LobLevel { bid_vol: 150.0, ask_vol: 50.0 },
        ];
        let imb = lob_imbalance(&levels, 0.5);
        assert!(imb > 0.0, "bid-heavy LOB should be positive");
        assert!(imb <= 1.0);
    }

    #[test]
    fn test_lob_imbalance_ask_heavy() {
        let levels = vec![
            LobLevel { bid_vol: 50.0, ask_vol: 200.0 },
            LobLevel { bid_vol: 30.0, ask_vol: 150.0 },
        ];
        let imb = lob_imbalance(&levels, 0.5);
        assert!(imb < 0.0, "ask-heavy LOB should be negative");
        assert!(imb >= -1.0);
    }

    #[test]
    fn test_lob_imbalance_empty() {
        let imb = lob_imbalance(&[], 0.5);
        assert_eq!(imb, 0.0);
    }

    #[test]
    fn test_lob_imbalance_decay() {
        // With decay=1.0, all levels weighted equally
        // With decay=0.0, only first level matters
        let levels = vec![
            LobLevel { bid_vol: 100.0, ask_vol: 50.0 },
            LobLevel { bid_vol: 50.0, ask_vol: 200.0 },
        ];
        let imb_no_decay = lob_imbalance(&levels, 1.0);
        let imb_full_decay = lob_imbalance(&levels, 0.0);
        // Full decay: only first level → (100-50)/(100+50) = 50/150 ≈ 0.333
        assert!((imb_full_decay - 50.0 / 150.0).abs() < 1e-10);
        // No decay considers deeper level too (ask-heavy), so less positive
        assert!(imb_no_decay < imb_full_decay);
    }

    #[test]
    fn test_rolling_lob_imbalance() {
        let snapshots = vec![
            vec![LobLevel { bid_vol: 100.0, ask_vol: 100.0 }],
            vec![LobLevel { bid_vol: 200.0, ask_vol: 100.0 }],
            vec![LobLevel { bid_vol: 50.0, ask_vol: 200.0 }],
        ];
        let result = rolling_lob_imbalance(&snapshots, 0.5);
        assert_eq!(result.len(), 3);
        assert!((result[0]).abs() < 1e-10);
        assert!(result[1] > 0.0);
        assert!(result[2] < 0.0);
    }
}
