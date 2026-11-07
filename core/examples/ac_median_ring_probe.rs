use finkit::indicators::talib_ext::ac;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn data(len: usize) -> (Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let close = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        high.push(close + 0.7 + (x * 0.023).sin().abs() * 0.3);
        low.push(close - 0.8 - (x * 0.019).cos().abs() * 0.2);
    }
    (high, low)
}

fn old_ac(high: &[f64], low: &[f64], fast: usize, slow: usize, signal: usize) -> Vec<f64> {
    let len = high.len();
    let window = slow.max(fast);
    let osc_start = window - 1;
    let lookback = window + signal - 2;
    let mut fast_sum = 0.0;
    let mut slow_sum = 0.0;
    let mut trailing_fast = osc_start - (fast - 1);
    let mut trailing_slow = osc_start - (slow - 1);
    for i in 0..osc_start {
        let median = (high[i] + low[i]) / 2.0;
        if i >= trailing_fast {
            fast_sum += median;
        }
        if i >= trailing_slow {
            slow_sum += median;
        }
    }

    let mut output = Vec::with_capacity(len);
    output.resize(lookback, f64::NAN);
    let output_spare = output.spare_capacity_mut();
    let mut ring = vec![0.0; signal];
    let mut ring_index = 0;
    let mut signal_sum = 0.0;
    for i in osc_start..len {
        let median = (high[i] + low[i]) / 2.0;
        fast_sum += median;
        slow_sum += median;
        let oscillator = fast_sum / fast as f64 - slow_sum / slow as f64;
        fast_sum -= (high[trailing_fast] + low[trailing_fast]) / 2.0;
        slow_sum -= (high[trailing_slow] + low[trailing_slow]) / 2.0;
        trailing_fast += 1;
        trailing_slow += 1;

        ring[ring_index] = oscillator;
        signal_sum += oscillator;
        if i >= lookback {
            output_spare[i - lookback].write(oscillator - signal_sum / signal as f64);
            ring_index = (ring_index + 1) % signal;
            signal_sum -= ring[ring_index];
        } else {
            ring_index = (ring_index + 1) % signal;
        }
    }
    // Match the prior production implementation: preallocate the full output
    // and initialize its tail through spare capacity.
    // SAFETY: the warm-up prefix was resized and every remaining slot was
    // written exactly once before the length is extended to `len`.
    unsafe { output.set_len(len) };
    output
}

fn reciprocal_ac(high: &[f64], low: &[f64], fast: usize, slow: usize, signal: usize) -> Vec<f64> {
    let len = high.len();
    let window = slow.max(fast);
    let osc_start = window - 1;
    let lookback = window + signal - 2;
    let inv_fast = 1.0 / fast as f64;
    let inv_slow = 1.0 / slow as f64;
    let inv_signal = 1.0 / signal as f64;
    let mut fast_sum = 0.0;
    let mut slow_sum = 0.0;
    let mut trailing_fast = osc_start - (fast - 1);
    let mut trailing_slow = osc_start - (slow - 1);
    for i in 0..osc_start {
        let median = (high[i] + low[i]) / 2.0;
        if i >= trailing_fast {
            fast_sum += median;
        }
        if i >= trailing_slow {
            slow_sum += median;
        }
    }

    let mut output = Vec::with_capacity(len);
    output.resize(lookback, f64::NAN);
    let output_spare = output.spare_capacity_mut();
    let mut ring = [0.0; 32];
    let mut ring_index = 0;
    let mut signal_sum = 0.0;
    for i in osc_start..len {
        let median = (high[i] + low[i]) / 2.0;
        fast_sum += median;
        slow_sum += median;
        let oscillator = fast_sum * inv_fast - slow_sum * inv_slow;
        fast_sum -= (high[trailing_fast] + low[trailing_fast]) / 2.0;
        slow_sum -= (high[trailing_slow] + low[trailing_slow]) / 2.0;
        trailing_fast += 1;
        trailing_slow += 1;

        ring[ring_index] = oscillator;
        signal_sum += oscillator;
        if i >= lookback {
            output_spare[i - lookback].write(oscillator - signal_sum * inv_signal);
            ring_index += 1;
            if ring_index == signal {
                ring_index = 0;
            }
            signal_sum -= ring[ring_index];
        } else {
            ring_index += 1;
            if ring_index == signal {
                ring_index = 0;
            }
        }
    }
    unsafe { output.set_len(len) };
    output
}

fn median_ring_ac(high: &[f64], low: &[f64], fast: usize, slow: usize, signal: usize) -> Vec<f64> {
    let len = high.len();
    let window = slow.max(fast);
    let osc_start = window - 1;
    let lookback = window + signal - 2;
    let mut fast_sum = 0.0;
    let mut slow_sum = 0.0;
    let trailing_fast = osc_start - (fast - 1);
    let trailing_slow = osc_start - (slow - 1);
    let mut median_ring = vec![0.0; window];
    let mut write_index = 0;
    for i in 0..osc_start {
        let median = (high[i] + low[i]) / 2.0;
        median_ring[write_index] = median;
        write_index += 1;
        if write_index == window {
            write_index = 0;
        }
        if i >= trailing_fast {
            fast_sum += median;
        }
        if i >= trailing_slow {
            slow_sum += median;
        }
    }

    let mut output = Vec::with_capacity(len);
    output.resize(lookback, f64::NAN);
    let output_spare = output.spare_capacity_mut();
    let mut ring = vec![0.0; signal];
    let mut ring_index = 0;
    let mut signal_sum = 0.0;
    for i in osc_start..len {
        let median = (high[i] + low[i]) / 2.0;
        median_ring[write_index] = median;
        fast_sum += median;
        slow_sum += median;
        let oscillator = fast_sum / fast as f64 - slow_sum / slow as f64;
        let fast_out = if write_index + 1 >= fast {
            write_index + 1 - fast
        } else {
            write_index + 1 + window - fast
        };
        let slow_out = if write_index + 1 >= slow {
            write_index + 1 - slow
        } else {
            write_index + 1 + window - slow
        };
        fast_sum -= median_ring[fast_out];
        slow_sum -= median_ring[slow_out];
        write_index += 1;
        if write_index == window {
            write_index = 0;
        }

        ring[ring_index] = oscillator;
        signal_sum += oscillator;
        if i >= lookback {
            output_spare[i - lookback].write(oscillator - signal_sum / signal as f64);
            ring_index += 1;
            if ring_index == signal {
                ring_index = 0;
            }
            signal_sum -= ring[ring_index];
        } else {
            ring_index += 1;
            if ring_index == signal {
                ring_index = 0;
            }
        }
    }
    unsafe { output.set_len(len) };
    output
}

fn time<T>(mut f: impl FnMut() -> T, duration: Duration) -> f64 {
    let start = Instant::now();
    let mut count = 0u64;
    while start.elapsed() < duration {
        black_box(f());
        count += 1;
    }
    start.elapsed().as_secs_f64() * 1e6 / count as f64
}

fn main() {
    for len in [1_000usize, 100_000, 1_000_000] {
        let (high, low) = data(len);
        let old = || old_ac(&high, &low, 5, 34, 5);
        let new = || ac(&high, &low, 5, 34, 5).unwrap();
        let reciprocal = || reciprocal_ac(&high, &low, 5, 34, 5);
        let candidate = || median_ring_ac(&high, &low, 5, 34, 5);
        let expected = old();
        let actual = new();
        let reciprocal_output = reciprocal();
        let candidate_output = candidate();
        let max_diff = expected
            .iter()
            .zip(actual.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert_eq!(
            expected.iter().map(|v| v.is_nan()).collect::<Vec<_>>(),
            actual.iter().map(|v| v.is_nan()).collect::<Vec<_>>()
        );
        assert!(max_diff <= 1e-10, "AC max diff={max_diff}");
        let reciprocal_max_diff = expected
            .iter()
            .zip(&reciprocal_output)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(
            reciprocal_max_diff <= 1e-10,
            "AC reciprocal diff={reciprocal_max_diff}"
        );
        assert_eq!(
            expected.iter().map(|v| v.is_nan()).collect::<Vec<_>>(),
            reciprocal_output
                .iter()
                .map(|v| v.is_nan())
                .collect::<Vec<_>>(),
            "AC reciprocal candidate changed the NaN mask"
        );
        assert_eq!(
            expected
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            candidate_output
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            "median-ring candidate differs from the legacy recurrence"
        );
        let (mut old_samples, mut new_samples, mut candidate_samples, mut reciprocal_samples) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for turn in 0..8 {
            match turn % 4 {
                0 => {
                    old_samples.push(time(&mut { || old() }, Duration::from_millis(120)));
                    new_samples.push(time(&mut { || new() }, Duration::from_millis(120)));
                    candidate_samples
                        .push(time(&mut { || candidate() }, Duration::from_millis(120)));
                    reciprocal_samples
                        .push(time(&mut { || reciprocal() }, Duration::from_millis(120)));
                }
                1 => {
                    new_samples.push(time(&mut { || new() }, Duration::from_millis(120)));
                    candidate_samples
                        .push(time(&mut { || candidate() }, Duration::from_millis(120)));
                    reciprocal_samples
                        .push(time(&mut { || reciprocal() }, Duration::from_millis(120)));
                    old_samples.push(time(&mut { || old() }, Duration::from_millis(120)));
                }
                2 => {
                    candidate_samples
                        .push(time(&mut { || candidate() }, Duration::from_millis(120)));
                    reciprocal_samples
                        .push(time(&mut { || reciprocal() }, Duration::from_millis(120)));
                    old_samples.push(time(&mut { || old() }, Duration::from_millis(120)));
                    new_samples.push(time(&mut { || new() }, Duration::from_millis(120)));
                }
                _ => {
                    reciprocal_samples
                        .push(time(&mut { || reciprocal() }, Duration::from_millis(120)));
                    old_samples.push(time(&mut { || old() }, Duration::from_millis(120)));
                    new_samples.push(time(&mut { || new() }, Duration::from_millis(120)));
                    candidate_samples
                        .push(time(&mut { || candidate() }, Duration::from_millis(120)));
                }
            }
        }
        old_samples.sort_by(f64::total_cmp);
        new_samples.sort_by(f64::total_cmp);
        candidate_samples.sort_by(f64::total_cmp);
        reciprocal_samples.sort_by(f64::total_cmp);
        println!(
            "AC {len}: old={:.2}us new={:.2}us median-ring={:.2}us reciprocal={:.2}us speedup={:.3}x candidate-speedup={:.3}x reciprocal-speedup={:.3}x max_abs={max_diff:.3e} reciprocal-abs={reciprocal_max_diff:.3e}",
            old_samples[4],
            new_samples[4],
            candidate_samples[4],
            reciprocal_samples[4],
            old_samples[2] / new_samples[2],
            old_samples[2] / candidate_samples[2],
            old_samples[2] / reciprocal_samples[2]
        );
    }
}
