//! Production-path baselines for the unified Factor/Composite execution API.
//!
//! These are evidence-producing benchmarks, not universal SLO claims. They
//! intentionally keep the graph and plan stable while changing the data
//! revision so plan-cache reuse is measured independently from result-cache
//! reuse.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use finkit::composite::{CompositeDefinition, CompositeExpr, CompositeOp};
use finkit::factors::{
    BorrowedFactorContext, FactorDefinition, FactorDirection, FactorKind, FactorRegistry,
};
use finkit::operation::{OperationRequest, UnifiedOperationEngine};
use std::sync::Arc;

const DATA_LEN: usize = 100_000;

fn close_series() -> Vec<f64> {
    (0..DATA_LEN)
        .map(|index| {
            let t = index as f64;
            100.0 + t * 0.01 + (t * 0.37).sin() * 2.0 + (t * 1.13).cos()
        })
        .collect()
}

fn factor_engine() -> UnifiedOperationEngine {
    let mut registry = FactorRegistry::new();
    registry
        .register(FactorDefinition::new(
            "DOUBLE_CLOSE",
            ["close"],
            FactorKind::TimeSeries,
            FactorDirection::Neutral,
            Arc::new(|inputs| {
                Ok(inputs
                    .get("close")?
                    .iter()
                    .map(|value| value * 2.0)
                    .collect())
            }),
        ))
        .expect("register benchmark factor");
    UnifiedOperationEngine::new(registry)
}

fn context(close: &[f64]) -> BorrowedFactorContext<'_> {
    BorrowedFactorContext::new()
        .with_series("close", close)
        .expect("create borrowed benchmark context")
}

fn composite_definitions() -> [CompositeDefinition; 1] {
    [CompositeDefinition::new(
        "SUM_CLOSE",
        CompositeExpr::Op {
            op: CompositeOp::Add,
            inputs: vec![CompositeExpr::series("close"), CompositeExpr::Constant(1.0)],
        },
    )]
}

fn bench_factor_plan_reuse(c: &mut Criterion) {
    let close = close_series();
    let context = context(&close);
    let mut engine = factor_engine();
    let request = || OperationRequest::Factor {
        name: "DOUBLE_CLOSE",
        context: &context,
    };
    engine.execute(request()).expect("warm factor plan");

    let mut group = c.benchmark_group("unified_factor");
    group.throughput(Throughput::Elements(DATA_LEN as u64));
    group.bench_function("plan_cached_borrowed_100k", |b| {
        b.iter(|| black_box(engine.execute(request()).expect("factor execution")))
    });
    group.finish();
}

fn bench_composite_plan_reuse(c: &mut Criterion) {
    let close = close_series();
    let context = context(&close);
    let definitions = composite_definitions();
    let outputs = ["SUM_CLOSE"];
    let mut engine = factor_engine();
    engine
        .execute(OperationRequest::Composite {
            definitions: &definitions,
            outputs: &outputs,
            context: &context,
            data_revision: Some(0),
            cache_scope: Some("BENCH@1d"),
        })
        .expect("warm composite plan");

    let mut revision = 1u64;
    let mut group = c.benchmark_group("unified_composite");
    group.throughput(Throughput::Elements(DATA_LEN as u64));
    group.bench_function("plan_cached_result_miss_100k", |b| {
        b.iter(|| {
            let data_revision = revision;
            revision = revision.wrapping_add(1);
            black_box(
                engine
                    .execute(OperationRequest::Composite {
                        definitions: &definitions,
                        outputs: &outputs,
                        context: &context,
                        data_revision: Some(data_revision),
                        cache_scope: Some("BENCH@1d"),
                    })
                    .expect("composite execution"),
            )
        })
    });
    group.bench_function("result_cached_100k", |b| {
        b.iter(|| {
            black_box(
                engine
                    .execute(OperationRequest::Composite {
                        definitions: &definitions,
                        outputs: &outputs,
                        context: &context,
                        data_revision: Some(0),
                        cache_scope: Some("BENCH@1d"),
                    })
                    .expect("cached composite execution"),
            )
        })
    });
    group.finish();
}

criterion_group!(
    unified_engine_benches,
    bench_factor_plan_reuse,
    bench_composite_plan_reuse
);
criterion_main!(unified_engine_benches);
