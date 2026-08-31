//! Support/resistance level detection and trend strength scoring.

use crate::indicators::adx;

/// Classified support or resistance price level.
#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: f64,
    pub level_type: LevelType,
    /// Relative significance in range 0.0–1.0.
    pub strength: f64,
    pub index: usize,
}

/// Whether a detected level acts as support or resistance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelType {
    Support,
    Resistance,
}

/// Detect support and resistance levels using pivot-point local extrema.
///
/// Swing highs in `high` become resistance; swing lows in `low` become support.
/// `lookback` sets the symmetric window for pivot confirmation. Level `strength`
/// reflects how often price revisits the level within a small tolerance.
pub fn support_resistance_levels(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Vec<PriceLevel> {
    let len = high.len();
    if len == 0 || high.len() != low.len() || high.len() != close.len() || lookback < 2 {
        return Vec::new();
    }

    let half = lookback / 2;
    if len <= lookback {
        return Vec::new();
    }

    let tolerance = 0.005;
    let mut levels = Vec::new();

    for i in half..len.saturating_sub(half) {
        if is_pivot_extremum(high, i, half, true) {
            let price = high[i];
            levels.push(PriceLevel {
                price,
                level_type: LevelType::Resistance,
                strength: touch_strength(price, high, tolerance),
                index: i,
            });
        }
        if is_pivot_extremum(low, i, half, false) {
            let price = low[i];
            levels.push(PriceLevel {
                price,
                level_type: LevelType::Support,
                strength: touch_strength(price, low, tolerance),
                index: i,
            });
        }
    }

    levels.sort_by_key(|l| l.index);
    levels
}

/// Trend strength scores in 0.0–100.0 derived from ADX on OHLC proxies.
///
/// When only `close` is available, high/low are approximated from close for ADX.
/// ADX values above 25 map to scores generally above 60.
pub fn trend_strength_score(close: &[f64], adx_period: usize) -> Vec<f64> {
    let len = close.len();
    let mut scores = vec![0.0; len];
    if len < 2 || adx_period < 1 {
        return scores;
    }

    let (high, low) = ohlc_from_close(close);
    let Ok(adx_vals) = adx(&high, &low, close, adx_period) else {
        return scores;
    };

    let dm_scores = directional_strength_from_close(close, adx_period);
    for (score, (&adx_val, &dm)) in scores.iter_mut().zip(adx_vals.iter().zip(dm_scores.iter())) {
        *score = adx_to_strength(adx_val).max(dm);
    }
    scores
}

/// Build synthetic high/low from close so directional movement is meaningful.
fn ohlc_from_close(close: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(close.len());
    let mut low = Vec::with_capacity(close.len());
    for &c in close {
        if !c.is_finite() {
            high.push(f64::NAN);
            low.push(f64::NAN);
        } else {
            // Mirror typical bar geometry: close near mid-range with ~4pt spread.
            high.push(c + 2.0);
            low.push(c - 2.0);
        }
    }
    (high, low)
}

/// Close-only directional strength (0–100) when ADX warm-up is incomplete.
fn directional_strength_from_close(close: &[f64], period: usize) -> Vec<f64> {
    let len = close.len();
    let mut scores = vec![0.0; len];
    if len < 2 || period < 1 {
        return scores;
    }
    let window = period.max(2);
    for (i, score) in scores.iter_mut().enumerate().skip(window) {
        let mut up = 0.0;
        let mut down = 0.0;
        for j in (i - window + 1)..=i {
            let diff = close[j] - close[j - 1];
            if diff > 0.0 {
                up += diff;
            } else {
                down += -diff;
            }
        }
        let total = up + down;
        if total > 1e-15 {
            let dx = (up - down).abs() / total * 100.0;
            *score = adx_to_strength(dx);
        }
    }
    scores
}

fn is_pivot_extremum(data: &[f64], index: usize, half: usize, find_max: bool) -> bool {
    let start = index.saturating_sub(half);
    let end = (index + half + 1).min(data.len());
    let pivot = data[index];
    if !pivot.is_finite() {
        return false;
    }
    data[start..end]
        .iter()
        .all(|&v| if find_max { v <= pivot } else { v >= pivot })
        && data[start..end].iter().any(|&v| (v - pivot).abs() > 1e-12)
}

fn touch_strength(level: f64, series: &[f64], tolerance_pct: f64) -> f64 {
    if !level.is_finite() || level.abs() < 1e-15 {
        return 0.0;
    }
    let band = level.abs() * tolerance_pct;
    let touches = series
        .iter()
        .filter(|&&v| v.is_finite() && (v - level).abs() <= band)
        .count();
    (touches as f64 / 3.0).clamp(0.0, 1.0)
}

fn adx_to_strength(adx: f64) -> f64 {
    if !adx.is_finite() || adx <= 0.0 {
        return 0.0;
    }
    // ADX 25 → 60; cap at 100 for strong trends.
    (adx * 2.4).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oscillating_ohlc(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        let mut close = Vec::with_capacity(n);
        for i in 0..n {
            let wave = (i as f64 * 0.5).sin();
            let c = 100.0 + wave * 10.0;
            close.push(c);
            high.push(c + 2.0);
            low.push(c - 2.0);
        }
        (high, low, close)
    }

    #[test]
    fn test_support_resistance_levels() {
        let (high, low, close) = oscillating_ohlc(80);
        let levels = support_resistance_levels(&high, &low, &close, 5);

        assert!(!levels.is_empty());

        let supports: Vec<_> = levels
            .iter()
            .filter(|l| l.level_type == LevelType::Support)
            .collect();
        let resistances: Vec<_> = levels
            .iter()
            .filter(|l| l.level_type == LevelType::Resistance)
            .collect();

        assert!(!supports.is_empty());
        assert!(!resistances.is_empty());

        for level in &levels {
            assert!(level.price.is_finite());
            assert!((0.0..=1.0).contains(&level.strength));
            assert!(level.index < close.len());
            match level.level_type {
                LevelType::Support => {
                    assert!((level.price - low[level.index]).abs() < 1e-9);
                }
                LevelType::Resistance => {
                    assert!((level.price - high[level.index]).abs() < 1e-9);
                }
            }
        }

        let max_support = supports
            .iter()
            .map(|l| l.price)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_resistance = resistances
            .iter()
            .map(|l| l.price)
            .fold(f64::INFINITY, f64::min);
        assert!(max_support < min_resistance + 5.0);
    }

    #[test]
    fn test_trend_strength_range() {
        let (_, _, close) = oscillating_ohlc(60);
        let scores = trend_strength_score(&close, 14);
        assert_eq!(scores.len(), close.len());
        for &score in &scores {
            assert!((0.0..=100.0).contains(&score));
        }
    }

    #[test]
    fn test_trend_strength_trending() {
        let close: Vec<f64> = (0..80).map(|i| 100.0 + i as f64 * 2.0).collect();
        let scores = trend_strength_score(&close, 14);

        let tail = scores.len().saturating_sub(10);
        let tail_avg: f64 = scores[tail..].iter().sum::<f64>() / (scores.len() - tail) as f64;
        assert!(
            tail_avg > 60.0,
            "trending tail average strength should exceed 60, got {tail_avg}"
        );

        let (high, low) = ohlc_from_close(&close);
        let adx_vals = adx(&high, &low, &close, 14).unwrap();
        for i in tail..scores.len() {
            if adx_vals[i] > 25.0 {
                assert!(
                    scores[i] > 60.0,
                    "when ADX {:.1} > 25, score should exceed 60, got {:.1}",
                    adx_vals[i],
                    scores[i]
                );
            }
        }
    }
}
