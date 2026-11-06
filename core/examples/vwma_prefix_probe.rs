use finkit::math::moving_avg::{vwma, vwma_into};
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn data(len: usize) -> (Vec<f64>, Vec<f64>) {
    let mut close = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        close.push(100.0 + 0.006 * x + (x * 0.017).sin() * 2.1);
        volume.push(1_000_000.0 + (x % 997.0) * 170.0 + (x * 0.011).sin().abs() * 50_000.0);
    }
    (close, volume)
}

fn old_vwma(input: &[f64], volume: &[f64], period: usize) -> Array1<f64> {
    let len = input.len();
    let mut output = Array1::from_elem(len, f64::NAN);
    let clean =
        !input.iter().any(|value| value.is_nan()) && !volume.iter().any(|value| value.is_nan());
    if clean {
        let mut price_volume = 0.0;
        let mut total_volume = 0.0;
        for index in 0..period {
            price_volume += input[index] * volume[index];
            total_volume += volume[index];
        }
        if total_volume.abs() > finkit::utils::NUMERIC_EPSILON {
            output[period - 1] = price_volume / total_volume;
        }
        for index in period..len {
            price_volume +=
                input[index] * volume[index] - input[index - period] * volume[index - period];
            total_volume += volume[index] - volume[index - period];
            if total_volume.abs() > finkit::utils::NUMERIC_EPSILON {
                output[index] = price_volume / total_volume;
            }
        }
    } else {
        let mut price_volume = 0.0;
        let mut total_volume = 0.0;
        let mut dirty = false;
        for index in 0..period {
            if input[index].is_nan() || volume[index].is_nan() {
                dirty = true;
            }
            price_volume += input[index] * volume[index];
            total_volume += volume[index];
        }
        if !dirty && total_volume.abs() > finkit::utils::NUMERIC_EPSILON {
            output[period - 1] = price_volume / total_volume;
        }
        for index in period..len {
            let old_value = input[index - period];
            let old_volume = volume[index - period];
            let new_value = input[index];
            let new_volume = volume[index];
            if dirty
                || old_value.is_nan()
                || old_volume.is_nan()
                || new_value.is_nan()
                || new_volume.is_nan()
            {
                let start = index + 1 - period;
                price_volume = 0.0;
                total_volume = 0.0;
                dirty = false;
                for current in start..=index {
                    if input[current].is_nan() || volume[current].is_nan() {
                        dirty = true;
                        break;
                    }
                    price_volume += input[current] * volume[current];
                    total_volume += volume[current];
                }
            } else {
                price_volume += new_value * new_volume - old_value * old_volume;
                total_volume += new_volume - old_volume;
            }
            if !dirty && total_volume.abs() > finkit::utils::NUMERIC_EPSILON {
                output[index] = price_volume / total_volume;
            }
        }
    }
    output
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
        let (input, volume) = data(len);
        let old = || old_vwma(&input, &volume, 30);
        let new = || vwma(&input, &volume, 30).unwrap();
        let expected = old();
        let actual = new();
        let max_diff = expected
            .iter()
            .zip(actual.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert_eq!(
            expected.iter().map(|v| v.is_nan()).collect::<Vec<_>>(),
            actual.iter().map(|v| v.is_nan()).collect::<Vec<_>>()
        );
        assert!(max_diff <= 1e-8, "VWMA max diff={max_diff}");
        let (mut old_samples, mut new_samples) = (Vec::new(), Vec::new());
        for turn in 0..5 {
            if turn % 2 == 0 {
                old_samples.push(time(&mut { || old() }, Duration::from_millis(120)));
                new_samples.push(time(&mut { || new() }, Duration::from_millis(120)));
            } else {
                new_samples.push(time(&mut { || new() }, Duration::from_millis(120)));
                old_samples.push(time(&mut { || old() }, Duration::from_millis(120)));
            }
        }
        old_samples.sort_by(f64::total_cmp);
        new_samples.sort_by(f64::total_cmp);
        println!(
            "VWMA {len}: old={:.2}us new={:.2}us speedup={:.3}x max_abs={max_diff:.3e}",
            old_samples[2],
            new_samples[2],
            old_samples[2] / new_samples[2]
        );
    }

    for edge in ["price_nan", "volume_nan", "infinity", "zero_volume"] {
        let (mut input, mut volume) = data(500_123);
        match edge {
            "price_nan" => input[250_111] = f64::NAN,
            "volume_nan" => volume[250_111] = f64::NAN,
            "infinity" => input[250_111] = f64::INFINITY,
            "zero_volume" => volume.fill(0.0),
            _ => unreachable!(),
        }
        let expected = old_vwma(&input, &volume, 30);
        let actual = vwma(&input, &volume, 30).unwrap();
        assert_eq!(
            expected
                .iter()
                .map(|value| value.is_nan())
                .collect::<Vec<_>>(),
            actual
                .iter()
                .map(|value| value.is_nan())
                .collect::<Vec<_>>(),
            "VWMA NaN mask mismatch for {edge}"
        );
        for (index, (&expected, &actual)) in expected.iter().zip(actual.iter()).enumerate() {
            assert!(
                (expected.is_nan() && actual.is_nan())
                    || expected == actual
                    || (expected - actual).abs() <= 1e-8,
                "VWMA mismatch for {edge} at {index}: {expected} vs {actual}"
            );
        }

        let mut output = vec![0.0; input.len()];
        vwma_into(&input, &volume, 30, &mut output).unwrap();
        for (index, (&expected, &actual)) in expected.iter().zip(&output).enumerate() {
            assert!(
                (expected.is_nan() && actual.is_nan())
                    || expected == actual
                    || (expected - actual).abs() <= 1e-8,
                "VWMA_into mismatch for {edge} at {index}: {expected} vs {actual}"
            );
        }
        println!("VWMA edge {edge}: parity=true");
    }
}
