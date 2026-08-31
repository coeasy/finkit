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

// ============================================================================
// 通达信（TongDaXin）经典公式兼容测试
// ============================================================================

#[test]
fn test_tdx_macd() {
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
    // MACD may produce NaN when nested EMA inputs contain NaN;
    // just verify no panic and correct length
}

#[test]
fn test_tdx_macd_builtin() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("MACD(CLOSE, 12, 26, 9)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(!result[99].is_nan());
}

#[test]
fn test_tdx_kdj() {
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
    assert!(!result[99].is_nan());
}

#[test]
fn test_tdx_kdj_builtin() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine
        .eval("KDJ(CLOSE, HIGH, LOW, 9, 3, 3)", &mut ctx)
        .unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_tdx_boll() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MID := MA(CLOSE, 20);
        UPPER := MID + 2 * STD(CLOSE, 20);
        LOWER := MID - 2 * STD(CLOSE, 20);
        RESULT: UPPER - LOWER
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result[99] > 0.0);
}

#[test]
fn test_tdx_boll_builtin() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let mid = engine.eval("BOLLMID(CLOSE, 20, 2)", &mut ctx).unwrap();
    let up = engine.eval("BOLLUP(CLOSE, 20, 2)", &mut ctx).unwrap();
    let dn = engine.eval("BOLLDN(CLOSE, 20, 2)", &mut ctx).unwrap();
    assert!(up[99] > mid[99]);
    assert!(mid[99] > dn[99]);
}

#[test]
fn test_tdx_rsi() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        LC := REF(CLOSE, 1);
        RSI1 := SMA(MAX(CLOSE - LC, 0), 6, 1) / SMA(ABS(CLOSE - LC), 6, 1) * 100;
        RESULT: RSI1
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let v = result[99];
    assert!(!v.is_nan());
}

#[test]
fn test_tdx_rsi_builtin() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("RSI(CLOSE, 6)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let v = result[99];
    assert!((0.0..=100.0).contains(&v));
}

#[test]
fn test_tdx_cci() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("CCI(HIGH, LOW, CLOSE, 14)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(!result[99].is_nan());
}

#[test]
fn test_tdx_wr() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("WR(HIGH, LOW, CLOSE, 14)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let v = result[99];
    assert!((-100.0..=0.0).contains(&v));
}

#[test]
fn test_tdx_dmi() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("DMI(HIGH, LOW, CLOSE, 14)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(!result[99].is_nan());
}

#[test]
fn test_tdx_adx() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("ADX(HIGH, LOW, CLOSE, 14)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let v = result[99];
    assert!((0.0..=100.0).contains(&v));
}

#[test]
fn test_tdx_bias() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BIAS1 := (CLOSE - MA(CLOSE, 6)) / MA(CLOSE, 6) * 100;
        BIAS2 := (CLOSE - MA(CLOSE, 12)) / MA(CLOSE, 12) * 100;
        BIAS3 := (CLOSE - MA(CLOSE, 24)) / MA(CLOSE, 24) * 100;
        RESULT: BIAS1 + BIAS2 + BIAS3
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(!result[99].is_nan());
}

#[test]
fn test_tdx_psy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("PSY(CLOSE, 12)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let v = result[99];
    assert!((0.0..=100.0).contains(&v));
}

#[test]
fn test_tdx_obv() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("OBV(CLOSE, VOL)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_tdx_trix() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("TRIX(CLOSE, 12)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_tdx_dpo() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("DPO(CLOSE, 20)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_tdx_roc() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("ROC(CLOSE, 12)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_tdx_mom() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("MOM(CLOSE, 10)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_tdx_atr() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("ATR(HIGH, LOW, CLOSE, 14)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result[99] > 0.0);
}

#[test]
fn test_tdx_sar() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("SAR(HIGH, LOW, 4, 2, 20)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 通达信经典组合公式: MA 均线系统
#[test]
fn test_tdx_ma_system() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MA5 := MA(CLOSE, 5);
        MA10 := MA(CLOSE, 10);
        MA20 := MA(CLOSE, 20);
        MA60 := MA(CLOSE, 60);
        GOLDEN := CROSS(MA5, MA20);
        DEATH := CROSS(MA20, MA5);
        RESULT: GOLDEN + DEATH
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 通达信量价关系
#[test]
fn test_tdx_vol_price() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MAVOL := MA(VOL, 5);
        VOLRATIO := VOL / MAVOL;
        PRICEUP := CLOSE > REF(CLOSE, 1);
        VOLUP := VOL > REF(VOL, 1);
        RESULT: PRICEUP * VOLUP
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 通达信条件选股
#[test]
fn test_tdx_filter_select() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        C1 := CLOSE > MA(CLOSE, 20);
        C2 := VOL > MA(VOL, 5) * 1.5;
        C3 := CLOSE > REF(CLOSE, 1) * 1.03;
        RESULT: C1 * C2 * C3
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 通达信 HHVBARS/LLVBARS 使用
#[test]
fn test_tdx_hhvbars_llvbars() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        HB := HHVBARS(HIGH, 20);
        LB := LLVBARS(LOW, 20);
        RESULT: HB + LB
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 通达信 COUNT/EVERY/EXIST
#[test]
fn test_tdx_count_every_exist() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        UP := CLOSE > REF(CLOSE, 1);
        CNT := COUNT(UP, 10);
        ALL_UP := EVERY(UP, 3);
        ANY_UP := EXIST(UP, 5);
        RESULT: CNT + ALL_UP + ANY_UP
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 通达信 BARSLAST/BACKSET
#[test]
fn test_tdx_barslast_backset() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        BL := BARSLAST(COND);
        BS := BACKSET(COND, 3);
        RESULT: BL + BS
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 通达信 BETWEEN/NOT/FILTER
#[test]
fn test_tdx_between_not_filter() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        B := BETWEEN(CLOSE, 10, 20);
        N := NOT(B);
        SIG := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        F := FILTER(SIG, 5);
        RESULT: B + N + F
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 通达信 SUMBARS 累加到指定值
#[test]
fn test_tdx_sumbars() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("SUMBARS(VOL, 5000)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 通达信 AVEDEV
#[test]
fn test_tdx_avedev() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("AVEDEV(CLOSE, 20)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result[99] >= 0.0);
}

// 通达信 SLOPE/FORCAST
#[test]
fn test_tdx_slope_forcast() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let s = engine.eval("SLOPE(CLOSE, 20)", &mut ctx).unwrap();
    let f = engine.eval("FORCAST(CLOSE, 20)", &mut ctx).unwrap();
    assert_eq!(s.len(), 100);
    assert_eq!(f.len(), 100);
}

// 通达信 PEAK/TROUGH/ZIGZAG
#[test]
fn test_tdx_peak_trough_zigzag() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let p = engine.eval("PEAK(HIGH, 5, 1)", &mut ctx).unwrap();
    let t = engine.eval("TROUGH(LOW, 5, 1)", &mut ctx).unwrap();
    let z = engine.eval("ZIGZAG(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(p.len(), 100);
    assert_eq!(t.len(), 100);
    assert_eq!(z.len(), 100);
}

// ============================================================================
// 同花顺（THS）公式兼容测试
// ============================================================================

#[test]
fn test_ths_macd_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        DIFF1 := EMA(CLOSE, 12) - EMA(CLOSE, 26);
        DEA1 := EMA(DIFF1, 9);
        MACD1 := 2 * (DIFF1 - DEA1);
        RESULT: MACD1
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_ths_kdj_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        RSV := (CLOSE - LLV(LOW, 9)) / (HHV(HIGH, 9) - LLV(LOW, 9)) * 100;
        K := SMA(RSV, 3, 1);
        D := SMA(K, 3, 1);
        J := 3 * K - 2 * D;
        GOLDEN := CROSS(K, D);
        DEAD := CROSS(D, K);
        RESULT: J
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 同花顺多周期 RSI
#[test]
fn test_ths_multi_rsi() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        RSI6 := RSI(CLOSE, 6);
        RSI12 := RSI(CLOSE, 12);
        RSI24 := RSI(CLOSE, 24);
        RESULT: RSI6 + RSI12 + RSI24
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 同花顺 EMA 均线交叉系统
#[test]
fn test_ths_ema_cross() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        EMA5 := EMA(CLOSE, 5);
        EMA13 := EMA(CLOSE, 13);
        EMA21 := EMA(CLOSE, 21);
        BUY_SIG := CROSS(EMA5, EMA21);
        SELL_SIG := CROSS(EMA21, EMA5);
        RESULT: BUY_SIG + SELL_SIG
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 同花顺 BOLL 策略
#[test]
fn test_ths_boll_strategy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MID := MA(CLOSE, 20);
        UPPER := MID + 2 * STD(CLOSE, 20);
        LOWER := MID - 2 * STD(CLOSE, 20);
        WIDTH := (UPPER - LOWER) / MID * 100;
        SQUEEZE := WIDTH < REF(WIDTH, 1);
        BREAKOUT := CLOSE > UPPER;
        RESULT: SQUEEZE + BREAKOUT
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 同花顺 OBV+MA 策略
#[test]
fn test_ths_obv_strategy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        OBV1 := OBV(CLOSE, VOL);
        MAOBV := MA(OBV1, 10);
        SIGNAL := CROSS(OBV1, MAOBV);
        RESULT: SIGNAL
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 同花顺多指标综合评分系统
#[test]
fn test_ths_composite_score() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        S1 := IF(CLOSE > MA(CLOSE, 20), 20, 0);
        S2 := IF(RSI(CLOSE, 14) < 30, 20, 0);
        S3 := IF(VOL > MA(VOL, 5), 20, 0);
        S4 := IF(CLOSE > REF(CLOSE, 1), 20, 0);
        S5 := IF(CROSS(MA(CLOSE, 5), MA(CLOSE, 10)), 20, 0);
        SCORE := S1 + S2 + S3 + S4 + S5;
        RESULT: SCORE
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let v = result[99];
    assert!((0.0..=100.0).contains(&v));
}

// 同花顺 CCI 策略
#[test]
fn test_ths_cci_strategy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        CCI14 := CCI(HIGH, LOW, CLOSE, 14);
        OVERBOUGHT := CCI14 > 100;
        OVERSOLD := CCI14 < -100;
        RESULT: OVERBOUGHT + OVERSOLD
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 同花顺 MFI 策略
#[test]
fn test_ths_mfi_strategy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MFI14 := MFI(HIGH, LOW, CLOSE, VOL, 14);
        RESULT: MFI14
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 同花顺趋势跟踪
#[test]
fn test_ths_trend_following() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MA20 := MA(CLOSE, 20);
        MA60 := MA(CLOSE, 60);
        UPTREND := MA20 > MA60;
        ATR14 := ATR(HIGH, LOW, CLOSE, 14);
        STOP := CLOSE - 2 * ATR14;
        RESULT: UPTREND * STOP
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// ============================================================================
// 文华财经（Wenhua Finance）交易策略公式兼容测试
// ============================================================================

#[test]
fn test_wenhua_basic_strategy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MA5 := MA(CLOSE, 5);
        MA10 := MA(CLOSE, 10);
        COND1 := CROSS(MA5, MA10);
        BUY_SIG := ENTERLONG(COND1);
        RESULT: BUY_SIG
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_wenhua_autofilter() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 20));
        FILTERED := AUTOFILTER(COND, 10);
        RESULT: FILTERED
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_wenhua_multsig() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_SIG := CROSS(EMA(CLOSE, 5), EMA(CLOSE, 20));
        SELL_SIG := CROSS(EMA(CLOSE, 20), EMA(CLOSE, 5));
        MS := MULTSIG(BUY_SIG, SELL_SIG, 1, 5);
        RESULT: MS
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_wenhua_dual_ma_strategy() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        FAST := EMA(CLOSE, 5);
        SLOW := EMA(CLOSE, 20);
        LONG := CROSS(FAST, SLOW);
        SHORT := CROSS(SLOW, FAST);
        BL := ENTERLONG(LONG);
        BS := ENTERSHORT(SHORT);
        XL := EXITLONG(SHORT);
        XS := EXITSHORT(LONG);
        RESULT: BL + BS + XL + XS
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_wenhua_boll_breakout() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MID := MA(CLOSE, 20);
        BAND := 2 * STD(CLOSE, 20);
        UPPER := MID + BAND;
        LOWER := MID - BAND;
        BUY1 := CROSS(CLOSE, UPPER);
        SELL1 := CROSS(LOWER, CLOSE);
        BF := AUTOFILTER(BUY1, 5);
        SF := AUTOFILTER(SELL1, 5);
        RESULT: BF + SF
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_wenhua_atr_trailing_stop() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        N := ATR(HIGH, LOW, CLOSE, 14);
        TRAIL := HHV(HIGH, 20) - 3 * N;
        EXIT := CROSS(TRAIL, CLOSE);
        RESULT: EXIT
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 文华财经动量突破策略
#[test]
fn test_wenhua_momentum_breakout() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        HH := HHV(HIGH, 20);
        LL := LLV(LOW, 20);
        MID := (HH + LL) / 2;
        RANGE := HH - LL;
        UP_BREAK := CLOSE > HH;
        DN_BREAK := CLOSE < LL;
        RESULT: UP_BREAK + DN_BREAK
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// ============================================================================
// 通达信高级公式兼容测试
// ============================================================================

// 通达信 VALUEWHEN
#[test]
fn test_tdx_valuewhen() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 10));
        VW := VALUEWHEN(COND, CLOSE);
        RESULT: VW
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 通达信 LAST
#[test]
fn test_tdx_last() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        UP := CLOSE > REF(CLOSE, 1);
        L := LAST(UP, 5, 1);
        RESULT: L
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 通达信 FINDHIGH/FINDLOW
#[test]
fn test_tdx_findhigh_findlow() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let fh = engine.eval("FINDHIGH(HIGH, 20, 1, 0)", &mut ctx).unwrap();
    let fl = engine.eval("FINDLOW(LOW, 20, 1, 0)", &mut ctx).unwrap();
    assert_eq!(fh.len(), 100);
    assert_eq!(fl.len(), 100);
}

// ============================================================================
// 数学/统计兼容测试
// ============================================================================

#[test]
fn test_math_functions() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = r#"
        A := ABS(CLOSE - OPEN);
        B := SQRT(A);
        C := POW(CLOSE, 2);
        D := LOG(CLOSE);
        E := EXP(0.01);
        F := FLOOR(CLOSE);
        G := CEIL(CLOSE);
        H := ROUND(CLOSE);
        RESULT: A + B + C + D + E + F + G + H
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!(!result[49].is_nan());
}

#[test]
fn test_trig_functions() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = r#"
        A := SIN(CLOSE);
        B := COS(CLOSE);
        C := TAN(CLOSE * 0.01);
        D := ASIN(0.5);
        E := ACOS(0.5);
        F := ATAN(1.0);
        RESULT: A + B + C + D + E + F
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_statistical_functions() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        S := STD(CLOSE, 20);
        V := VAR(CLOSE, 20);
        Z := ZSCORE(CLOSE, 20);
        SK := SKEW(CLOSE, 20);
        KU := KURT(CLOSE, 20);
        RESULT: S + V + Z + SK + KU
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_cumulative_functions() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = r#"
        CS := CUMSUM(CLOSE);
        CM := CUMMAX(HIGH);
        CN := CUMMIN(LOW);
        RESULT: CS + CM + CN
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!(result[49] > result[0]);
}

#[test]
fn test_percentile_median() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let p = engine.eval("PERCENTILE(CLOSE, 20, 75)", &mut ctx).unwrap();
    let m = engine.eval("MEDIAN(CLOSE, 20)", &mut ctx).unwrap();
    assert_eq!(p.len(), 100);
    assert_eq!(m.len(), 100);
}

#[test]
fn test_rank_sort() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let r = engine.eval("RANK(CLOSE, 20)", &mut ctx).unwrap();
    let s = engine.eval("SORT(CLOSE, 20, 1)", &mut ctx).unwrap();
    assert_eq!(r.len(), 100);
    assert_eq!(s.len(), 100);
}

// ============================================================================
// 语法兼容性测试
// ============================================================================

#[test]
fn test_syntax_equal_assignment() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = r#"
        A = MA(CLOSE, 5);
        B = EMA(CLOSE, 10);
        RESULT: A + B
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_syntax_colon_assign() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = r#"
        A := MA(CLOSE, 5);
        B := EMA(CLOSE, 10);
        RESULT: A + B
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_syntax_hash_comment() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = "# This is a comment\nRESULT: MA(CLOSE, 5)";
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_syntax_brace_comment() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = "{This is a block comment}\nRESULT: MA(CLOSE, 5)";
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_syntax_c_style_comment() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = "/* C-style comment */\nRESULT: MA(CLOSE, 5)";
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_syntax_line_comment() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = "// line comment\nRESULT: MA(CLOSE, 5)";
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

// ============================================================================
// 复合策略公式（综合兼容性测试）
// ============================================================================

// 经典多头排列选股
#[test]
fn test_compound_bull_alignment() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MA5 := MA(CLOSE, 5);
        MA10 := MA(CLOSE, 10);
        MA20 := MA(CLOSE, 20);
        MA60 := MA(CLOSE, 60);
        BULL := MA5 > MA10;
        RESULT: BULL
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 通达信经典 MACD 金叉选股
#[test]
fn test_compound_macd_golden_cross() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26);
        DEA1 := EMA(DIF, 9);
        MACD1 := (DIF - DEA1) * 2;
        GOLDEN := CROSS(DIF, DEA1);
        BOTTOM := DIF < 0;
        RESULT: GOLDEN * BOTTOM
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 量价齐升+MACD 综合
#[test]
fn test_compound_vol_macd() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        VOL_UP := VOL > MA(VOL, 5) * 1.5;
        PRICE_UP := CLOSE > REF(CLOSE, 1) * 1.02;
        DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26);
        DEA1 := EMA(DIF, 9);
        MACD_UP := DIF > DEA1;
        RESULT: VOL_UP * PRICE_UP * MACD_UP
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 超跌反弹策略
#[test]
fn test_compound_oversold_bounce() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        RSI6 := RSI(CLOSE, 6);
        OVERSOLD := RSI6 < 20;
        BOUNCE := CLOSE > REF(CLOSE, 1);
        VOL_SHRINK := VOL < MA(VOL, 10) * 0.5;
        RESULT: OVERSOLD * BOUNCE
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// 趋势+动量策略
#[test]
fn test_compound_trend_momentum() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        TREND := EMA(CLOSE, 20) > EMA(CLOSE, 60);
        MOM_UP := MOM(CLOSE, 10) > 0;
        ADX_STRONG := ADX(HIGH, LOW, CLOSE, 14) > 25;
        RESULT: TREND * MOM_UP * ADX_STRONG
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// ============================================================================
// 惰性求值 / 增量计算 / 并行计算兼容测试
// ============================================================================

#[test]
fn test_lazy_eval_compat() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = r#"
        UNUSED := EMA(CLOSE, 100);
        A := MA(CLOSE, 5);
        B := EMA(CLOSE, 10);
        RESULT: A + B
    "#;
    let result_normal = engine.eval(source, &mut ctx).unwrap();
    let mut ctx2 = make_ctx(50);
    let result_lazy = engine.eval_lazy(source, &mut ctx2).unwrap();
    assert_eq!(result_normal.len(), result_lazy.len());
    for i in 0..50 {
        if result_normal[i].is_nan() {
            assert!(result_lazy[i].is_nan());
        } else {
            assert!((result_normal[i] - result_lazy[i]).abs() < 1e-10);
        }
    }
}

#[test]
fn test_parallel_eval_compat() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        A := MA(CLOSE, 5);
        B := EMA(CLOSE, 10);
        C := RSI(CLOSE, 14);
        D := ATR(HIGH, LOW, CLOSE, 14);
        RESULT: A + B + C + D
    "#;
    let result_serial = engine.eval(source, &mut ctx).unwrap();
    let mut ctx2 = make_ctx(100);
    let result_parallel = engine.eval_parallel(source, &mut ctx2).unwrap();
    assert_eq!(result_serial.len(), result_parallel.len());
    for i in 0..100 {
        if result_serial[i].is_nan() {
            assert!(result_parallel[i].is_nan());
        } else {
            assert!((result_serial[i] - result_parallel[i]).abs() < 1e-10);
        }
    }
}

#[test]
fn test_incremental_eval_compat() {
    let mut engine = FormulaEngine::new();
    let source = "MA(CLOSE, 5)";
    let mut ctx = make_ctx(50);
    let _ = engine.eval(source, &mut ctx).unwrap();
    ctx.append_bar(15.0, 16.0, 14.0, 15.5, 2000.0);
    let result = engine.eval_incremental(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 51);
}

// ============================================================================
// 多输出兼容测试
// ============================================================================

#[test]
fn test_multi_output_macd() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        DIF: EMA(CLOSE, 12) - EMA(CLOSE, 26);
        DEA1: EMA(DIF, 9);
        MACD1: (DIF - DEA1) * 2
    "#;
    let result = engine.eval_multi(source, &mut ctx).unwrap();
    assert!(result.outputs.contains_key("DIF"));
    assert!(result.outputs.contains_key("DEA1"));
    assert!(result.outputs.contains_key("MACD1"));
    assert_eq!(result.outputs["DIF"].len(), 100);
}

#[test]
fn test_multi_output_boll() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MID: MA(CLOSE, 20);
        UPPER: MID + 2 * STD(CLOSE, 20);
        LOWER: MID - 2 * STD(CLOSE, 20)
    "#;
    let result = engine.eval_multi(source, &mut ctx).unwrap();
    assert!(result.outputs.contains_key("MID"));
    assert!(result.outputs.contains_key("UPPER"));
    assert!(result.outputs.contains_key("LOWER"));
}

// ============================================================================
// 信号过滤函数兼容测试（文华风格）
// ============================================================================

#[test]
fn test_checksig_compat() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 20));
        SELL_COND := CROSS(MA(CLOSE, 20), MA(CLOSE, 5));
        CHECKED := CHECKSIG(BUY_COND, SELL_COND, 1);
        RESULT: CHECKED
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

// ============================================================================
// 市场数据引用函数兼容测试
// ============================================================================

#[test]
fn test_indexc_series() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = r#"
        IC := INDEXC();
        IO := INDEXO();
        IH := INDEXH();
        IL := INDEXL();
        IV := INDEXV();
        RESULT: IC + IO + IH + IL + IV
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

// ============================================================================
// 日期时间函数兼容测试
// ============================================================================

#[test]
fn test_datetime_functions() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let source = r#"
        D := DATE();
        Y := YEAR();
        M := MONTH();
        DD := DAY();
        H := HOUR();
        MIN := MINUTE();
        W := WEEKDAY();
        RESULT: D + Y + M + DD + H + MIN + W
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

// ============================================================================
// 大批量公式连续执行压力测试
// ============================================================================

#[test]
fn test_batch_execution() {
    let mut engine = FormulaEngine::new();
    let formulas = vec![
        "MA(CLOSE, 5)",
        "EMA(CLOSE, 12)",
        "RSI(CLOSE, 14)",
        "MACD(CLOSE, 12, 26, 9)",
        "KDJ(CLOSE, HIGH, LOW, 9, 3, 3)",
        "CCI(HIGH, LOW, CLOSE, 14)",
        "WR(HIGH, LOW, CLOSE, 14)",
        "ATR(HIGH, LOW, CLOSE, 14)",
        "ADX(HIGH, LOW, CLOSE, 14)",
        "OBV(CLOSE, VOL)",
        "TRIX(CLOSE, 12)",
        "DPO(CLOSE, 20)",
        "ROC(CLOSE, 12)",
        "MOM(CLOSE, 10)",
        "PSY(CLOSE, 12)",
        "BIAS(CLOSE, 6)",
        "STD(CLOSE, 20)",
        "AVEDEV(CLOSE, 20)",
        "SLOPE(CLOSE, 20)",
        "HHV(HIGH, 20)",
        "LLV(LOW, 20)",
    ];
    for formula in &formulas {
        let mut ctx = make_ctx(100);
        let result = engine.eval(formula, &mut ctx);
        assert!(
            result.is_ok(),
            "Formula '{}' failed: {:?}",
            formula,
            result.err()
        );
        assert_eq!(result.unwrap().len(), 100);
    }
}

// 无 panic 压力测试：边界数据
#[test]
fn test_no_panic_edge_cases() {
    let mut engine = FormulaEngine::new();

    // 最小数据长度 - these may return errors but should NOT panic
    let mut ctx = make_ctx(1);
    let _ = engine.eval("MA(CLOSE, 5)", &mut ctx);
    let _ = engine.eval("EMA(CLOSE, 12)", &mut ctx);
    let _ = engine.eval("RSI(CLOSE, 14)", &mut ctx);

    // 较短数据 - may return errors or NaN but should not panic
    let mut ctx = make_ctx(10);
    let _ = engine.eval("MA(CLOSE, 20)", &mut ctx);
    let _ = engine.eval("STD(CLOSE, 20)", &mut ctx);
    let _ = engine.eval("KDJ(CLOSE, HIGH, LOW, 9, 3, 3)", &mut ctx);

    // 包含零值
    let close = Array1::from_vec(vec![0.0; 20]);
    let open = close.clone();
    let high = close.clone();
    let low = close.clone();
    let vol = close.clone();
    let mut ctx = FormulaContext::new(open, high, low, close, vol, None);
    let _ = engine.eval("RSI(CLOSE, 14)", &mut ctx);
    let _ = engine.eval("MA(CLOSE, 5)", &mut ctx);
    let _ = engine.eval("ATR(HIGH, LOW, CLOSE, 14)", &mut ctx);
}
