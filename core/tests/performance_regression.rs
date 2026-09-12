use finkit::indicators::ht_sine;
use finkit::math::simd_kernels::{sma_scalar_naive_into, sma_simd_into};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn sample(len: usize) -> Vec<f64> {
    (0..len)
        .map(|index| {
            let x = index as f64;
            100.0 + x * 0.001 + (x * 0.17).sin() * 2.0
        })
        .collect()
}

fn best_of(mut run: impl FnMut(), rounds: usize) -> Duration {
    (0..rounds)
        .map(|_| {
            let start = Instant::now();
            run();
            start.elapsed()
        })
        .min()
        .expect("at least one timing round")
}

fn median_ns_per_bar(mut run: impl FnMut(), bars_per_round: usize, rounds: usize) -> f64 {
    assert!(rounds > 0 && rounds % 2 == 1);
    let mut samples = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        let start = Instant::now();
        run();
        samples.push(start.elapsed().as_nanos() as f64 / bars_per_round as f64);
    }
    samples.sort_by(f64::total_cmp);
    samples[rounds / 2]
}

#[test]
fn optimized_sma_keeps_linear_path_advantage() {
    // Compare two implementations inside the same process instead of using an
    // absolute wall-clock budget. This makes the gate resilient to shared CI
    // runner speed while still catching accidental O(n * period) regressions.
    const LEN: usize = 120_000;
    const PERIOD: usize = 64;
    let input = sample(LEN);
    let mut optimized = vec![f64::NAN; LEN];
    let mut naive = vec![f64::NAN; LEN];

    // Warm CPU dispatch/caches before measuring either path.
    sma_simd_into(&input, PERIOD, &mut optimized);
    sma_scalar_naive_into(&input, PERIOD, &mut naive);

    for index in PERIOD - 1..LEN {
        let delta = (optimized[index] - naive[index]).abs();
        assert!(delta <= 1e-9, "SMA mismatch at {index}: {delta}");
    }

    let optimized_time = best_of(
        || sma_simd_into(black_box(&input), PERIOD, black_box(&mut optimized)),
        5,
    );
    let naive_time = best_of(
        || sma_scalar_naive_into(black_box(&input), PERIOD, black_box(&mut naive)),
        3,
    );

    // The optimized rolling implementation should have a substantial margin
    // over the deliberately O(n * period) reference. A 1.25x allowance keeps
    // the assertion stable while detecting loss of the linear rolling path.
    assert!(
        optimized_time.as_nanos() * 5 <= naive_time.as_nanos() * 4,
        "optimized SMA regression: optimized={optimized_time:?}, naive={naive_time:?}"
    );
}

#[test]
fn ht_sine_release_throughput_stays_within_budget() {
    // Keep the historical <1000 ns/bar contract, but enforce it in the
    // dedicated release-mode, single-threaded performance gate. The ordinary
    // debug unit-test suite runs many tests concurrently on shared runners and
    // is not a reliable place for an absolute wall-clock assertion.
    const LEN: usize = 4_000;
    const ITERS: usize = 64;
    const ROUNDS: usize = 5;
    const BUDGET_NS_PER_BAR: f64 = 1_000.0;

    let input: Vec<f64> = (0..LEN)
        .map(|index| {
            let x = index as f64;
            100.0 + 10.0 * (x * 0.13).sin() + (x * 0.7).cos()
        })
        .collect();

    for _ in 0..8 {
        black_box(ht_sine(black_box(&input)).unwrap());
    }

    let ns_per_bar = median_ns_per_bar(
        || {
            for _ in 0..ITERS {
                black_box(ht_sine(black_box(&input)).unwrap());
            }
        },
        LEN * ITERS,
        ROUNDS,
    );

    assert!(
        ns_per_bar < BUDGET_NS_PER_BAR,
        "ht_sine release throughput regression: {ns_per_bar:.2} ns/bar >= {BUDGET_NS_PER_BAR:.0} ns/bar"
    );
}
