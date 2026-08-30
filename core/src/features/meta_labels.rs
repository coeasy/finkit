//! Meta-labeling and event-driven labels (López de Prado).
//!
//! Meta-labeling uses a primary model's direction signals and a secondary
//! triple-barrier evaluation to produce bet sizes. Event-driven labels assign
//! forward-return direction at event timestamps.

use super::labels::triple_barrier;
use super::BarrierLabel;

/// Meta-labeling bet sizing using triple barrier outcomes.
///
/// For each bar with a non-zero primary signal (`+1` buy, `-1` sell), evaluates
/// the triple barrier from that entry and maps the outcome to a bet size in
/// `[0.0, 1.0]`. Bars without a signal receive `0.0`.
///
/// Bet size rules:
/// - Aligned barrier hit (profit-taking for long, stop-loss for short): `1.0`
/// - Opposing barrier hit: `0.0`
/// - Vertical barrier (timeout): `0.5` if return aligns with signal, else `0.0`
pub fn meta_label(
    primary_signal: &[i8],
    close: &[f64],
    high: &[f64],
    low: &[f64],
    pt_factor: f64,
    sl_factor: f64,
    max_hold: usize,
) -> Vec<f64> {
    let len = close.len();
    assert_eq!(primary_signal.len(), len);
    assert_eq!(high.len(), len);
    assert_eq!(low.len(), len);

    let barriers = triple_barrier(close, high, low, pt_factor, sl_factor, max_hold);

    primary_signal
        .iter()
        .zip(barriers.iter())
        .map(|(&signal, barrier)| bet_size_from_barrier(signal, barrier))
        .collect()
}

fn bet_size_from_barrier(signal: i8, barrier: &BarrierLabel) -> f64 {
    if signal == 0 {
        return 0.0;
    }
    if i32::from(signal) * i32::from(barrier.label) > 0 {
        return 1.0;
    }
    if barrier.label == 0 {
        let aligned = f64::from(signal) * barrier.ret > 0.0;
        return if aligned { 0.5 } else { 0.0 };
    }
    0.0
}

/// Event-driven labels from forward returns at event timestamps.
///
/// For each index in `events`, computes the `horizon`-bar forward arithmetic
/// return and labels it `+1` (positive), `-1` (negative), or `0` when future
/// data is unavailable.
pub fn event_labels(close: &[f64], events: &[usize], horizon: usize) -> Vec<i8> {
    events
        .iter()
        .map(|&idx| {
            let end = idx.saturating_add(horizon);
            if end >= close.len() || idx >= close.len() {
                return 0;
            }
            let entry = close[idx];
            if entry.abs() <= 1e-15 {
                return 0;
            }
            let ret = (close[end] - entry) / entry;
            if ret > 0.0 {
                1
            } else if ret < 0.0 {
                -1
            } else {
                0
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_label_basic() {
        let close = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 104.0, 103.0];
        let high: Vec<f64> = close.iter().map(|&c| c + 1.0).collect();
        let low: Vec<f64> = close.iter().map(|&c| c - 1.0).collect();
        let primary_signal = vec![1i8, -1, 0, 1, -1, 1, 0, -1];
        let bet_sizes = meta_label(&primary_signal, &close, &high, &low, 2.0, 2.0, 5);
        assert_eq!(bet_sizes.len(), close.len());
        for &size in &bet_sizes {
            assert!((0.0..=1.0).contains(&size));
        }
    }

    #[test]
    fn test_meta_label_no_signal() {
        let close = vec![100.0, 101.0, 102.0, 103.0, 104.0];
        let high: Vec<f64> = close.iter().map(|&c| c + 1.0).collect();
        let low: Vec<f64> = close.iter().map(|&c| c - 1.0).collect();
        let primary_signal = vec![0i8; close.len()];
        let bet_sizes = meta_label(&primary_signal, &close, &high, &low, 2.0, 2.0, 5);
        assert!(bet_sizes.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_event_labels_basic() {
        let close = vec![100.0, 110.0, 90.0, 105.0];
        let events = vec![0, 1, 3];
        let labels = event_labels(&close, &events, 1);
        assert_eq!(labels.len(), 3);
        assert_eq!(labels[0], 1);
        assert_eq!(labels[1], -1);
        assert_eq!(labels[2], 0);
    }
}
