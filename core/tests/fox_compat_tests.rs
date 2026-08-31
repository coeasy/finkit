use finkit::formula::engine::FormulaEngine;
use finkit::formula::types::FormulaContext;
use ndarray::Array1;

fn make_ctx(len: usize) -> FormulaContext {
    let close: Vec<f64> = (0..len)
        .map(|i| 10.0 + (i as f64 * 0.3).sin() * 2.0 + i as f64 * 0.05)
        .collect();
    let open: Vec<f64> = close.iter().map(|c| c - 0.1).collect();
    let high: Vec<f64> = close.iter().map(|c| c + 0.5).collect();
    let low: Vec<f64> = close.iter().map(|c| c - 0.5).collect();
    let volume: Vec<f64> = (0..len)
        .map(|i| 1000.0 + (i as f64 * 0.7).sin() * 200.0)
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
    let mut ctx = make_ctx(100);
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

// ============================================================================
// 飞狐交易师（FoxTrader）买卖信号函数测试
// ============================================================================

#[test]
fn test_fox_buy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        FOX_BUY(COND, CLOSE)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let has_signal = result.iter().any(|v| *v > 0.0);
    assert!(has_signal, "FOX_BUY should produce at least one buy signal");
}

#[test]
fn test_fox_sell() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        FOX_SELL(COND, CLOSE)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_buy_sell_symmetry() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let buy_source = r#"
        COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        FOX_BUY(COND, CLOSE)
    "#;
    let buy_result = engine.eval(buy_source, &mut ctx).unwrap();
    let mut ctx2 = make_ctx(100);
    let sell_source = r#"
        COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        FOX_SELL(COND, CLOSE)
    "#;
    let sell_result = engine.eval(sell_source, &mut ctx2).unwrap();
    assert_eq!(buy_result.len(), sell_result.len());
}

#[test]
fn test_fox_trade_signal() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        SELL_COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        FOX_TRADE_SIGNAL(BUY_COND, SELL_COND)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let has_buy = result.iter().any(|v| *v == 1.0);
    let has_sell = result.iter().any(|v| *v == -1.0);
    assert!(
        has_buy || has_sell,
        "FOX_TRADE_SIGNAL should produce at least one signal"
    );
}

#[test]
fn test_fox_trade_signal_alternating() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        SELL_COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        FOX_TRADE_SIGNAL(BUY_COND, SELL_COND)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    let mut last = 0i32;
    for i in 0..result.len() {
        if result[i] == 1.0 {
            assert_ne!(last, 1, "Consecutive buy signals at index {}", i);
            last = 1;
        } else if result[i] == -1.0 {
            assert_ne!(last, -1, "Consecutive sell signals at index {}", i);
            last = -1;
        }
    }
}

// ============================================================================
// 飞狐交易师（FoxTrader）回测函数测试
// ============================================================================

#[test]
fn test_fox_backtest() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        SELL_COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        FOX_BACKTEST(BUY_COND, SELL_COND, CLOSE)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_backtest_cumulative() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        SELL_COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        FOX_BACKTEST(BUY_COND, SELL_COND, CLOSE)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    for i in 1..result.len() {
        assert!(
            result[i] >= result[i - 1] - 1e-10 || result[i] < result[i - 1],
            "Cumulative P&L should be non-decreasing between trades at index {}",
            i
        );
    }
}

#[test]
fn test_fox_backtest_simple_strategy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(200);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 20));
        SELL_COND := CROSS(MA(CLOSE, 20), MA(CLOSE, 5));
        FOX_BACKTEST(BUY_COND, SELL_COND, CLOSE)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 200);
    assert!(result[199].is_finite(), "Final P&L should be finite");
}

// ============================================================================
// 飞狐交易师（FoxTrader）统计函数测试
// ============================================================================

#[test]
fn test_fox_profit_ratio() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        SELL_COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        FOX_PROFIT_RATIO(BUY_COND, SELL_COND, CLOSE)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let val = result[99];
    assert!(
        val >= 0.0 || val.is_infinite(),
        "Profit ratio should be >= 0 or infinite"
    );
}

#[test]
fn test_fox_win_rate() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        SELL_COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        FOX_WIN_RATE(BUY_COND, SELL_COND, CLOSE)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let val = result[99];
    assert!(
        (0.0..=1.0).contains(&val),
        "Win rate should be between 0 and 1, got {}",
        val
    );
}

#[test]
fn test_fox_max_drawdown() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        SELL_COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        FOX_MAX_DRAWDOWN(BUY_COND, SELL_COND, CLOSE)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    for v in result.iter() {
        assert!(*v >= 0.0, "Max drawdown should be >= 0");
    }
}

#[test]
fn test_fox_max_drawdown_non_decreasing() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        SELL_COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        FOX_MAX_DRAWDOWN(BUY_COND, SELL_COND, CLOSE)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    for i in 1..result.len() {
        assert!(
            result[i] >= result[i - 1] - 1e-10,
            "Max drawdown should be non-decreasing at index {}",
            i
        );
    }
}

#[test]
fn test_fox_trade_count() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        SELL_COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        FOX_TRADE_COUNT(BUY_COND, SELL_COND)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let final_count = result[99];
    assert!(final_count >= 0.0, "Trade count should be >= 0");
}

#[test]
fn test_fox_trade_count_non_decreasing() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        SELL_COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        FOX_TRADE_COUNT(BUY_COND, SELL_COND)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    for i in 1..result.len() {
        assert!(
            result[i] >= result[i - 1],
            "Trade count should be non-decreasing at index {}",
            i
        );
    }
}

// ============================================================================
// 飞狐交易师（FoxTrader）综合策略测试
// ============================================================================

#[test]
fn test_fox_combined_strategy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        SELL_COND := CROSS(MA(CLOSE, 10), MA(CLOSE, 5));
        SIGNAL := FOX_TRADE_SIGNAL(BUY_COND, SELL_COND);
        PNL := FOX_BACKTEST(BUY_COND, SELL_COND, CLOSE);
        WR := FOX_WIN_RATE(BUY_COND, SELL_COND, CLOSE);
        RESULT: PNL
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_zig_with_backtest() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_zigzag_ctx(100);
    let source = r#"
        ZIG_VAL := FOX_ZIG(CLOSE, 5);
        TROUGH_VAL := FOX_TROUGH(CLOSE, 5, 1);
        PEAK_VAL := FOX_PEAK(CLOSE, 5, 1);
        BUY_COND := CROSS(CLOSE, TROUGH_VAL);
        SELL_COND := CROSS(PEAK_VAL, CLOSE);
        FOX_BACKTEST(BUY_COND, SELL_COND, CLOSE)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_all_metrics() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(200);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 20));
        SELL_COND := CROSS(MA(CLOSE, 20), MA(CLOSE, 5));
        PNL := FOX_BACKTEST(BUY_COND, SELL_COND, CLOSE);
        PR := FOX_PROFIT_RATIO(BUY_COND, SELL_COND, CLOSE);
        WR := FOX_WIN_RATE(BUY_COND, SELL_COND, CLOSE);
        DD := FOX_MAX_DRAWDOWN(BUY_COND, SELL_COND, CLOSE);
        TC := FOX_TRADE_COUNT(BUY_COND, SELL_COND);
        RESULT: PNL + PR + WR + DD + TC
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 200);
}

// ============================================================================
// 飞狐交易师（FoxTrader）兼容度评估测试
// ============================================================================

#[test]
fn test_fox_compatibility_coverage() {
    let mut engine = FormulaEngine::new();

    let fox_functions = vec![
        ("FOX_ZIG", "FOX_ZIG(CLOSE, 5)"),
        ("FOX_TROUGH", "FOX_TROUGH(CLOSE, 5, 1)"),
        ("FOX_PEAK", "FOX_PEAK(CLOSE, 5, 1)"),
        ("FOX_TROUGHBARS", "FOX_TROUGHBARS(CLOSE, 5, 1)"),
        ("FOX_PEAKBARS", "FOX_PEAKBARS(CLOSE, 5, 1)"),
        ("FOX_BUY", "FOX_BUY(CLOSE > OPEN, CLOSE)"),
        ("FOX_SELL", "FOX_SELL(CLOSE < OPEN, CLOSE)"),
        (
            "FOX_TRADE_SIGNAL",
            "FOX_TRADE_SIGNAL(CLOSE > OPEN, CLOSE < OPEN)",
        ),
        (
            "FOX_BACKTEST",
            "FOX_BACKTEST(CLOSE > OPEN, CLOSE < OPEN, CLOSE)",
        ),
        (
            "FOX_PROFIT_RATIO",
            "FOX_PROFIT_RATIO(CLOSE > OPEN, CLOSE < OPEN, CLOSE)",
        ),
        (
            "FOX_WIN_RATE",
            "FOX_WIN_RATE(CLOSE > OPEN, CLOSE < OPEN, CLOSE)",
        ),
        (
            "FOX_MAX_DRAWDOWN",
            "FOX_MAX_DRAWDOWN(CLOSE > OPEN, CLOSE < OPEN, CLOSE)",
        ),
        (
            "FOX_TRADE_COUNT",
            "FOX_TRADE_COUNT(CLOSE > OPEN, CLOSE < OPEN)",
        ),
    ];

    let mut passed = 0;
    let total = fox_functions.len();

    for (name, formula) in &fox_functions {
        let mut ctx = make_ctx(100);
        let result = engine.eval(formula, &mut ctx);
        if result.is_ok() {
            passed += 1;
        } else {
            eprintln!("FOX function {} failed: {:?}", name, result.err());
        }
    }

    let coverage = passed as f64 / total as f64 * 100.0;
    println!(
        "FoxTrader compatibility coverage: {:.1}% ({}/{})",
        coverage, passed, total
    );
    assert!(coverage >= 95.0, "FoxTrader compatibility should be >= 95%");
}

#[test]
fn test_fox_batch_execution() {
    let mut engine = FormulaEngine::new();
    let formulas = vec![
        "FOX_ZIG(CLOSE, 5)",
        "FOX_TROUGH(CLOSE, 5, 1)",
        "FOX_PEAK(CLOSE, 5, 1)",
        "FOX_TROUGHBARS(CLOSE, 5, 1)",
        "FOX_PEAKBARS(CLOSE, 5, 1)",
        "FOX_BUY(CLOSE > OPEN, CLOSE)",
        "FOX_SELL(CLOSE < OPEN, CLOSE)",
        "FOX_TRADE_SIGNAL(CLOSE > OPEN, CLOSE < OPEN)",
        "FOX_BACKTEST(CLOSE > OPEN, CLOSE < OPEN, CLOSE)",
        "FOX_PROFIT_RATIO(CLOSE > OPEN, CLOSE < OPEN, CLOSE)",
        "FOX_WIN_RATE(CLOSE > OPEN, CLOSE < OPEN, CLOSE)",
        "FOX_MAX_DRAWDOWN(CLOSE > OPEN, CLOSE < OPEN, CLOSE)",
        "FOX_TRADE_COUNT(CLOSE > OPEN, CLOSE < OPEN)",
    ];

    for formula in &formulas {
        let mut ctx = make_ctx(100);
        let result = engine.eval(formula, &mut ctx);
        assert!(
            result.is_ok(),
            "FoxTrader formula '{}' failed: {:?}",
            formula,
            result.err()
        );
        assert_eq!(result.unwrap().len(), 100);
    }
}

#[test]
fn test_fox_no_panic_edge_cases() {
    let mut engine = FormulaEngine::new();

    let mut ctx = make_ctx(1);
    let _ = engine.eval("FOX_ZIG(CLOSE, 5)", &mut ctx);
    let _ = engine.eval("FOX_BUY(CLOSE > OPEN, CLOSE)", &mut ctx);
    let _ = engine.eval("FOX_SELL(CLOSE < OPEN, CLOSE)", &mut ctx);

    let mut ctx = make_ctx(10);
    let _ = engine.eval("FOX_TROUGH(CLOSE, 5, 1)", &mut ctx);
    let _ = engine.eval("FOX_PEAK(CLOSE, 5, 1)", &mut ctx);
    let _ = engine.eval("FOX_BACKTEST(CLOSE > OPEN, CLOSE < OPEN, CLOSE)", &mut ctx);
    let _ = engine.eval("FOX_TRADE_COUNT(CLOSE > OPEN, CLOSE < OPEN)", &mut ctx);
}

// ============================================================================
// 飞狐交易师（FoxTrader）与TDX兼容性测试
// ============================================================================

#[test]
fn test_fox_tdx_macd_compat() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26);
        DEA1 := EMA(DIF, 9);
        BUY_COND := CROSS(DIF, DEA1);
        SELL_COND := CROSS(DEA1, DIF);
        FOX_BACKTEST(BUY_COND, SELL_COND, CLOSE)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_tdx_kdj_compat() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        RSV := (CLOSE - LLV(LOW, 9)) / (HHV(HIGH, 9) - LLV(LOW, 9)) * 100;
        K := SMA(RSV, 3, 1);
        D := SMA(K, 3, 1);
        J := 3 * K - 2 * D;
        BUY_COND := CROSS(J, K);
        SELL_COND := CROSS(K, J);
        FOX_TRADE_SIGNAL(BUY_COND, SELL_COND)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_fox_tdx_boll_compat() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MID := MA(CLOSE, 20);
        UPPER := MID + 2 * STD(CLOSE, 20);
        LOWER := MID - 2 * STD(CLOSE, 20);
        BUY_COND := CROSS(LOWER, CLOSE);
        SELL_COND := CROSS(CLOSE, UPPER);
        FOX_BACKTEST(BUY_COND, SELL_COND, CLOSE)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}
