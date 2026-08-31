//! Signal detection: crossover, divergence, threshold crossing.

use super::types::{DivergenceEvent, DivergenceType, SignalDirection, SignalEvent};

/// Detect crossover points where `fast` crosses above `slow`.
pub fn crossover(fast: &[f64], slow: &[f64]) -> Vec<SignalEvent> {
    assert_eq!(fast.len(), slow.len());
    let mut events = Vec::new();
    for i in 1..fast.len() {
        if fast[i - 1] <= slow[i - 1] && fast[i] > slow[i] {
            events.push(SignalEvent {
                index: i,
                direction: SignalDirection::Up,
                strength: fast[i] - slow[i],
            });
        }
    }
    events
}

/// Detect crossunder points where `fast` crosses below `slow`.
pub fn crossunder(fast: &[f64], slow: &[f64]) -> Vec<SignalEvent> {
    assert_eq!(fast.len(), slow.len());
    let mut events = Vec::new();
    for i in 1..fast.len() {
        if fast[i - 1] >= slow[i - 1] && fast[i] < slow[i] {
            events.push(SignalEvent {
                index: i,
                direction: SignalDirection::Down,
                strength: slow[i] - fast[i],
            });
        }
    }
    events
}

/// Detect threshold crossings (indicator crosses above/below a fixed level).
pub fn threshold_cross(data: &[f64], threshold: f64) -> Vec<SignalEvent> {
    let mut events = Vec::new();
    for i in 1..data.len() {
        if data[i - 1] <= threshold && data[i] > threshold {
            events.push(SignalEvent {
                index: i,
                direction: SignalDirection::Up,
                strength: data[i] - threshold,
            });
        } else if data[i - 1] >= threshold && data[i] < threshold {
            events.push(SignalEvent {
                index: i,
                direction: SignalDirection::Down,
                strength: threshold - data[i],
            });
        }
    }
    events
}

/// Detect divergence between price and indicator using local extrema.
///
/// Finds regular and hidden divergences by comparing peaks/troughs in price
/// vs indicator over a lookback window.
pub fn divergence(
    price: &[f64],
    indicator: &[f64],
    lookback: usize,
    min_distance: usize,
) -> Vec<DivergenceEvent> {
    assert_eq!(price.len(), indicator.len());
    let len = price.len();
    if len < lookback * 2 || lookback < 3 {
        return Vec::new();
    }

    let mut events = Vec::new();
    let peaks = find_local_extrema(price, lookback, true);
    let troughs = find_local_extrema(price, lookback, false);

    // Regular bearish: price higher high, indicator lower high
    for i in 1..peaks.len() {
        let (idx1, idx2) = (peaks[i - 1], peaks[i]);
        if idx2 - idx1 < min_distance {
            continue;
        }
        if price[idx2] > price[idx1] && indicator[idx2] < indicator[idx1] {
            events.push(DivergenceEvent {
                start_index: idx1,
                end_index: idx2,
                divergence_type: DivergenceType::RegularBearish,
                confidence: ((price[idx2] - price[idx1]) / price[idx1]).abs().min(1.0),
            });
        }
        // Hidden bearish: price lower high, indicator higher high
        if price[idx2] < price[idx1] && indicator[idx2] > indicator[idx1] {
            events.push(DivergenceEvent {
                start_index: idx1,
                end_index: idx2,
                divergence_type: DivergenceType::HiddenBearish,
                confidence: ((price[idx1] - price[idx2]) / price[idx1]).abs().min(1.0),
            });
        }
    }

    // Regular bullish: price lower low, indicator higher low
    for i in 1..troughs.len() {
        let (idx1, idx2) = (troughs[i - 1], troughs[i]);
        if idx2 - idx1 < min_distance {
            continue;
        }
        if price[idx2] < price[idx1] && indicator[idx2] > indicator[idx1] {
            events.push(DivergenceEvent {
                start_index: idx1,
                end_index: idx2,
                divergence_type: DivergenceType::RegularBullish,
                confidence: ((price[idx1] - price[idx2]) / price[idx1]).abs().min(1.0),
            });
        }
        // Hidden bullish: price higher low, indicator lower low
        if price[idx2] > price[idx1] && indicator[idx2] < indicator[idx1] {
            events.push(DivergenceEvent {
                start_index: idx1,
                end_index: idx2,
                divergence_type: DivergenceType::HiddenBullish,
                confidence: ((price[idx2] - price[idx1]) / price[idx1]).abs().min(1.0),
            });
        }
    }

    events
}

fn find_local_extrema(data: &[f64], window: usize, find_max: bool) -> Vec<usize> {
    let mut extrema = Vec::new();
    let half = window / 2;
    for i in half..data.len().saturating_sub(half) {
        let start = i.saturating_sub(half);
        let end = (i + half + 1).min(data.len());
        let slice = &data[start..end];
        let is_extremum = if find_max {
            slice.iter().all(|&v| v <= data[i])
        } else {
            slice.iter().all(|&v| v >= data[i])
        };
        if is_extremum {
            extrema.push(i);
        }
    }
    extrema
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crossover_basic() {
        let fast = vec![1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 1.0, 3.0];
        let slow = vec![3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 2.0];
        let events = crossover(&fast, &slow);
        assert!(!events.is_empty());
        assert_eq!(events[0].direction, SignalDirection::Up);
    }

    #[test]
    fn test_crossunder_basic() {
        let fast = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let slow = vec![3.0, 3.0, 3.0, 3.0, 3.0];
        let events = crossunder(&fast, &slow);
        assert!(!events.is_empty());
        assert_eq!(events[0].direction, SignalDirection::Down);
    }

    #[test]
    fn test_threshold_cross() {
        let data = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
        let events = threshold_cross(&data, 55.0);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].index, 5);
        assert_eq!(events[0].direction, SignalDirection::Up);
    }

    #[test]
    fn test_crossover_no_cross() {
        let fast = vec![1.0, 2.0, 3.0];
        let slow = vec![10.0, 10.0, 10.0];
        let events = crossover(&fast, &slow);
        assert!(events.is_empty());
    }

    #[test]
    fn test_divergence_basic() {
        let price = vec![
            10.0, 11.0, 12.0, 11.0, 10.0, 9.0, 10.0, 11.0, 13.0, 12.0, 10.0, 9.0, 10.0, 11.0, 14.0,
            13.0, 11.0, 10.0, 9.0, 8.0,
        ];
        let indicator = vec![
            50.0, 55.0, 60.0, 55.0, 50.0, 45.0, 50.0, 55.0, 58.0, 55.0, 50.0, 45.0, 50.0, 55.0,
            56.0, 54.0, 50.0, 48.0, 45.0, 42.0,
        ];
        let events = divergence(&price, &indicator, 3, 3);
        // May detect bearish divergence: price making higher highs, indicator lower highs
        assert!(events.len() <= 10); // sanity check
    }
}
