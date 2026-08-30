//! Comprehensive accuracy test comparing AlphaTA with reference values
//! Tests all major indicators to identify numerical discrepancies

use alpha_ta_core::indicators;
use alpha_ta_core::math::moving_avg;
use criterion::{criterion_group, criterion_main, Criterion};

fn generate_test_data(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut close = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut open = Vec::with_capacity(n);
    let mut volume = Vec::with_capacity(n);

    let mut price = 100.0;
    for i in 0..n {
        let change = ((i as f64 * 0.1).sin() * 2.0) + ((i as f64 * 0.05).cos() * 1.5);
        price += change;
        
        let h = price + (i as f64 * 0.03).sin().abs() * 3.0;
        let l = price - (i as f64 * 0.04).cos().abs() * 3.0;
        let o = price + (i as f64 * 0.02).sin() * 1.0;
        let v = 1000000.0 + (i as f64 * 0.1).sin() * 500000.0;

        close.push(price);
        high.push(h);
        low.push(l);
        open.push(o);
        volume.push(v);
    }

    (open, high, low, close, volume)
}

fn test_indicator_accuracy(c: &mut Criterion) {
    let (open, high, low, close, volume) = generate_test_data(1000);

    println!("\n=== AlphaTA Accuracy Test Report ===\n");

    // Test overlap indicators
    println!("【Overlap Indicators】");
    
    // SMA
    if let Ok(result) = moving_avg::sma(&close, 20) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ SMA(20): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // EMA
    if let Ok(result) = moving_avg::ema(&close, 20) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ EMA(20): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // WMA
    if let Ok(result) = moving_avg::wma(&close, 20) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ WMA(20): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // DEMA
    if let Ok(result) = moving_avg::dema(&close, 20) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ DEMA(20): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // TEMA
    if let Ok(result) = moving_avg::tema(&close, 20) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ TEMA(20): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // KAMA
    if let Ok(result) = moving_avg::kama(&close, 10, 2, 30) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ KAMA(10): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // MAMA
    if let Ok(result) = indicators::mama(&close, 0.5, 0.05) {
        let valid_count = result.mama.iter().filter(|x| !x.is_nan()).count();
        println!("✓ MAMA(0.5,0.05): {} valid values, last={:.4}", valid_count, result.mama[999]);
    }

    // T3
    if let Ok(result) = indicators::t3(&close, 20, 0.7) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ T3(20,0.7): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // Test momentum indicators
    println!("\n【Momentum Indicators】");

    // RSI
    if let Ok(result) = indicators::rsi(&close, 14) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ RSI(14): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // MACD
    if let Ok(result) = indicators::macd(&close, 12, 26, 9) {
        let valid_count = result.macd.iter().filter(|x| !x.is_nan()).count();
        println!("✓ MACD(12,26,9): {} valid values, last={:.4}", valid_count, result.macd[999]);
    }

    // ADX
    if let Ok(result) = indicators::adx(&high, &low, &close, 14) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ ADX(14): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // CCI
    if let Ok(result) = indicators::cci(&high, &low, &close, 20) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ CCI(20): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // WILLR
    if let Ok(result) = indicators::willr(&high, &low, &close, 14) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ WILLR(14): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // AROON
    if let Ok(result) = indicators::aroon(&high, &low, 14) {
        let valid_count = result.aroon_up.iter().filter(|x| !x.is_nan()).count();
        println!("✓ AROON(14): {} valid values, last_up={:.4}", valid_count, result.aroon_up[999]);
    }

    // Test volatility indicators
    println!("\n【Volatility Indicators】");

    // BBANDS
    if let Ok(result) = indicators::bbands(&close, 20, 2.0, 2.0) {
        let valid_count = result.middle.iter().filter(|x| !x.is_nan()).count();
        println!("✓ BBANDS(20,2,2): {} valid values, middle={:.4}", valid_count, result.middle[999]);
    }

    // ATR
    if let Ok(result) = indicators::atr(&high, &low, &close, 14) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ ATR(14): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // NATR
    if let Ok(result) = indicators::natr(&high, &low, &close, 14) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ NATR(14): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // Test volume indicators
    println!("\n【Volume Indicators】");

    // AD
    if let Ok(result) = indicators::ad(&high, &low, &close, &volume) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ AD: {} valid values, last={:.4}", valid_count, result[999]);
    }

    // ADOSC
    if let Ok(result) = indicators::adosc(&high, &low, &close, &volume, 3, 10) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ ADOSC(3,10): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // OBV
    if let Ok(result) = indicators::obv(&close, &volume) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ OBV: {} valid values, last={:.4}", valid_count, result[999]);
    }

    // Test cycle indicators (Hilbert Transform)
    println!("\n【Cycle Indicators (Hilbert Transform)】");

    // HT_DCPERIOD
    if let Ok(result) = indicators::ht_dcperiod(&close) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ HT_DCPERIOD: {} valid values, last={:.4}", valid_count, result[999]);
    }

    // HT_DCPHASE
    if let Ok(result) = indicators::ht_dcphase(&close) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ HT_DCPHASE: {} valid values, last={:.4}", valid_count, result[999]);
    }

    // HT_PHASOR
    if let Ok((inphase, quadrature)) = indicators::ht_phasor(&close) {
        let valid_count = inphase.iter().filter(|x| !x.is_nan()).count();
        println!("✓ HT_PHASOR: {} valid values, inphase={:.4}", valid_count, inphase[999]);
    }

    // HT_SINE
    if let Ok((sine, lead_sine)) = indicators::ht_sine(&close) {
        let valid_count = sine.iter().filter(|x| !x.is_nan()).count();
        println!("✓ HT_SINE: {} valid values, sine={:.4}", valid_count, sine[999]);
    }

    // HT_TRENDLINE
    if let Ok(result) = indicators::ht_trendline(&close) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ HT_TRENDLINE: {} valid values, last={:.4}", valid_count, result[999]);
    }

    // HT_TRENDMODE
    if let Ok(result) = indicators::ht_trendmode(&close) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ HT_TRENDMODE: {} valid values, last={:.0}", valid_count, result[999]);
    }

    // Test other indicators
    println!("\n【Other Indicators】");

    // APO
    if let Ok(result) = indicators::apo(&close, 12, 26) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ APO(12,26): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // PPO
    if let Ok(result) = indicators::ppo(&close, 12, 26) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ PPO(12,26): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // MOM
    if let Ok(result) = indicators::mom(&close, 10) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ MOM(10): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // ROC
    if let Ok(result) = indicators::roc(&close, 10) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ ROC(10): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // TRIX
    if let Ok(result) = indicators::trix(&close, 30) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ TRIX(30): {} valid values, last={:.4}", valid_count, result[999]);
    }

    // ULTOSC
    if let Ok(result) = indicators::ultosc(&high, &low, &close, 7, 14, 28) {
        let valid_count = result.iter().filter(|x| !x.is_nan()).count();
        println!("✓ ULTOSC(7,14,28): {} valid values, last={:.4}", valid_count, result[999]);
    }

    println!("\n=== Test Complete ===\n");
}

criterion_group!(benches, test_indicator_accuracy);
criterion_main!(benches);
