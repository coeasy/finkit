use finkit::compute::FactorPlan;
use finkit::factors::{
    BorrowedFactorContext, FactorDefinition, FactorDirection, FactorEngine, FactorKind,
    FactorRegistry,
};
use finkit::indicators::ht_sine;
use finkit::math::simd_kernels::{sma_scalar_naive_into, sma_simd_into};
use finkit::unified_runtime::{DirtyRange, RuntimeExecutionMode, UnifiedRuntime};
use std::hint::black_box;
use std::sync::Arc;
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

#[test]
fn dirty_range_execution_keeps_work_local_and_matches_full_recompute() {
    // A one-row historical correction in a 100K-row series with a fixed
    // 20-row lookback should evaluate only 41 rows: 20 rows of history,
    // the dirty row itself, and the 20 future outputs that depend on it.
    // This is a row-efficiency contract, not a noisy wall-clock assertion.
    const ROWS: usize = 100_000;
    const LOOKBACK: usize = 20;
    const DIRTY_ROW: usize = 50_000;

    let mut registry = FactorRegistry::new();
    registry
        .register(FactorDefinition::new(
            "rolling_21_sum",
            ["close"],
            FactorKind::TimeSeries,
            FactorDirection::Neutral,
            Arc::new(|inputs| {
                let close = inputs.get("close")?;
                let mut output = vec![f64::NAN; close.len()];
                for index in LOOKBACK..close.len() {
                    output[index] = close[index - LOOKBACK..=index].iter().sum();
                }
                Ok(output)
            }),
        ))
        .unwrap();
    let plan = FactorPlan::compile(&registry, &["rolling_21_sum"]).unwrap();
    let engine = FactorEngine::new(registry);

    let original: Vec<f64> = (0..ROWS)
        .map(|index| 100.0 + (index as f64 * 0.013).sin())
        .collect();
    let original_context = BorrowedFactorContext::new()
        .with_series("close", &original)
        .unwrap();
    let full =
        UnifiedRuntime::execute_factor_plan_borrowed(&plan, &engine, &original_context).unwrap();
    let mut ranged_output = full.output;

    let mut changed = original.clone();
    changed[DIRTY_ROW] += 7.0;
    let changed_context = BorrowedFactorContext::new()
        .with_series("close", &changed)
        .unwrap();
    let trace = UnifiedRuntime::execute_factor_plan_range_into_borrowed(
        &plan,
        &engine,
        &changed_context,
        &mut ranged_output,
        DirtyRange::new(DIRTY_ROW, DIRTY_ROW + 1),
        LOOKBACK,
    )
    .unwrap();

    assert_eq!(trace.recomputed_rows, LOOKBACK * 2 + 1);
    assert!(
        trace.recomputed_rows * 1_000 < ROWS,
        "dirty-range execution recomputed too much work: {} of {ROWS} rows",
        trace.recomputed_rows
    );
    assert!(matches!(trace.mode, RuntimeExecutionMode::Range { .. }));

    let expected =
        UnifiedRuntime::execute_factor_plan_borrowed(&plan, &engine, &changed_context).unwrap();
    let ranged = &ranged_output["rolling_21_sum"];
    let full_changed = &expected.output["rolling_21_sum"];
    for index in 0..ROWS {
        if ranged[index].is_nan() && full_changed[index].is_nan() {
            continue;
        }
        assert_eq!(
            ranged[index], full_changed[index],
            "dirty-range result diverged from full recompute at row {index}"
        );
    }
}
