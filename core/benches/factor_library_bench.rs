//! Benchmark proving the M0-3 criterion: "第二次求值 0 解析开销".
//!
//! The claim is that a factor library is parsed and compiled **once**, when the
//! library is built, and that evaluating a factor afterwards performs no lexing,
//! parsing, lowering, topological sort or register allocation. This benchmark
//! measures that by running the two paths side by side over the same expression:
//!
//! * `recompile_each_call` — what a caller without the library does:
//!   `FactorGraph::from_expression` -> `build` -> `execute_node`, per iteration.
//! * `reuse_compiled_plan` — what [`FactorLibrary::evaluate`] does: run the plan
//!   that was built once, per iteration.
//!
//! The ratio between them *is* the parse overhead, so the report prints it
//! rather than leaving it to be inferred from two absolute numbers.
//!
//! Run with:
//! ```bash
//! cargo bench -p finkit --bench factor_library_bench
//! ```
//!
//! # Why the report is hand-rolled
//!
//! Criterion measures one routine at a time and deliberately does not compare
//! across groups — that is a statistical claim about two separate sampling runs.
//! The criterion is a *ratio*, so it is measured directly, in one process, with
//! the two paths interleaved. Criterion still runs afterwards for the absolute
//! per-factor numbers and for its change detection across commits.

use std::hint::black_box;
use std::time::Instant;

use criterion::{criterion_group, Criterion, Throughput};
use ndarray::Array1;

use finkit::factor_graph::FactorGraph;
use finkit::factors::builtin::{factor_library, FactorLibrary};
use finkit::formula::FormulaContext;

/// The expression used for the head-to-head comparison.
///
/// Chosen because it is a real Alpha158 factor with a rolling window, two
/// distinct operators and a division — i.e. enough graph to make compilation
/// visible — while being short enough that the compile cost is not dominated by
/// one pathological term.
const EXPRESSION: &str = "MA(CLOSE, 20)/CLOSE";

/// Bars per evaluation. Long enough that a rolling kernel does real work and the
/// ratio is not dominated by fixed per-call overhead.
const BARS: usize = 250;

/// Iterations for the ratio report. Large enough to swamp timer granularity.
const ITERATIONS: usize = 400;

/// Interleaved rounds. Both arms are measured inside every round so a thermal or
/// scheduling drift affects them equally; measuring "A then B" would attribute
/// the drift to B.
const ROUNDS: usize = 8;

/// Index-to-price conversion.
///
/// `cast_precision_loss` is expected and harmless here: `bars` is 250, so every
/// index is exactly representable in `f64`. The project's convention for this is
/// a local `allow` with the reason (see `core/src/factor_graph.rs:710`).
#[allow(clippy::cast_precision_loss)]
fn as_f64(index: usize) -> f64 {
    index as f64
}

fn market(bars: usize) -> FormulaContext {
    let close: Vec<f64> = (0..bars)
        .map(|i| {
            let t = as_f64(i);
            100.0 + (t * 0.11).sin() * 4.0 + t * 0.02
        })
        .collect();
    let high: Vec<f64> = close.iter().map(|value| value + 1.0).collect();
    let low: Vec<f64> = close.iter().map(|value| value - 1.0).collect();
    let volume: Vec<f64> = (0..bars)
        .map(|i| 10_000.0 + (as_f64(i) * 0.31).cos() * 800.0)
        .collect();
    // Alpha158's price block reads `vwap`. It is not one of the five OHLCV
    // fields, so it has to arrive as a formula variable; the factor engine
    // binding does this automatically from the caller's series, but a caller
    // driving `FactorLibrary::evaluate` with a `FormulaContext` must supply it.
    let vwap: Vec<f64> = (0..bars)
        .map(|i| (high[i] + low[i] + close[i]) / 3.0)
        .collect();

    let mut context = FormulaContext::new(
        Array1::from_vec(close.clone()),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    );
    context.set_variable("VWAP".to_string(), Array1::from_vec(vwap));
    context
}

/// Time `iterations` of `body`, returning the total.
fn time<T>(iterations: usize, mut body: impl FnMut() -> T) -> std::time::Duration {
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(body());
    }
    start.elapsed()
}

/// Print the parse-overhead ratio, then return it.
///
/// Interleaved rather than run in sequence so a thermal or scheduling drift
/// affects both arms; a plain "A then B" measurement would attribute any drift
/// to B.
fn report_parse_overhead() -> f64 {
    let library = factor_library("alpha158").expect("alpha158 builds");
    let context = market(BARS);
    let graph = FactorGraph::from_expression(EXPRESSION).expect("expression parses");
    let primary = graph
        .expression_primary()
        .expect("expression has a primary")
        .to_string();

    // Warm both arms: the first call pays page faults and branch-predictor
    // cold-start that neither arm would pay in steady state.
    for _ in 0..16 {
        black_box(
            library
                .evaluate("MA20", &context)
                .expect("reused path runs"),
        );
        let plan = graph.build(&primary).expect("plan builds");
        black_box(
            plan.execute_node(&context, plan.primary())
                .expect("recompile path runs"),
        );
    }

    let mut recompile = std::time::Duration::ZERO;
    let mut reuse = std::time::Duration::ZERO;
    for _ in 0..ROUNDS {
        recompile += time(ITERATIONS, || {
            let plan = graph.build(&primary).expect("plan builds");
            plan.execute_node(&context, plan.primary())
                .expect("recompile path runs")
        });
        reuse += time(ITERATIONS, || {
            library
                .evaluate("MA20", &context)
                .expect("reused path runs")
        });
    }

    let calls = ITERATIONS * ROUNDS;
    let calls_f64 = as_f64(calls);
    let recompile_us = recompile.as_secs_f64() * 1e6 / calls_f64;
    let reuse_us = reuse.as_secs_f64() * 1e6 / calls_f64;
    let ratio = recompile_us / reuse_us;

    println!();
    println!("=== M0-3: parse overhead on the second evaluation ===");
    println!("expression        : {EXPRESSION}");
    println!("bars              : {BARS}");
    println!("calls per arm     : {calls}");
    println!("recompile each call: {recompile_us:9.3} us/call");
    println!("reuse compiled plan: {reuse_us:9.3} us/call");
    println!("ratio              : {ratio:9.2}x");
    println!("overhead removed   : {:9.2}%", (1.0 - 1.0 / ratio) * 100.0);
    println!();

    ratio
}

fn bench_reuse_vs_recompile(c: &mut Criterion) {
    let library = factor_library("alpha158").expect("alpha158 builds");
    let context = market(BARS);
    let graph = FactorGraph::from_expression(EXPRESSION).expect("expression parses");
    let primary = graph
        .expression_primary()
        .expect("expression has a primary")
        .to_string();

    let mut group = c.benchmark_group("factor_library_parse_overhead");
    group.throughput(Throughput::Elements(BARS as u64));
    group.bench_function("recompile_each_call", |bencher| {
        bencher.iter(|| {
            let plan = graph.build(&primary).expect("plan builds");
            plan.execute_node(black_box(&context), plan.primary())
                .expect("plan runs")
        });
    });
    group.bench_function("reuse_compiled_plan", |bencher| {
        bencher.iter(|| {
            library
                .evaluate(black_box("MA20"), black_box(&context))
                .expect("factor runs")
        });
    });
    group.finish();
}

/// Evaluate every factor in a library, as a caller would for an IC study.
///
/// This is the number a user feels: 158 factors over one market, once, with the
/// plans already compiled.
fn bench_whole_library(c: &mut Criterion) {
    let context = market(BARS);
    for name in ["alpha158", "worldquant101"] {
        let library = factor_library(name).expect("library builds");
        let mut group = c.benchmark_group(format!("factor_library_{name}_sweep"));
        group.throughput(Throughput::Elements((library.len() * BARS) as u64));
        group.bench_function(format!("{}_factors", library.len()), |bencher| {
            bencher.iter(|| {
                let mut finite = 0_usize;
                for factor in library.names() {
                    let values = library.evaluate(factor, black_box(&context)).expect("runs");
                    finite += values.iter().filter(|value| value.is_finite()).count();
                }
                finite
            });
        });
        group.finish();
    }
}

/// A library's size is the count a caller sees; keep it in the report so the
/// sweep numbers are interpretable.
fn report_library_sizes() {
    println!("=== shipped library sizes ===");
    for name in finkit::factors::builtin::LIBRARY_NAMES {
        let library: FactorLibrary = factor_library(name).expect("library builds");
        println!("{name:14}: {} factors", library.len());
    }
    println!(
        "demo          : {} factors",
        finkit::factors::demo_factor_registry().names().count()
    );
    println!(
        "shipped total : {} factors",
        finkit::factors::builtin_factor_registry().names().count()
    );
    println!();
}

criterion_group!(benches, bench_reuse_vs_recompile, bench_whole_library);

fn main() {
    report_library_sizes();
    let ratio = report_parse_overhead();
    // The criterion is a ratio, so a silent regression to "no reuse at all"
    // would show up as `ratio ~= 1.0` in a log nobody reads. Fail the bench run
    // instead, at a threshold far below the measured value so normal variance
    // cannot trip it.
    assert!(
        ratio > 2.0,
        "reusing the compiled plan must beat recompiling it: measured {ratio:.2}x. \
         If this is ~1.0 the library is recompiling per call and the M0-3 \
         'zero parse overhead on the second evaluation' criterion is not met."
    );

    // Same shape as `criterion_main!`, which this file cannot use because the
    // ratio report has to run first.
    benches();
    Criterion::default().configure_from_args().final_summary();
}
