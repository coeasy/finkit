use finkit::indicators::talib_ext::percentile;
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn select_each_window(input: &[f64], period: usize, percentile: f64) -> Array1<f64> {
    let rank = ((percentile * period as f64 / 100.0).ceil() as usize).clamp(1, period);
    let mut output = vec![f64::NAN; input.len()];
    let mut window = input[..period].to_vec();
    for i in period - 1..input.len() {
        window.select_nth_unstable_by(rank - 1, f64::total_cmp);
        output[i] = window[rank - 1];
        let next = i + 1;
        if next < input.len() {
            window.copy_from_slice(&input[next + 1 - period..=next]);
        }
    }
    Array1::from_vec(output)
}

fn measure(mut f: impl FnMut() -> Array1<f64>) -> Duration {
    black_box(f());
    let loops = 7;
    let start = Instant::now();
    for _ in 0..loops {
        black_box(f());
    }
    start.elapsed() / loops
}

fn main() {
    let period = 30;
    for len in [100_000, 1_000_000] {
        let input = (0..len)
            .map(|i| {
                let x = i as f64;
                100.0 + 0.006 * x + (x * 0.017).sin() * 2.1
            })
            .collect::<Vec<_>>();
        let baseline = percentile(&input, period, 50.0).unwrap();
        let candidate = select_each_window(&input, period, 50.0);
        let max_abs = baseline
            .iter()
            .zip(candidate.iter())
            .filter(|(a, b)| a.is_finite() && b.is_finite())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, f64::max);
        let baseline_time = measure(|| percentile(black_box(&input), period, 50.0).unwrap());
        let candidate_time = measure(|| select_each_window(black_box(&input), period, 50.0));
        println!(
            "n={len} current={baseline_time:?} select_each_window={candidate_time:?} ratio={:.3} max_abs={max_abs:.3e}",
            baseline_time.as_secs_f64() / candidate_time.as_secs_f64()
        );
    }
}
