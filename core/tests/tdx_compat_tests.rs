use ndarray::Array1;
use finkit::formula::engine::FormulaEngine;
use finkit::formula::types::{ChipData, DynaInfo, FinanceData, FormulaContext, IndexData};

fn make_ctx(len: usize) -> FormulaContext {
    let close: Vec<f64> = (0..len).map(|i| 10.0 + (i as f64 * 0.3).sin() * 2.0 + i as f64 * 0.05).collect();
    let open: Vec<f64> = close.iter().map(|c| c - 0.1).collect();
    let high: Vec<f64> = close.iter().map(|c| c + 0.5).collect();
    let low: Vec<f64> = close.iter().map(|c| c - 0.5).collect();
    let volume: Vec<f64> = (0..len).map(|i| 1000.0 + (i as f64 * 0.7).sin() * 200.0).collect();
    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    )
}

fn make_ctx_with_chip_data(len: usize) -> FormulaContext {
    let ctx = make_ctx(len);
    let price_levels: Vec<f64> = vec![9.0, 9.5, 10.0, 10.5, 11.0, 11.5, 12.0];
    let volume_ratios: Vec<f64> = vec![0.05, 0.15, 0.30, 0.50, 0.70, 0.85, 1.00];
    let chip_data = ChipData::with_data(price_levels, volume_ratios, 1000000.0);
    ctx.with_chip_data(chip_data)
}

fn make_ctx_with_finance_data(len: usize) -> FormulaContext {
    let ctx = make_ctx(len);
    let mut finance_data = FinanceData::default();
    finance_data.fields.insert(1, 100000.0);
    finance_data.fields.insert(7, 50000.0);
    finance_data.fields.insert(40, 2.5);
    ctx.with_finance_data(finance_data)
}

fn make_ctx_with_dynainfo(len: usize) -> FormulaContext {
    let ctx = make_ctx(len);
    let mut dynainfo = DynaInfo::default();
    dynainfo.fields.insert(3, 10.5);
    dynainfo.fields.insert(4, 11.0);
    dynainfo.fields.insert(5, 10.0);
    dynainfo.fields.insert(6, 10.3);
    dynainfo.fields.insert(7, 5000.0);
    dynainfo.fields.insert(10, 50000.0);
    dynainfo.fields.insert(17, 1.5);
    ctx.with_dynainfo(dynainfo)
}

fn make_ctx_with_index_data(len: usize) -> FormulaContext {
    let ctx = make_ctx(len);
    let mut index_data = IndexData::default();
    let index_close: Vec<f64> = (0..len).map(|i| 3000.0 + i as f64 * 0.5).collect();
    index_data.close = Some(Array1::from_vec(index_close));
    ctx.with_index_data(index_data)
}

#[test]
fn test_chip_winner_with_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_chip_data(50);
    let result = engine.eval("WINNER(CLOSE)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        if !result[i].is_nan() {
            assert!((0.0..=1.0).contains(&result[i]));
        }
    }
}

#[test]
fn test_chip_winner_without_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("WINNER(CLOSE)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        assert!(result[i].is_nan());
    }
}

#[test]
fn test_chip_cost_with_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_chip_data(50);
    let result = engine.eval("COST(50)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        if !result[i].is_nan() {
            assert!((9.0..=12.0).contains(&result[i]));
        }
    }
}

#[test]
fn test_chip_cost_boundary() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_chip_data(50);
    let result_0 = engine.eval("COST(0)", &mut ctx).unwrap();
    let result_100 = engine.eval("COST(100)", &mut ctx).unwrap();
    for i in 0..50 {
        if !result_0[i].is_nan() {
            assert!((result_0[i] - 9.0).abs() < 0.5);
        }
        if !result_100[i].is_nan() {
            assert!((result_100[i] - 12.0).abs() < 0.5);
        }
    }
}

#[test]
fn test_lwinner_with_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_chip_data(50);
    let result = engine.eval("LWINNER(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..5 {
        assert!(result[i].is_nan());
    }
    for i in 5..50 {
        if !result[i].is_nan() {
            assert!((0.0..=1.0).contains(&result[i]));
        }
    }
}

#[test]
fn test_finance_with_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_finance_data(50);
    let result = engine.eval("FINANCE(1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        assert!((result[i] - 100000.0).abs() < 1e-10);
    }
}

#[test]
fn test_finance_multiple_fields() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_finance_data(50);
    let f1 = engine.eval("FINANCE(1)", &mut ctx).unwrap();
    let f7 = engine.eval("FINANCE(7)", &mut ctx).unwrap();
    let f40 = engine.eval("FINANCE(40)", &mut ctx).unwrap();
    assert!((f1[0] - 100000.0).abs() < 1e-10);
    assert!((f7[0] - 50000.0).abs() < 1e-10);
    assert!((f40[0] - 2.5).abs() < 1e-10);
}

#[test]
fn test_finance_without_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("FINANCE(1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        assert!(result[i].is_nan());
    }
}

#[test]
fn test_dynainfo_with_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_dynainfo(50);
    let result = engine.eval("DYNAINFO(3)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        assert!((result[i] - 10.5).abs() < 1e-10);
    }
}

#[test]
fn test_dynainfo_multiple_fields() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_dynainfo(50);
    let d3 = engine.eval("DYNAINFO(3)", &mut ctx).unwrap();
    let d4 = engine.eval("DYNAINFO(4)", &mut ctx).unwrap();
    let d5 = engine.eval("DYNAINFO(5)", &mut ctx).unwrap();
    let d6 = engine.eval("DYNAINFO(6)", &mut ctx).unwrap();
    let d7 = engine.eval("DYNAINFO(7)", &mut ctx).unwrap();
    assert!((d3[0] - 10.5).abs() < 1e-10);
    assert!((d4[0] - 11.0).abs() < 1e-10);
    assert!((d5[0] - 10.0).abs() < 1e-10);
    assert!((d6[0] - 10.3).abs() < 1e-10);
    assert!((d7[0] - 5000.0).abs() < 1e-10);
}

#[test]
fn test_dynainfo_without_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("DYNAINFO(3)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        assert!(result[i].is_nan());
    }
}

#[test]
fn test_capital_with_data() {
    let mut engine = FormulaEngine::new();
    let ctx = make_ctx(50).with_capital(500000.0);
    let mut ctx = ctx;
    let result = engine.eval("CAPITAL", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        assert!((result[i] - 500000.0).abs() < 1e-10);
    }
}

#[test]
fn test_indexc_with_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_index_data(50);
    let result = engine.eval("INDEXC()", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        let expected = 3000.0 + i as f64 * 0.5;
        assert!((result[i] - expected).abs() < 1e-10);
    }
}

#[test]
fn test_indexc_without_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("INDEXC()", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        assert!(result[i].is_nan());
    }
}

#[test]
fn test_peak_trough_zigzag() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let peak = engine.eval("PEAK(HIGH, 5, 1)", &mut ctx).unwrap();
    let trough = engine.eval("TROUGH(LOW, 5, 1)", &mut ctx).unwrap();
    let zigzag = engine.eval("ZIGZAG(CLOSE, 5)", &mut ctx).unwrap();
    assert_eq!(peak.len(), 100);
    assert_eq!(trough.len(), 100);
    assert_eq!(zigzag.len(), 100);
}

#[test]
fn test_peakbars_troughbars() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let peakbars = engine.eval("PEAKBARS(HIGH, 5, 1)", &mut ctx).unwrap();
    let troughbars = engine.eval("TROUGHBARS(LOW, 5, 1)", &mut ctx).unwrap();
    assert_eq!(peakbars.len(), 100);
    assert_eq!(troughbars.len(), 100);
}

#[test]
fn test_findhigh_findlow() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let findhigh = engine.eval("FINDHIGH(HIGH, 20, 1, 0)", &mut ctx).unwrap();
    let findlow = engine.eval("FINDLOW(LOW, 20, 1, 0)", &mut ctx).unwrap();
    assert_eq!(findhigh.len(), 100);
    assert_eq!(findlow.len(), 100);
}

#[test]
fn test_topn() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("TOPN(CLOSE, 10)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    let count = result.iter().filter(|&&v| v > 0.0).count();
    assert!(count <= 10);
}

#[test]
fn test_drawnull() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("DRAWNULL", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        assert!(result[i].is_nan());
    }
}

#[test]
fn test_ceiling_function() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("CEILING(CLOSE, 0.1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_autofilter() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("AUTOFILTER()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    for i in 0..100 {
        assert!((result[i] - 1.0).abs() < 1e-10);
    }
}

#[test]
fn test_checksig() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 20));
        SELL_COND := CROSS(MA(CLOSE, 20), MA(CLOSE, 5));
        RESULT: CHECKSIG(BUY_COND, SELL_COND, 1)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_multsig() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        BUY_COND := CROSS(MA(CLOSE, 5), MA(CLOSE, 20));
        SELL_COND := CROSS(MA(CLOSE, 20), MA(CLOSE, 5));
        RESULT: MULTSIG(BUY_COND, SELL_COND, 5, 3)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_enterlong_exitlong() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let bl = engine.eval("ENTERLONG()", &mut ctx).unwrap();
    let xl = engine.eval("EXITLONG()", &mut ctx).unwrap();
    assert_eq!(bl.len(), 100);
    assert_eq!(xl.len(), 100);
}

#[test]
fn test_cumsum_cummax_cummin() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let cumsum = engine.eval("CUMSUM(CLOSE)", &mut ctx).unwrap();
    let cummax = engine.eval("CUMMAX(HIGH)", &mut ctx).unwrap();
    let cummin = engine.eval("CUMMIN(LOW)", &mut ctx).unwrap();
    assert_eq!(cumsum.len(), 50);
    assert_eq!(cummax.len(), 50);
    assert_eq!(cummin.len(), 50);
    assert!(cumsum[49] > cumsum[0]);
}

#[test]
fn test_percentile_median() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let percentile = engine.eval("PERCENTILE(CLOSE, 20, 75)", &mut ctx).unwrap();
    let median = engine.eval("MEDIAN(CLOSE, 20)", &mut ctx).unwrap();
    assert_eq!(percentile.len(), 100);
    assert_eq!(median.len(), 100);
}

#[test]
fn test_skew_kurt_mode() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let skew = engine.eval("SKEW(CLOSE, 20)", &mut ctx).unwrap();
    let kurt = engine.eval("KURT(CLOSE, 20)", &mut ctx).unwrap();
    let mode = engine.eval("MODE(CLOSE, 20)", &mut ctx).unwrap();
    assert_eq!(skew.len(), 100);
    assert_eq!(kurt.len(), 100);
    assert_eq!(mode.len(), 100);
}

#[test]
fn test_rank_sort() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let rank = engine.eval("RANK(CLOSE, 20)", &mut ctx).unwrap();
    let sort = engine.eval("SORT(CLOSE, 20, 1)", &mut ctx).unwrap();
    assert_eq!(rank.len(), 100);
    assert_eq!(sort.len(), 100);
}

#[test]
fn test_periodtype() {
    let mut engine = FormulaEngine::new();
    let ctx = make_ctx(50).with_period_type(1);
    let mut ctx = ctx;
    let result = engine.eval("PERIODTYPE()", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        assert!((result[i] - 1.0).abs() < 1e-10);
    }
}

#[test]
fn test_refdate() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("REFDATE(CLOSE, 10)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_avedev() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("AVEDEV(CLOSE, 20)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result[99] >= 0.0);
}

#[test]
fn test_devsq() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("DEVSQ(CLOSE, 20)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result[99] >= 0.0);
}

#[test]
fn test_slope_forcast() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let slope = engine.eval("SLOPE(CLOSE, 20)", &mut ctx).unwrap();
    let forcast = engine.eval("FORCAST(CLOSE, 20)", &mut ctx).unwrap();
    assert_eq!(slope.len(), 100);
    assert_eq!(forcast.len(), 100);
}

#[test]
fn test_intpart_fracpart() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let intpart = engine.eval("INTPART(CLOSE)", &mut ctx).unwrap();
    let fracpart = engine.eval("FRACPART(CLOSE)", &mut ctx).unwrap();
    assert_eq!(intpart.len(), 50);
    assert_eq!(fracpart.len(), 50);
}

#[test]
fn test_mod_function() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("MOD(CLOSE, 10)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_reverse() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("REVERSE(CLOSE)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_tr_function() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("TR()", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!(result[0] >= 0.0);
}

#[test]
fn test_const_function() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("CONST(CLOSE)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    let last_val = result[0];
    for i in 0..50 {
        assert!((result[i] - last_val).abs() < 1e-10);
    }
}

#[test]
fn test_sumbars() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("SUMBARS(VOL, 5000)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_range_function() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("RANGE(CLOSE, 10, 15)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_valuewhen() {
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

#[test]
fn test_last_function() {
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

#[test]
fn test_barslastcount() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        UP := CLOSE > REF(CLOSE, 1);
        BLC := BARSLASTCOUNT(UP);
        RESULT: BLC
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_datetime_functions() {
    let mut engine = FormulaEngine::new();
    let datetime = Array1::from_vec((0..50).map(|i| 1704067200 + i * 86400).collect());
    let ctx = make_ctx(50).with_datetime(datetime);
    let mut ctx = ctx;
    let year = engine.eval("YEAR()", &mut ctx).unwrap();
    let month = engine.eval("MONTH()", &mut ctx).unwrap();
    let day = engine.eval("DAY()", &mut ctx).unwrap();
    let weekday = engine.eval("WEEKDAY()", &mut ctx).unwrap();
    assert_eq!(year.len(), 50);
    assert_eq!(month.len(), 50);
    assert_eq!(day.len(), 50);
    assert_eq!(weekday.len(), 50);
}

#[test]
fn test_currbarscount() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("CURRBARSCOUNT()", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!((result[0] - 50.0).abs() < 1e-10);
    assert!((result[49] - 1.0).abs() < 1e-10);
}

#[test]
fn test_totalbarscount() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("TOTALBARSCOUNT()", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    for i in 0..50 {
        assert!((result[i] - 50.0).abs() < 1e-10);
    }
}

#[test]
fn test_barssince() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("BARSSINCE(CLOSE > 10.5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_barssincen() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("BARSSINCEN(CLOSE > 10.5, 3)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_barscount() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("BARSCOUNT(CLOSE)", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_barstatus() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("BARSTATUS()", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!((result[0] - 1.0).abs() < 1e-10);
    assert!((result[49] - 2.0).abs() < 1e-10);
}

#[test]
fn test_islastbar() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(50);
    let result = engine.eval("ISLASTBAR()", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
    assert!((result[0] - 0.0).abs() < 1e-10);
    assert!((result[49] - 1.0).abs() < 1e-10);
}

#[test]
fn test_fromopen() {
    let mut engine = FormulaEngine::new();
    let datetime = Array1::from_vec((0..50).map(|i| 1704067200 + i * 86400 + 34200).collect());
    let ctx = make_ctx(50).with_datetime(datetime);
    let mut ctx = ctx;
    let result = engine.eval("FROMOPEN()", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_date_tdx() {
    let mut engine = FormulaEngine::new();
    let datetime = Array1::from_vec((0..50).map(|i| 1704067200 + i * 86400).collect());
    let ctx = make_ctx(50).with_datetime(datetime);
    let mut ctx = ctx;
    let result = engine.eval("DATE()", &mut ctx).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn test_compatibility_summary() {
    let mut engine = FormulaEngine::new();
    let formulas = vec![
        ("WINNER(CLOSE)", true),
        ("COST(50)", true),
        ("LWINNER(CLOSE, 5)", true),
        ("FINANCE(1)", true),
        ("DYNAINFO(3)", true),
        ("CAPITAL", true),
        ("INDEXC()", true),
        ("PEAK(HIGH, 5, 1)", true),
        ("TROUGH(LOW, 5, 1)", true),
        ("PEAKBARS(HIGH, 5, 1)", true),
        ("TROUGHBARS(LOW, 5, 1)", true),
        ("ZIGZAG(CLOSE, 5)", true),
        ("FINDHIGH(HIGH, 20, 1, 0)", true),
        ("FINDLOW(LOW, 20, 1, 0)", true),
        ("TOPN(CLOSE, 10)", true),
        ("DRAWNULL", true),
        ("AUTOFILTER()", true),
        ("CUMSUM(CLOSE)", true),
        ("CUMMAX(HIGH)", true),
        ("CUMMIN(LOW)", true),
        ("PERCENTILE(CLOSE, 20, 75)", true),
        ("MEDIAN(CLOSE, 20)", true),
        ("SKEW(CLOSE, 20)", true),
        ("KURT(CLOSE, 20)", true),
        ("MODE(CLOSE, 20)", true),
        ("RANK(CLOSE, 20)", true),
        ("SORT(CLOSE, 20, 1)", true),
        ("AVEDEV(CLOSE, 20)", true),
        ("DEVSQ(CLOSE, 20)", true),
        ("SLOPE(CLOSE, 20)", true),
        ("FORCAST(CLOSE, 20)", true),
        ("INTPART(CLOSE)", true),
        ("FRACPART(CLOSE)", true),
        ("MOD(CLOSE, 10)", true),
        ("REVERSE(CLOSE)", true),
        ("TR()", true),
        ("CONST(CLOSE)", true),
        ("SUMBARS(VOL, 5000)", true),
        ("RANGE(CLOSE, 10, 15)", true),
        ("VALUEWHEN(CLOSE > 10, CLOSE)", true),
        ("LAST(CLOSE > REF(CLOSE, 1), 5, 1)", true),
        ("BARSLASTCOUNT(CLOSE > 10)", true),
        ("CURRBARSCOUNT()", true),
        ("TOTALBARSCOUNT()", true),
        ("BARSSINCE(CLOSE > 10.5)", true),
        ("BARSSINCEN(CLOSE > 10.5, 3)", true),
        ("BARSCOUNT(CLOSE)", true),
        ("BARSTATUS()", true),
        ("ISLASTBAR()", true),
        ("PERIODTYPE()", true),
        ("REFDATE(CLOSE, 10)", true),
    ];
    
    let mut passed = 0;
    let mut failed = 0;
    
    for (formula, should_pass) in &formulas {
        let mut ctx = make_ctx(100);
        let result = engine.eval(formula, &mut ctx);
        if *should_pass {
            if result.is_ok() {
                passed += 1;
            } else {
                failed += 1;
                eprintln!("FAILED: {} - {:?}", formula, result.err());
            }
        } else {
            if result.is_err() {
                passed += 1;
            } else {
                failed += 1;
            }
        }
    }
    
    let total = formulas.len();
    let compatibility_rate = (passed as f64 / total as f64) * 100.0;
    
    println!("TDX Compatibility Test Summary:");
    println!("  Total formulas tested: {}", total);
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);
    println!("  Compatibility rate: {:.1}%", compatibility_rate);
    
    assert!(compatibility_rate >= 95.0, "Compatibility rate should be >= 95%");
}