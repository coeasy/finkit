use finkit::formula::{FormulaContext, FormulaEngine};
use finkit::indicators;
use finkit::math::moving_avg;
use ndarray::Array1;

fn sample(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    let mut volume = Vec::with_capacity(n);
    for i in 0..n {
        let x = i as f64;
        let c = 100.0 + x * 0.03 + (x * 0.13).sin() * 1.7;
        open.push(c - 0.12);
        high.push(c + 0.7 + (x * 0.03).sin().abs());
        low.push(c - 0.8 - (x * 0.05).cos().abs() * 0.2);
        close.push(c);
        volume.push(1_000_000.0 + x * 31.0);
    }
    (open, high, low, close, volume)
}

fn assert_series_eq(left: &[f64], right: &[f64], tolerance: f64) {
    assert_eq!(left.len(), right.len());
    for (i, (&a, &b)) in left.iter().zip(right).enumerate() {
        assert_eq!(
            a.is_finite(),
            b.is_finite(),
            "finite mask differs at {i}: {a} vs {b}"
        );
        if a.is_finite() {
            let scale = a.abs().max(b.abs()).max(1.0);
            assert!(
                (a - b).abs() <= tolerance * scale,
                "value differs at {i}: {a} vs {b}"
            );
        }
    }
}

#[test]
fn warmup_masks_match_talib_contracts() {
    let (_, high, low, close, volume) = sample(128);

    let tr = indicators::trange(&high, &low, &close).unwrap();
    assert!(tr[0].is_nan());
    assert!(tr[1].is_finite());

    let adosc = indicators::adosc(&high, &low, &close, &volume, 3, 10).unwrap();
    assert!(adosc.iter().take(9).all(|v| v.is_nan()));
    assert!(adosc[9].is_finite());

    let kama = moving_avg::kama(&close, 20, 2, 30).unwrap();
    assert!(kama.iter().take(20).all(|v| v.is_nan()));
    assert!(kama[20].is_finite());

    let macd = indicators::macd(&close, 12, 26, 9).unwrap();
    let lookback = 26 + 9 - 2;
    for output in [&macd.macd, &macd.signal, &macd.hist] {
        assert!(output.iter().take(lookback).all(|v| v.is_nan()));
        assert!(output[lookback].is_finite());
    }
}

#[test]
fn formula_atr_std_and_boll_reuse_core_semantics() {
    let (open, high, low, close, volume) = sample(256);
    let mut context = FormulaContext::new(
        Array1::from(open),
        Array1::from(high.clone()),
        Array1::from(low.clone()),
        Array1::from(close.clone()),
        Array1::from(volume),
        None,
    );
    let mut engine = FormulaEngine::new();

    let formula_atr = engine.eval("ATR(HIGH,LOW,CLOSE,14)", &mut context).unwrap();
    let direct_atr = indicators::atr(&high, &low, &close, 14).unwrap();
    assert_series_eq(
        formula_atr.as_slice().unwrap(),
        direct_atr.as_slice().unwrap(),
        1e-12,
    );

    let formula_std = engine.eval("STD(CLOSE,20)", &mut context).unwrap();
    let direct_std = indicators::std_dev(&close, 20, 1.0).unwrap();
    assert_series_eq(
        formula_std.as_slice().unwrap(),
        direct_std.as_slice().unwrap(),
        1e-12,
    );

    let formula_boll = engine.eval("BOLL(CLOSE,20,2)", &mut context).unwrap();
    let direct_boll = indicators::bbands(&close, 20, 2.0, 2.0).unwrap();
    assert_series_eq(
        formula_boll.as_slice().unwrap(),
        direct_boll.upper.as_slice().unwrap(),
        1e-12,
    );
}

#[test]
fn zero_copy_range_matches_full_formula_result() {
    let (open, high, low, close, volume) = sample(512);
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("EMA(CLOSE,20)").unwrap();
    let range = engine
        .eval_range_zero_copy_inputs(
            &compiled, &open, &high, &low, &close, &volume, 450, 512, None,
        )
        .unwrap();

    let mut context = FormulaContext::new(
        Array1::from(open),
        Array1::from(high),
        Array1::from(low),
        Array1::from(close),
        Array1::from(volume),
        None,
    );
    let full = engine.execute(&compiled, &mut context).unwrap();
    assert_series_eq(
        range.as_slice().unwrap(),
        &full.as_slice().unwrap()[450..512],
        1e-12,
    );
}
