use finkit::indicators::talib_ext::marketfi;
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn data(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let close = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        let open = close + (x * 0.013).sin() * 0.8;
        high.push(open.max(close) + 0.7 + (x * 0.023).sin().abs() * 0.3);
        low.push(open.min(close) - 0.8 - (x * 0.019).cos().abs() * 0.2);
        volume.push(1_000_000.0 + (x % 997.0) * 170.0 + (x * 0.011).sin().abs() * 50_000.0);
    }
    (high, low, volume)
}

fn old(high: &[f64], low: &[f64], volume: &[f64]) -> Array1<f64> {
    let mut out = Array1::from_elem(high.len(), f64::NAN);
    for i in 0..high.len() {
        out[i] = if volume[i] == 0.0 {
            0.0
        } else {
            (high[i] - low[i]) / volume[i]
        };
    }
    out
}

fn time(mut f: impl FnMut() -> Array1<f64>, duration: Duration) -> f64 {
    let started = Instant::now();
    let mut calls = 0u64;
    while started.elapsed() < duration {
        black_box(f());
        calls += 1;
    }
    started.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn main() {
    for len in [1_000usize, 100_000, 1_000_000] {
        let (high, low, volume) = data(len);
        let before = || old(&high, &low, &volume);
        let after = || marketfi(&high, &low, &volume).unwrap();
        let expected = before();
        let actual = after();
        assert_eq!(expected, actual);
        let (mut before_samples, mut after_samples) = (Vec::new(), Vec::new());
        for turn in 0..5 {
            if turn % 2 == 0 {
                before_samples.push(time(&mut { || before() }, Duration::from_millis(120)));
                after_samples.push(time(&mut { || after() }, Duration::from_millis(120)));
            } else {
                after_samples.push(time(&mut { || after() }, Duration::from_millis(120)));
                before_samples.push(time(&mut { || before() }, Duration::from_millis(120)));
            }
        }
        before_samples.sort_by(f64::total_cmp);
        after_samples.sort_by(f64::total_cmp);
        println!(
            "MARKETFI {len}: old={:.2}us new={:.2}us speedup={:.3}x",
            before_samples[2],
            after_samples[2],
            before_samples[2] / after_samples[2]
        );
    }
}
