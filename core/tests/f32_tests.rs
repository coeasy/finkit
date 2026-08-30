use finkit::streaming::float_trait::*;

const F32_EPS: f32 = 1e-4;

#[test]
fn test_f32_sma_basic() {
    let mut sma = GenericSma::<f32>::new(3);
    assert_eq!(sma.next(1.0_f32), None);
    assert_eq!(sma.next(2.0_f32), None);
    let val = sma.next(3.0_f32).unwrap();
    assert!((val - 2.0_f32).abs() < F32_EPS);
    let val = sma.next(4.0_f32).unwrap();
    assert!((val - 3.0_f32).abs() < F32_EPS);
}

#[test]
fn test_f32_sma_reset() {
    let mut sma = GenericSma::<f32>::new(2);
    sma.next(10.0_f32);
    sma.next(20.0_f32);
    assert!(sma.is_ready());
    sma.reset();
    assert!(!sma.is_ready());
}

#[test]
fn test_f32_ema_basic() {
    let mut ema = GenericEma::<f32>::new(3);
    assert_eq!(ema.next(2.0_f32), None);
    assert_eq!(ema.next(4.0_f32), None);
    let val = ema.next(6.0_f32).unwrap();
    assert!((val - 4.0_f32).abs() < F32_EPS);
}

#[test]
fn test_f32_ema_reset() {
    let mut ema = GenericEma::<f32>::new(2);
    ema.next(10.0_f32);
    ema.next(20.0_f32);
    assert!(ema.is_ready());
    ema.reset();
    assert!(!ema.is_ready());
}

#[test]
fn test_f32_rsi_range() {
    let mut rsi = GenericRsi::<f32>::new(14);
    let data: Vec<f32> = (0..30).map(|i| 50.0_f32 + (i as f32 * 0.1).sin() * 10.0).collect();
    let mut last = None;
    for &d in &data {
        last = rsi.next(d);
    }
    let val = last.unwrap();
    assert!((0.0_f32..=100.0_f32).contains(&val), "RSI f32 out of range: {val}");
}

#[test]
fn test_f32_rsi_reset() {
    let mut rsi = GenericRsi::<f32>::new(5);
    for i in 0..10 {
        rsi.next(i as f32);
    }
    assert!(rsi.is_ready());
    rsi.reset();
    assert!(!rsi.is_ready());
}

#[test]
fn test_f32_macd_basic() {
    let mut macd = GenericMacd::<f32>::new(3, 5, 3);
    let mut ready = false;
    for i in 1..=10 {
        if let Some(out) = macd.next(i as f32) {
            assert!(!out.macd.is_nan());
            assert!(!out.signal.is_nan());
            assert!(!out.histogram.is_nan());
            ready = true;
        }
    }
    assert!(ready);
}

#[test]
fn test_f32_macd_reset() {
    let mut macd = GenericMacd::<f32>::new(3, 5, 3);
    for i in 1..=10 {
        macd.next(i as f32);
    }
    assert!(macd.is_ready());
    macd.reset();
    assert!(!macd.is_ready());
}

#[test]
fn test_f32_boll_basic() {
    let mut boll = GenericBoll::<f32>::new(5, 2.0_f32, 2.0_f32);
    for i in 1..=4 {
        assert!(boll.next(i as f32).is_none());
    }
    let out = boll.next(5.0_f32).unwrap();
    assert!(out.upper > out.middle);
    assert!(out.lower < out.middle);
}

#[test]
fn test_f32_boll_reset() {
    let mut boll = GenericBoll::<f32>::new(3, 2.0_f32, 2.0_f32);
    for i in 1..=5 {
        boll.next(i as f32);
    }
    assert!(boll.is_ready());
    boll.reset();
    assert!(!boll.is_ready());
}

#[test]
fn test_f32_atr_basic() {
    let mut atr = GenericAtr::<f32>::new(3);
    atr.next((12.0_f32, 10.0_f32, 11.0_f32));
    atr.next((13.0_f32, 11.0_f32, 12.0_f32));
    let val = atr.next((14.0_f32, 12.0_f32, 13.0_f32)).unwrap();
    assert!(val > 0.0_f32);
}

#[test]
fn test_f32_atr_reset() {
    let mut atr = GenericAtr::<f32>::new(2);
    atr.next((12.0_f32, 10.0_f32, 11.0_f32));
    atr.next((13.0_f32, 11.0_f32, 12.0_f32));
    assert!(atr.is_ready());
    atr.reset();
    assert!(!atr.is_ready());
}

#[test]
fn test_f32_sma_longer_series() {
    let mut sma = GenericSma::<f32>::new(10);
    let data: Vec<f32> = (0..100).map(|i| 50.0_f32 + (i as f32 * 0.1).sin() * 10.0).collect();
    let mut count = 0;
    for &v in &data {
        if sma.next(v).is_some() {
            count += 1;
        }
    }
    assert_eq!(count, 91);
}

#[test]
fn test_f32_f64_sma_convergence() {
    let data: Vec<f64> = (0..50).map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0).collect();
    let mut sma64 = GenericSma::<f64>::new(5);
    let mut sma32 = GenericSma::<f32>::new(5);

    for &v in &data {
        let v64 = sma64.next(v);
        let v32 = sma32.next(v as f32);
        match (v64, v32) {
            (Some(a), Some(b)) => {
                assert!((a - b as f64).abs() < 0.01, "f32/f64 divergence: {a} vs {b}");
            }
            (None, None) => {}
            _ => panic!("Readiness mismatch"),
        }
    }
}

#[test]
fn test_f32_ema_longer_series() {
    let mut ema = GenericEma::<f32>::new(14);
    let data: Vec<f32> = (0..100).map(|i| 50.0_f32 + (i as f32 * 0.1).sin() * 10.0).collect();
    let mut count = 0;
    for &v in &data {
        if ema.next(v).is_some() {
            count += 1;
        }
    }
    assert_eq!(count, 87);
}

#[test]
fn test_f32_value_caching() {
    let mut sma = GenericSma::<f32>::new(3);
    assert_eq!(sma.value(), None);
    sma.next(1.0_f32);
    sma.next(2.0_f32);
    sma.next(3.0_f32);
    assert!(sma.value().is_some());
    let cached = sma.value().unwrap();
    assert!((cached - 2.0_f32).abs() < F32_EPS);
}
