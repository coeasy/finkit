use finkit::formula::{FormulaContext, FormulaDialect, FormulaEngine, FormulaExecutionMode};
use finkit::math::kernels::{rolling_beta_into, rolling_correlation_into};
use finkit::talib_ffi::{TA_BETA, TA_CORREL};
use ndarray::Array1;
use std::hint::black_box;
use std::time::Instant;

fn inputs(len: usize) -> (Vec<f64>, Vec<f64>) {
    let mut left = Vec::with_capacity(len);
    let mut right = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let close = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        let open = close - 0.15 + (x * 0.031).sin() * 0.05;
        left.push(close);
        right.push(open);
    }
    (left, right)
}

fn c_correl_into(left: &[f64], right: &[f64], period: usize, output: &mut [f64]) {
    let mut begin = 0;
    let mut elements = 0;
    // SAFETY: all slices cover the TA_CORREL request and remain alive for the call.
    let status = unsafe {
        TA_CORREL(
            0,
            (left.len() - 1) as i32,
            left.as_ptr(),
            right.as_ptr(),
            period as i32,
            &mut begin,
            &mut elements,
            output.as_mut_ptr(),
        )
    };
    assert_eq!(status, 0);
    assert_eq!(begin, (period - 1) as i32);
    assert_eq!(elements, output.len() as i32 - (period - 1) as i32);
}

fn c_beta_into(left: &[f64], right: &[f64], period: usize, output: &mut [f64]) -> usize {
    let mut begin = 0;
    let mut elements = 0;
    // SAFETY: all slices cover the TA_BETA request and remain alive for the call.
    let status = unsafe {
        TA_BETA(
            0,
            (left.len() - 1) as i32,
            left.as_ptr(),
            right.as_ptr(),
            period as i32,
            &mut begin,
            &mut elements,
            output.as_mut_ptr(),
        )
    };
    assert_eq!(status, 0);
    assert!(begin >= (period - 1) as i32);
    assert_eq!(elements, output.len() as i32 - begin);
    begin as usize
}

fn elapsed_us(mut body: impl FnMut()) -> f64 {
    let started = Instant::now();
    body();
    started.elapsed().as_secs_f64() * 1e6
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn run(len: usize, period: usize) {
    let (left, right) = inputs(len);
    let mut rust = vec![0.0; len];
    let mut c = vec![0.0; len];
    rolling_correlation_into(&left, &right, period, &mut rust).unwrap();
    c_correl_into(&left, &right, period, &mut c);
    let mut mismatches = 0;
    let mut max_abs = 0.0f64;
    for (actual, expected) in rust.iter().skip(period - 1).zip(&c[..len - period + 1]) {
        let difference = (actual - expected).abs();
        max_abs = max_abs.max(difference);
        if difference > 1e-8 + 1e-8 * actual.abs().max(expected.abs()) {
            mismatches += 1;
        }
    }

    let mut rust_samples = [0.0; 9];
    let mut c_samples = [0.0; 9];
    for index in 0..rust_samples.len() {
        if index % 2 == 0 {
            rust_samples[index] = elapsed_us(|| {
                black_box(rolling_correlation_into(
                    black_box(&left),
                    black_box(&right),
                    period,
                    black_box(&mut rust),
                ))
                .unwrap();
            });
            c_samples[index] = elapsed_us(|| {
                c_correl_into(
                    black_box(&left),
                    black_box(&right),
                    period,
                    black_box(&mut c),
                );
            });
        } else {
            c_samples[index] = elapsed_us(|| {
                c_correl_into(
                    black_box(&left),
                    black_box(&right),
                    period,
                    black_box(&mut c),
                );
            });
            rust_samples[index] = elapsed_us(|| {
                black_box(rolling_correlation_into(
                    black_box(&left),
                    black_box(&right),
                    period,
                    black_box(&mut rust),
                ))
                .unwrap();
            });
        }
    }
    let rust_us = median(&mut rust_samples);
    let c_us = median(&mut c_samples);
    println!(
        "formula CORREL kernel n={len} period={period} mismatches={mismatches} max_abs={max_abs:.3e} rust={rust_us:.3}us c={c_us:.3}us speedup={:.3}x",
        c_us / rust_us,
    );
}

fn run_beta(len: usize, period: usize) {
    let (left, right) = inputs(len);
    let mut rust = vec![0.0; len];
    let mut c = vec![0.0; len];
    rolling_beta_into(&left, &right, period, &mut rust).unwrap();
    let c_begin = c_beta_into(&left, &right, period, &mut c);
    let mut mismatches = 0;
    let mut max_abs = 0.0f64;
    for (actual, expected) in rust.iter().skip(c_begin).zip(&c[..len - c_begin]) {
        let difference = (actual - expected).abs();
        max_abs = max_abs.max(difference);
        if difference > 1e-8 + 1e-8 * actual.abs().max(expected.abs()) {
            mismatches += 1;
        }
    }

    let mut rust_samples = [0.0; 9];
    let mut c_samples = [0.0; 9];
    for index in 0..rust_samples.len() {
        if index % 2 == 0 {
            rust_samples[index] = elapsed_us(|| {
                black_box(rolling_beta_into(
                    black_box(&left),
                    black_box(&right),
                    period,
                    black_box(&mut rust),
                ))
                .unwrap();
            });
            c_samples[index] = elapsed_us(|| {
                c_beta_into(
                    black_box(&left),
                    black_box(&right),
                    period,
                    black_box(&mut c),
                );
            });
        } else {
            c_samples[index] = elapsed_us(|| {
                c_beta_into(
                    black_box(&left),
                    black_box(&right),
                    period,
                    black_box(&mut c),
                );
            });
            rust_samples[index] = elapsed_us(|| {
                black_box(rolling_beta_into(
                    black_box(&left),
                    black_box(&right),
                    period,
                    black_box(&mut rust),
                ))
                .unwrap();
            });
        }
    }
    let rust_us = median(&mut rust_samples);
    let c_us = median(&mut c_samples);
    println!(
        "formula BETA kernel n={len} period={period} mismatches={mismatches} max_abs={max_abs:.3e} rust={rust_us:.3}us c={c_us:.3}us speedup={:.3}x",
        c_us / rust_us,
    );
}

fn check_formula_routes(len: usize, period: usize) {
    let (left, right) = inputs(len);
    let mut expected = vec![0.0; len];
    rolling_correlation_into(&left, &right, period, &mut expected).unwrap();
    let source = format!("CORREL(CLOSE,OPEN,{period})");
    for mode in [FormulaExecutionMode::Tree, FormulaExecutionMode::Plan] {
        let mut context = FormulaContext::new(
            Array1::from(right.clone()),
            Array1::from(left.clone()),
            Array1::from(left.clone()),
            Array1::from(left.clone()),
            Array1::ones(len),
            None,
        );
        let mut engine = FormulaEngine::new().with_execution_mode(mode);
        let actual = engine
            .eval_with_dialect(&source, FormulaDialect::AlphaTA, &mut context)
            .unwrap();
        assert_eq!(actual.len(), expected.len());
        for (index, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
            assert!(
                (actual.is_nan() && expected.is_nan()) || (actual - expected).abs() <= 1e-12,
                "{mode:?} formula result differs at {index}: {actual} vs {expected}"
            );
        }
    }
}

fn main() {
    let non_finite_left = [1.0, 2.0, f64::NAN, 4.0, 5.0, 6.0];
    let non_finite_right = [2.0, 4.0, 6.0, 8.0, 10.0, 12.0];
    let mut non_finite_output = [0.0; 6];
    rolling_correlation_into(
        &non_finite_left,
        &non_finite_right,
        2,
        &mut non_finite_output,
    )
    .unwrap();
    assert!(non_finite_output[0].is_nan());
    assert_eq!(non_finite_output[1], 1.0);
    assert!(non_finite_output[2].is_nan());
    assert!(non_finite_output[3].is_nan());
    assert_eq!(non_finite_output[4], 1.0);
    assert_eq!(non_finite_output[5], 1.0);

    let constant = [5.0; 4];
    let mut constant_output = [0.0; 4];
    rolling_correlation_into(&constant, &constant, 2, &mut constant_output).unwrap();
    assert!(constant_output.iter().all(|value| value.is_nan()));

    check_formula_routes(1_000, 20);
    check_formula_routes(100_000, 20);

    for period in [2, 5, 20, 100] {
        for len in [1_000, 100_000, 1_000_000] {
            run(len, period);
            run_beta(len, period);
        }
    }
}
