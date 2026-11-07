use finkit::indicators::math_operators::{max, min, minmax};
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn data(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let x = i as f64;
            100.0 + 0.006 * x + (x * 0.017).sin() * 2.1
        })
        .collect()
}

fn cached<const WANT_MAX: bool>(input: &[f64], period: usize) -> Array1<f64> {
    let mut output = vec![f64::NAN; input.len()];
    let mut best = if WANT_MAX {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    let mut best_index = 0usize;
    let mut has_value = false;
    for i in 0..input.len() {
        let value = input[i];
        let dominates = if WANT_MAX {
            value >= best
        } else {
            value <= best
        };
        if dominates {
            best = value;
            best_index = i;
            has_value = true;
        } else if has_value && best_index + period <= i {
            best = if WANT_MAX {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
            has_value = false;
            let start = i + 1 - period;
            for j in start..=i {
                let candidate = input[j];
                let wins = if WANT_MAX {
                    candidate >= best
                } else {
                    candidate <= best
                };
                if wins {
                    best = candidate;
                    best_index = j;
                    has_value = true;
                }
            }
        }
        if i + 1 >= period {
            output[i] = if has_value { best } else { f64::NAN };
        }
    }
    Array1::from_vec(output)
}

fn block_nan_aware<const WANT_MAX: bool>(input: &[f64], period: usize) -> Array1<f64> {
    let len = input.len();
    let mut output = vec![f64::NAN; len];
    assert!(period <= 64);
    let mut suffix = [f64::NAN; 64];
    let mut prefix = [f64::NAN; 64];
    let mut suffix_has = [false; 64];
    let mut prefix_has = [false; 64];
    let mut block_start = 0usize;
    let mut today = period - 1;
    while today < len {
        let block_end = block_start + period - 1;
        let mut extreme = input[block_end];
        let mut found = !extreme.is_nan();
        suffix[period - 1] = extreme;
        suffix_has[period - 1] = found;
        let mut offset = period - 1;
        while offset > 0 {
            offset -= 1;
            let value = input[block_start + offset];
            let takes = !value.is_nan()
                && (!found
                    || if WANT_MAX {
                        value > extreme
                    } else {
                        value < extreme
                    });
            if takes {
                extreme = value;
                found = true;
            }
            suffix[offset] = extreme;
            suffix_has[offset] = found;
        }
        output[today] = if found { suffix[0] } else { f64::NAN };

        let block_next = block_start + period;
        if block_next >= len {
            break;
        }
        let n_available = (len - block_next).min(period - 1);
        extreme = input[block_next];
        found = !extreme.is_nan();
        prefix[0] = extreme;
        prefix_has[0] = found;
        let mut grown = 1usize;
        while grown < n_available {
            let value = input[block_next + grown];
            let takes = !value.is_nan()
                && (!found
                    || if WANT_MAX {
                        value > extreme
                    } else {
                        value < extreme
                    });
            if takes {
                extreme = value;
                found = true;
            }
            prefix[grown] = extreme;
            prefix_has[grown] = found;
            grown += 1;
        }
        let mut offset = 1usize;
        while offset <= n_available {
            let right = prefix[offset - 1];
            let left = suffix[offset];
            let right_has = prefix_has[offset - 1];
            let left_has = suffix_has[offset];
            output[today + offset] = match (left_has, right_has) {
                (true, true) => {
                    let right_takes = if WANT_MAX { right > left } else { right < left };
                    if right_takes {
                        right
                    } else {
                        left
                    }
                }
                (true, false) => left,
                (false, true) => right,
                (false, false) => f64::NAN,
            };
            offset += 1;
        }
        block_start += period;
        today += n_available + 1;
    }
    Array1::from_vec(output)
}

fn fused_minmax(input: &[f64], period: usize) -> (Array1<f64>, Array1<f64>) {
    let len = input.len();
    assert!(period <= 64);
    let mut min_output = vec![f64::NAN; len];
    let mut max_output = vec![f64::NAN; len];
    let mut suffix_min = [0.0; 64];
    let mut suffix_max = [0.0; 64];
    let mut prefix_min = [0.0; 64];
    let mut prefix_max = [0.0; 64];
    let mut block_start = 0usize;
    let mut today = period - 1;
    while today < len {
        let block_end = block_start + period - 1;
        let mut low = input[block_end];
        let mut high = low;
        suffix_min[period - 1] = low;
        suffix_max[period - 1] = high;
        let mut offset = period - 1;
        while offset > 0 {
            offset -= 1;
            let value = input[block_start + offset];
            if value < low {
                low = value;
            }
            if value > high {
                high = value;
            }
            suffix_min[offset] = low;
            suffix_max[offset] = high;
        }
        min_output[today] = suffix_min[0];
        max_output[today] = suffix_max[0];

        let block_next = block_start + period;
        if block_next >= len {
            break;
        }
        let n_available = (len - block_next).min(period - 1);
        low = input[block_next];
        high = low;
        prefix_min[0] = low;
        prefix_max[0] = high;
        let mut grown = 1usize;
        while grown < n_available {
            let value = input[block_next + grown];
            if value < low {
                low = value;
            }
            if value > high {
                high = value;
            }
            prefix_min[grown] = low;
            prefix_max[grown] = high;
            grown += 1;
        }
        let mut crossing = 1usize;
        while crossing <= n_available {
            let left_min = suffix_min[crossing];
            let right_min = prefix_min[crossing - 1];
            let left_max = suffix_max[crossing];
            let right_max = prefix_max[crossing - 1];
            min_output[today + crossing] = if right_min < left_min {
                right_min
            } else {
                left_min
            };
            max_output[today + crossing] = if right_max > left_max {
                right_max
            } else {
                left_max
            };
            crossing += 1;
        }
        block_start += period;
        today += n_available + 1;
    }
    (Array1::from_vec(min_output), Array1::from_vec(max_output))
}

fn fused_minmax_nan(input: &[f64], period: usize) -> (Array1<f64>, Array1<f64>) {
    let len = input.len();
    assert!(period <= 64);
    let mut min_output = vec![f64::NAN; len];
    let mut max_output = vec![f64::NAN; len];
    let mut suffix_min = [0.0; 64];
    let mut suffix_max = [0.0; 64];
    let mut prefix_min = [0.0; 64];
    let mut prefix_max = [0.0; 64];
    let mut suffix_has = [false; 64];
    let mut prefix_has = [false; 64];
    let warm_at = input
        .iter()
        .position(|value| !value.is_nan())
        .map_or(len, |start| start + period - 1);
    let mut block_start = 0usize;
    let mut today = period - 1;
    while today < len {
        let block_end = block_start + period - 1;
        let mut low = input[block_end];
        let mut high = low;
        let mut found = !low.is_nan();
        suffix_min[period - 1] = low;
        suffix_max[period - 1] = high;
        suffix_has[period - 1] = found;
        let mut offset = period - 1;
        while offset > 0 {
            offset -= 1;
            let value = input[block_start + offset];
            if !value.is_nan() {
                if !found || value < low {
                    low = value;
                }
                if !found || value > high {
                    high = value;
                }
                found = true;
            }
            suffix_min[offset] = low;
            suffix_max[offset] = high;
            suffix_has[offset] = found;
        }
        min_output[today] = if found { low } else { f64::NAN };
        max_output[today] = if found { high } else { f64::NAN };
        let block_next = block_start + period;
        if block_next >= len {
            break;
        }
        let n_available = (len - block_next).min(period - 1);
        low = input[block_next];
        high = low;
        found = !low.is_nan();
        prefix_min[0] = low;
        prefix_max[0] = high;
        prefix_has[0] = found;
        let mut grown = 1usize;
        while grown < n_available {
            let value = input[block_next + grown];
            if !value.is_nan() {
                if !found || value < low {
                    low = value;
                }
                if !found || value > high {
                    high = value;
                }
                found = true;
            }
            prefix_min[grown] = low;
            prefix_max[grown] = high;
            prefix_has[grown] = found;
            grown += 1;
        }
        let mut crossing = 1usize;
        while crossing <= n_available {
            let left_has = suffix_has[crossing];
            let right_has = prefix_has[crossing - 1];
            let left_min = suffix_min[crossing];
            let right_min = prefix_min[crossing - 1];
            let left_max = suffix_max[crossing];
            let right_max = prefix_max[crossing - 1];
            min_output[today + crossing] = match (left_has, right_has) {
                (true, true) => {
                    if right_min < left_min {
                        right_min
                    } else {
                        left_min
                    }
                }
                (true, false) => left_min,
                (false, true) => right_min,
                (false, false) => f64::NAN,
            };
            max_output[today + crossing] = match (left_has, right_has) {
                (true, true) => {
                    if right_max > left_max {
                        right_max
                    } else {
                        left_max
                    }
                }
                (true, false) => left_max,
                (false, true) => right_max,
                (false, false) => f64::NAN,
            };
            crossing += 1;
        }
        block_start += period;
        today += n_available + 1;
    }
    let warm_end = warm_at.min(len);
    if warm_end > period - 1 {
        min_output[period - 1..warm_end].fill(f64::NAN);
        max_output[period - 1..warm_end].fill(f64::NAN);
    }
    (Array1::from_vec(min_output), Array1::from_vec(max_output))
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

fn time_pair(mut f: impl FnMut() -> (Array1<f64>, Array1<f64>), duration: Duration) -> f64 {
    let started = Instant::now();
    let mut calls = 0u64;
    while started.elapsed() < duration {
        black_box(f());
        calls += 1;
    }
    started.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn measure(label: &str, input: &[f64], want_max: bool) {
    let period = 30;
    let before = || {
        if want_max {
            max(input, period).unwrap()
        } else {
            min(input, period).unwrap()
        }
    };
    let after = || {
        if want_max {
            cached::<true>(input, period)
        } else {
            cached::<false>(input, period)
        }
    };
    let block_nan = || {
        if want_max {
            block_nan_aware::<true>(input, period)
        } else {
            block_nan_aware::<false>(input, period)
        }
    };
    let expected = before();
    let actual = after();
    assert_eq!(expected.len(), actual.len());
    for (index, (&left, &right)) in expected.iter().zip(actual.iter()).enumerate() {
        assert_eq!(left.is_nan(), right.is_nan(), "{label} mask at {index}");
        if left.is_finite() {
            assert_eq!(left, right, "{label} value at {index}");
        }
    }
    let (mut before_samples, mut after_samples) = (Vec::new(), Vec::new());
    for turn in 0..5 {
        if turn % 2 == 0 {
            before_samples.push(time(&mut { || before() }, Duration::from_millis(120)));
            after_samples.push(time(&mut { || after() }, Duration::from_millis(120)));
        } else {
            after_samples.push(time(&mut { || after() }, Duration::from_millis(120)));
            before_samples.push(time(&mut { || before() }, Duration::from_millis(120)));
        }
    }
    before_samples.sort_by(f64::total_cmp);
    after_samples.sort_by(f64::total_cmp);
    println!(
        "{label}: block={:.2}us cached={:.2}us speedup={:.3}x",
        before_samples[2],
        after_samples[2],
        before_samples[2] / after_samples[2]
    );
    let block_expected = block_nan();
    for (index, (&left, &right)) in expected.iter().zip(block_expected.iter()).enumerate() {
        assert_eq!(
            left.is_nan(),
            right.is_nan(),
            "{label} block mask at {index}"
        );
        if left.is_finite() {
            assert_eq!(left, right, "{label} block value at {index}");
        }
    }
    let mut block_samples = Vec::new();
    for _ in 0..5 {
        block_samples.push(time(&mut { || block_nan() }, Duration::from_millis(120)));
    }
    block_samples.sort_by(f64::total_cmp);
    println!(
        "{label}: block_nan={:.2}us speedup={:.3}x",
        block_samples[2],
        before_samples[2] / block_samples[2]
    );
}

fn verify_missing_extrema() {
    let mut input = data(200);
    input[45] = f64::NAN;
    input[73] = f64::INFINITY;
    input[111] = f64::NEG_INFINITY;
    input[130..165].fill(f64::NAN);
    let period = 30;
    let warm_at = input.iter().position(|value| !value.is_nan()).unwrap() + period - 1;

    for want_max in [true, false] {
        let actual = if want_max {
            max(&input, period).unwrap()
        } else {
            min(&input, period).unwrap()
        };
        let expected = if want_max {
            cached::<true>(&input, period)
        } else {
            cached::<false>(&input, period)
        };
        assert!(actual.as_slice().unwrap()[period - 1..warm_at]
            .iter()
            .all(|value| value.is_nan()));
        for index in warm_at..input.len() {
            assert_eq!(actual[index].is_nan(), expected[index].is_nan());
            if actual[index].is_finite() || actual[index].is_infinite() {
                assert_eq!(actual[index], expected[index], "extrema edge value {index}");
            }
        }
    }
    println!("NaN/Inf extrema fallback: parity passed");
}

fn verify_infinite_block() {
    let mut input = data(200);
    input[71] = f64::INFINITY;
    input[133] = f64::NEG_INFINITY;
    for want_max in [true, false] {
        let actual = if want_max {
            max(&input, 30).unwrap()
        } else {
            min(&input, 30).unwrap()
        };
        let expected = if want_max {
            cached::<true>(&input, 30)
        } else {
            cached::<false>(&input, 30)
        };
        for (index, (&left, &right)) in actual.iter().zip(expected.iter()).enumerate() {
            assert_eq!(left.is_nan(), right.is_nan(), "infinite block mask {index}");
            if left.is_finite() || left.is_infinite() {
                assert_eq!(left, right, "infinite block value {index}");
            }
        }
    }
    println!("Infinity-only block path: parity passed");
}

fn main() {
    verify_missing_extrema();
    verify_infinite_block();
    let mut edge_input = data(200);
    edge_input[..5].fill(f64::NAN);
    edge_input[61] = f64::NAN;
    edge_input[62] = f64::NAN;
    edge_input[119] = f64::INFINITY;
    let reference = minmax(&edge_input, 30).unwrap();
    let separate = (min(&edge_input, 30).unwrap(), max(&edge_input, 30).unwrap());
    for (index, ((minimum, maximum), (separate_minimum, separate_maximum))) in reference
        .0
        .iter()
        .zip(reference.1.iter())
        .zip(separate.0.iter().zip(separate.1.iter()))
        .enumerate()
    {
        assert_eq!(
            minimum.is_nan(),
            separate_minimum.is_nan(),
            "separate min mask {index}"
        );
        assert_eq!(
            maximum.is_nan(),
            separate_maximum.is_nan(),
            "separate max mask {index}"
        );
        if minimum.is_finite() || minimum.is_infinite() {
            assert_eq!(minimum, separate_minimum, "separate min value {index}");
        }
        if maximum.is_finite() || maximum.is_infinite() {
            assert_eq!(maximum, separate_maximum, "separate max value {index}");
        }
    }
    let candidate = fused_minmax_nan(&edge_input, 30);
    for (index, ((reference_min, reference_max), (candidate_min, candidate_max))) in reference
        .0
        .iter()
        .zip(reference.1.iter())
        .zip(candidate.0.iter().zip(candidate.1.iter()))
        .enumerate()
    {
        assert_eq!(
            reference_min.is_nan(),
            candidate_min.is_nan(),
            "edge min mask {index}"
        );
        assert_eq!(
            reference_max.is_nan(),
            candidate_max.is_nan(),
            "edge max mask {index}"
        );
        if reference_min.is_finite() || reference_min.is_infinite() {
            assert_eq!(reference_min, candidate_min, "edge min value {index}");
        }
        if reference_max.is_finite() || reference_max.is_infinite() {
            assert_eq!(reference_max, candidate_max, "edge max value {index}");
        }
    }

    for len in [1_000usize, 100_000, 1_000_000] {
        let input = data(len);
        println!("n={len}");
        measure("MAX", &input, true);
        measure("MIN", &input, false);
        let mut before = || (min(&input, 30).unwrap(), max(&input, 30).unwrap());
        let finite_candidate = fused_minmax(&input, 30);
        let mut after = || minmax(&input, 30).unwrap();
        let expected = before();
        let actual = after();
        for (index, ((min_a, max_a), (min_b, max_b))) in expected
            .0
            .iter()
            .zip(expected.1.iter())
            .zip(actual.0.iter().zip(actual.1.iter()))
            .enumerate()
        {
            assert_eq!(min_a.is_nan(), min_b.is_nan(), "min mask {index}");
            assert_eq!(max_a.is_nan(), max_b.is_nan(), "max mask {index}");
            if min_a.is_finite() {
                assert_eq!(min_a, min_b, "min value {index}");
            }
            if max_a.is_finite() {
                assert_eq!(max_a, max_b, "max value {index}");
            }
        }
        for (index, ((min_a, max_a), (min_b, max_b))) in expected
            .0
            .iter()
            .zip(expected.1.iter())
            .zip(finite_candidate.0.iter().zip(finite_candidate.1.iter()))
            .enumerate()
        {
            assert_eq!(min_a.is_nan(), min_b.is_nan(), "finite min mask {index}");
            assert_eq!(max_a.is_nan(), max_b.is_nan(), "finite max mask {index}");
            if min_a.is_finite() {
                assert_eq!(min_a, min_b, "finite min {index}");
            }
            if max_a.is_finite() {
                assert_eq!(max_a, max_b, "finite max {index}");
            }
        }
        let (mut old_samples, mut new_samples) = (Vec::new(), Vec::new());
        for turn in 0..5 {
            if turn % 2 == 0 {
                old_samples.push(time_pair(&mut before, Duration::from_millis(120)));
                new_samples.push(time_pair(&mut after, Duration::from_millis(120)));
            } else {
                new_samples.push(time_pair(&mut after, Duration::from_millis(120)));
                old_samples.push(time_pair(&mut before, Duration::from_millis(120)));
            }
        }
        old_samples.sort_by(f64::total_cmp);
        new_samples.sort_by(f64::total_cmp);
        println!(
            "MINMAX: separate={:.2}us paired-block={:.2}us speedup={:.3}x",
            old_samples[2],
            new_samples[2],
            old_samples[2] / new_samples[2]
        );
    }
}
