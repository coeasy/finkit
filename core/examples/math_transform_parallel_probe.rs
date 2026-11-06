use finkit::indicators::math_transform::log10;
use ndarray::Array1;
use rayon::prelude::*;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn serial(input: &[f64]) -> Array1<f64> {
    let output = input.iter().map(|value| value.log10()).collect::<Vec<_>>();
    assert!(output.iter().all(|value| value.is_finite()));
    Array1::from_vec(output)
}

fn parallel(input: &[f64]) -> Array1<f64> {
    let output = input
        .par_iter()
        .map(|value| value.log10())
        .collect::<Vec<_>>();
    assert!(output.iter().all(|value| value.is_finite()));
    Array1::from_vec(output)
}

fn time(mut f: impl FnMut() -> Array1<f64>) -> f64 {
    let start = Instant::now();
    let mut calls = 0_u64;
    while start.elapsed() < Duration::from_millis(150) {
        black_box(f());
        calls += 1;
    }
    start.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn main() {
    for len in [1_000usize, 10_000, 100_000, 1_000_000] {
        let input = (0..len)
            .map(|index| 0.1 + (index % 1_000) as f64 * 0.01)
            .collect::<Vec<_>>();
        let public = || log10(&input).unwrap();
        let expected = serial(&input);
        let actual = public();
        assert_eq!(expected.as_slice().unwrap(), actual.as_slice().unwrap());
        let (mut serial_times, mut parallel_times, mut public_times) =
            (Vec::new(), Vec::new(), Vec::new());
        for round in 0..7 {
            if round % 2 == 0 {
                serial_times.push(time(|| serial(&input)));
                parallel_times.push(time(|| parallel(&input)));
                public_times.push(time(public));
            } else {
                parallel_times.push(time(|| parallel(&input)));
                serial_times.push(time(|| serial(&input)));
                public_times.push(time(public));
            }
        }
        serial_times.sort_by(f64::total_cmp);
        parallel_times.sort_by(f64::total_cmp);
        public_times.sort_by(f64::total_cmp);
        println!(
            "LOG10 {len}: serial={:.2}us parallel={:.2}us public={:.2}us serial/parallel={:.3}x",
            serial_times[3],
            parallel_times[3],
            public_times[3],
            serial_times[3] / parallel_times[3]
        );
    }
}
