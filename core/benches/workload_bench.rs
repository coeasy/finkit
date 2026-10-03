//! §21 workload benchmarks — real work, not per-function micro-benchmarks.
//!
//! The V4 plan is explicit that `RSI vs TA-Lib` and friends are not a
//! performance story: they measure one kernel on one short series, which is not
//! what a scanner or a factor library does. This file adds the four workloads
//! the plan names, at the sizes it names:
//!
//! ```text
//! Scanner      5000 symbols x 20 indicators x 50 formulas
//! Factor DAG   1000 symbols x 100 factors with shared intermediates
//! DirtyRange   1M rows, dirty 1/10/100 rows, lookback 5/20/60/252
//! Streaming    10K symbols, one-bar update
//! ```
//!
//! Cross-language workloads (Python -> Rust -> `NumPy`, Node -> Rust ->
//! `TypedArray`, C ABI, JVM, .NET, WASM) live with their bindings, where the
//! marshalling boundary actually is; they are not duplicated here.
//!
//! # Why these are *workloads*
//!
//! Each group builds one plan and then runs it over a whole universe, so the
//! number that comes out includes plan compilation, buffer lifetime reuse,
//! arena recycling, and dispatch overhead — the things that actually dominate a
//! scan. A per-kernel benchmark would divide all of that away and report a
//! figure nobody can act on.
//!
//! The Factor DAG group is the one that measures an optimization rather than an
//! implementation: it runs the same factor library twice, once as a naive
//! frontend would declare it (each factor re-declaring the intermediate it
//! needs) and once after [`SemanticGraph::eliminate_common_subexpressions`].
//! The two are asserted to produce identical values before either is timed, so
//! the reported speedup is a real one.
//!
//! Run with:
//! ```bash
//! cargo bench -p finkit --bench workload_bench
//! cargo bench -p finkit --bench workload_bench -- workload_scanner
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use finkit::compute::{
    ComputeCapabilities, ComputeEffect, ComputeNodeId, DependencyShape, FactorPlan,
    LookbackRequirement,
};
use finkit::execution_plan::{HotExecutionPlan, KernelId};
use finkit::factors::{
    BorrowedFactorContext, FactorDefinition, FactorDirection, FactorEngine, FactorKind,
    FactorRegistry,
};
use finkit::math::moving_avg::sma_into;
use finkit::math::rolling_stats::stddev_into;
use finkit::runtime_context::RuntimeContext;
use finkit::semantic_graph::{NodeKind, SemanticGraph, SemanticNodeId};
use finkit::state_arena::StateArena;
use finkit::streaming::indicators::{StreamingEma, StreamingRsi, StreamingSma};
use finkit::streaming::StreamingIndicator;
use finkit::unified_executor::{
    KernelCall, KernelDispatchError, KernelDispatcher, UnifiedExecutor,
};
use finkit::unified_runtime::{DirtyRange, UnifiedRuntime};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Bars per symbol. Long enough to cover every window the Scanner uses.
const BARS: usize = 260;

/// A symbol universe of `symbols` synthetic random-walk series.
///
/// Generated once per group, outside the timed region — a real scanner reads
/// its data from a store, it does not synthesize it per iteration.
fn universe(symbols: usize, bars: usize) -> Vec<Vec<f64>> {
    (0..symbols)
        .map(|symbol| {
            let mut value = 20.0 + (symbol % 97) as f64;
            (0..bars)
                .map(|bar| {
                    value += 0.35 + ((symbol * 31 + bar * 17) % 23) as f64 / 23.0 - 0.5;
                    value
                })
                .collect()
        })
        .collect()
}

// ───────────────────────────── dispatch ─────────────────────────────

/// The small kernel set these workloads need.
///
/// Windows are baked into the operation name, because that is how the frontend
/// lowering already hands parameters to an executor (`KernelId::compile`
/// hashes the operation label). Keeping the window in the name means the
/// dispatcher needs no parameter plumbing and no per-dispatch parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum RollingKernel {
    Sma(usize),
    StdDev(usize),
    Add,
    Sub,
    Mul,
}

impl RollingKernel {
    fn operation(self) -> String {
        match self {
            Self::Sma(window) => format!("WORKLOAD_SMA_{window}"),
            Self::StdDev(window) => format!("WORKLOAD_STDD_{window}"),
            Self::Add => "WORKLOAD_ADD".to_string(),
            Self::Sub => "WORKLOAD_SUB".to_string(),
            Self::Mul => "WORKLOAD_MUL".to_string(),
        }
    }

    /// Capabilities for this kernel at a given dependency.
    ///
    /// `FixedLookback` is the honest shape for every kernel here: a rolling
    /// window of `window - 1` historical rows, plus elementwise ops that need
    /// no history at all.
    fn capabilities(self) -> ComputeCapabilities {
        let rows = match self {
            Self::Sma(window) | Self::StdDev(window) => window.saturating_sub(1),
            Self::Add | Self::Sub | Self::Mul => 0,
        };
        ComputeCapabilities {
            deterministic: true,
            streaming: matches!(self, Self::Sma(_) | Self::StdDev(_)),
            stateful: false,
            lookback: LookbackRequirement::Fixed(rows),
            dependency: DependencyShape::FixedLookback(rows),
            effect: ComputeEffect::Pure,
        }
    }
}

/// Resolve a [`KernelId`] back to its kernel without hashing per dispatch.
struct RollingDispatcher {
    table: Vec<(KernelId, RollingKernel)>,
}

impl RollingDispatcher {
    fn new(kernels: &[RollingKernel]) -> Self {
        let mut table: Vec<(KernelId, RollingKernel)> = Vec::new();
        for kernel in kernels {
            let id = KernelId::compile(&kernel.operation());
            if !table.iter().any(|(known, _)| *known == id) {
                table.push((id, *kernel));
            }
        }
        Self { table }
    }

    fn kernel(&self, id: KernelId) -> Option<RollingKernel> {
        self.table
            .iter()
            .find(|(known, _)| *known == id)
            .map(|(_, kernel)| *kernel)
    }
}

/// Split the buffer arena into the operand view and the mutable output.
///
/// This is safe and allocation-free because `PlanBufferLayout` guarantees that
/// an output slot never aliases an input of the same kernel — see
/// `PlanBufferLayout::compile` in `core/src/buffer_arena.rs`, where a slot is
/// only recycled once *every* consumer of its previous value has run.
///
/// The naive alternative — cloning the operands, as the convergence test's
/// dispatcher does — would put a heap allocation per kernel on the hot path and
/// swamp everything these benchmarks are trying to measure.
fn split_slots(
    buffers: &mut [Vec<f64>],
    output: usize,
) -> (&[Vec<f64>], &mut Vec<f64>, &[Vec<f64>]) {
    let (before, tail) = buffers.split_at_mut(output);
    let (slot, after) = tail
        .split_first_mut()
        .expect("the output slot is part of the buffer arena");
    (before, slot, after)
}

impl KernelDispatcher for RollingDispatcher {
    fn dispatch(
        &mut self,
        call: KernelCall<'_>,
        buffers: &mut [Vec<f64>],
        _states: &mut StateArena,
    ) -> Result<(), KernelDispatchError> {
        let kernel = self
            .kernel(call.kernel)
            .ok_or_else(|| KernelDispatchError::new(1))?;

        let (before, output, after) = split_slots(buffers, call.output.0);
        // `&'a [f64]` is tied to the arena, not to this closure, so the operand
        // reads may coexist with the mutable output borrow.
        let operand = |slot: usize| -> &[f64] {
            if slot < before.len() {
                &before[slot]
            } else {
                &after[slot - before.len() - 1]
            }
        };

        match kernel {
            RollingKernel::Sma(window) => {
                sma_into(operand(call.inputs[0].0), window, output)
                    .map_err(|_| KernelDispatchError::new(2))?;
            }
            RollingKernel::StdDev(window) => {
                stddev_into(operand(call.inputs[0].0), window, 1.0, output)
                    .map_err(|_| KernelDispatchError::new(3))?;
            }
            RollingKernel::Add | RollingKernel::Sub | RollingKernel::Mul => {
                let left = operand(call.inputs[0].0);
                let right = operand(call.inputs[1].0);
                for (index, slot) in output.iter_mut().enumerate() {
                    // NaN in the warm-up region propagates, exactly as the
                    // batching kernels' own warm-up does.
                    *slot = match kernel {
                        RollingKernel::Add => left[index] + right[index],
                        RollingKernel::Sub => left[index] - right[index],
                        _ => left[index] * right[index],
                    };
                }
            }
        }
        Ok(())
    }
}

/// Lower a semantic graph and hot-compile it against its declared target.
fn hot_plan_for(graph: &SemanticGraph) -> HotExecutionPlan {
    let plan = graph.lower().expect("a validated graph lowers");
    let target = graph
        .targets()
        .first()
        .copied()
        .expect("every benchmark graph declares a target");
    HotExecutionPlan::compile(&plan, [ComputeNodeId(target.0)])
        .expect("the lowered plan is hot-compilable")
}

/// Sum the retained outputs so the optimizer cannot elide the work.
fn checksum(output: &finkit::unified_executor::ExecutionOutput) -> f64 {
    output
        .values
        .iter()
        .map(|series| series.iter().filter(|value| value.is_finite()).sum::<f64>())
        .sum()
}

/// The same, for the named-output map the factor runtime returns.
fn map_checksum(output: &BTreeMap<String, Vec<f64>>) -> f64 {
    output
        .values()
        .map(|series| series.iter().filter(|value| value.is_finite()).sum::<f64>())
        .sum()
}

// ───────────────────────────── Scanner ─────────────────────────────

/// The ten rolling windows the Scanner's twenty indicators are built from.
const SCANNER_WINDOWS: [usize; 10] = [3, 5, 8, 10, 13, 20, 30, 40, 50, 60];

/// Twenty indicators: ten SMAs and ten standard deviations over CLOSE.
fn scanner_kernels() -> Vec<RollingKernel> {
    let mut kernels = Vec::with_capacity(20);
    for window in SCANNER_WINDOWS {
        kernels.push(RollingKernel::Sma(window));
    }
    for window in SCANNER_WINDOWS {
        kernels.push(RollingKernel::StdDev(window));
    }
    kernels
}

/// Twenty indicators shared by fifty formulas.
///
/// The indicators are declared once and read many times, which is what makes
/// this a *graph* rather than fifty independent formula evaluations.
fn scanner_graph() -> (SemanticGraph, Vec<RollingKernel>) {
    let mut builder = SemanticGraph::builder();
    let close = builder.push_leaf(NodeKind::Input, "VARIABLE:CLOSE");

    let mut kernels = Vec::new();
    let mut indicators = Vec::with_capacity(20);
    for kernel in scanner_kernels() {
        let id = builder.push(
            NodeKind::Indicator,
            kernel.operation(),
            vec![close],
            kernel.capabilities(),
        );
        indicators.push(id);
        kernels.push(kernel);
    }

    let mut last = indicators[0];
    for index in 0..50usize {
        // A fixed pairing rule, so the workload is identical on every run.
        let left = indicators[index % indicators.len()];
        let right = indicators[(index * 7 + 3) % indicators.len()];
        let kernel = match index % 3 {
            0 => RollingKernel::Add,
            1 => RollingKernel::Sub,
            _ => RollingKernel::Mul,
        };
        let id = builder.push(
            NodeKind::Formula,
            kernel.operation(),
            vec![left, right],
            kernel.capabilities(),
        );
        kernels.push(kernel);
        last = id;
    }

    builder.target(last);
    (
        builder.build().expect("the scanner graph is a DAG"),
        kernels,
    )
}

fn bench_scanner(c: &mut Criterion) {
    let (graph, kernels) = scanner_graph();
    let mut group = c.benchmark_group("workload_scanner");
    group.sample_size(10);

    for symbols in [500usize, 5000] {
        let data = universe(symbols, BARS);
        let mut executor =
            UnifiedExecutor::new(hot_plan_for(&graph), RollingDispatcher::new(&kernels));
        group.throughput(Throughput::Elements((symbols * BARS) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(symbols), &symbols, |b, _| {
            b.iter(|| {
                let mut total = 0.0f64;
                for series in &data {
                    let output = executor
                        .execute(&[series])
                        .expect("the scanner plan executes");
                    total += black_box(checksum(&output));
                }
                black_box(total)
            });
        });
    }
    group.finish();
}

// ────────────────────────── Factor DAG ──────────────────────────

/// Ten shared base windows, re-declared once per factor by a naive frontend.
const FACTOR_INTERMEDIATE_WINDOWS: [usize; 10] = [5, 10, 15, 20, 30, 40, 50, 60, 90, 120];

const FACTOR_COUNT: usize = 100;

/// One hundred factors over ten distinct shared intermediates.
///
/// `duplicate_intermediates` controls whether the graph looks like a naive
/// frontend's output (every factor re-declaring the rolling window it needs,
/// 201 nodes) or a deduplicated one (111 nodes).
///
/// The factor pairings are index-dependent, so the hundred factor nodes are
/// genuinely hundred distinct computations. Only the intermediates collapse —
/// which is exactly the win §20, priority 1 is about.
fn factor_dag_graph(duplicate_intermediates: bool) -> (SemanticGraph, Vec<RollingKernel>) {
    let mut builder = SemanticGraph::builder();
    let close = builder.push_leaf(NodeKind::Input, "VARIABLE:CLOSE");

    let mut kernels = Vec::new();
    let mut declared: BTreeMap<RollingKernel, SemanticNodeId> = BTreeMap::new();
    let mut intermediates = Vec::with_capacity(FACTOR_COUNT);

    for index in 0..FACTOR_COUNT {
        let window = FACTOR_INTERMEDIATE_WINDOWS[index % FACTOR_INTERMEDIATE_WINDOWS.len()];
        let kernel = RollingKernel::Sma(window);

        // A naive frontend emits a fresh node for every factor that needs a
        // window, even when it already emitted the identical one. The dedup
        // build reuses the node it already has, which is what CSE would achieve
        // anyway — so the two graphs differ only in how many copies were
        // declared, not in what they compute.
        let id = match declared.get(&kernel) {
            Some(existing) if !duplicate_intermediates => *existing,
            _ => {
                let id = builder.push(
                    NodeKind::Indicator,
                    kernel.operation(),
                    vec![close],
                    kernel.capabilities(),
                );
                declared.entry(kernel).or_insert(id);
                id
            }
        };
        intermediates.push(id);
        // Extra entries are harmless: the dispatcher de-duplicates its table.
        kernels.push(kernel);
    }

    let mut last = intermediates[0];
    for index in 0..FACTOR_COUNT {
        let left = intermediates[index % FACTOR_COUNT];
        let right = intermediates[(index + 1 + index / 10) % FACTOR_COUNT];
        let kernel = RollingKernel::Sub;
        let id = builder.push(
            NodeKind::Factor,
            kernel.operation(),
            vec![left, right],
            kernel.capabilities(),
        );
        kernels.push(kernel);
        last = id;
    }

    builder.target(last);
    (builder.build().expect("the factor DAG is a DAG"), kernels)
}

/// How each scan iteration receives its result.
///
/// `Owned` is `UnifiedExecutor::execute`, which hands the result buffers to the
/// caller and therefore re-allocates one buffer per execution. `Persistent` is
/// `UnifiedExecutor::execute_into`, where the caller keeps one destination and
/// the whole working set returns to the arena — the shape a real scan loop
/// uses, and the reason §20, priority 2 lists persistent output alongside the
/// `_into` kernels.
#[derive(Debug, Clone, Copy)]
enum OutputMode {
    Owned,
    Persistent,
}

fn bench_factor_dag(c: &mut Criterion) {
    let (naive, naive_kernels) = factor_dag_graph(true);
    let outcome = naive.eliminate_common_subexpressions();
    let deduplicated = outcome.graph;
    let deduplicated_kernels = naive_kernels.clone();

    // The benchmark is only meaningful if the two graphs agree numerically, so
    // verify that on the smallest universe before timing anything.
    {
        let probe = universe(4, BARS);
        let mut naive_executor =
            UnifiedExecutor::new(hot_plan_for(&naive), RollingDispatcher::new(&naive_kernels));
        let mut fold_executor = UnifiedExecutor::new(
            hot_plan_for(&deduplicated),
            RollingDispatcher::new(&deduplicated_kernels),
        );
        for series in &probe {
            let before = naive_executor.execute(&[series]).expect("naive execution");
            let after = fold_executor.execute(&[series]).expect("cse execution");
            assert_eq!(before.values.len(), after.values.len());
            for (left, right) in before.values[0].iter().zip(after.values[0].iter()) {
                assert!(
                    (left.is_nan() && right.is_nan()) || (left - right).abs() < 1e-9,
                    "CSE changed a factor value: {left} vs {right}"
                );
            }
        }
    }

    println!(
        "workload_factor_dag: {} nodes -> {} after CSE ({} merged), declared {} factors",
        outcome.report.nodes_before,
        outcome.report.nodes_after,
        outcome.report.merged,
        FACTOR_COUNT
    );

    let mut group = c.benchmark_group("workload_factor_dag");
    group.sample_size(10);

    for symbols in [100usize, 1000] {
        let data = universe(symbols, BARS);
        group.throughput(Throughput::Elements((symbols * BARS) as u64));
        for (label, graph, kernels) in [
            ("naive", &naive, &naive_kernels),
            ("cse", &deduplicated, &deduplicated_kernels),
        ] {
            for mode in [OutputMode::Owned, OutputMode::Persistent] {
                let mut executor =
                    UnifiedExecutor::new(hot_plan_for(graph), RollingDispatcher::new(kernels));
                let mut destination = vec![0.0; BARS];
                let name = match mode {
                    OutputMode::Owned => BenchmarkId::new(label, symbols),
                    OutputMode::Persistent => BenchmarkId::new(format!("{label}_into"), symbols),
                };
                group.bench_with_input(name, &symbols, |b, _| {
                    b.iter(|| {
                        let mut total = 0.0f64;
                        for series in &data {
                            match mode {
                                OutputMode::Owned => {
                                    let output = executor
                                        .execute(&[series])
                                        .expect("the factor DAG executes");
                                    total += black_box(checksum(&output));
                                }
                                OutputMode::Persistent => {
                                    let mut destinations: [&mut [f64]; 1] =
                                        [destination.as_mut_slice()];
                                    executor
                                        .execute_into(&[series], &mut destinations)
                                        .expect("the factor DAG executes into the destination");
                                    total += black_box(
                                        destination
                                            .iter()
                                            .filter(|value| value.is_finite())
                                            .sum::<f64>(),
                                    );
                                }
                            }
                        }
                        black_box(total)
                    });
                });
            }
        }
    }
    group.finish();
}

// ────────────────────────── DirtyRange ──────────────────────────

/// One million rows, as the plan specifies.
const DIRTY_RANGE_ROWS: usize = 1_000_000;

/// The lookbacks the plan names, plus the dirty widths it names.
const DIRTY_RANGE_LOOKBACKS: [usize; 4] = [5, 20, 60, 252];
const DIRTY_RANGE_WIDTHS: [usize; 3] = [1, 10, 100];

/// Register a trailing-window factor with a genuine fixed lookback of
/// `lookback - 1` rows: `close[i] * 2 - close[i - (lookback - 1)]`.
///
/// The plan is then honest about its dependency, which is what lets the runtime
/// prove that a dirty window can be recomputed with `lookback` rows of history
/// instead of the whole series.
fn register_window_factor(registry: &mut FactorRegistry, lookback: usize) {
    let name = format!("window_{lookback}");
    registry
        .register(FactorDefinition::new(
            name,
            ["close"],
            FactorKind::TimeSeries,
            FactorDirection::HigherBetter,
            Arc::new(move |inputs| {
                let close = inputs.get("close")?;
                Ok(close
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        if index < lookback - 1 {
                            f64::NAN
                        } else {
                            value * 2.0 - close[index - (lookback - 1)]
                        }
                    })
                    .collect())
            }),
        ))
        .expect("the window factor registers");
}

fn bench_dirty_range(c: &mut Criterion) {
    let close: Vec<f64> = (0..DIRTY_RANGE_ROWS)
        .map(|index| 50.0 + (index as f64) * 0.0001)
        .collect();

    let mut group = c.benchmark_group("workload_dirty_range");
    group.sample_size(10);

    for lookback in DIRTY_RANGE_LOOKBACKS {
        let name = format!("window_{lookback}");
        let mut registry = FactorRegistry::new();
        register_window_factor(&mut registry, lookback);
        let plan = FactorPlan::compile(&registry, &[name.as_str()])
            .expect("the window factor plan compiles");
        let engine = FactorEngine::new(registry);

        let full_context = BorrowedFactorContext::new()
            .with_series("close", &close)
            .expect("the series binds");
        let materialized =
            UnifiedRuntime::execute_factor_plan_borrowed(&plan, &engine, &full_context)
                .expect("the full baseline executes")
                .output;
        assert!(
            materialized.contains_key(&name),
            "the plan must retain the factor it was compiled for"
        );

        group.throughput(Throughput::Elements(DIRTY_RANGE_ROWS as u64));
        group.bench_with_input(BenchmarkId::new("full", lookback), &lookback, |b, _| {
            b.iter(|| {
                let output =
                    UnifiedRuntime::execute_factor_plan_borrowed(&plan, &engine, &full_context)
                        .expect("full execution");
                black_box(map_checksum(&output.output))
            });
        });

        let mut mutated = close.clone();
        mutated[DIRTY_RANGE_ROWS / 2] += 1.0;
        let mutated_context = BorrowedFactorContext::new()
            .with_series("close", &mutated)
            .expect("the mutated series binds");

        for width in DIRTY_RANGE_WIDTHS {
            if width >= DIRTY_RANGE_ROWS {
                continue;
            }
            let start = DIRTY_RANGE_ROWS / 2 - width / 2;
            let dirty = DirtyRange::new(start, start + width);

            group.throughput(Throughput::Elements(width as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("range_{width}"), lookback),
                &width,
                |b, _| {
                    let mut runtime = RuntimeContext::new();
                    // The retained materialization is what a dirty-range update
                    // splices into; cloning it once per bench, not per
                    // iteration, is the whole point of the call.
                    let mut spliced = materialized.clone();
                    b.iter(|| {
                        runtime.reset();
                        let trace =
                            UnifiedRuntime::execute_factor_plan_range_into_borrowed_with_context(
                                &plan,
                                &engine,
                                &mutated_context,
                                &mut spliced,
                                dirty,
                                DependencyShape::FixedLookback(lookback - 1),
                                &mut runtime,
                            )
                            .expect("dirty-range execution");
                        // Touching the middle row plus the trace keeps the
                        // recompute observable to the optimizer.
                        let probe = spliced.get(&name).expect("the factor output is retained")
                            [DIRTY_RANGE_ROWS / 2];
                        black_box((trace.recomputed_rows, probe))
                    });
                },
            );
        }
    }
    group.finish();
}

// ────────────────────────── Streaming ──────────────────────────

/// Ten thousand symbols, each streaming three indicators.
const STREAMING_SYMBOLS: usize = 10_000;

/// Bars pushed during warm-up so every indicator is past its lookback.
const STREAMING_WARMUP: usize = 40;

fn bench_streaming(c: &mut Criterion) {
    let mut group = c.benchmark_group("workload_streaming");
    group.sample_size(50);

    // Warm every symbol's indicators once, then measure a single one-bar update
    // across the whole universe — the shape a live quote feed actually has.
    let mut sma: Vec<StreamingSma> = (0..STREAMING_SYMBOLS)
        .map(|_| StreamingSma::new(20))
        .collect();
    let mut ema: Vec<StreamingEma> = (0..STREAMING_SYMBOLS)
        .map(|_| StreamingEma::new(20))
        .collect();
    let mut rsi: Vec<StreamingRsi> = (0..STREAMING_SYMBOLS)
        .map(|_| StreamingRsi::new(14))
        .collect();

    let warmup: Vec<f64> = (0..STREAMING_WARMUP)
        .map(|index| 30.0 + index as f64)
        .collect();
    for value in &warmup {
        for indicator in &mut sma {
            indicator.next(*value);
        }
        for indicator in &mut ema {
            indicator.next(*value);
        }
        for indicator in &mut rsi {
            indicator.next(*value);
        }
    }

    group.throughput(Throughput::Elements((STREAMING_SYMBOLS * 3) as u64));
    group.bench_function("one_bar_update", |b| {
        let mut tick = 0.0f64;
        b.iter(|| {
            tick += 0.01;
            let value = 70.0 + tick;
            let mut total = 0.0f64;
            for indicator in &mut sma {
                total += indicator.next(value).unwrap_or(0.0);
            }
            for indicator in &mut ema {
                total += indicator.next(value).unwrap_or(0.0);
            }
            for indicator in &mut rsi {
                total += indicator.next(value).unwrap_or(0.0);
            }
            black_box(total)
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_scanner,
    bench_factor_dag,
    bench_dirty_range,
    bench_streaming,
);
criterion_main!(benches);
