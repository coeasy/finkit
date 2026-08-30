use ndarray::Array1;
use finkit::formula::{FormulaContext, FormulaEngine, ParamValues};

fn make_test_ctx(len: usize) -> FormulaContext {
    let open = Array1::from_vec((0..len).map(|i| 100.0 + i as f64 * 0.5).collect());
    let high = Array1::from_vec((0..len).map(|i| 105.0 + i as f64 * 0.7).collect());
    let low = Array1::from_vec((0..len).map(|i| 95.0 + i as f64 * 0.3).collect());
    let close = Array1::from_vec((0..len).map(|i| 102.0 + i as f64 * 0.6).collect());
    let volume = Array1::from_vec((0..len).map(|i| 10000.0 + i as f64 * 100.0).collect());
    FormulaContext::new(open, high, low, close, volume, None)
}

/// Test 1: MA Golden Cross (金叉)
#[test]
fn test_ma_golden_cross() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);

    let source = r#"
        MA5 := MA(CLOSE, 5);
        MA10 := MA(CLOSE, 10);
        CROSS(MA5, MA10)
    "#;

    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!(ctx.variables.contains_key("MA5"));
    assert!(ctx.variables.contains_key("MA10"));
}

/// Test 2: MA Death Cross (死叉)
#[test]
fn test_ma_death_cross() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);

    let source = r#"
        MA5 := MA(CLOSE, 5);
        MA10 := MA(CLOSE, 10);
        CROSSBELOW(MA5, MA10)
    "#;

    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

/// Test 3: MACD Formula
#[test]
fn test_macd_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);

    let source = r#"
        DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26);
        DEA := EMA(DIF, 9);
        MACD := (DIF - DEA) * 2;
        MACD
    "#;

    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!(ctx.variables.contains_key("DIF"));
    assert!(ctx.variables.contains_key("DEA"));
    assert!(ctx.variables.contains_key("MACD"));
}

/// Test 4: KDJ Formula
#[test]
fn test_kdj_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);

    let source = r#"
        RSV := (CLOSE - LLV(LOW, 9)) / (HHV(HIGH, 9) - LLV(LOW, 9)) * 100;
        K := EMA(RSV, 3);
        D := EMA(K, 3);
        J := 3 * K - 2 * D;
        J
    "#;

    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!(ctx.variables.contains_key("RSV"));
    assert!(ctx.variables.contains_key("K"));
    assert!(ctx.variables.contains_key("D"));
    assert!(ctx.variables.contains_key("J"));
}

/// Test 5: Bollinger Bands (布林带)
#[test]
fn test_bollinger_bands() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);

    let source = r#"
        MID := MA(CLOSE, 20);
        STD_VAL := STD(CLOSE, 20);
        UPPER := MID + 2 * STD_VAL;
        LOWER := MID - 2 * STD_VAL;
        UPPER
    "#;

    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!(ctx.variables.contains_key("MID"));
    assert!(ctx.variables.contains_key("UPPER"));
    assert!(ctx.variables.contains_key("LOWER"));
}

/// Test 6: RSI Overbought/Oversold (RSI超买超卖)
#[test]
fn test_rsi_overbought_oversold() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);

    let source = r#"
        CHANGE := CLOSE - REF(CLOSE, 1);
        UP := IF(CHANGE > 0, CHANGE, 0);
        DOWN := IF(CHANGE < 0, -CHANGE, 0);
        RS := SUM(UP, 6) / SUM(DOWN, 6);
        RSI := RS / (1 + RS) * 100;
        OVERBOUGHT := RSI > 80;
        OVERSOLD := RSI < 20;
        OVERBOUGHT
    "#;

    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!(ctx.variables.contains_key("RSI"));
    assert!(ctx.variables.contains_key("OVERBOUGHT"));
    assert!(ctx.variables.contains_key("OVERSOLD"));
}

/// Test 7: Parameter Formula
#[test]
fn test_param_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(30);

    let source = "PARAMS: N(1, 100, 10); MA_N: MA(CLOSE, N)";

    let param_defs = {
        let formula = engine.compile(source).unwrap();
        engine.get_param_defs(&formula).unwrap()
    };
    assert_eq!(param_defs.len(), 1);
    assert_eq!(param_defs[0].name, "N");
    assert_eq!(param_defs[0].default, 10.0);

    let mut params = ParamValues::new();
    params.insert("N".to_string(), 20.0);

    let result = engine.eval_with_params(source, &mut ctx, &params).unwrap();
    assert_eq!(result.len(), 30);
}

/// Test 8: Parameter Validation
#[test]
fn test_param_validation() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(30);
    let source = "PARAMS: N(1, 100, 20); MA_N: MA(CLOSE, N)";

    let mut valid_params = ParamValues::new();
    valid_params.insert("N".to_string(), 50.0);
    assert!(engine
        .eval_with_validation(source, &mut ctx, &valid_params)
        .is_ok());

    let mut invalid_params = ParamValues::new();
    invalid_params.insert("N".to_string(), 150.0);
    assert!(engine
        .eval_with_validation(source, &mut ctx, &invalid_params)
        .is_err());
}

/// Test 9: Default Parameters
#[test]
fn test_default_params() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(30);
    let source = "PARAMS: N(1, 100, 20); MA_N: MA(CLOSE, N)";

    let result = engine.eval_with_defaults(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 30);
}

/// Test 10: Cache Hit
#[test]
fn test_cache_hit() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(10);
    let source = "CLOSE + OPEN";

    engine.eval(source, &mut ctx).unwrap();
    assert!(engine.cache_hit(source));
    assert_eq!(engine.cache_size(), 1);
}

/// Test 11: Cache Multiple Formulas
#[test]
fn test_cache_multiple_formulas() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(10);

    engine.eval("CLOSE + 1", &mut ctx).unwrap();
    engine.eval("CLOSE + 2", &mut ctx).unwrap();
    engine.eval("CLOSE + 3", &mut ctx).unwrap();

    assert_eq!(engine.cache_size(), 3);
    assert!(engine.cache_hit("CLOSE + 1"));
    assert!(engine.cache_hit("CLOSE + 2"));
    assert!(engine.cache_hit("CLOSE + 3"));
}

/// Test 12: Cache Clear
#[test]
fn test_cache_clear() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(10);

    engine.eval("CLOSE + 1", &mut ctx).unwrap();
    engine.eval("CLOSE + 2", &mut ctx).unwrap();
    assert_eq!(engine.cache_size(), 2);

    engine.clear_cache();
    assert_eq!(engine.cache_size(), 0);
    assert!(!engine.cache_hit("CLOSE + 1"));
}

/// Test 13: Error Handling - Invalid Formula
#[test]
fn test_error_invalid_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(10);

    let result = engine.eval("CLOSE +", &mut ctx);
    assert!(result.is_err());
}

/// Test 14: Error Handling - Unknown Variable
#[test]
fn test_error_unknown_variable() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(10);

    let result = engine.eval("UNKNOWN_VAR", &mut ctx);
    assert!(result.is_err());
}

/// Test 15: Error Handling - Unknown Function
#[test]
fn test_error_unknown_function() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(10);

    let result = engine.eval("UNKNOWN_FUNC(CLOSE)", &mut ctx);
    assert!(result.is_err());
}

/// Test 16: Complex Trading Strategy
#[test]
fn test_complex_trading_strategy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(100);

    let source = r#"
        SHORT := 12;
        LONG := 26;
        MID := 9;
        DIF := EMA(CLOSE, SHORT) - EMA(CLOSE, LONG);
        DEA := EMA(DIF, MID);
        MACD := (DIF - DEA) * 2;
        BUY := CROSS(DIF, DEA) AND MACD > 0;
        SELL := CROSSBELOW(DIF, DEA) AND MACD < 0;
        BUY
    "#;

    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(ctx.variables.contains_key("DIF"));
    assert!(ctx.variables.contains_key("DEA"));
    assert!(ctx.variables.contains_key("MACD"));
    assert!(ctx.variables.contains_key("BUY"));
    assert!(ctx.variables.contains_key("SELL"));
}

/// Test 17: Multiple Parameters Formula
#[test]
fn test_multi_param_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);

    let source = r#"
        PARAMS: SHORT(5, 100, 12), LONG(5, 200, 26);
        DIF := EMA(CLOSE, SHORT) - EMA(CLOSE, LONG);
        DIF
    "#;

    let formula = engine.compile(source).unwrap();
    let param_defs = engine.get_param_defs(&formula).unwrap();
    assert_eq!(param_defs.len(), 2);

    let mut params = ParamValues::new();
    params.insert("SHORT".to_string(), 10.0);
    params.insert("LONG".to_string(), 20.0);

    let result = engine.eval_with_params(source, &mut ctx, &params).unwrap();
    assert_eq!(result.len(), 50);
}

/// Test 18: Reuse Compiled Formula
#[test]
fn test_reuse_compiled_formula() {
    let mut engine = FormulaEngine::new();
    let source = "MA5 := MA(CLOSE, 5); MA5";

    let formula = engine.compile(source).unwrap();

    let mut ctx1 = make_test_ctx(20);
    let result1 = engine.execute(&formula, &mut ctx1).unwrap();
    assert_eq!(result1.len(), 20);

    let mut ctx2 = make_test_ctx(30);
    let result2 = engine.execute(&formula, &mut ctx2).unwrap();
    assert_eq!(result2.len(), 30);
}

/// Test 19: Formula with Comparison and Logic
#[test]
fn test_comparison_and_logic() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(20);

    let source = r#"
        UP := CLOSE > OPEN;
        BIG_VOL := VOLUME > 10500;
        SIGNAL := UP AND BIG_VOL;
        SIGNAL
    "#;

    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 20);
    assert!(ctx.variables.contains_key("UP"));
    assert!(ctx.variables.contains_key("BIG_VOL"));
    assert!(ctx.variables.contains_key("SIGNAL"));
}

/// Test 20: Formula with Nested Functions
#[test]
fn test_nested_functions() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(30);

    let source = "EMA(MA(CLOSE, 5), 10)";

    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 30);
}

/// Test 21: DX formula (shorthand: auto-expand HIGH/LOW/CLOSE from context)
#[test]
fn test_dx_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let result = engine.eval("DX(CLOSE, 14)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

/// Test 22: PLUS_DI formula
#[test]
fn test_plus_di_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let result = engine.eval("PLUS_DI(CLOSE, 14)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

/// Test 23: MINUS_DI formula
#[test]
fn test_minus_di_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let result = engine.eval("MINUS_DI(CLOSE, 14)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

/// Test 24: ADXR formula
#[test]
fn test_adxr_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let result = engine.eval("ADXR(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

/// Test 25: AROONOSC formula
#[test]
fn test_aroonosc_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let result = engine.eval("AROONOSC(5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

// ==================== Phase 2.3: TDX Classic Formula Tests ====================

/// TDX: KDJ完整版
#[test]
fn test_tdx_kdj_full() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let source = r#"
        RSV := (CLOSE - LLV(LOW,9)) / (HHV(HIGH,9) - LLV(LOW,9)) * 100;
        K := SMA(RSV, 3, 1);
        D := SMA(K, 3, 1);
        J := 3 * K - 2 * D;
        J
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!(!result[49].is_nan());
}

/// TDX: MACD完整版
#[test]
fn test_tdx_macd_full() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(100);
    let source = r#"
        DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26);
        DEA := EMA(DIF, 9);
        MACD_BAR := (DIF - DEA) * 2;
        MACD_BAR
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

/// TDX: BOLL完整版
#[test]
fn test_tdx_boll_full() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let source = r#"
        MID := MA(CLOSE, 20);
        UPPER := MID + 2 * STD(CLOSE, 20);
        LOWER := MID - 2 * STD(CLOSE, 20);
        MID
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!(!result[49].is_nan());
}

/// TDX: DMI完整版
#[test]
fn test_tdx_dmi_full() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(100);
    let source = r#"
        PDI_VAL := PLUS_DI(HIGH, LOW, CLOSE, 14);
        MDI_VAL := MINUS_DI(HIGH, LOW, CLOSE, 14);
        ADX_VAL := ADX(HIGH, LOW, CLOSE, 14);
        ADX_VAL
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

/// TDX: Time functions
#[test]
fn test_tdx_time_functions() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let dt = Array1::from_vec((0..50).map(|i| 1704067200i64 + i * 86400).collect());
    ctx.datetime = Some(dt);
    let result = engine.eval("YEAR()", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert_eq!(result[0], 2024.0);
}

/// TDX: Bar counting functions
#[test]
fn test_tdx_bar_counting() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let result = engine.eval("CURRBARSCOUNT()", &mut ctx).unwrap();
    assert_eq!(result[0], 50.0);
    assert_eq!(result[49], 1.0);
    let result2 = engine.eval("TOTALBARSCOUNT()", &mut ctx).unwrap();
    assert_eq!(result2[0], 50.0);
    assert_eq!(result2[49], 50.0);
}

/// TDX: Math extensions
#[test]
fn test_tdx_math_extensions() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let result = engine.eval("AVEDEV(CLOSE, 10)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!(result[9] >= 0.0);
    let result2 = engine.eval("SLOPE(CLOSE, 10)", &mut ctx).unwrap();
    assert!(result2[9] > 0.0); // upward sloping data
}

/// TDX: FORCAST function
#[test]
fn test_tdx_forcast() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let result = engine.eval("FORCAST(CLOSE, 10)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!(!result[49].is_nan());
}

/// TDX: <> operator
#[test]
fn test_tdx_neq_operator() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let result = engine.eval("CLOSE <> OPEN", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert_eq!(result[0], 1.0); // close != open
}

/// TDX: RANGE function
#[test]
fn test_tdx_range() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let result = engine.eval("RANGE(CLOSE, LOW, HIGH)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert_eq!(result[0], 1.0); // close should be between low and high
}

/// TDX: PDI/MDI aliases
#[test]
fn test_tdx_pdi_mdi_aliases() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(50);
    let pdi = engine.eval("PDI(CLOSE, 14)", &mut ctx).unwrap();
    let plus_di = engine.eval("PLUS_DI(CLOSE, 14)", &mut ctx).unwrap();
    for i in 0..50 {
        assert!((pdi[i] - plus_di[i]).abs() < 1e-10 || (pdi[i].is_nan() && plus_di[i].is_nan()));
    }
}

// ==================== Phase 2.4: Edge Case Tests ====================

/// Edge case: Empty input should not panic
#[test]
fn test_edge_empty_input() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(1);
    let result = engine.eval("MA(CLOSE, 5)", &mut ctx);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

/// Edge case: NaN propagation in formula
#[test]
fn test_edge_nan_propagation() {
    let mut engine = FormulaEngine::new();
    let close = Array1::from_vec(vec![1.0, 2.0, f64::NAN, 4.0, 5.0]);
    let ctx_len = close.len();
    let mut ctx = FormulaContext::new(
        close.clone(), close.clone(), close.clone(), close.clone(),
        Array1::from_elem(ctx_len, 1000.0), None,
    );
    let result = engine.eval("MA(CLOSE, 3)", &mut ctx).unwrap();
    assert_eq!(result.len(), 5);
}

/// Edge case: Division by zero in formula
#[test]
fn test_edge_division_by_zero() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(10);
    let source = "CLOSE / (CLOSE - CLOSE)";
    let result = engine.eval(source, &mut ctx).unwrap();
    for val in result.iter() {
        assert!(val.is_nan() || val.is_infinite());
    }
}

/// Edge case: Unsupported function gives clear error
#[test]
fn test_edge_unsupported_function() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(10);
    let result = engine.eval("NONEXISTENT_FUNC(CLOSE, 5)", &mut ctx);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("NONEXISTENT_FUNC"));
}

/// Edge case: Very large period
#[test]
fn test_edge_large_period() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(10);
    let result = engine.eval("MA(CLOSE, 100)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
}

/// Edge case: Single bar data
#[test]
fn test_edge_single_bar() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_test_ctx(1);
    let result = engine.eval("EMA(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn test_valuewhen() {
    let mut engine = FormulaEngine::new();
    let open = Array1::from_vec(vec![10.0; 10]);
    let high = Array1::from_vec(vec![12.0; 10]);
    let low = Array1::from_vec(vec![8.0; 10]);
    let close = Array1::from_vec(vec![9.0, 11.5, 10.0, 12.0, 9.5, 13.0, 10.0, 8.0, 14.0, 11.0]);
    let volume = Array1::from_vec(vec![1000.0; 10]);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);

    let source = "VALUEWHEN(CLOSE > 11, CLOSE)";
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    assert!(result[0].is_nan());
    assert!((result[1] - 11.5).abs() < 1e-10);
    assert!((result[2] - 11.5).abs() < 1e-10);
    assert!((result[3] - 12.0).abs() < 1e-10);
    assert!((result[4] - 12.0).abs() < 1e-10);
    assert!((result[5] - 13.0).abs() < 1e-10);
    assert!((result[6] - 13.0).abs() < 1e-10);
    assert!((result[8] - 14.0).abs() < 1e-10);
}

#[test]
fn test_last() {
    let mut engine = FormulaEngine::new();
    let open = Array1::from_vec(vec![10.0; 10]);
    let high = Array1::from_vec(vec![12.0; 10]);
    let low = Array1::from_vec(vec![8.0; 10]);
    let close = Array1::from_vec(vec![11.0, 11.0, 11.0, 11.0, 9.0, 11.0, 11.0, 11.0, 11.0, 9.0]);
    let volume = Array1::from_vec(vec![1000.0; 10]);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);

    let source = "LAST(CLOSE > 10, 3, 1)";
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    assert!((result[3] - 1.0).abs() < 1e-10);
    assert!((result[5] - 0.0).abs() < 1e-10);
    assert!((result[8] - 1.0).abs() < 1e-10);
}

#[test]
fn test_barslastcount() {
    let mut engine = FormulaEngine::new();
    let open = Array1::from_vec(vec![10.0; 10]);
    let high = Array1::from_vec(vec![12.0; 10]);
    let low = Array1::from_vec(vec![8.0; 10]);
    let close = Array1::from_vec(vec![11.0, 11.0, 11.0, 9.0, 9.0, 11.0, 11.0, 11.0, 11.0, 9.0]);
    let volume = Array1::from_vec(vec![1000.0; 10]);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);

    let source = "BARSLASTCOUNT(CLOSE > 10)";
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    assert!((result[0] - 1.0).abs() < 1e-10);
    assert!((result[1] - 2.0).abs() < 1e-10);
    assert!((result[2] - 3.0).abs() < 1e-10);
    assert!((result[3] - 0.0).abs() < 1e-10);
    assert!((result[4] - 0.0).abs() < 1e-10);
    assert!((result[5] - 1.0).abs() < 1e-10);
    assert!((result[8] - 4.0).abs() < 1e-10);
    assert!((result[9] - 0.0).abs() < 1e-10);
}

#[test]
fn test_formula_zigzag_functions() {
    let mut engine = FormulaEngine::new();
    let high = Array1::from_vec(vec![
        10.0, 12.0, 11.0, 15.0, 14.0, 13.0, 9.0, 8.0, 7.0, 11.0,
        13.0, 16.0, 15.0, 14.0, 10.0, 9.0, 8.0, 12.0, 14.0, 13.0,
    ]);
    let low = Array1::from_vec(vec![
        8.0, 10.0, 9.0, 13.0, 12.0, 11.0, 7.0, 6.0, 5.0, 9.0,
        11.0, 14.0, 13.0, 12.0, 8.0, 7.0, 6.0, 10.0, 12.0, 11.0,
    ]);
    let open = Array1::from_vec(vec![9.0; 20]);
    let close = Array1::from_vec(vec![
        9.0, 11.0, 10.0, 14.0, 13.0, 12.0, 8.0, 7.0, 6.0, 10.0,
        12.0, 15.0, 14.0, 13.0, 9.0, 8.0, 7.0, 11.0, 13.0, 12.0,
    ]);
    let volume = Array1::from_vec(vec![1000.0; 20]);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);

    let result = engine.eval("ZIGZAG(CLOSE, 10)", &mut ctx).unwrap();
    assert_eq!(result.len(), 20);

    let result = engine.eval("PEAK(CLOSE, 10, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 20);

    let result = engine.eval("TROUGH(CLOSE, 10, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 20);

    let result = engine.eval("PEAKBARS(CLOSE, 10, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 20);

    let result = engine.eval("TROUGHBARS(CLOSE, 10, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 20);
}

#[test]
fn test_formula_advanced_find_functions() {
    let mut engine = FormulaEngine::new();
    let high = Array1::from_vec(vec![10.0, 15.0, 12.0, 8.0, 20.0, 11.0, 14.0, 9.0, 18.0, 13.0]);
    let low = Array1::from_vec(vec![8.0, 13.0, 10.0, 6.0, 18.0, 9.0, 12.0, 7.0, 16.0, 11.0]);
    let open = Array1::from_vec(vec![9.0; 10]);
    let close = Array1::from_vec(vec![9.0, 14.0, 11.0, 7.0, 19.0, 10.0, 13.0, 8.0, 17.0, 12.0]);
    let volume = Array1::from_vec(vec![1000.0; 10]);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);

    // FINDHIGH: find local highs within N bars with M-bar neighborhood
    let result = engine.eval("FINDHIGH(CLOSE, 10, 2, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // Bar 4 (value=19) is highest in its neighborhood
    assert!((result[4] - 1.0).abs() < 1e-10);

    // FINDLOW: find local lows within N bars with M-bar neighborhood
    let result = engine.eval("FINDLOW(CLOSE, 10, 2, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // Bar 3 (value=7) is lowest in its neighborhood
    assert!((result[3] - 1.0).abs() < 1e-10);

    // TOPN: marks top N values in entire series
    let result = engine.eval("TOPN(CLOSE, 3)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // Top 3 values are 19 (bar4), 17 (bar8), 14 (bar1)
    assert!((result[4] - 1.0).abs() < 1e-10);
    assert!((result[8] - 1.0).abs() < 1e-10);
    assert!((result[1] - 1.0).abs() < 1e-10);
    // Others should be 0
    assert!((result[0]).abs() < 1e-10);
    assert!((result[3]).abs() < 1e-10);

    // DRAWNULL: returns NaN series (used for drawing null lines)
    let result = engine.eval("DRAWNULL()", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    assert!(result[0].is_nan());
    assert!(result[5].is_nan());

    // CEILING: round up to precision
    let result = engine.eval("CEILING(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // 9.0 -> ceil(9/5)*5 = 10.0
    assert!((result[0] - 10.0).abs() < 1e-10);
    // 14.0 -> ceil(14/5)*5 = 15.0
    assert!((result[1] - 15.0).abs() < 1e-10);
    // 19.0 -> ceil(19/5)*5 = 20.0
    assert!((result[4] - 20.0).abs() < 1e-10);
}

#[test]
fn test_formula_signal_functions() {
    let mut engine = FormulaEngine::new();
    let open = Array1::from_vec(vec![10.0; 10]);
    let high = Array1::from_vec(vec![12.0; 10]);
    let low = Array1::from_vec(vec![8.0; 10]);
    // Buy signals at bars 1,2,4; Sell signals at bars 3,6,7
    let close = Array1::from_vec(vec![10.0, 12.0, 13.0, 8.0, 14.0, 11.0, 7.0, 6.0, 9.0, 10.0]);
    let volume = Array1::from_vec(vec![1000.0; 10]);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);

    // AUTOFILTER returns 1.0 series
    let result = engine.eval("AUTOFILTER()", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    assert!((result[0] - 1.0).abs() < 1e-10);
    assert!((result[5] - 1.0).abs() < 1e-10);

    // CHECKSIG: buy when CLOSE > 11, sell when CLOSE < 9, confirm mode=1
    let result = engine.eval("CHECKSIG(CLOSE > 11, CLOSE < 9, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // First buy at bar 1 (12>11)
    assert!((result[1] - 1.0).abs() < 1e-10);
    // Bar 2 (13>11) is suppressed (same direction)
    assert!((result[2]).abs() < 1e-10);
    // First sell at bar 3 (8<9)
    assert!((result[3] - (-1.0)).abs() < 1e-10);
    // Buy again at bar 4 (14>11)
    assert!((result[4] - 1.0).abs() < 1e-10);
    // Sell at bar 6 (7<9)
    assert!((result[6] - (-1.0)).abs() < 1e-10);

    // MULTSIG: allows up to 2 same-direction signals within 5 bars
    let result = engine.eval("MULTSIG(CLOSE > 11, CLOSE < 9, 5, 2)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // First buy at bar 1
    assert!((result[1] - 1.0).abs() < 1e-10);
    // Second buy at bar 2 (allowed, count=2 within N=5 bars)
    assert!((result[2] - 1.0).abs() < 1e-10);

    // ENTERLONG/EXITLONG pass-through
    let result = engine.eval("ENTERLONG(CLOSE > 11)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);

    let result = engine.eval("EXITLONG(CLOSE < 9)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
}

#[test]
fn test_formula_draw_extensions() {
    let mut engine = FormulaEngine::new();
    let open = Array1::from_vec(vec![10.0; 10]);
    let high = Array1::from_vec(vec![12.0; 10]);
    let low = Array1::from_vec(vec![8.0; 10]);
    let close = Array1::from_vec(vec![10.0, 12.0, 11.0, 8.0, 14.0, 11.0, 7.0, 6.0, 9.0, 10.0]);
    let volume = Array1::from_vec(vec![1000.0; 10]);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);

    // DRAWSL: slope line between two conditions
    let result = engine.eval("DRAWSL(CLOSE > 11, CLOSE, CLOSE < 9, CLOSE)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);

    // DRAWTEXT_FIX: fixed-position text
    let result = engine.eval("DRAWTEXT_FIX(0.5, 0.8, \"Hello\")", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);

    // DRAWNUMBER: draw number at condition
    let result = engine.eval("DRAWNUMBER(CLOSE > 11, CLOSE, CLOSE, 2)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);

    // VERTLINE: vertical line at condition
    let result = engine.eval("VERTLINE(CLOSE > 13)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);

    // Verify draw commands were added
    let cmds = ctx.draw_commands.borrow();
    assert!(cmds.commands.len() >= 4);
}

#[test]
fn test_formula_multi_output() {
    let mut engine = FormulaEngine::new();
    let open = Array1::from_vec(vec![10.0; 30]);
    let high = Array1::from_vec(vec![12.0; 30]);
    let low = Array1::from_vec(vec![8.0; 30]);
    let close = Array1::from_vec((0..30).map(|i| 10.0 + (i as f64) * 0.5).collect());
    let volume = Array1::from_vec(vec![1000.0; 30]);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);

    // MACD-style multi-output formula
    let source = "DIF: EMA(CLOSE, 12) - EMA(CLOSE, 26); DEA: EMA(DIF, 9); MACD: (DIF - DEA) * 2";
    let result = engine.eval_multi(source, &mut ctx).unwrap();

    // Should have 3 named outputs
    assert!(result.len() >= 3);
    assert!(result.get("DIF").is_some());
    assert!(result.get("DEA").is_some());
    assert!(result.get("MACD").is_some());

    // Final value should be MACD (last statement)
    let macd = result.get("MACD").unwrap();
    assert_eq!(macd.len(), 30);
    assert_eq!(result.final_value.len(), 30);

    // Single output formula still works
    let mut ctx2 = FormulaContext::new(
        Array1::from_vec(vec![10.0; 10]),
        Array1::from_vec(vec![12.0; 10]),
        Array1::from_vec(vec![8.0; 10]),
        Array1::from_vec(vec![10.0; 10]),
        Array1::from_vec(vec![1000.0; 10]),
        None,
    );
    let result2 = engine.eval_multi("MA(CLOSE, 5)", &mut ctx2).unwrap();
    assert_eq!(result2.final_value.len(), 10);
    assert!(result2.is_empty()); // no named outputs for plain expression
}

#[test]
fn test_formula_cumulative_functions() {
    let mut engine = FormulaEngine::new();
    let open = Array1::from_vec(vec![10.0; 10]);
    let high = Array1::from_vec(vec![12.0; 10]);
    let low = Array1::from_vec(vec![8.0; 10]);
    let close = Array1::from_vec(vec![1.0, 3.0, 2.0, 5.0, 4.0, 7.0, 6.0, 9.0, 8.0, 10.0]);
    let volume = Array1::from_vec(vec![1000.0; 10]);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);

    // CUMSUM: cumulative sum
    let result = engine.eval("CUMSUM(CLOSE)", &mut ctx).unwrap();
    assert!((result[0] - 1.0).abs() < 1e-10);
    assert!((result[1] - 4.0).abs() < 1e-10);  // 1+3
    assert!((result[2] - 6.0).abs() < 1e-10);  // 1+3+2
    assert!((result[9] - 55.0).abs() < 1e-10); // sum of 1..10

    // CUM alias
    let result = engine.eval("CUM(CLOSE)", &mut ctx).unwrap();
    assert!((result[9] - 55.0).abs() < 1e-10);

    // CUMMAX: cumulative maximum
    let result = engine.eval("CUMMAX(CLOSE)", &mut ctx).unwrap();
    assert!((result[0] - 1.0).abs() < 1e-10);
    assert!((result[1] - 3.0).abs() < 1e-10);
    assert!((result[3] - 5.0).abs() < 1e-10);
    assert!((result[9] - 10.0).abs() < 1e-10);

    // CUMMIN: cumulative minimum
    let result = engine.eval("CUMMIN(CLOSE)", &mut ctx).unwrap();
    assert!((result[0] - 1.0).abs() < 1e-10);
    assert!((result[5] - 1.0).abs() < 1e-10); // min stays at 1
    assert!((result[9] - 1.0).abs() < 1e-10);

    // PERCENTILE(X, N, P): 50th percentile over 5 bars = median
    let result = engine.eval("PERCENTILE(CLOSE, 5, 50)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // At bar 4 (window: 1,3,2,5,4 -> sorted: 1,2,3,4,5 -> 50% idx=2 -> 3.0)
    assert!((result[4] - 3.0).abs() < 1e-10);

    // MEDIAN(X, N): median over 5 bars
    let result = engine.eval("MEDIAN(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // At bar 4 (window: 1,3,2,5,4 -> sorted: 1,2,3,4,5 -> median=3)
    assert!((result[4] - 3.0).abs() < 1e-10);
}

#[test]
fn test_formula_stats_functions() {
    let mut engine = FormulaEngine::new();
    let open = Array1::from_vec(vec![10.0; 10]);
    let high = Array1::from_vec(vec![12.0; 10]);
    let low = Array1::from_vec(vec![8.0; 10]);
    let close = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
    let volume = Array1::from_vec(vec![1000.0; 10]);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);

    // SKEW: skewness of uniform sequence should be ~0
    let result = engine.eval("SKEW(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // For uniform spacing [1,2,3,4,5], skewness = 0
    assert!((result[4]).abs() < 1e-10);

    // KURT: kurtosis of uniform sequence
    let result = engine.eval("KURT(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // Excess kurtosis for uniform: -1.3 (platykurtic)
    assert!(result[4].is_finite());

    // MODE: most frequent value (with linear data, all unique, returns first)
    let result = engine.eval("MODE(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    assert!(result[4].is_finite());

    // SORT: ascending rank of current value in window
    let result = engine.eval("SORT(CLOSE, 5, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // At bar 4, window=[1,2,3,4,5], current=5 is rank 5 (ascending)
    assert!((result[4] - 5.0).abs() < 1e-10);
    // At bar 9, window=[6,7,8,9,10], current=10 is rank 5 (ascending)
    assert!((result[9] - 5.0).abs() < 1e-10);

    // SORT descending: current=10 is rank 1
    let result = engine.eval("SORT(CLOSE, 5, 0)", &mut ctx).unwrap();
    assert!((result[9] - 1.0).abs() < 1e-10);

    // RANK: percentile rank
    let result = engine.eval("RANK(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // At bar 4 (val=5 in [1,2,3,4,5]): 4 values below -> 4/4*100=100%
    assert!((result[4] - 100.0).abs() < 1e-10);
}

#[test]
fn test_formula_multi_period() {
    let mut engine = FormulaEngine::new();
    let open = Array1::from_vec(vec![10.0; 10]);
    let high = Array1::from_vec(vec![12.0; 10]);
    let low = Array1::from_vec(vec![8.0; 10]);
    let close = Array1::from_vec(vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0]);
    let volume = Array1::from_vec(vec![1000.0; 10]);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);

    // PERIODTYPE: default is 0 (daily)
    let result = engine.eval("PERIODTYPE()", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    assert!((result[0]).abs() < 1e-10); // 0 = daily

    // Set period type to weekly
    ctx.period_type = 1;
    let result = engine.eval("PERIODTYPE()", &mut ctx).unwrap();
    assert!((result[0] - 1.0).abs() < 1e-10);

    // REFDATE: get value at specific bar index
    let result = engine.eval("REFDATE(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 10);
    // All values should be CLOSE[5] = 15.0
    assert!((result[0] - 15.0).abs() < 1e-10);
    assert!((result[9] - 15.0).abs() < 1e-10);

    // REFDATE with out-of-range index returns NaN
    let result = engine.eval("REFDATE(CLOSE, 100)", &mut ctx).unwrap();
    assert!(result[0].is_nan());
}

#[test]
fn test_formula_lazy_eval() {
    let mut engine = FormulaEngine::new();
    let open = Array1::from_vec(vec![10.0; 30]);
    let high = Array1::from_vec(vec![12.0; 30]);
    let low = Array1::from_vec(vec![8.0; 30]);
    let close = Array1::from_vec((0..30).map(|i| 10.0 + i as f64 * 0.5).collect());
    let volume = Array1::from_vec(vec![1000.0; 30]);

    // Full eval: computes all variables
    let mut ctx1 = FormulaContext::new(
        open.clone(), high.clone(), low.clone(), close.clone(), volume.clone(), None,
    );
    let source = "A := MA(CLOSE, 5); B := MA(CLOSE, 10); C := MA(CLOSE, 20); RESULT: A + C";
    let result_full = engine.eval(source, &mut ctx1).unwrap();

    // Lazy eval: should skip B since it's not referenced by RESULT
    let mut ctx2 = FormulaContext::new(open, high, low, close, volume, None);
    let result_lazy = engine.eval_lazy(source, &mut ctx2).unwrap();

    // Results must match
    assert_eq!(result_full.len(), result_lazy.len());
    for i in 0..30 {
        if result_full[i].is_nan() {
            assert!(result_lazy[i].is_nan());
        } else {
            assert!((result_full[i] - result_lazy[i]).abs() < 1e-10,
                "Mismatch at {}: full={}, lazy={}", i, result_full[i], result_lazy[i]);
        }
    }

    // Verify B was not computed in lazy mode
    assert!(!ctx2.variables.contains_key("B"));
    // A and C should still be computed
    assert!(ctx2.variables.contains_key("A"));
    assert!(ctx2.variables.contains_key("C"));
}

#[test]
fn test_formula_incremental() {
    let mut engine = FormulaEngine::new();
    let open = Array1::from_vec(vec![10.0; 10]);
    let high = Array1::from_vec(vec![12.0; 10]);
    let low = Array1::from_vec(vec![8.0; 10]);
    let close = Array1::from_vec((0..10).map(|i| 10.0 + i as f64).collect());
    let volume = Array1::from_vec(vec![1000.0; 10]);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);

    let source = "MA(CLOSE, 5)";

    // Initial computation
    let result1 = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result1.len(), 10);

    // Append a new bar
    ctx.append_bar(10.0, 12.0, 8.0, 20.0, 1000.0);

    // Incremental recomputation
    let result2 = engine.eval_incremental(source, &mut ctx).unwrap();
    assert_eq!(result2.len(), 11);

    // Verify first 10 values match
    for i in 4..10 {
        assert!((result1[i] - result2[i]).abs() < 1e-10,
            "Mismatch at {}: original={}, incremental={}", i, result1[i], result2[i]);
    }

    // Verify new bar's MA is correct: avg of bars 6,7,8,9,10 = (16+17+18+19+20)/5 = 18
    assert!((result2[10] - 18.0).abs() < 1e-10);

    // Compare with full recomputation
    let open_full = Array1::from_vec(vec![10.0; 11]);
    let high_full = Array1::from_vec(vec![12.0; 11]);
    let low_full = Array1::from_vec(vec![8.0; 11]);
    let close_full = Array1::from_vec((0..11).map(|i| if i < 10 { 10.0 + i as f64 } else { 20.0 }).collect());
    let volume_full = Array1::from_vec(vec![1000.0; 11]);
    let mut ctx_full = FormulaContext::new(open_full, high_full, low_full, close_full, volume_full, None);
    let result_full = engine.eval(source, &mut ctx_full).unwrap();

    for i in 0..11 {
        if result_full[i].is_nan() {
            assert!(result2[i].is_nan());
        } else {
            assert!((result_full[i] - result2[i]).abs() < 1e-10);
        }
    }
}

#[test]
fn test_formula_parallel() {
    let mut engine = FormulaEngine::new();
    let open = Array1::from_vec(vec![10.0; 30]);
    let high = Array1::from_vec(vec![12.0; 30]);
    let low = Array1::from_vec(vec![8.0; 30]);
    let close = Array1::from_vec((0..30).map(|i| 10.0 + i as f64 * 0.5).collect());
    let volume = Array1::from_vec(vec![1000.0; 30]);

    let source = "A := MA(CLOSE, 5); B := MA(CLOSE, 10); C := MA(CLOSE, 20); RESULT: A + B + C";

    // Serial eval
    let mut ctx1 = FormulaContext::new(
        open.clone(), high.clone(), low.clone(), close.clone(), volume.clone(), None,
    );
    let result_serial = engine.eval(source, &mut ctx1).unwrap();

    // Parallel eval (falls back to serial when rayon feature not enabled)
    let mut ctx2 = FormulaContext::new(open, high, low, close, volume, None);
    let result_parallel = engine.eval_parallel(source, &mut ctx2).unwrap();

    // Results must match
    assert_eq!(result_serial.len(), result_parallel.len());
    for i in 0..30 {
        if result_serial[i].is_nan() {
            assert!(result_parallel[i].is_nan());
        } else {
            assert!((result_serial[i] - result_parallel[i]).abs() < 1e-10,
                "Mismatch at {}: serial={}, parallel={}", i, result_serial[i], result_parallel[i]);
        }
    }
}
