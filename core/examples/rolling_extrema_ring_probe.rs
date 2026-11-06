use finkit::indicators::math_operators::{max, min};
use ndarray::Array1;
use std::collections::VecDeque;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn rolling_extreme_ring<const WANT_MAX: bool>(input: &[f64], window: usize) -> Array1<f64> {
    let len = input.len();
    let warm_at = input
        .iter()
        .position(|value| value.is_finite())
        .map_or(len, |start| start + window - 1);
    let mut output = Vec::with_capacity(len);
    // SAFETY: the warm-up prefix is initialized below, and every later output
    // slot is assigned once `i >= warm_at`.
    unsafe { output.set_len(len) };
    output[..window - 1].fill(f64::NAN);
    if warm_at > window - 1 {
        output[window - 1..warm_at].fill(f64::NAN);
    }

    let mut candidates = vec![0_usize; window];
    let mut head = 0_usize;
    let mut count = 0_usize;
    for i in 0..len {
        if count > 0 && candidates[head] + window <= i {
            head += 1;
            if head == window {
                head = 0;
            }
            count -= 1;
        }

        let value = input[i];
        if !value.is_nan() {
            while count > 0 {
                let mut back = head + count - 1;
                if back >= window {
                    back -= window;
                }
                let tail_value = input[candidates[back]];
                let dominated = if WANT_MAX {
                    tail_value < value
                } else {
                    tail_value > value
                };
                if dominated {
                    count -= 1;
                } else {
                    break;
                }
            }
            let mut tail = head + count;
            if tail >= window {
                tail -= window;
            }
            candidates[tail] = i;
            count += 1;
        }

        if i >= warm_at {
            output[i] = if count == 0 {
                f64::NAN
            } else {
                input[candidates[head]]
            };
        }
    }
    Array1::from_vec(output)
}

fn rolling_extreme_deque<const WANT_MAX: bool>(input: &[f64], window: usize) -> Array1<f64> {
    let len = input.len();
    let warm_at = input
        .iter()
        .position(|value| value.is_finite())
        .map_or(len, |start| start + window - 1);
    let mut output = vec![f64::NAN; len];
    let mut candidates = VecDeque::with_capacity(window);
    for i in 0..len {
        if !input[i].is_nan() {
            while candidates.back().is_some_and(|&index| {
                if WANT_MAX {
                    input[index] < input[i]
                } else {
                    input[index] > input[i]
                }
            }) {
                candidates.pop_back();
            }
            candidates.push_back(i);
        }
        while candidates.front().is_some_and(|&index| index + window <= i) {
            candidates.pop_front();
        }
        if i >= warm_at {
            output[i] = candidates.front().map_or(f64::NAN, |&index| input[index]);
        }
    }
    Array1::from_vec(output)
}

fn measure(mut f: impl FnMut() -> Array1<f64>) -> Duration {
    black_box(f());
    let loops = 5;
    let start = Instant::now();
    for _ in 0..loops {
        black_box(f());
    }
    start.elapsed() / loops
}

fn main() {
    let window = 30;
    for len in [100_000, 1_000_000] {
        let input = (0..len)
            .map(|i| {
                let x = i as f64;
                100.0 + 0.006 * x + (x * 0.017).sin() * 2.1
            })
            .collect::<Vec<_>>();
        for want_max in [true, false] {
            let (current, deque, ring) = if want_max {
                (
                    max(&input, window).unwrap(),
                    rolling_extreme_deque::<true>(&input, window),
                    rolling_extreme_ring::<true>(&input, window),
                )
            } else {
                (
                    min(&input, window).unwrap(),
                    rolling_extreme_deque::<false>(&input, window),
                    rolling_extreme_ring::<false>(&input, window),
                )
            };
            let deque_mismatches = current
                .iter()
                .zip(deque.iter())
                .filter(|(left, right)| left != right && !(left.is_nan() && right.is_nan()))
                .count();
            let ring_mismatches = current
                .iter()
                .zip(ring.iter())
                .filter(|(left, right)| left != right && !(left.is_nan() && right.is_nan()))
                .count();
            let current_time = if want_max {
                measure(|| max(black_box(&input), window).unwrap())
            } else {
                measure(|| min(black_box(&input), window).unwrap())
            };
            let deque_time = if want_max {
                measure(|| rolling_extreme_deque::<true>(black_box(&input), window))
            } else {
                measure(|| rolling_extreme_deque::<false>(black_box(&input), window))
            };
            let ring_time = if want_max {
                measure(|| rolling_extreme_ring::<true>(black_box(&input), window))
            } else {
                measure(|| rolling_extreme_ring::<false>(black_box(&input), window))
            };
            println!(
                "{} n={len} current={current_time:?} VecDeque={deque_time:?} fixed-ring={ring_time:?} ring/current={:.3} deque/current={:.3} mismatches={deque_mismatches}/{ring_mismatches}",
                if want_max { "MAX" } else { "MIN" },
                ring_time.as_secs_f64() / current_time.as_secs_f64(),
                deque_time.as_secs_f64() / current_time.as_secs_f64()
            );
        }
    }
}
