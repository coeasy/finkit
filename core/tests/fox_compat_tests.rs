//! FoxTrader (飞狐交易师) compatibility tests.
//!
//! Scope: the *indicator* half of the FoxTrader dialect only — `FOX_ZIG`,
//! `FOX_PEAK`, `FOX_TROUGH` and their bar offsets. The dialect's trade-signal
//! and backtest helpers (`FOX_BUY`, `FOX_SELL`, `FOX_TRADE_SIGNAL`,
//! `FOX_BACKTEST`, `FOX_PROFIT_RATIO`, `FOX_WIN_RATE`, `FOX_MAX_DRAWDOWN`,
//! `FOX_TRADE_COUNT`) were removed: finkit does not do backtesting.

use finkit::formula::engine::FormulaEngine;
use finkit::formula::types::FormulaContext;
use ndarray::Array1;

fn make_zigzag_ctx(len: usize) -> FormulaContext {
    let close: Vec<f64> = (0..len)
        .map(|i| {
            let phase = (i as f64) / 10.0;
            10.0 + phase.sin() * 3.0 + (phase * 2.7).sin() * 1.5
        })
        .collect();
    let open: Vec<f64> = close.iter().map(|c| c - 0.1).collect();
    let high: Vec<f64> = close.iter().map(|c| c + 0.5).collect();
    let low: Vec<f64> = close.iter().map(|c| c - 0.5).collect();
    let volume: Vec<f64> = (0..len)
        .map(|i| 1000.0 + (i as f64 * 0.5).sin() * 200.0)
        .collect();
    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    )
}

// ============================================================================
// 飞狐交易师（FoxTrader）之字转向函数测试
// ============================================================================

#[test]
fn test_fox_zig() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_zigzag_ctx(100);
    let result = engine.eval("FOX_ZIG(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_zig_basic() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_zigzag_ctx(100);
    let result = engine.eval("FOX_ZIG(CLOSE, 10)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_zig_with_high_low() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_zigzag_ctx(100);
    let result = engine.eval("FOX_ZIG(HIGH, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_trough() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_zigzag_ctx(100);
    let result = engine.eval("FOX_TROUGH(CLOSE, 5, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_peak() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_zigzag_ctx(100);
    let result = engine.eval("FOX_PEAK(CLOSE, 5, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_troughbars() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_zigzag_ctx(100);
    let result = engine
        .eval("FOX_TROUGHBARS(CLOSE, 5, 1)", &mut ctx)
        .unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_peakbars() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_zigzag_ctx(100);
    let result = engine.eval("FOX_PEAKBARS(CLOSE, 5, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_zig_consistency_with_zigzag() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_zigzag_ctx(100);
    let fox_result = engine.eval("FOX_ZIG(CLOSE, 5)", &mut ctx).unwrap();
    let mut ctx2 = make_zigzag_ctx(100);
    let zigzag_result = engine.eval("ZIGZAG(CLOSE, 5)", &mut ctx2).unwrap();
    for i in 0..100 {
        if fox_result[i].is_nan() && zigzag_result[i].is_nan() {
            continue;
        }
        assert!(
            (fox_result[i] - zigzag_result[i]).abs() < 1e-10,
            "FOX_ZIG and ZIGZAG differ at index {}: {} vs {}",
            i,
            fox_result[i],
            zigzag_result[i]
        );
    }
}

#[test]
fn test_fox_peak_trough_m2() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_zigzag_ctx(100);
    let result = engine.eval("FOX_PEAK(CLOSE, 5, 2)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let mut ctx2 = make_zigzag_ctx(100);
    let result2 = engine.eval("FOX_TROUGH(CLOSE, 5, 2)", &mut ctx2).unwrap();
    assert_eq!(result2.len(), 100);
}
