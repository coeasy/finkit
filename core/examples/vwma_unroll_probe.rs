use finkit::math::moving_avg::vwma;
use ndarray::Array1;
use std::hint::black_box;
use std::mem::MaybeUninit;
use std::time::{Duration, Instant};

fn data(len: usize) -> (Vec<f64>, Vec<f64>) {
    let mut input = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        input.push(100.0 + 0.006 * x + (x * 0.017).sin() * 2.1);
        volume.push(1_000_000.0 + (i % 997) as f64 * 170.0 + (x * 0.011).sin().abs() * 50_000.0);
    }
    (input, volume)
}

fn uninit_output(len: usize, period: usize) -> Vec<MaybeUninit<f64>> {
    let mut output = vec![MaybeUninit::uninit(); len];
    for slot in &mut output[..period - 1] {
        slot.write(f64::NAN);
    }
    output
}

fn finish_output(mut output: Vec<MaybeUninit<f64>>) -> Array1<f64> {
    let len = output.len();
    let capacity = output.capacity();
    let pointer = output.as_mut_ptr().cast::<f64>();
    std::mem::forget(output);
    // SAFETY: Both candidate loops initialize the entire output, including
    // their warm-up prefixes, before this conversion.
    Array1::from(unsafe { Vec::from_raw_parts(pointer, len, capacity) })
}

fn unrolled(input: &[f64], volume: &[f64], period: usize) -> Array1<f64> {
    let len = input.len();
    if !input
        .iter()
        .zip(volume)
        .all(|(&price, &weight)| !price.is_nan() && !weight.is_nan())
    {
        return vwma(input, volume, period).unwrap();
    }
    let mut output = uninit_output(len, period);
    let mut price_volume = 0.0;
    let mut total_volume = 0.0;
    for i in 0..period {
        price_volume += input[i] * volume[i];
        total_volume += volume[i];
    }
    output[period - 1].write(if total_volume.abs() > finkit::utils::NUMERIC_EPSILON {
        price_volume / total_volume
    } else {
        f64::NAN
    });
    let mut i = period;
    while i + 3 < len {
        for index in i..i + 4 {
            price_volume +=
                input[index] * volume[index] - input[index - period] * volume[index - period];
            total_volume += volume[index] - volume[index - period];
            output[index].write(if total_volume.abs() > finkit::utils::NUMERIC_EPSILON {
                price_volume / total_volume
            } else {
                f64::NAN
            });
        }
        i += 4;
    }
    while i < len {
        price_volume += input[i] * volume[i] - input[i - period] * volume[i - period];
        total_volume += volume[i] - volume[i - period];
        output[i].write(if total_volume.abs() > finkit::utils::NUMERIC_EPSILON {
            price_volume / total_volume
        } else {
            f64::NAN
        });
        i += 1;
    }
    finish_output(output)
}

fn combined_scan(input: &[f64], volume: &[f64], period: usize) -> Array1<f64> {
    let len = input.len();
    let mut output = uninit_output(len, period);
    let clean = input
        .iter()
        .zip(volume)
        .all(|(&price, &weight)| !price.is_nan() && !weight.is_nan());
    if !clean {
        return vwma(input, volume, period).unwrap();
    }
    let mut price_volume = 0.0;
    let mut total_volume = 0.0;
    for index in 0..period {
        price_volume += input[index] * volume[index];
        total_volume += volume[index];
    }
    if total_volume.abs() > finkit::utils::NUMERIC_EPSILON {
        output[period - 1].write(price_volume / total_volume);
    }
    for index in period..len {
        price_volume +=
            input[index] * volume[index] - input[index - period] * volume[index - period];
        total_volume += volume[index] - volume[index - period];
        if total_volume.abs() > finkit::utils::NUMERIC_EPSILON {
            output[index].write(price_volume / total_volume);
        }
    }
    finish_output(output)
}

fn single_pass(input: &[f64], volume: &[f64], period: usize) -> Array1<f64> {
    let len = input.len();
    let mut output = uninit_output(len, period);
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
    output[period - 1].write(
        if !dirty && total_volume.abs() > finkit::utils::NUMERIC_EPSILON {
            price_volume / total_volume
        } else {
            f64::NAN
        },
    );

    for index in period..len {
        let old_index = index - period;
        if dirty
            || input[old_index].is_nan()
            || volume[old_index].is_nan()
            || input[index].is_nan()
            || volume[index].is_nan()
        {
            let start = index + 1 - period;
            price_volume = 0.0;
            total_volume = 0.0;
            dirty = false;
            for cursor in start..=index {
                if input[cursor].is_nan() || volume[cursor].is_nan() {
                    dirty = true;
                    break;
                }
                price_volume += input[cursor] * volume[cursor];
                total_volume += volume[cursor];
            }
        } else {
            price_volume += input[index] * volume[index] - input[old_index] * volume[old_index];
            total_volume += volume[index] - volume[old_index];
        }
        output[index].write(
            if !dirty && total_volume.abs() > finkit::utils::NUMERIC_EPSILON {
                price_volume / total_volume
            } else {
                f64::NAN
            },
        );
    }
    finish_output(output)
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
    for len in [1_000usize, 100_000, 250_000, 500_000, 750_000, 1_000_000] {
        let (input, volume) = data(len);
        let current = || vwma(&input, &volume, 30).unwrap();
        let candidate = || unrolled(&input, &volume, 30);
        let scan_candidate = || combined_scan(&input, &volume, 30);
        let one_pass_candidate = || single_pass(&input, &volume, 30);
        let expected = current();
        let actual = candidate();
        let max_diff = expected
            .iter()
            .zip(actual.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(max_diff <= 1e-8, "VWMA max diff={max_diff}");
        let scan_actual = scan_candidate();
        let scan_diff = expected
            .iter()
            .zip(scan_actual.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(scan_diff <= 1e-8, "VWMA scan diff={scan_diff}");
        let one_pass_actual = one_pass_candidate();
        let one_pass_diff = expected
            .iter()
            .zip(one_pass_actual.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(one_pass_diff <= 1e-8, "VWMA one-pass diff={one_pass_diff}");

        let (mut current_samples, mut candidate_samples, mut scan_samples, mut one_pass_samples) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for turn in 0..8 {
            if turn % 2 == 0 {
                current_samples.push(time(&mut { || current() }, Duration::from_millis(120)));
                candidate_samples.push(time(&mut { || candidate() }, Duration::from_millis(120)));
                scan_samples.push(time(
                    &mut { || scan_candidate() },
                    Duration::from_millis(120),
                ));
                one_pass_samples.push(time(
                    &mut { || one_pass_candidate() },
                    Duration::from_millis(120),
                ));
            } else {
                scan_samples.push(time(
                    &mut { || scan_candidate() },
                    Duration::from_millis(120),
                ));
                candidate_samples.push(time(&mut { || candidate() }, Duration::from_millis(120)));
                current_samples.push(time(&mut { || current() }, Duration::from_millis(120)));
                one_pass_samples.push(time(
                    &mut { || one_pass_candidate() },
                    Duration::from_millis(120),
                ));
            }
        }
        current_samples.sort_by(f64::total_cmp);
        candidate_samples.sort_by(f64::total_cmp);
        scan_samples.sort_by(f64::total_cmp);
        one_pass_samples.sort_by(f64::total_cmp);
        println!(
            "VWMA {len}: current={:.2}us unroll={:.2}us ({:.3}x) combined={:.2}us ({:.3}x) single-pass={:.2}us ({:.3}x) max_abs={max_diff:.3e}/{scan_diff:.3e}/{one_pass_diff:.3e}",
            current_samples[4], candidate_samples[4], current_samples[4] / candidate_samples[4],
            scan_samples[4], current_samples[4] / scan_samples[4],
            one_pass_samples[4], current_samples[4] / one_pass_samples[4]
        );
    }
}
