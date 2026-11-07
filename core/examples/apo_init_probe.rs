use finkit::indicators::{apo, sma};
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn data(len: usize) -> Vec<f64> {
    let mut close = Vec::with_capacity(len);
    for index in 0..len {
        let x = index as f64;
        close.push(100.0 + 0.006 * x + (x * 0.017).sin() * 2.1);
    }
    close
}

fn materialized(input: &[f64]) -> Array1<f64> {
    let fast = sma(input, 12).unwrap();
    let slow = sma(input, 26).unwrap();
    Array1::from_iter(fast.iter().zip(slow.iter()).map(|(&a, &b)| a - b))
}

fn time(mut run: impl FnMut() -> Array1<f64>) -> f64 {
    let start = Instant::now();
    let mut calls = 0_u64;
    while start.elapsed() < Duration::from_millis(150) {
        black_box(run());
        calls += 1;
    }
    start.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn main() {
    for len in [1_000usize, 100_000, 1_000_000] {
        let input = data(len);
        let fused = || apo(&input, 12, 26).unwrap();
        let reference = || materialized(&input);
        let actual = fused();
        let expected = reference();
        assert!(actual
            .iter()
            .zip(expected.iter())
            .all(|(&a, &b)| a.to_bits() == b.to_bits()));
        let (mut fused_times, mut reference_times) = (Vec::new(), Vec::new());
        for round in 0..7 {
            if round % 2 == 0 {
                fused_times.push(time(&mut { || fused() }));
                reference_times.push(time(&mut { || reference() }));
            } else {
                reference_times.push(time(&mut { || reference() }));
                fused_times.push(time(&mut { || fused() }));
            }
        }
        fused_times.sort_by(f64::total_cmp);
        reference_times.sort_by(f64::total_cmp);
        println!(
            "APO {len}: fused={:.2}us materialized={:.2}us speedup={:.3}x",
            fused_times[3],
            reference_times[3],
            reference_times[3] / fused_times[3]
        );
    }
}
