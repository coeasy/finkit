//! Harmonic patterns (谐波形态).
//!
//! 5 classic harmonic patterns based on Fibonacci ratios. All functions
//! return a [`PatternResult`] (TA-Lib compatible: `100` = bullish at point D,
//! `-100` = bearish at point D, `0` = no pattern).
//!
//! # Pattern conventions
//!
//! Each harmonic pattern is described by 4 pivot points `X → A → B → C → D`.
//! - `X`, `B`, `D` are the same direction (all highs in bullish, all lows in bearish).
//! - `A`, `C` are the opposite direction.
//! - The D point is the Potential Reversal Zone (PRZ): the trade entry/exit point.
//!
//! # Fibonacci ratios
//!
//! | Pattern   | AB/XA | BC/AB | CD/BC | AD/XA |
//! |-----------|-------|-------|-------|-------|
//! | Gartley   | 0.618 | 0.382-0.886 | 1.272-1.618 | 0.786 |
//! | Butterfly | 0.786 | 0.382-0.886 | 1.618-2.618 | 1.272-1.618 |
//! | Bat       | 0.382-0.500 | 0.382-0.886 | 1.618-2.618 | 0.886 |
//! | Crab      | 0.382-0.618 | 0.382-0.886 | 2.618-3.618 | 1.618 |
//! | Shark     | 0.446-0.618 | 1.130-1.618 | 1.618-2.236 | 0.886-1.130 |
//!
//! # Examples
//!
//! ```
//! use alpha_ta_core::patterns::harmonic::gartley;
//! // Need at least 20 bars for the gartley pattern recognition.
//! let n = 30;
//! let high: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64) * 0.1 + ((i % 5) as f64) * 0.05).collect();
//! let low: Vec<f64>  = (0..n).map(|i| 10.0 + (i as f64) * 0.1 - ((i % 5) as f64) * 0.05).collect();
//! let out = gartley(&high, &low, 0.05).unwrap();
//! // No assertion: just smoke-test the API surface.
//! let _ = out.len();
//! ```

use crate::error::{Result, TaError};
use crate::patterns::common::{validate_ohlcv, Signal};
use crate::utils::validate_input;
use ndarray::Array1;

/// Pattern result alias (TA-Lib compatible: 100/-100/0).
pub type PatternResult = Array1<Signal>;

/// Direction of the harmonic pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    /// Bullish: X/B/D are lows, A/C are highs (M shape).
    Bullish,
    /// Bearish: X/B/D are highs, A/C are lows (W shape).
    Bearish,
}

/// A pivot (turning) point.
#[derive(Debug, Clone, Copy)]
struct Pivot {
    /// Bar index.
    idx: usize,
    /// Price value at the pivot.
    price: f64,
}

/// Find alternating pivots (high, low, high, low, ...) within a window.
///
/// Returns at least 4 pivots if the window is large enough. The returned
/// sequence is `pivot_0, pivot_1, pivot_2, ...` where even indices are
/// one direction (e.g. highs for bullish) and odd indices are the other
/// (e.g. lows).
fn find_pivots(high: &[f64], low: &[f64], start: usize, end: usize, order: usize) -> Vec<Pivot> {
    let mut pivots: Vec<Pivot> = Vec::new();
    if end - start < 2 * order + 1 {
        return pivots;
    }
    // Scan for local maxima (highs) and local minima (lows)
    let mut highs: Vec<Pivot> = Vec::new();
    let mut lows: Vec<Pivot> = Vec::new();
    for i in (start + order)..(end - order) {
        let h = high[i];
        let l = low[i];
        let is_max = (i - order..i).all(|k| high[k] <= h) && (i + 1..=i + order).all(|k| high[k] <= h);
        let is_min = (i - order..i).all(|k| low[k] >= l) && (i + 1..=i + order).all(|k| low[k] >= l);
        if is_max {
            highs.push(Pivot { idx: i, price: h });
        }
        if is_min {
            lows.push(Pivot { idx: i, price: l });
        }
    }
    // Merge alternating sequence by index order
    let mut hi = 0;
    let mut lo = 0;
    loop {
        if hi >= highs.len() && lo >= lows.len() {
            break;
        }
        let next_h = highs.get(hi).map(|p| p.idx).unwrap_or(usize::MAX);
        let next_l = lows.get(lo).map(|p| p.idx).unwrap_or(usize::MAX);
        if next_h <= next_l {
            pivots.push(highs[hi]);
            hi += 1;
        } else {
            pivots.push(lows[lo]);
            lo += 1;
        }
    }
    pivots
}

/// Check whether `actual` is within `tolerance` of `expected`.
#[inline]
#[allow(dead_code)]
fn fib_ratio_match(actual: f64, expected: f64, tolerance: f64) -> bool {
    if expected.abs() < 1e-9 {
        return actual.abs() < 1e-9;
    }
    (actual - expected).abs() / expected.abs() <= tolerance
}

/// Check whether `actual` is in the `[lo, hi]` range (Fibonacci tolerance).
#[inline]
fn fib_range_match(actual: f64, lo: f64, hi: f64, tolerance: f64) -> bool {
    actual >= lo * (1.0 - tolerance) && actual <= hi * (1.0 + tolerance)
}

/// Generic harmonic detector: 5 pivots X/A/B/C/D with specified ratios.
///
/// Returns `Some(signal)` if a match is found (signal = 100 bullish, -100 bearish),
/// or `None` otherwise.
fn check_harmonic(
    pivots: &[Pivot],
    direction: Direction,
    ab_xa: (f64, f64),
    bc_ab: (f64, f64),
    cd_bc: (f64, f64),
    ad_xa: (f64, f64),
    pivot_tolerance: f64,
) -> Option<Signal> {
    if pivots.len() < 5 {
        return None;
    }
    // Take the last 5 pivots
    let n = pivots.len();
    let x = pivots[n - 5];
    let a = pivots[n - 4];
    let b = pivots[n - 3];
    let c = pivots[n - 2];
    let d = pivots[n - 1];
    // Sanity: alternating direction
    let xa = (a.price - x.price).abs();
    let ab = (b.price - a.price).abs();
    let bc = (c.price - b.price).abs();
    let cd = (d.price - c.price).abs();
    let ad = (d.price - a.price).abs();
    if xa <= 0.0 || ab <= 0.0 || bc <= 0.0 {
        return None;
    }
    let r_ab_xa = ab / xa;
    let r_bc_ab = bc / ab;
    let r_cd_bc = cd / bc;
    let r_ad_xa = ad / xa;
    if !fib_range_match(r_ab_xa, ab_xa.0, ab_xa.1, pivot_tolerance) {
        return None;
    }
    if !fib_range_match(r_bc_ab, bc_ab.0, bc_ab.1, pivot_tolerance) {
        return None;
    }
    if !fib_range_match(r_cd_bc, cd_bc.0, cd_bc.1, pivot_tolerance) {
        return None;
    }
    if !fib_range_match(r_ad_xa, ad_xa.0, ad_xa.1, pivot_tolerance) {
        return None;
    }
    // D should be in PRZ: between A and the projection
    Some(match direction {
        Direction::Bullish => 100,
        Direction::Bearish => -100,
    })
}

/// Detect both bullish and bearish variants of a harmonic pattern at the last bar.
fn detect_harmonic(
    high: &[f64],
    low: &[f64],
    pivot_order: usize,
    pivot_tolerance: f64,
    ratios: ((f64, f64), (f64, f64), (f64, f64), (f64, f64)),
) -> Result<PatternResult> {
    let n = high.len();
    let mut out = init_signal(n);
    if n < 2 * pivot_order + 5 {
        return Ok(out);
    }
    // Bullish: X=low, A=high, B=low, C=high, D=low
    // We look for pivot triples; the bullish pattern has odd pivots (0,2,4) as lows.
    for i in (2 * pivot_order + 4)..n {
        let pivots = find_pivots(high, low, 0, i, pivot_order);
        // Try bullish: pivot[0]=low, [1]=high, [2]=low, [3]=high, [4]=low
        if pivots.len() >= 5 {
            let last5 = &pivots[pivots.len() - 5..];
            if is_bullish_sequence(last5) {
                if let Some(_) = check_harmonic(last5, Direction::Bullish, ratios.0, ratios.1, ratios.2, ratios.3, pivot_tolerance) {
                    out[i] = 100;
                }
            } else if is_bearish_sequence(last5) {
                if let Some(_) = check_harmonic(last5, Direction::Bearish, ratios.0, ratios.1, ratios.2, ratios.3, pivot_tolerance) {
                    out[i] = -100;
                }
            }
        }
    }
    Ok(out)
}

#[inline]
fn is_bullish_sequence(p: &[Pivot]) -> bool {
    p[0].price <= p[1].price && p[1].price >= p[2].price && p[2].price <= p[3].price && p[3].price >= p[4].price
}

#[inline]
fn is_bearish_sequence(p: &[Pivot]) -> bool {
    p[0].price >= p[1].price && p[1].price <= p[2].price && p[2].price >= p[3].price && p[3].price <= p[4].price
}

#[inline]
fn init_signal(n: usize) -> PatternResult {
    Array1::zeros(n)
}

// ============================================================================
// Gartley 222 (加特利)
// ============================================================================

/// Gartley 222 (加特利) — M 形 Fibonacci 回撤 + 延伸
///
/// # 识别规则
/// - AB = 0.618 × XA
/// - BC = 0.382-0.886 × AB
/// - CD = 1.272-1.618 × BC
/// - D = 0.786 × XA (回撤)
pub fn gartley(high: &[f64], low: &[f64], pivot_tolerance: f64) -> Result<PatternResult> {
    validate_ohlcv_inputs(high, low, pivot_tolerance)?;
    detect_harmonic(
        high,
        low,
        3,
        pivot_tolerance,
        ((0.618, 0.618), (0.382, 0.886), (1.272, 1.618), (0.786, 0.786)),
    )
}

// ============================================================================
// Butterfly (蝴蝶)
// ============================================================================

/// Butterfly (蝴蝶) — Gartley 变形，D 点超过 XA
///
/// # 识别规则
/// - AB = 0.786 × XA
/// - BC = 0.382-0.886 × AB
/// - CD = 1.618-2.618 × BC
/// - D = 1.272-1.618 × XA
pub fn butterfly(high: &[f64], low: &[f64], pivot_tolerance: f64) -> Result<PatternResult> {
    validate_ohlcv_inputs(high, low, pivot_tolerance)?;
    detect_harmonic(
        high,
        low,
        3,
        pivot_tolerance,
        ((0.786, 0.786), (0.382, 0.886), (1.618, 2.618), (1.272, 1.618)),
    )
}

// ============================================================================
// Bat (蝙蝠)
// ============================================================================

/// Bat (蝙蝠) — D 点在 0.886 XA (更深回撤)
///
/// # 识别规则
/// - AB = 0.382-0.500 × XA
/// - BC = 0.382-0.886 × AB
/// - CD = 1.618-2.618 × BC
/// - D = 0.886 × XA
pub fn bat(high: &[f64], low: &[f64], pivot_tolerance: f64) -> Result<PatternResult> {
    validate_ohlcv_inputs(high, low, pivot_tolerance)?;
    detect_harmonic(
        high,
        low,
        3,
        pivot_tolerance,
        ((0.382, 0.500), (0.382, 0.886), (1.618, 2.618), (0.886, 0.886)),
    )
}

// ============================================================================
// Crab (螃蟹)
// ============================================================================

/// Crab (螃蟹) — D 点在 1.618 XA (极深延伸)
///
/// # 识别规则
/// - AB = 0.382-0.618 × XA
/// - BC = 0.382-0.886 × AB
/// - CD = 2.618-3.618 × BC
/// - D = 1.618 × XA
pub fn crab(high: &[f64], low: &[f64], pivot_tolerance: f64) -> Result<PatternResult> {
    validate_ohlcv_inputs(high, low, pivot_tolerance)?;
    detect_harmonic(
        high,
        low,
        3,
        pivot_tolerance,
        ((0.382, 0.618), (0.382, 0.886), (2.618, 3.618), (1.618, 1.618)),
    )
}

// ============================================================================
// Shark (鲨鱼)
// ============================================================================

/// Shark (鲨鱼) — 5-0-5 形态 (与 Gartley 不同)
///
/// # 识别规则
/// - AB = 0.446-0.618 × XA
/// - BC = 1.130-1.618 × AB
/// - CD = 1.618-2.236 × BC
/// - D = 0.886-1.130 × XA
pub fn shark(high: &[f64], low: &[f64], pivot_tolerance: f64) -> Result<PatternResult> {
    validate_ohlcv_inputs(high, low, pivot_tolerance)?;
    detect_harmonic(
        high,
        low,
        3,
        pivot_tolerance,
        ((0.446, 0.618), (1.130, 1.618), (1.618, 2.236), (0.886, 1.130)),
    )
}

fn validate_ohlcv_inputs(high: &[f64], low: &[f64], pivot_tolerance: f64) -> Result<()> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if !(0.0..=1.0).contains(&pivot_tolerance) {
        return Err(TaError::InvalidParameter {
            name: "pivot_tolerance".to_string(),
            constraint: "must be in [0.0, 1.0]".to_string(),
        });
    }
    validate_input(high.len(), 20)
}

// Silence the unused-import warning for `validate_ohlcv` (kept for API symmetry)
#[allow(dead_code)]
fn _unused() {
    let _ = validate_ohlcv;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic 5-pivot harmonic series.
    /// `pattern`: choose "bullish" or "bearish".
    fn synth_pivots(direction: &str) -> (Vec<f64>, Vec<f64>) {
        // Bullish Gartley-like
        let (h, l) = match direction {
            "bullish" => {
                let h = vec![100.0, 120.0, 107.64, 113.82, 98.1];
                let l = vec![100.0, 120.0, 107.64, 113.82, 98.1];
                (h, l)
            }
            "bearish" => {
                // Bearish: invert
                let h = vec![100.0, 80.0, 92.36, 86.18, 101.9];
                let l = vec![100.0, 80.0, 92.36, 86.18, 101.9];
                (h, l)
            }
            _ => panic!("unknown direction"),
        };
        // Pad with some noise bars before/after to make `pivot_order` work
        let mut h_padded = vec![100.0; 20];
        let mut l_padded = vec![100.0; 20];
        h_padded.extend(h.iter().map(|x| x + 100.0));
        l_padded.extend(l.iter().map(|x| x + 100.0));
        // Add a few more bars after
        for _ in 0..5 {
            h_padded.push(200.0);
            l_padded.push(200.0);
        }
        (h_padded, l_padded)
    }

    #[test]
    fn test_fib_ratio_match() {
        assert!(fib_ratio_match(0.62, 0.618, 0.05));
        assert!(!fib_ratio_match(0.7, 0.618, 0.05));
    }

    #[test]
    fn test_fib_range_match() {
        assert!(fib_range_match(0.5, 0.382, 0.886, 0.05));
        assert!(!fib_range_match(0.2, 0.382, 0.886, 0.05));
    }

    #[test]
    fn test_gartley_basic() {
        let (h, l) = synth_pivots("bullish");
        let out = gartley(&h, &l, 0.05).unwrap();
        assert_eq!(out.len(), h.len());
    }

    #[test]
    fn test_butterfly_basic() {
        let (h, l) = synth_pivots("bullish");
        let out = butterfly(&h, &l, 0.05).unwrap();
        assert_eq!(out.len(), h.len());
    }

    #[test]
    fn test_bat_basic() {
        let (h, l) = synth_pivots("bullish");
        let out = bat(&h, &l, 0.05).unwrap();
        assert_eq!(out.len(), h.len());
    }

    #[test]
    fn test_crab_basic() {
        let (h, l) = synth_pivots("bullish");
        let out = crab(&h, &l, 0.05).unwrap();
        assert_eq!(out.len(), h.len());
    }

    #[test]
    fn test_shark_basic() {
        let (h, l) = synth_pivots("bullish");
        let out = shark(&h, &l, 0.05).unwrap();
        assert_eq!(out.len(), h.len());
    }

    #[test]
    fn test_input_validation() {
        // Need at least 20 bars for harmonic
        let h = vec![1.0; 25];
        let l = vec![1.0; 25];
        assert!(gartley(&h, &l, 0.05).is_ok());
        // Mismatched lengths
        let l2 = vec![1.0; 5];
        assert!(gartley(&h, &l2, 0.05).is_err());
        // Bad tolerance
        assert!(gartley(&h, &l, 1.5).is_err());
        // Too short
        let h_short = vec![1.0; 10];
        let l_short = vec![1.0; 10];
        assert!(gartley(&h_short, &l_short, 0.05).is_err());
    }
}
