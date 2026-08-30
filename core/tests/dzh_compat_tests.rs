use ndarray::Array1;
use finkit::formula::engine::FormulaEngine;
use finkit::formula::types::{BlockData, FormulaContext, MoneyFlowData};

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

fn make_ctx_with_block_data(len: usize) -> FormulaContext {
    let ctx = make_ctx(len);
    let mut block_data = BlockData::default();

    let index_close: Vec<f64> = (0..len).map(|i| 3000.0 + i as f64 * 5.0).collect();
    block_data
        .index_close
        .insert("科技板块".to_string(), Array1::from_vec(index_close.clone()));
    block_data
        .index_close
        .insert("金融板块".to_string(), Array1::from_vec(index_close.clone()));

    let avg_price: Vec<f64> = (0..len).map(|i| 15.0 + i as f64 * 0.1).collect();
    block_data
        .avg_price
        .insert("科技板块".to_string(), Array1::from_vec(avg_price.clone()));
    block_data
        .avg_price
        .insert("金融板块".to_string(), Array1::from_vec(avg_price.clone()));

    let pct_change: Vec<f64> = (0..len).map(|i| (i as f64 * 0.5).sin() * 3.0).collect();
    block_data
        .pct_change
        .insert("科技板块".to_string(), Array1::from_vec(pct_change.clone()));

    let vol: Vec<f64> = (0..len).map(|i| 500000.0 + i as f64 * 1000.0).collect();
    block_data
        .volume
        .insert("科技板块".to_string(), Array1::from_vec(vol.clone()));

    let amt: Vec<f64> = (0..len).map(|i| 10000000.0 + i as f64 * 50000.0).collect();
    block_data
        .amount
        .insert("科技板块".to_string(), Array1::from_vec(amt.clone()));

    ctx.with_block_data(block_data)
}

fn make_ctx_with_money_flow(len: usize) -> FormulaContext {
    let ctx = make_ctx(len);
    let mf = MoneyFlowData {
        main_inflow: Array1::from_vec((0..len).map(|i| (i as f64 * 0.3).sin() * 500.0).collect()),
        super_big_inflow: Array1::from_vec(
            (0..len).map(|i| (i as f64 * 0.2).sin() * 200.0).collect(),
        ),
        big_inflow: Array1::from_vec((0..len).map(|i| (i as f64 * 0.25).sin() * 150.0).collect()),
        medium_inflow: Array1::from_vec(
            (0..len).map(|i| (i as f64 * 0.35).sin() * 100.0).collect(),
        ),
        small_inflow: Array1::from_vec(
            (0..len).map(|i| (i as f64 * 0.4).sin() * 50.0).collect(),
        ),
        main_inflow_pct: Array1::from_vec(
            (0..len).map(|i| (i as f64 * 0.3).sin() * 10.0).collect(),
        ),
        big_order_pct: Array1::from_vec(
            (0..len).map(|i| 20.0 + (i as f64 * 0.2).sin() * 5.0).collect(),
        ),
        small_order_pct: Array1::from_vec(
            (0..len).map(|i| 30.0 + (i as f64 * 0.25).sin() * 8.0).collect(),
        ),
        money_flow: Array1::from_vec(
            (0..len).map(|i| (i as f64 * 0.3).sin() * 1000.0).collect(),
        ),
    };
    ctx.with_money_flow_data(mf)
}

// ============================================================================
// 大智慧（DZH）板块引用函数测试
// ============================================================================

#[test]
fn test_dzh_blockindex() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_block_data(100);
    let result = engine.eval("BLOCKINDEX('科技板块')", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(!result[99].is_nan());
    assert!(result[99] > 3000.0);
}

#[test]
fn test_dzh_blockavg() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_block_data(100);
    let result = engine.eval("BLOCKAVG('科技板块')", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(!result[99].is_nan());
    assert!(result[99] > 15.0);
}

#[test]
fn test_dzh_blockdata_index() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_block_data(100);
    let result = engine.eval("BLOCKDATA('科技板块', 'INDEX')", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(!result[99].is_nan());
}

#[test]
fn test_dzh_blockdata_avg() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_block_data(100);
    let result = engine.eval("BLOCKDATA('科技板块', 'AVG')", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(!result[99].is_nan());
}

#[test]
fn test_dzh_blockdata_pct() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_block_data(100);
    let result = engine.eval("BLOCKDATA('科技板块', 'PCT')", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_blockdata_vol() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_block_data(100);
    let result = engine.eval("BLOCKDATA('科技板块', 'VOL')", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result[99] > 0.0);
}

#[test]
fn test_dzh_blockdata_amount() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_block_data(100);
    let result = engine.eval("BLOCKDATA('科技板块', 'AMOUNT')", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result[99] > 0.0);
}

#[test]
fn test_dzh_blockdata_missing_block() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_block_data(100);
    let result = engine.eval("BLOCKINDEX('未知板块')", &mut ctx);
    assert!(result.is_err());
}

#[test]
fn test_dzh_block_data_not_set() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("BLOCKINDEX('科技板块')", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result.iter().all(|v| v.is_nan()));
}

// ============================================================================
// 大智慧（DZH）资金流向函数测试
// ============================================================================

#[test]
fn test_dzh_moneyflow() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_money_flow(100);
    let result = engine.eval("MONEYFLOW()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_netinflow_default() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_money_flow(100);
    let result = engine.eval("NETINFLOW()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_netinflow_level_0() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_money_flow(100);
    let result = engine.eval("NETINFLOW(0)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_netinflow_level_1() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_money_flow(100);
    let result = engine.eval("NETINFLOW(1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_netinflow_level_2() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_money_flow(100);
    let result = engine.eval("NETINFLOW(2)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_bigorder() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_money_flow(100);
    let result = engine.eval("BIGORDER()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let v = result[99];
    assert!((0.0..=100.0).contains(&v));
}

#[test]
fn test_dzh_smallorder() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_money_flow(100);
    let result = engine.eval("SMALLORDER()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let v = result[99];
    assert!((0.0..=100.0).contains(&v));
}

#[test]
fn test_dzh_maininflow() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_money_flow(100);
    let result = engine.eval("MAININFLOW()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_maininflowpct() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_money_flow(100);
    let result = engine.eval("MAININFLOWPCT()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_superbigorder() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_money_flow(100);
    let result = engine.eval("SUPERBIGORDER()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_money_flow_not_set() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("MONEYFLOW()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result.iter().all(|v| v.is_nan()));
}

// ============================================================================
// 大智慧（DZH）特有统计函数测试
// ============================================================================

#[test]
fn test_dzh_sumbars() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("SUMBARS(VOL, 5000)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_sumbars_dynamic() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        TARGET := MA(VOL, 10);
        BARS := SUMBARS(VOL, TARGET);
        RESULT: BARS
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_intpart() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("INTPART(CLOSE)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for v in result.iter() {
        if !v.is_nan() {
            assert_eq!(v.fract(), 0.0);
        }
    }
}

#[test]
fn test_dzh_fracpart() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("FRACPART(CLOSE)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for v in result.iter() {
        if !v.is_nan() {
            assert!(*v >= 0.0 && *v < 1.0);
        }
    }
}

#[test]
fn test_dzh_int_frac_sum() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = r#"
        I := INTPART(CLOSE);
        F := FRACPART(CLOSE);
        SUM := I + F;
        RESULT: SUM
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    let close = ctx.close.as_slice().unwrap();
    for i in 0..50 {
        if !result[i].is_nan() && !close[i].is_nan() {
            assert!((result[i] - close[i]).abs() < 1e-10);
        }
    }
}

#[test]
fn test_dzh_mod() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("MOD(CLOSE, 3)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_dzh_reverse() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("REVERSE(CLOSE)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    let close = ctx.close.as_slice().unwrap();
    for i in 0..50 {
        assert!((result[i] - close[49 - i]).abs() < 1e-10);
    }
}

#[test]
fn test_dzh_tr() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("TR()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result[0] > 0.0);
}

// ============================================================================
// 大智慧（DZH）综合策略公式测试
// ============================================================================

#[test]
fn test_dzh_block_strategy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_block_data(100);
    let source = r#"
        TECH_INDEX := BLOCKINDEX('科技板块');
        TECH_AVG := BLOCKAVG('科技板块');
        FIN_INDEX := BLOCKINDEX('金融板块');
        RELATIVE := TECH_INDEX / FIN_INDEX;
        RESULT: RELATIVE
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_money_flow_strategy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_money_flow(100);
    let source = r#"
        MF := MONEYFLOW();
        NI := NETINFLOW();
        BO := BIGORDER();
        SO := SMALLORDER();
        SIGNAL := IF(NI > 0, 1, 0);
        RESULT: SIGNAL
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_combined_block_money() {
    let mut engine = FormulaEngine::new();
    let ctx = make_ctx_with_block_data(100);
    let ctx = ctx.with_money_flow_data(MoneyFlowData {
        main_inflow: Array1::from_vec((0..100).map(|i| (i as f64 * 0.3).sin() * 500.0).collect()),
        super_big_inflow: Array1::zeros(100),
        big_inflow: Array1::zeros(100),
        medium_inflow: Array1::zeros(100),
        small_inflow: Array1::zeros(100),
        main_inflow_pct: Array1::zeros(100),
        big_order_pct: Array1::zeros(100),
        small_order_pct: Array1::zeros(100),
        money_flow: Array1::zeros(100),
    });
    let mut ctx = ctx;
    let source = r#"
        TECH := BLOCKINDEX('科技板块');
        MF := MAININFLOW();
        COMBINED := TECH + MF;
        RESULT: COMBINED
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_block_pct_change_strategy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_block_data(100);
    let source = r#"
        PCT := BLOCKDATA('科技板块', 'PCT');
        MA_PCT := MA(PCT, 5);
        TREND_UP := PCT > MA_PCT;
        RESULT: TREND_UP
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// ============================================================================
// 大智慧（DZH）与TDX兼容性测试
// ============================================================================

#[test]
fn test_dzh_tdx_macd_compat() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26);
        DEA1 := EMA(DIF, 9);
        MACD1 := (DIF - DEA1) * 2;
        RESULT: MACD1
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_tdx_kdj_compat() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        RSV := (CLOSE - LLV(LOW, 9)) / (HHV(HIGH, 9) - LLV(LOW, 9)) * 100;
        K := SMA(RSV, 3, 1);
        D := SMA(K, 3, 1);
        J := 3 * K - 2 * D;
        RESULT: J
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_tdx_boll_compat() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MID := MA(CLOSE, 20);
        UPPER := MID + 2 * STD(CLOSE, 20);
        LOWER := MID - 2 * STD(CLOSE, 20);
        WIDTH := (UPPER - LOWER) / MID * 100;
        RESULT: WIDTH
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// ============================================================================
// 大智慧（DZH）绘图函数兼容性测试
// ============================================================================

#[test]
fn test_dzh_draw_line_compat() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MA5 := MA(CLOSE, 5);
        MA10 := MA(CLOSE, 10);
        DRAWLINE(CROSS(MA5, MA10), MA5, CROSS(MA10, MA5), MA10, 1);
        RESULT: MA5
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_draw_icon_compat() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        GOLDEN := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        DRAWICON(GOLDEN, LOW, 1);
        RESULT: GOLDEN
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_dzh_stick_line_compat() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        UP := CLOSE > REF(CLOSE, 1);
        STICKLINE(UP, CLOSE, OPEN, 8, TRUE);
        RESULT: UP
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// ============================================================================
// 大智慧（DZH）块注释语法测试
// ============================================================================

#[test]
fn test_dzh_block_comment_syntax() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = r#"
        {这是大智慧风格的块注释}
        MA5 := MA(CLOSE, 5);
        {另一个块注释}
        MA10 := MA(CLOSE, 10);
        RESULT: MA5 + MA10
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_dzh_nested_block_comment() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = r#"
        {外层注释}
        {内层注释}
        RESULT: MA(CLOSE, 5)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

// ============================================================================
// DZH 兼容度评估测试
// ============================================================================

#[test]
fn test_dzh_compatibility_coverage() {
    let mut engine = FormulaEngine::new();

    let dzh_functions = vec![
        ("BLOCKINDEX", true),
        ("BLOCKAVG", true),
        ("BLOCKDATA", true),
        ("MONEYFLOW", true),
        ("NETINFLOW", true),
        ("BIGORDER", true),
        ("SMALLORDER", true),
        ("SUMBARS", true),
        ("INTPART", true),
        ("FRACPART", true),
        ("MOD", true),
        ("REVERSE", true),
        ("TR", true),
        ("MA", true),
        ("EMA", true),
        ("SMA", true),
        ("WMA", true),
        ("HHV", true),
        ("LLV", true),
        ("REF", true),
        ("CROSS", true),
        ("IF", true),
        ("COUNT", true),
        ("SUM", true),
        ("STD", true),
        ("RSI", true),
        ("MACD", true),
        ("KDJ", true),
        ("BOLL", true),
        ("ATR", true),
        ("VOL", true),
        ("CLOSE", true),
        ("OPEN", true),
        ("HIGH", true),
        ("LOW", true),
    ];

    let mut passed = 0;
    let total = dzh_functions.len();

    for (func, _) in &dzh_functions {
        let mut ctx = make_ctx_with_block_data(50);
        let formula = if *func == "BLOCKINDEX" || *func == "BLOCKAVG" {
            format!("{}('科技板块')", func)
        } else if *func == "BLOCKDATA" {
            format!("{}('科技板块', 'INDEX')", func)
        } else if *func == "MONEYFLOW"
            || *func == "BIGORDER"
            || *func == "SMALLORDER"
            || *func == "MAININFLOW"
            || *func == "TR"
        {
            format!("{}()", func)
        } else if *func == "NETINFLOW" {
            format!("{}(0)", func)
        } else if *func == "SUMBARS" {
            format!("{}(VOL, 5000)", func)
        } else if *func == "MOD" {
            format!("{}(CLOSE, 3)", func)
        } else if *func == "VOL" {
            "VOL".to_string()
        } else if *func == "CLOSE" {
            "CLOSE".to_string()
        } else if *func == "OPEN" {
            "OPEN".to_string()
        } else if *func == "HIGH" {
            "HIGH".to_string()
        } else if *func == "LOW" {
            "LOW".to_string()
        } else if *func == "MACD" {
            format!("{}(CLOSE, 12)", func)
        } else if *func == "KDJ" {
            format!("{}(CLOSE, HIGH, LOW)", func)
        } else if *func == "BOLL" {
            format!("{}(CLOSE, 20)", func)
        } else if *func == "ATR" {
            format!("{}(HIGH, LOW, CLOSE, 14)", func)
        } else if *func == "INTPART" || *func == "FRACPART" || *func == "REVERSE" {
            format!("{}(CLOSE)", func)
        } else if *func == "SMA" {
            format!("{}(CLOSE, 5, 1)", func)
        } else if *func == "IF" {
            format!("{}(CLOSE > OPEN, CLOSE, OPEN)", func)
        } else {
            format!("{}(CLOSE, 10)", func)
        };

        let result = engine.eval(&formula, &mut ctx);
        if result.is_ok() {
            passed += 1;
        }
    }

    let coverage = passed as f64 / total as f64 * 100.0;
    println!("DZH compatibility coverage: {:.1}% ({}/{})", coverage, passed, total);
    assert!(coverage >= 95.0, "DZH compatibility should be >= 95%");
}

#[test]
fn test_dzh_batch_execution() {
    let mut engine = FormulaEngine::new();
    let formulas = vec![
        "MA(CLOSE, 5)",
        "EMA(CLOSE, 12)",
        "SMA(CLOSE, 5, 1)",
        "WMA(CLOSE, 10)",
        "HHV(HIGH, 20)",
        "LLV(LOW, 20)",
        "REF(CLOSE, 5)",
        "CROSS(MA(CLOSE, 5), MA(CLOSE, 10))",
        "SUMBARS(VOL, 5000)",
        "INTPART(CLOSE)",
        "FRACPART(CLOSE)",
        "MOD(CLOSE, 3)",
        "REVERSE(CLOSE)",
        "TR()",
        "RSI(CLOSE, 14)",
        "MACD(CLOSE, 12)",
        "KDJ(CLOSE, HIGH, LOW)",
        "BOLL(CLOSE, 20)",
        "ATR(HIGH, LOW, CLOSE, 14)",
        "STD(CLOSE, 20)",
    ];

    for formula in &formulas {
        let mut ctx = make_ctx(100);
        let result = engine.eval(formula, &mut ctx);
        assert!(
            result.is_ok(),
            "DZH formula '{}' failed: {:?}",
            formula,
            result.err()
        );
        assert_eq!(result.unwrap().len(), 100);
    }
}

#[test]
fn test_dzh_no_panic_edge_cases() {
    let mut engine = FormulaEngine::new();

    let mut ctx = make_ctx(1);
    let _ = engine.eval("MA(CLOSE, 5)", &mut ctx);
    let _ = engine.eval("SUMBARS(VOL, 100)", &mut ctx);
    let _ = engine.eval("INTPART(CLOSE)", &mut ctx);

    let mut ctx = make_ctx(10);
    let _ = engine.eval("HHV(HIGH, 20)", &mut ctx);
    let _ = engine.eval("LLV(LOW, 20)", &mut ctx);
    let _ = engine.eval("REVERSE(CLOSE)", &mut ctx);

    let close = Array1::from_vec(vec![0.0; 20]);
    let open = close.clone();
    let high = close.clone();
    let low = close.clone();
    let vol = close.clone();
    let mut ctx = FormulaContext::new(open, high, low, close, vol, None);
    let _ = engine.eval("TR()", &mut ctx);
    let _ = engine.eval("MOD(CLOSE, 3)", &mut ctx);
}