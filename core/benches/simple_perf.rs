use finkit::indicators;
use finkit::math::moving_avg;
use std::time::Instant;

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

fn benchmark_indicator<F>(name: &str, iterations: usize, f: F) -> f64
where
    F: Fn(),
{
    // Warmup
    for _ in 0..10 {
        f();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / iterations as f64;
    avg_ms
}

#[test]
fn test_all_indicators_performance() {
    let (_open, high, low, close, volume) = generate_test_data(10000);
    let iterations = 100;

    println!(
        "\n=== AlphaTA Performance Benchmark (10000 bars, {} iterations) ===\n",
        iterations
    );

    // Overlap Indicators
    println!("【Overlap Indicators】");

    let time = benchmark_indicator("SMA_20", iterations, || {
        let _ = moving_avg::sma(&close, 20).unwrap();
    });
    println!("SMA(20): {:.3} ms", time);

    let time = benchmark_indicator("EMA_20", iterations, || {
        let _ = moving_avg::ema(&close, 20).unwrap();
    });
    println!("EMA(20): {:.3} ms", time);

    let time = benchmark_indicator("WMA_20", iterations, || {
        let _ = moving_avg::wma(&close, 20).unwrap();
    });
    println!("WMA(20): {:.3} ms", time);

    let time = benchmark_indicator("DEMA_20", iterations, || {
        let _ = moving_avg::dema(&close, 20).unwrap();
    });
    println!("DEMA(20): {:.3} ms", time);

    let time = benchmark_indicator("TEMA_20", iterations, || {
        let _ = moving_avg::tema(&close, 20).unwrap();
    });
    println!("TEMA(20): {:.3} ms", time);

    let time = benchmark_indicator("KAMA_10", iterations, || {
        let _ = moving_avg::kama(&close, 10, 2, 30).unwrap();
    });
    println!("KAMA(10): {:.3} ms", time);

    let time = benchmark_indicator("MAMA", iterations, || {
        let _ = indicators::mama(&close, 0.5, 0.05).unwrap();
    });
    println!("MAMA: {:.3} ms", time);

    let time = benchmark_indicator("T3_20", iterations, || {
        let _ = indicators::t3(&close, 20, 0.7).unwrap();
    });
    println!("T3(20): {:.3} ms", time);

    // Momentum Indicators
    println!("\n【Momentum Indicators】");

    let time = benchmark_indicator("RSI_14", iterations, || {
        let _ = indicators::rsi(&close, 14).unwrap();
    });
    println!("RSI(14): {:.3} ms", time);

    let time = benchmark_indicator("MACD", iterations, || {
        let _ = indicators::macd(&close, 12, 26, 9).unwrap();
    });
    println!("MACD: {:.3} ms", time);

    let time = benchmark_indicator("ADX_14", iterations, || {
        let _ = indicators::adx(&high, &low, &close, 14).unwrap();
    });
    println!("ADX(14): {:.3} ms", time);

    let time = benchmark_indicator("CCI_20", iterations, || {
        let _ = indicators::cci(&high, &low, &close, 20).unwrap();
    });
    println!("CCI(20): {:.3} ms", time);

    let time = benchmark_indicator("WILLR_14", iterations, || {
        let _ = indicators::willr(&high, &low, &close, 14).unwrap();
    });
    println!("WILLR(14): {:.3} ms", time);

    let time = benchmark_indicator("AROON_14", iterations, || {
        let _ = indicators::aroon(&high, &low, 14).unwrap();
    });
    println!("AROON(14): {:.3} ms", time);

    let time = benchmark_indicator("MOM_10", iterations, || {
        let _ = indicators::mom(&close, 10).unwrap();
    });
    println!("MOM(10): {:.3} ms", time);

    let time = benchmark_indicator("ROC_10", iterations, || {
        let _ = indicators::roc(&close, 10).unwrap();
    });
    println!("ROC(10): {:.3} ms", time);

    let time = benchmark_indicator("APO", iterations, || {
        let _ = indicators::apo(&close, 12, 26).unwrap();
    });
    println!("APO: {:.3} ms", time);

    let time = benchmark_indicator("PPO", iterations, || {
        let _ = indicators::ppo(&close, 12, 26).unwrap();
    });
    println!("PPO: {:.3} ms", time);

    let time = benchmark_indicator("TRIX_30", iterations, || {
        let _ = indicators::trix(&close, 30).unwrap();
    });
    println!("TRIX(30): {:.3} ms", time);

    // Volatility Indicators
    println!("\n【Volatility Indicators】");

    let time = benchmark_indicator("BBANDS_20", iterations, || {
        let _ = indicators::bbands(&close, 20, 2.0, 2.0).unwrap();
    });
    println!("BBANDS(20): {:.3} ms", time);

    let time = benchmark_indicator("ATR_14", iterations, || {
        let _ = indicators::atr(&high, &low, &close, 14).unwrap();
    });
    println!("ATR(14): {:.3} ms", time);

    let time = benchmark_indicator("NATR_14", iterations, || {
        let _ = indicators::natr(&high, &low, &close, 14).unwrap();
    });
    println!("NATR(14): {:.3} ms", time);

    // Volume Indicators
    println!("\n【Volume Indicators】");

    let time = benchmark_indicator("AD", iterations, || {
        let _ = indicators::ad(&high, &low, &close, &volume).unwrap();
    });
    println!("AD: {:.3} ms", time);

    let time = benchmark_indicator("ADOSC", iterations, || {
        let _ = indicators::adosc(&high, &low, &close, &volume, 3, 10).unwrap();
    });
    println!("ADOSC: {:.3} ms", time);

    let time = benchmark_indicator("OBV", iterations, || {
        let _ = indicators::obv(&close, &volume).unwrap();
    });
    println!("OBV: {:.3} ms", time);

    // Cycle Indicators
    println!("\n【Cycle Indicators (Hilbert Transform)】");

    let time = benchmark_indicator("HT_DCPERIOD", iterations, || {
        let _ = indicators::ht_dcperiod(&close).unwrap();
    });
    println!("HT_DCPERIOD: {:.3} ms", time);

    let time = benchmark_indicator("HT_DCPHASE", iterations, || {
        let _ = indicators::ht_dcphase(&close).unwrap();
    });
    println!("HT_DCPHASE: {:.3} ms", time);

    let time = benchmark_indicator("HT_PHASOR", iterations, || {
        let _ = indicators::ht_phasor(&close).unwrap();
    });
    println!("HT_PHASOR: {:.3} ms", time);

    let time = benchmark_indicator("HT_SINE", iterations, || {
        let _ = indicators::ht_sine(&close).unwrap();
    });
    println!("HT_SINE: {:.3} ms", time);

    let time = benchmark_indicator("HT_TRENDLINE", iterations, || {
        let _ = indicators::ht_trendline(&close).unwrap();
    });
    println!("HT_TRENDLINE: {:.3} ms", time);

    let time = benchmark_indicator("HT_TRENDMODE", iterations, || {
        let _ = indicators::ht_trendmode(&close).unwrap();
    });
    println!("HT_TRENDMODE: {:.3} ms", time);

    println!("\n=== Benchmark Complete ===\n");
}
