use finkit::formula::engine::FormulaEngine;
use finkit::formula::types::{EmData, FormulaContext};
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

fn make_ctx_with_em_data(len: usize) -> FormulaContext {
    let ctx = make_ctx(len);
    let mut em_data = EmData::default();

    let buy_vol: Vec<f64> = (0..len)
        .map(|i| 500.0 + (i as f64 * 0.5).sin() * 100.0)
        .collect();
    let sell_vol: Vec<f64> = (0..len)
        .map(|i| 400.0 + (i as f64 * 0.3).cos() * 80.0)
        .collect();
    let zlccv: Vec<f64> = (0..len)
        .map(|i| 100.0 + (i as f64 * 0.4).sin() * 30.0)
        .collect();

    em_data
        .dkcol_data
        .insert("BUYVOL".to_string(), Array1::from_vec(buy_vol));
    em_data
        .dkcol_data
        .insert("SELLVOL".to_string(), Array1::from_vec(sell_vol));
    em_data
        .dkcol_data
        .insert("ZLCCV".to_string(), Array1::from_vec(zlccv));

    let ext_close: Vec<f64> = (0..len).map(|i| 3000.0 + i as f64 * 0.5).collect();
    let ext_vol: Vec<f64> = (0..len).map(|i| 200000.0 + i as f64 * 100.0).collect();
    em_data
        .external_data
        .insert("INDEX_CLOSE".to_string(), Array1::from_vec(ext_close));
    em_data
        .external_data
        .insert("INDEX_VOL".to_string(), Array1::from_vec(ext_vol));

    ctx.with_em_data(em_data)
}

#[test]
fn test_em_dkcol_with_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_em_data(100);
    let result = engine.eval("DKCOL()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    for i in 0..100 {
        if !result[i].is_nan() {
            let expected =
                500.0 + (i as f64 * 0.5).sin() * 100.0 - (400.0 + (i as f64 * 0.3).cos() * 80.0);
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }
}

#[test]
fn test_em_dkcol_without_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("DKCOL()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result.iter().all(|v| v.is_nan()));
}

#[test]
fn test_em_cross_basic() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let source = r#"
        MA5 := MA(CLOSE, 5);
        MA10 := MA(CLOSE, 10);
        RESULT: EM_CROSS(MA5, MA10)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    for i in 0..100 {
        assert!(result[i] == 0.0 || result[i] == 1.0);
    }
}

#[test]
fn test_em_cross_with_crossing_data() {
    let mut engine = FormulaEngine::new();
    let close: Vec<f64> = (0..20)
        .map(|i| {
            if i < 10 {
                10.0 - i as f64 * 0.5
            } else {
                5.0 + (i - 10) as f64 * 0.5
            }
        })
        .collect();
    let open: Vec<f64> = close.iter().map(|c| c - 0.1).collect();
    let high: Vec<f64> = close.iter().map(|c| c + 0.5).collect();
    let low: Vec<f64> = close.iter().map(|c| c - 0.5).collect();
    let volume: Vec<f64> = vec![1000.0; 20];
    let ctx = FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    );
    let mut ctx = ctx;
    let source = r#"
        A := CLOSE;
        B := 8.0;
        RESULT: EM_CROSS(A, B)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 20);
    let cross_count: usize = result.iter().filter(|&&v| v == 1.0).count();
    assert!(cross_count >= 1);
}

#[test]
fn test_em_ref_with_external_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_em_data(100);
    let idx = ctx.string_table.len();
    ctx.string_table.push("INDEX_CLOSE".to_string());
    let formula = format!("EM_REF({}, 5)", idx);
    let result = engine.eval(&formula, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    for i in 5..100 {
        if !result[i].is_nan() {
            let expected = 3000.0 + (i - 5) as f64 * 0.5;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }
    for i in 0..5 {
        assert!(result[i].is_nan());
    }
}

#[test]
fn test_em_ref_without_em_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let idx = ctx.string_table.len();
    ctx.string_table.push("INDEX_CLOSE".to_string());
    let formula = format!("EM_REF({}, 5)", idx);
    let result = engine.eval(&formula, &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result.iter().all(|v| v.is_nan()));
}

#[test]
fn test_em_zig() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("EM_ZIG(1, 5)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_em_trough() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("EM_TROUGH(1, 5, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_em_peak() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("EM_PEAK(1, 5, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_em_troughbars() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("EM_TROUGHBARS(1, 5, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_em_peakbars() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("EM_PEAKBARS(1, 5, 1)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
}

#[test]
fn test_em_costex() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("EM_COSTEX(CLOSE, VOLUME)", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    for i in 1..100 {
        if !result[i].is_nan() {
            assert!(result[i] > 0.0);
        }
    }
}

#[test]
fn test_em_costex_cumulative() {
    let mut engine = FormulaEngine::new();
    let close = Array1::from_vec(vec![10.0, 11.0, 12.0, 10.0, 13.0]);
    let open = Array1::from_vec(vec![9.9, 10.9, 11.9, 9.9, 12.9]);
    let high = Array1::from_vec(vec![10.5, 11.5, 12.5, 10.5, 13.5]);
    let low = Array1::from_vec(vec![9.5, 10.5, 11.5, 9.5, 12.5]);
    let vol = Array1::from_vec(vec![100.0, 200.0, 150.0, 300.0, 250.0]);
    let ctx = FormulaContext::new(open, high, low, close, vol, None);
    let mut ctx = ctx;
    let result = engine.eval("EM_COSTEX(CLOSE, VOLUME)", &mut ctx).unwrap();
    assert_eq!(result.len(), 5);
    let total_cost = 10.0 * 100.0 + 11.0 * 200.0 + 12.0 * 150.0 + 10.0 * 300.0 + 13.0 * 250.0;
    let total_vol = 100.0 + 200.0 + 150.0 + 300.0 + 250.0;
    let expected_avg = total_cost / total_vol;
    assert!((result[4] - expected_avg).abs() < 1e-10);
}

#[test]
fn test_em_zlccv_with_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx_with_em_data(100);
    let result = engine.eval("EM_ZLCCV()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    for i in 0..100 {
        let expected = 100.0 + (i as f64 * 0.4).sin() * 30.0;
        assert!((result[i] - expected).abs() < 1e-10);
    }
}

#[test]
fn test_em_zlccv_without_data() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(100);
    let result = engine.eval("EM_ZLCCV()", &mut ctx).unwrap();
    assert_eq!(result.len(), 100);
    assert!(result.iter().all(|v| v.is_nan()));
}

#[test]
fn test_em_data_injection_interface() {
    let mut em_data = EmData::default();
    em_data.dkcol_data.insert(
        "BUYVOL".to_string(),
        Array1::from_vec(vec![100.0, 200.0, 150.0]),
    );
    em_data.dkcol_data.insert(
        "SELLVOL".to_string(),
        Array1::from_vec(vec![80.0, 180.0, 120.0]),
    );
    em_data.external_data.insert(
        "SH000001".to_string(),
        Array1::from_vec(vec![3000.0, 3010.0, 3020.0]),
    );

    assert_eq!(em_data.dkcol_data.len(), 2);
    assert_eq!(em_data.external_data.len(), 1);
    assert_eq!(em_data.dkcol_data.get("BUYVOL").unwrap().len(), 3);
    assert_eq!(em_data.external_data.get("SH000001").unwrap().len(), 3);
}

#[test]
fn test_em_dkcol_with_injected_data() {
    let mut engine = FormulaEngine::new();
    let close = Array1::from_vec(vec![10.0, 11.0, 12.0]);
    let open = Array1::from_vec(vec![9.9, 10.9, 11.9]);
    let high = Array1::from_vec(vec![10.5, 11.5, 12.5]);
    let low = Array1::from_vec(vec![9.5, 10.5, 11.5]);
    let vol = Array1::from_vec(vec![1000.0, 2000.0, 1500.0]);

    let mut em_data = EmData::default();
    em_data.dkcol_data.insert(
        "BUYVOL".to_string(),
        Array1::from_vec(vec![500.0, 800.0, 600.0]),
    );
    em_data.dkcol_data.insert(
        "SELLVOL".to_string(),
        Array1::from_vec(vec![300.0, 700.0, 400.0]),
    );

    let ctx = FormulaContext::new(open, high, low, close, vol, None).with_em_data(em_data);
    let mut ctx = ctx;
    let result = engine.eval("DKCOL()", &mut ctx).unwrap();
    assert_eq!(result.len(), 3);
    assert!((result[0] - 200.0).abs() < 1e-10);
    assert!((result[1] - 100.0).abs() < 1e-10);
    assert!((result[2] - 200.0).abs() < 1e-10);
}

#[test]
fn test_em_cross_with_explicit_crossing() {
    let mut engine = FormulaEngine::new();
    let close = Array1::from_vec(vec![3.0, 3.5, 3.8, 4.2, 4.5, 5.0, 4.5, 4.0]);
    let open = close.clone();
    let high = close.clone();
    let low = close.clone();
    let vol = Array1::from_vec(vec![1000.0; 8]);
    let ctx = FormulaContext::new(open, high, low, close, vol, None);
    let mut ctx = ctx;
    let source = r#"
        A := CLOSE;
        B := 4.0;
        RESULT: EM_CROSS(A, B)
    "#;
    let result = engine.eval(source, &mut ctx).unwrap();
    assert_eq!(result.len(), 8);
    let cross_count: usize = result.iter().filter(|&&v| v == 1.0).count();
    assert!(cross_count >= 1);
}

#[test]
fn test_em_ref_with_external_data_lookup() {
    let mut engine = FormulaEngine::new();
    let close = Array1::from_vec(vec![10.0, 11.0, 12.0, 13.0, 14.0]);
    let open = close.clone();
    let high = close.clone();
    let low = close.clone();
    let vol = Array1::from_vec(vec![1000.0; 5]);

    let mut em_data = EmData::default();
    em_data.external_data.insert(
        "SH000001".to_string(),
        Array1::from_vec(vec![3000.0, 3010.0, 3020.0, 3030.0, 3040.0]),
    );

    let mut ctx = FormulaContext::new(open, high, low, close, vol, None).with_em_data(em_data);
    let idx = ctx.string_table.len();
    ctx.string_table.push("SH000001".to_string());
    let formula = format!("EM_REF({}, 2)", idx);
    let result = engine.eval(&formula, &mut ctx).unwrap();
    assert_eq!(result.len(), 5);
    assert!(result[0].is_nan());
    assert!(result[1].is_nan());
    assert!((result[2] - 3000.0).abs() < 1e-10);
    assert!((result[3] - 3010.0).abs() < 1e-10);
    assert!((result[4] - 3020.0).abs() < 1e-10);
}

#[test]
fn test_em_compatibility_summary() {
    let mut engine = FormulaEngine::new();

    let formulas = vec![
        ("DKCOL()", true),
        ("EM_CROSS(MA(CLOSE,5), MA(CLOSE,10))", true),
        ("EM_ZIG(1, 5)", true),
        ("EM_TROUGH(1, 5, 1)", true),
        ("EM_PEAK(1, 5, 1)", true),
        ("EM_TROUGHBARS(1, 5, 1)", true),
        ("EM_PEAKBARS(1, 5, 1)", true),
        ("EM_COSTEX(CLOSE, VOLUME)", true),
        ("EM_ZLCCV()", true),
    ];

    let mut passed = 0;
    let total = formulas.len();

    for (formula, should_pass) in &formulas {
        let mut ctx = make_ctx_with_em_data(100);
        let result = engine.eval(formula, &mut ctx);
        if *should_pass {
            if result.is_ok() {
                passed += 1;
            } else {
                eprintln!("FAILED: {} - {:?}", formula, result.err());
            }
        } else {
            if result.is_err() {
                passed += 1;
            } else {
                eprintln!("EXPECTED FAIL: {} - passed unexpectedly", formula);
            }
        }
    }

    let compatibility_rate = (passed as f64 / total as f64) * 100.0;
    println!("EM Compatibility Test Summary:");
    println!("  Total formulas tested: {}", total);
    println!("  Passed: {}", passed);
    println!("  Compatibility rate: {:.1}%", compatibility_rate);
    assert!(
        compatibility_rate >= 95.0,
        "Compatibility rate should be >= 95%"
    );
}

#[test]
fn test_em_no_panic_edge_cases() {
    let mut engine = FormulaEngine::new();

    let mut ctx = make_ctx(1);
    let _ = engine.eval("DKCOL()", &mut ctx);
    let _ = engine.eval("EM_ZLCCV()", &mut ctx);

    let mut ctx = make_ctx(10);
    let _ = engine.eval("EM_ZIG(1, 5)", &mut ctx);
    let _ = engine.eval("EM_TROUGH(1, 5, 1)", &mut ctx);
    let _ = engine.eval("EM_PEAK(1, 5, 1)", &mut ctx);
    let _ = engine.eval("EM_COSTEX(CLOSE, VOLUME)", &mut ctx);

    let close = Array1::from_vec(vec![0.0; 20]);
    let open = close.clone();
    let high = close.clone();
    let low = close.clone();
    let vol = close.clone();
    let mut ctx = FormulaContext::new(open, high, low, close, vol, None);
    let _ = engine.eval("EM_COSTEX(CLOSE, VOLUME)", &mut ctx);
    let _ = engine.eval("EM_CROSS(CLOSE, OPEN)", &mut ctx);
}
