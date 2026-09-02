# Core Contracts

This page documents the stable v0.1.2 Rust contracts around the indicator,
formula, factor, and runtime engines. They are intentionally data-source
agnostic and do not turn Finkit into a data or trading platform.

## Runtime and zero-copy boundaries

`MarketFrame` validates aligned OHLCV columns and optionally carries amount and
timestamps. `MarketFrame::series` resolves common `O/H/L/C/V` and terminal aliases
without allocating a temporary uppercase string.

`SeriesView::normalized_cow` returns a borrowed slice for `Preserve`, for
finite `Error` input, and for `ForwardFill` when no values need changing. It
returns an owned buffer only when a missing value actually requires
forward-filling. Use `normalized` when an owned `Vec<f64>` is required.

```rust
use finkit::runtime::{MarketFrame, NanPolicy};

let open = [9.5, 10.0, 10.5];
let high = [10.2, 10.7, 11.2];
let low = [9.0, 9.8, 10.0];
let close = [10.0, 10.5, 11.0];
let volume = [100.0, 120.0, 140.0];
let amount = [1_000.0, 1_200.0, 1_400.0];
let frame = MarketFrame::new(&open, &high, &low, &close, &volume)?
    .with_amount(&amount)?;
let close_view = frame.series(" CLOSE ").expect("close column");
let borrowed = close_view.normalized_cow(NanPolicy::Preserve)?;
assert_eq!(borrowed.as_ref(), &close);
```

`WarmupPolicy::Nan` preserves row alignment by filling the lookback prefix
with `NaN`; `WarmupPolicy::Trim` returns only stable rows. `NanPolicy::Error`
rejects non-finite numeric fields before execution.

## Unified Compute Plan

`finkit::compute` separates semantic planning from numerical execution. A
`ComputePlan` validates a dependency DAG and stores a deterministic topological
order. Each node carries planner-visible capabilities rather than forcing an
optimizer to infer semantics from AST shape.

The key metadata is:

- `LookbackRequirement`: no history, period-based, fixed, or dynamic history.
- `ComputeEffect`: pure computation, variable write, named output, drawing, or
  an opaque stateful operation.
- `ComputeCapabilities`: deterministic, streaming, stateful, lookback, and
  effect flags.
- `ExecutionPolicy`: shared NaN and warm-up policy.
- `ComputeInput`: validated borrowed `MarketFrame` plus execution policy.

```rust
use finkit::compute::{
    ComputeCapabilities, ComputeEffect, ComputeNode, ComputeNodeId, ComputePlan,
    LookbackRequirement,
};

let pure = ComputeCapabilities {
    deterministic: true,
    streaming: true,
    stateful: false,
    lookback: LookbackRequirement::None,
    effect: ComputeEffect::Pure,
};
let plan = ComputePlan::compile([
    ComputeNode::new(ComputeNodeId(0), "CLOSE", vec![], pure.clone()),
    ComputeNode::new(ComputeNodeId(1), "MA", vec![ComputeNodeId(0)], pure),
])?;
assert_eq!(plan.execution_order(), &[ComputeNodeId(0), ComputeNodeId(1)]);
```

Compilation rejects duplicate node ids, unknown dependencies, empty operation
names, and dependency cycles. Duplicate dependency edges are normalized before
topological sorting.

This is intentionally an execution-neutral IR foundation. Batch, streaming,
SIMD, bytecode, factor, and future JIT backends can consume the same semantic
metadata without changing user-facing numerical APIs.

## Formula AST to Compute IR

`FormulaComputePlan` lowers the existing formula AST into the unified compute
plan. This directly protects formula semantics that are observable through
`FormulaContext`:

- `:=` is `ComputeEffect::WriteVariable`.
- named output `:` is `ComputeEffect::EmitOutput` and also becomes the latest
  data dependency for later reads of that name.
- drawing statements are `ComputeEffect::Draw`.
- unknown/custom functions are conservatively treated as stateful and dynamic
  until they are registered in the canonical function registry.
- string literals are conservatively stateful because the current executor
  interns them into the context string table.
- `FOR`/`WHILE` are opaque stateful barriers until a future CFG/SSA layer can
  represent loop-carried dependencies safely.

```rust
use finkit::formula::{parse_formula, FormulaComputePlan};

let ast = parse_formula("MA5:=MA(CLOSE,5);SELL:CROSS(CLOSE,MA5);")?;
let formula_plan = FormulaComputePlan::compile(&ast)?;
assert!(formula_plan.plan().has_observable_effects());
```

A later `MA5` read is linked to the latest `MA5` assignment node. Effectful
nodes are also chained in source execution order. This means future DCE/CSE or
backend lowering no longer has to guess whether an apparently unused statement
is externally observable.

## Factor Engine and FactorPlan

`FactorRegistry` stores named factor definitions and their dependencies.
`FactorEngine` evaluates dependencies once per request, detects cycles, checks
aligned output lengths, and supports weighted composite scores.

```rust
use finkit::factors::{builtin_factor_registry, FactorContext, FactorEngine};

let context = FactorContext::new()
    .with_series("close", vec![100.0, 101.0, 103.0, 104.0])?;
let engine = FactorEngine::new(builtin_factor_registry());
let momentum = engine.evaluate("momentum_5", &context)?;
assert_eq!(momentum.len(), context.len());
```

`FactorPlan` moves graph discovery into an explicit compile phase. It validates
targets, resolves factor dependencies, detects cycles, produces a stable
dependency-first order, and records the required raw-series manifest before
numerical execution starts.

```rust
use finkit::compute::FactorPlan;
use finkit::factors::{builtin_factor_registry, FactorEngine};

let registry = builtin_factor_registry();
let plan = FactorPlan::compile(&registry, &["momentum_5"])?;
let engine = FactorEngine::new(registry);
let values = plan.execute(&engine, &context)?;
assert!(values.contains_key("momentum_5"));
```

The first v0.1.2 implementation deliberately delegates numerical evaluation to
the existing `FactorEngine`, preserving established results while creating a
stable planning seam. A plan detects if factor dependencies have changed since
it was compiled instead of silently executing stale graph assumptions.

Built-in factors are deliberately small reference factors. Register custom
factors with `FactorDefinition::new` and return one value per context row. Factor
and dependency names must be non-empty, and every computed result must remain
aligned with the context row count.

## Function metadata registry and canonical API schema

`builtin_function_registry` provides deterministic discovery metadata for
common indicator and formula functions, including aliases such as `MA`,
`BOLL`, `SHIFT`, `CROSSOVER`, and `IFF`.

```rust
use finkit::registry::builtin_function_registry;

let registry = builtin_function_registry();
let sma = registry.get("ma").expect("SMA metadata");
assert_eq!(sma.name, "SMA");
assert!(sma.streaming);
```

The registry rejects empty names, alias/canonical collisions, duplicate aliases,
and aliases already used by another function. Canonical iteration order is
stable.

`finkit::schema::FunctionApiSchema` turns registry entries into owned,
machine-readable metadata for CLI/docs/binding generation. The schema keeps a
contract version (`finkit.function.v1`) separate from the package version and
includes aliases, category, input kind, parameters, output count, lookback,
streaming/deterministic/stateful flags, and effect class.

```rust
use finkit::schema::FunctionApiSchema;

let schema = FunctionApiSchema::builtin();
let sma = schema.get("SMA").expect("SMA schema");
assert_eq!(sma.effect, "pure");
assert_eq!(sma.lookback, "period_minus_one");
```

This schema is the migration path away from manually duplicating defaults and
capabilities across Python, Node, Java, C#, Go, C headers, CLI help, and docs.

## Shared Buffer Arena

`finkit::buffer_arena::BufferArena` is a bounded reusable `Vec<f64>` scratch
pool intended for compute backends. It uses logical series length as the reuse
key, initializes every checked-out buffer, and enforces both per-length count
and global retained-byte limits.

```rust
use finkit::buffer_arena::BufferArena;

let mut arena = BufferArena::default();
let mut scratch = arena.take_filled(1024, f64::NAN);
// use scratch as a temporary result
scratch[0] = 1.0;
arena.recycle(scratch);
assert_eq!(arena.stats().cached_buffers, 1);
```

The arena does not replace caller-owned `_into` output APIs. Its purpose is to
reuse unavoidable intermediate memory across Formula/Factor/Batch planners
without letting idle retained memory grow without bound.

## Formula optimizer equivalence contract

The integration suite now compares raw AST execution against the normal
execution-optimized compile path for formulas with multiple assignments and
named outputs. It verifies both the final numeric result and every observable
`FormulaContext::variables` entry.

This regression contract exists specifically because ordinary compiler DCE is
not valid for formula statements whose assignments/outputs remain observable
after execution.

## Formula terminal compatibility

The formula module keeps one canonical parser/runtime while routing terminal
names through compatibility metadata:

```rust
use finkit::formula::{parse_formula_for_terminal, FormulaTerminal};

let ast = parse_formula_for_terminal(
    "MA5:=MA(CLOSE,5); CROSS(CLOSE,MA5);",
    FormulaTerminal::TongDaXin,
)?;
```

The v0.1.2 adapter normalizes transport artifacts such as UTF-8 BOM and line
endings. It does not claim full semantic compatibility with every terminal;
terminal-specific syntax remains an explicit future extension point.
