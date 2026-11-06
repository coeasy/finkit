use finkit::indicators::math_operators::maxindex;
use ndarray::Array1;
use std::collections::VecDeque;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn deque_maxindex(input: &[f64], period: usize) -> Array1<i64> {
    let len = input.len();
    let mut output = vec![-1_i64; len];
    let mut candidates = VecDeque::with_capacity(period);
    for i in 0..len {
        while candidates
            .front()
            .is_some_and(|&candidate| candidate + period <= i)
        {
            candidates.pop_front();
        }
        while candidates
            .back()
            .is_some_and(|&candidate| input[candidate] < input[i])
        {
            candidates.pop_back();
        }
        candidates.push_back(i);
        if i + 1 >= period {
            output[i] = (candidates[0] - (i + 1 - period)) as i64;
        }
    }
    Array1::from_vec(output)
}

fn cached_maxindex(input: &[f64], period: usize) -> Array1<i64> {
    let len = input.len();
    let mut output = vec![-1_i64; len];
    if period == 1 {
        output.fill(0);
        return Array1::from_vec(output);
    }

    let mut best_index = 0;
    let mut best_value = input[0];
    for i in 1..period {
        if input[i] > best_value {
            best_index = i;
            best_value = input[i];
        }
    }

    for i in period - 1..len {
        let start = i + 1 - period;
        if best_index < start {
            best_index = start;
            best_value = input[start];
            for j in start + 1..=i {
                if input[j] > best_value {
                    best_index = j;
                    best_value = input[j];
                }
            }
        } else if input[i] > best_value {
            best_index = i;
            best_value = input[i];
        }
        output[i] = (best_index - start) as i64;
    }
    Array1::from_vec(output)
}

fn measure(mut f: impl FnMut() -> Array1<i64>) -> Duration {
    black_box(f());
    let loops = 5;
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
        let baseline = maxindex(&input, period).unwrap();
        let deque = deque_maxindex(&input, period);
        let candidate = cached_maxindex(&input, period);
        let deque_mismatches = baseline
            .iter()
            .zip(deque.iter())
            .filter(|(left, right)| left != right)
            .count();
        let mismatches = baseline
            .iter()
            .zip(candidate.iter())
            .filter(|(left, right)| left != right)
            .count();
        let baseline_time = measure(|| maxindex(black_box(&input), period).unwrap());
        let deque_time = measure(|| deque_maxindex(black_box(&input), period));
        let candidate_time = measure(|| cached_maxindex(black_box(&input), period));
        println!(
            "n={len} ring={baseline_time:?} VecDeque={deque_time:?} cached={candidate_time:?} ring/VecDeque={:.3} ring-deque-mismatches={deque_mismatches} ring-cached-mismatches={mismatches}",
            baseline_time.as_secs_f64() / deque_time.as_secs_f64()
        );
    }
}
