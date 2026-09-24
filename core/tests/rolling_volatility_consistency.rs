//! Convergence gate for the second-moment family: variance, standard deviation,
//! covariance, correlation and z-score.
//!
//! Every one of these was implemented with the textbook **one-pass** form
//! `sum(x^2) - sum(x)^2 / n`. That form subtracts two large, nearly equal totals,
//! so it is exact for the small-baseline data the unit tests use and wrong by a
//! wide margin for data whose mean dwarfs its spread. Two examples, both
//! measured before the fix:
//!
//! * a series and its exact affine image `2x + 3` are perfectly correlated, yet
//!   the one-pass coefficient was `0.667` at a `1e9` baseline (exposed to Python
//!   as `correlation`);
//! * the rolling sample deviation of `1e12 + i` with `window = 3` is exactly
//!   `1.0`, yet the sliding-window spelling returned `0.0` for every window.
//!
//! The gates below assert **absolute** properties — offset invariance, the
//! exact value for a known input, and agreement between the spellings of one
//! statistic — rather than only comparing implementations against each other. A
//! mutual-agreement gate would have been green the whole time, because the
//! spellings were consistently wrong in the same direction on the test data.

use finkit::features::{
    batch_zscore_simd, correlation_simd, rolling_correlation, rolling_ic, rolling_mean_simd,
    rolling_std_simd, rolling_zscore, IcMethod,
};
use finkit::math::kernels::rolling_sample_stddev_into;
use finkit::math::statistics::{correlation, covariance, rolling_mean, rolling_std_dev};

/// A small-baseline ramp and the same shape at a large baseline. Both are
/// arithmetic progressions of unit step, so every statistic of interest is
/// identical up to a shift.
fn ramps(len: usize) -> (Vec<f64>, Vec<f64>) {
    let small: Vec<f64> = (0..len).map(|i| i as f64).collect();
    let large: Vec<f64> = (0..len).map(|i| 1.0e9 + i as f64).collect();
    (small, large)
}

/// Approximate equality with an absolute floor, so values near zero compare
/// sensibly.
fn close(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance * right.abs().max(1.0)
}

// ---------------------------------------------------------------------------
// Correlation
// ---------------------------------------------------------------------------

#[test]
fn correlation_of_an_affine_image_is_exactly_one_at_any_offset() {
    for len in [8usize, 16, 64, 256] {
        let (small, large) = ramps(len);
        // `b = 2a + 3` is an exact positive affine map, so `r` is exactly 1.0.
        for (label, base) in [("small", &small), ("large", &large)] {
            let image: Vec<f64> = base.iter().map(|value| value * 2.0 + 3.0).collect();
            let anti: Vec<f64> = base.iter().map(|value| -value).collect();

            let simd = correlation_simd(base, &image);
            let canonical = correlation(base, &image).expect("correlation");

            assert!(
                close(simd, 1.0, 1e-9),
                "{label}/{len}: correlation_simd(series, 2x+3) = {simd}, expected 1.0"
            );
            assert!(
                close(canonical, 1.0, 1e-9),
                "{label}/{len}: statistics::correlation(series, 2x+3) = {canonical}, expected 1.0"
            );
            assert!(
                close(correlation_simd(base, &anti), -1.0, 1e-9),
                "{label}/{len}: correlation_simd(series, -x) must be -1.0"
            );
        }
    }
}

#[test]
fn correlation_is_invariant_under_offset_and_scale() {
    let (small, large) = ramps(64);

    let reference = correlation_simd(&small, &large);
    // The two ramps differ only by a constant, so they are perfectly correlated
    // whatever their baselines are.
    assert!(
        close(reference, 1.0, 1e-9),
        "two ramps differing by a constant must correlate at 1.0, got {reference}"
    );

    // A non-trivial signal compared across baselines: the coefficient must not
    // move when the whole series is shifted.
    let signal: Vec<f64> = (0..64)
        .map(|i| (i as f64 * 0.37).sin() * 5.0 + (i as f64 * 0.11).cos())
        .collect();
    let shifted: Vec<f64> = signal.iter().map(|value| value + 1.0e8).collect();
    let other: Vec<f64> = (0..64)
        .map(|i| (i as f64 * 0.37).sin() * 3.0 - (i as f64 * 0.29).cos())
        .collect();

    let plain = correlation_simd(&signal, &other);
    let offset = correlation_simd(&shifted, &other);
    assert!(
        close(plain, offset, 1e-6),
        "correlation moved with the offset: {plain} (baseline 0) vs {offset} (baseline 1e8)"
    );
}

#[test]
fn correlation_stays_within_its_documented_range() {
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        // Map to [-1, 1] and add a large offset.
        (state >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0 + 1.0e7
    };
    let a: Vec<f64> = (0..512).map(|_| next()).collect();
    let b: Vec<f64> = (0..512).map(|_| next()).collect();

    let coefficient = correlation_simd(&a, &b);
    assert!(
        coefficient.is_finite() && (-1.0..=1.0).contains(&coefficient),
        "correlation out of range: {coefficient}"
    );
}

// ---------------------------------------------------------------------------
// Covariance
// ---------------------------------------------------------------------------

#[test]
fn covariance_is_invariant_under_offset_and_scales_linearly() {
    let (small, large) = ramps(64);

    let image_small: Vec<f64> = small.iter().map(|value| value * 2.0 + 3.0).collect();
    let image_large: Vec<f64> = large.iter().map(|value| value * 2.0 + 3.0).collect();

    let cov_small = covariance(&small, &image_small).expect("covariance");
    let cov_large = covariance(&large, &image_large).expect("covariance");

    // `cov(x, 2x+3) = 2 * var(x)`, and `var` is offset invariant.
    assert!(
        close(cov_large, cov_small, 1e-6),
        "covariance moved with the offset: {cov_small} (baseline 0) vs {cov_large} (baseline 1e9)"
    );

    // Sample variance of `0..64` is `n(n+1)/12 * n/(n-1)`; `cov = 2 * var`.
    let n = 64.0_f64;
    let population_var: f64 = (n * n - 1.0) / 12.0;
    let sample_var = population_var * n / (n - 1.0);
    assert!(
        close(cov_small, 2.0 * sample_var, 1e-9),
        "covariance = {cov_small}, expected {}",
        2.0 * sample_var
    );
}

// ---------------------------------------------------------------------------
// Z-score
// ---------------------------------------------------------------------------

#[test]
fn batch_zscore_is_offset_invariant() {
    let (small, large) = ramps(64);

    let z_small = batch_zscore_simd(&small);
    let z_large = batch_zscore_simd(&large);
    let small_slice = z_small.as_slice().expect("contiguous");
    let large_slice = z_large.as_slice().expect("contiguous");

    for index in 0..small.len() {
        assert!(
            close(large_slice[index], small_slice[index], 1e-6),
            "z-score moved with the offset at {index}: {} (baseline 0) vs {} (baseline 1e9)",
            small_slice[index],
            large_slice[index]
        );
    }

    // Absolute check against the closed form for a unit ramp of length 64.
    let n = 64.0_f64;
    let population_var: f64 = (n * n - 1.0) / 12.0;
    let std = population_var.sqrt();
    let expected_first = (0.0 - (n - 1.0) / 2.0) / std;
    assert!(
        close(small_slice[0], expected_first, 1e-9),
        "z-score[0] = {}, expected {expected_first}",
        small_slice[0]
    );
}

// ---------------------------------------------------------------------------
// Rolling spellings must agree with the canonical kernels
// ---------------------------------------------------------------------------

#[test]
fn rolling_simd_spellings_match_the_canonical_kernels() {
    let datasets: Vec<(&str, Vec<f64>)> = vec![
        ("small_ramp", (0..64).map(|i| i as f64).collect()),
        ("large_ramp", (0..64).map(|i| 1.0e12 + i as f64).collect()),
        (
            "ordinary",
            (0..128).map(|i| (i as f64 * 0.3).sin() + 2.0).collect(),
        ),
        ("constant_large", vec![1.0e8; 40]),
    ];

    for (name, data) in datasets {
        for window in [3usize, 5, 20] {
            let mean_canonical = rolling_mean(&data, window).expect("rolling_mean");
            let mean_simd = rolling_mean_simd(&data, window);
            let std_canonical = rolling_std_dev(&data, window).expect("rolling_std_dev");
            let std_simd = rolling_std_simd(&data, window);
            let mut kernel = vec![f64::NAN; data.len()];
            rolling_sample_stddev_into(&data, window, &mut kernel).expect("kernel");

            for index in 0..data.len() {
                let left = mean_canonical[index];
                let right = mean_simd[index];
                assert!(
                    (left.is_nan() && right.is_nan()) || close(left, right, 1e-9),
                    "{name}/w{window}[{index}]: rolling_mean {left} vs rolling_mean_simd {right}"
                );

                let left = std_canonical[index];
                let right = std_simd[index];
                assert!(
                    (left.is_nan() && right.is_nan()) || close(left, right, 1e-9),
                    "{name}/w{window}[{index}]: rolling_std_dev {left} vs rolling_std_simd {right}"
                );
                assert!(
                    (left.is_nan() && kernel[index].is_nan()) || close(left, kernel[index], 1e-12),
                    "{name}/w{window}[{index}]: rolling_std_dev {left} vs canonical kernel {}",
                    kernel[index]
                );
            }
        }
    }
}

#[test]
fn rolling_sample_deviation_of_a_unit_ramp_is_one() {
    // The window `[c, c+1, c+2]` has sample variance exactly 1, so its sample
    // deviation is exactly 1 — at any baseline.
    for base in [0.0, 1.0e6, 1.0e12] {
        let data: Vec<f64> = (0..32).map(|i| base + i as f64).collect();
        let canonical = rolling_std_dev(&data, 3).expect("rolling_std_dev");
        let simd = rolling_std_simd(&data, 3);

        for index in 2..data.len() {
            assert!(
                close(canonical[index], 1.0, 1e-6),
                "baseline {base}: rolling_std_dev[{index}] = {}, expected 1.0",
                canonical[index]
            );
            assert!(
                close(simd[index], 1.0, 1e-6),
                "baseline {base}: rolling_std_simd[{index}] = {}, expected 1.0",
                simd[index]
            );
        }
    }
}

#[test]
fn rolling_correlation_of_an_affine_pair_is_one_at_a_large_baseline() {
    let base: Vec<f64> = (0..64).map(|i| 1.0e6 + i as f64).collect();
    let image: Vec<f64> = base.iter().map(|value| value * 2.0 + 3.0).collect();

    let result = rolling_correlation(&base, &image, 20).to_vec();
    for index in 19..base.len() {
        assert!(
            close(result[index], 1.0, 1e-9),
            "rolling_correlation[{index}] = {}, expected 1.0",
            result[index]
        );
    }
}

#[test]
fn rolling_zscore_is_offset_invariant() {
    let signal: Vec<f64> = (0..128)
        .map(|i| (i as f64 * 0.37).sin() * 5.0 + (i as f64 * 0.11).cos())
        .collect();
    let shifted: Vec<f64> = signal.iter().map(|value| value + 1.0e8).collect();

    let plain = rolling_zscore(&signal, 20).to_vec();
    let offset = rolling_zscore(&shifted, 20).to_vec();

    for index in 19..signal.len() {
        assert!(
            close(offset[index], plain[index], 1e-6),
            "rolling_zscore moved with the offset at {index}: {} (baseline 0) vs {} (baseline 1e8)",
            plain[index],
            offset[index]
        );
    }

    // Absolute check on a unit ramp: the last value of each window sits exactly
    // one population deviation above the mean of `0..w`.
    let ramp: Vec<f64> = (0..64).map(|i| i as f64).collect();
    let z = rolling_zscore(&ramp, 5).to_vec();
    let w = 5.0_f64;
    let population_var: f64 = (w * w - 1.0) / 12.0;
    let expected = (w - 1.0 - (w - 1.0) / 2.0) / population_var.sqrt();
    assert!(
        close(z[4], expected, 1e-9),
        "rolling_zscore[4] = {}, expected {expected}",
        z[4]
    );
}

#[test]
fn rolling_ic_pearson_is_perfect_for_an_affine_pair_and_offset_invariant() {
    let factor: Vec<f64> = (0..64)
        .map(|i| (i as f64 * 0.37).sin() * 5.0 + (i as f64 * 0.11).cos())
        .collect();
    let affine: Vec<f64> = factor.iter().map(|value| value * 2.0 + 3.0).collect();
    let shifted: Vec<f64> = factor.iter().map(|value| value + 1.0e8).collect();

    let perfect = rolling_ic(&factor, &affine, 20, IcMethod::Pearson)
        .expect("rolling_ic")
        .to_vec();
    for index in 19..factor.len() {
        assert!(
            close(perfect[index], 1.0, 1e-9),
            "rolling_ic(series, 2x+3)[{index}] = {}, expected 1.0",
            perfect[index]
        );
    }

    let plain = rolling_ic(&factor, &affine, 20, IcMethod::Pearson)
        .expect("rolling_ic")
        .to_vec();
    let offset = rolling_ic(&shifted, &affine, 20, IcMethod::Pearson)
        .expect("rolling_ic")
        .to_vec();
    for index in 19..factor.len() {
        assert!(
            close(offset[index], plain[index], 1e-6),
            "rolling_ic moved with the offset at {index}: {} (baseline 0) vs {} (baseline 1e8)",
            plain[index],
            offset[index]
        );
    }
}

// ---------------------------------------------------------------------------
// The volatility family must be finite for finite input
// ---------------------------------------------------------------------------

#[test]
fn sample_deviation_is_finite_for_every_finite_window() {
    let datasets: Vec<(&str, Vec<f64>)> = vec![
        (
            "large_baseline_tiny_wiggle",
            (0..200)
                .map(|i| 1.0e6 + (i as f64 * 0.01).sin() * 0.001)
                .collect(),
        ),
        ("constant_large", vec![1.0e8; 50]),
        (
            "large_baseline_tiny_drift",
            (0..80).map(|i| 1.0e9 + i as f64 * 1.0e-3).collect(),
        ),
        (
            "ordinary",
            (0..64).map(|i| (i as f64 * 0.3).sin() + 2.0).collect(),
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (name, data) in datasets {
        for window in [3usize, 5, 20] {
            let std = rolling_std_dev(&data, window).expect("rolling_std_dev");
            let var = finkit::math::statistics::rolling_variance(&data, window).expect("variance");
            let mut kernel = vec![f64::NAN; data.len()];
            rolling_sample_stddev_into(&data, window, &mut kernel).expect("kernel");

            for index in window - 1..data.len() {
                if !std[index].is_finite() || !kernel[index].is_finite() {
                    failures.push(format!(
                        "{name}/w{window}[{index}]: std={} kernel={}",
                        std[index], kernel[index]
                    ));
                }
                if var[index].is_finite() && var[index] < 0.0 {
                    failures.push(format!(
                        "{name}/w{window}[{index}]: negative variance {}",
                        var[index]
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "non-finite or negative sample moments ({} sites, first 10):\n{}",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
