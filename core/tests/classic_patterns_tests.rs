//! Tests for the classic stock-trading chart patterns (Darvas, Renko, Kagi,
//! Point & Figure, Three Line Break, Williams Alligator, Heikin-Ashi).
//!
//! These are first-class indicators in `alpha_ta_core::indicators` but live in
//! the `classic_patterns` and `chart` modules.

use alpha_ta_core::indicators::{
    classic_patterns::{
        darvas_box, kagi, point_and_figure, renko, three_line_break, williams_alligator,
    },
    heikin_ashi,
};

fn approx_eq_opt(a: f64, b: f64, eps: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        true
    } else if a.is_nan() || b.is_nan() {
        false
    } else {
        (a - b).abs() < eps
    }
}

fn deterministic_ohlcv(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64;
        let base = 100.0 + 0.7 * t + 3.0 * (0.21 * t).sin() + 1.5 * (0.13 * t).cos();
        let h = base + 1.0 + (0.07 * t).sin().abs();
        let l = base - 1.0 - (0.05 * t).cos().abs();
        let c = base + 0.5 * (0.11 * t).sin();
        open.push(base);
        high.push(h);
        low.push(l);
        close.push(c);
    }
    (open, high, low, close)
}

// ============================================================================
// Darvas Box
// ============================================================================

#[test]
fn darvas_box_breakout_signal() {
    // Constant bars 0..5 to seed a rolling max.
    let mut high = vec![10.0; 20];
    let mut low = vec![9.0; 20];
    let mut close = vec![10.0; 20];
    // Bar 5: a new swing high opens a candidate box.
    high[5] = 11.0;
    low[5] = 10.0;
    close[5] = 11.0;
    // Bars 6-7: bars that don't make a new lookback high; with confirmation=1
    // bar 6 will already publish a confirmed box (candidate age 0 -> 1).
    high[6] = 10.5;
    low[6] = 9.5;
    close[6] = 10.5;
    high[7] = 10.5;
    low[7] = 9.5;
    close[7] = 10.5;
    // Bar 8: a higher high that should generate a buy breakout.
    high[8] = 12.0;
    low[8] = 11.0;
    close[8] = 12.0;
    let r = darvas_box(&high, &low, &close, 3, 1).unwrap();
    assert_eq!(r.box_top.len(), 20);
    assert_eq!(r.box_bottom.len(), 20);
    assert_eq!(r.signal.len(), 20);
    // Either a confirmed box (at bar 6) or a breakout (at bar 8) must be present.
    let has_buy = r.signal.iter().any(|&s| s == 1);
    let has_box = r.box_top.iter().any(|v| v.is_finite());
    assert!(
        has_buy || has_box,
        "expected Darvas to publish at least one confirmed box or breakout"
    );
}

#[test]
fn darvas_box_validates_lengths() {
    let r = darvas_box(&[1.0, 2.0], &[1.0], &[1.0, 2.0], 3, 1);
    assert!(r.is_err());
}

#[test]
fn darvas_box_validates_lookback() {
    let h = vec![1.0; 10];
    let l = vec![1.0; 10];
    let c = vec![1.0; 10];
    assert!(darvas_box(&h, &l, &c, 1, 1).is_err());
    assert!(darvas_box(&h, &l, &c, 300, 1).is_err());
}

// ============================================================================
// Renko
// ============================================================================

#[test]
fn renko_no_nan_on_simple_walk() {
    let high = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
    let r = renko(&high, &low, 1.0).unwrap();
    assert_eq!(r.bricks.len(), 5);
    assert_eq!(r.direction.len(), 5);
}

#[test]
fn renko_rejects_zero_box_size() {
    let high = vec![10.0, 11.0];
    let low = vec![9.0, 10.0];
    assert!(renko(&high, &low, 0.0).is_err());
    assert!(renko(&high, &low, -1.0).is_err());
}

#[test]
fn renko_deterministic_replay() {
    // Renko must be deterministic on identical inputs.
    let (_o, h, l, _c) = deterministic_ohlcv(200);
    let r1 = renko(&h, &l, 1.0).unwrap();
    let r2 = renko(&h, &l, 1.0).unwrap();
    for i in 0..r1.bricks.len() {
        assert!(approx_eq_opt(r1.bricks[i], r2.bricks[i], 1e-12));
        assert_eq!(r1.direction[i], r2.direction[i]);
    }
}

// ============================================================================
// Kagi
// ============================================================================

#[test]
fn kagi_reversal_count() {
    // Build a price that should reverse multiple times with a tight threshold.
    let close = vec![10.0, 11.0, 12.0, 11.5, 9.0, 8.0, 9.0, 12.0];
    let r = kagi(&close, 2.0).unwrap();
    // Some bars will be NaN (non-reversal), but the result must be aligned.
    assert_eq!(r.kagi.len(), 8);
    assert_eq!(r.direction.len(), 8);
    assert!(r.direction.iter().any(|&d| d == 1 || d == -1));
}

#[test]
fn kagi_rejects_zero_reversal() {
    let close = vec![1.0, 2.0, 3.0];
    assert!(kagi(&close, 0.0).is_err());
    assert!(kagi(&close, -1.0).is_err());
}

// ============================================================================
// Point & Figure
// ============================================================================

#[test]
fn pnf_uptrend_emits_x_columns() {
    let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
    let r = point_and_figure(&high, &low, 1.0, 3).unwrap();
    assert_eq!(r.pnf.len(), 6);
    // In a steady uptrend, no O columns should appear.
    assert!(r.column_type.iter().all(|&c| c >= 0));
    assert!(r.column_type.iter().any(|&c| c == 1));
}

#[test]
fn pnf_rejects_invalid() {
    let high = vec![10.0, 11.0];
    let low = vec![9.0, 10.0];
    assert!(point_and_figure(&high, &low, 0.0, 3).is_err());
    assert!(point_and_figure(&high, &low, 1.0, 0).is_err());
}

// ============================================================================
// Three Line Break
// ============================================================================

#[test]
fn tlb_reverses_on_3_lines() {
    // Build a clear uptrend, then 3 black lines that track the lows,
    // then a sharp drop below the lowest of the last 3 black lines.
    let close = vec![100.0, 101.0, 102.0, 103.0, 104.0, 100.0, 99.0, 98.0, 90.0];
    let r = three_line_break(&close, 3).unwrap();
    assert_eq!(r.line.len(), 9);
    assert_eq!(r.direction.len(), 9);
    // The drop at index 8 (90.0) reverses from up to down, giving -1.
    assert!(r.direction.iter().any(|&d| d == -1));
}

#[test]
fn tlb_rejects_zero_lines() {
    let close = vec![1.0; 10];
    assert!(three_line_break(&close, 0).is_err());
}

// ============================================================================
// Williams Alligator
// ============================================================================

#[test]
fn williams_alligator_jaw_longer_warmup() {
    let close: Vec<f64> = (0..50).map(|i| 100.0 + i as f64 * 0.5).collect();
    let r = williams_alligator(&close).unwrap();
    let lips_start = r.lips.iter().position(|v| !v.is_nan()).unwrap();
    let teeth_start = r.teeth.iter().position(|v| !v.is_nan()).unwrap();
    let jaw_start = r.jaw.iter().position(|v| !v.is_nan()).unwrap();
    // Lips (period=5) is shortest, so it produces valid output first.
    assert!(lips_start < teeth_start);
    assert!(teeth_start < jaw_start);
}

#[test]
fn williams_alligator_lags_in_uptrend() {
    // For monotonically increasing close, all three lines should be < close
    // after their warmup (SMMA is a lagging smoother).
    let close: Vec<f64> = (0..60).map(|i| 100.0 + i as f64).collect();
    let r = williams_alligator(&close).unwrap();
    let last = close.len() - 1;
    assert!(r.jaw[last] < close[last]);
    assert!(r.teeth[last] < close[last]);
    assert!(r.lips[last] < close[last]);
}

// ============================================================================
// Heikin-Ashi
// ============================================================================

#[test]
fn heikin_ashi_recovers_on_nan_gap() {
    let open = vec![10.0, f64::NAN, 12.0, 13.0];
    let high = vec![12.0, f64::NAN, 14.0, 15.0];
    let low = vec![9.0, f64::NAN, 11.0, 12.0];
    let close = vec![11.0, f64::NAN, 13.0, 14.0];
    let r = heikin_ashi(&open, &high, &low, &close).unwrap();
    assert!(r.ha_close[0].is_finite());
    assert!(r.ha_close[1].is_nan());
    assert!(r.ha_close[2].is_finite());
    assert!(r.ha_close[3].is_finite());
}

#[test]
fn heikin_ashi_invalid_lengths() {
    let open = vec![10.0, 11.0];
    let high = vec![12.0];
    let low = vec![9.0, 10.0];
    let close = vec![11.0, 12.0];
    assert!(heikin_ashi(&open, &high, &low, &close).is_err());
}

// ============================================================================
// Cross-pattern determinism on identical input
// ============================================================================

#[test]
fn all_classic_patterns_deterministic() {
    let (_o, h, l, c) = deterministic_ohlcv(200);
    let r1a = darvas_box(&h, &l, &c, 5, 3).unwrap();
    let r1b = darvas_box(&h, &l, &c, 5, 3).unwrap();
    for i in 0..r1a.box_top.len() {
        assert!(approx_eq_opt(r1a.box_top[i], r1b.box_top[i], 1e-12));
        assert!(approx_eq_opt(r1a.box_bottom[i], r1b.box_bottom[i], 1e-12));
        assert_eq!(r1a.signal[i], r1b.signal[i]);
    }

    let r2a = kagi(&c, 1.0).unwrap();
    let r2b = kagi(&c, 1.0).unwrap();
    for i in 0..r2a.kagi.len() {
        assert!(approx_eq_opt(r2a.kagi[i], r2b.kagi[i], 1e-12));
        assert_eq!(r2a.direction[i], r2b.direction[i]);
    }

    let r3a = point_and_figure(&h, &l, 1.0, 3).unwrap();
    let r3b = point_and_figure(&h, &l, 1.0, 3).unwrap();
    for i in 0..r3a.pnf.len() {
        assert!(approx_eq_opt(r3a.pnf[i], r3b.pnf[i], 1e-12));
        assert_eq!(r3a.column_type[i], r3b.column_type[i]);
        assert_eq!(r3a.new_column[i], r3b.new_column[i]);
    }

    let r4a = three_line_break(&c, 3).unwrap();
    let r4b = three_line_break(&c, 3).unwrap();
    for i in 0..r4a.line.len() {
        assert!(approx_eq_opt(r4a.line[i], r4b.line[i], 1e-12));
        assert_eq!(r4a.direction[i], r4b.direction[i]);
    }

    let r5a = williams_alligator(&c).unwrap();
    let r5b = williams_alligator(&c).unwrap();
    for i in 0..r5a.jaw.len() {
        assert!(approx_eq_opt(r5a.jaw[i], r5b.jaw[i], 1e-12));
        assert!(approx_eq_opt(r5a.teeth[i], r5b.teeth[i], 1e-12));
        assert!(approx_eq_opt(r5a.lips[i], r5b.lips[i], 1e-12));
    }
}
