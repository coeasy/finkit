# Core Contracts

This page documents the stable v0.1.15 Rust contracts around the indicator,
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

### The indicator `_into` family is API, and it is pinned

Most allocating indicators in `finkit::indicators` have a caller-owned-buffer
counterpart named `<name>_into`. Treat the whole family as public API:
language bindings and out-of-tree callers bind to it, even though **nothing in
this crate does**. Internal code reaches for the canonical kernels under
`finkit::math::` directly, so `cargo check` cannot notice when one goes
missing, and a snapshot gate such as `docs/generated/indicators.md` will record
the removal as the new baseline rather than object to it. That is not
hypothetical: `indicators::volume::adosc_into` was deleted by a refactor and
every gate stayed green.

`core/tests/indicator_api_surface.rs` therefore names all 53 `_into` paths plus
the root re-export seam, which turns removal or renaming into a compile error.
It is one-directional by design — it constrains deletions, not additions — so
adding an entry point never requires editing it.

Two resolution rules in `core/src/indicators/mod.rs` are easy to get wrong:

* `pub use volume::*` and the explicit
  `pub use finkit::math::volume_kernels::{ad, adosc, obv}` coexist, and **an
  explicit re-export shadows a glob re-export of the same name**. So
  `indicators::ad` and `indicators::adosc` resolve to the kernels, while
  `indicators::volume::ad` and `indicators::volume::adosc` resolve to the
  module wrappers that forward to them. Both paths are public and both are in
  use; do not assume they name the same item.
* Because the shadowing is silent, a name can also disappear as a side effect
  of deleting the glob member it used to come from — which is what happened to
  `adosc_into`. Restoring such a name means defining it in the module or adding
  it to the explicit list, not just deleting the replacement.

The `_into` entry points validate their own pre-conditions and then forward, so
their error semantics are the kernel's. Where the two disagree, the kernel is
the contract: for example `indicators::volume::adosc_into` rejects a zero
`fast_period`/`slow_period` and reports `TaError::InsufficientData` for a series
no longer than `max(fast_period, slow_period) - 1`, because the canonical kernel
validates those cases instead of reading past warm-up.

## NaN is a value, never a panic

A missing value arrives as `NaN` and must propagate as `NaN`; it must never
abort an evaluation. Indicator and pattern kernels therefore order floats with
`f64::total_cmp` — a total order that never returns `None` — or with
`PartialOrd::partial_cmp(...).unwrap_or(Ordering::Equal)`. They must **not** use
`partial_cmp(...).unwrap()`: `f64::partial_cmp` returns `None` whenever either
operand is `NaN`, so the `unwrap` panics on a gap value. This is enforced by
`scripts/check_nan_unsafe_ordering.py` (`make check-nan-safety`), which scans
`core/`, `factor-analysis/`, `visualization/`, `cli/`, `ffi/` and `wasm/`.

## Unified Compute Plan

`finkit::compute` separates semantic planning from numerical execution. A
`ComputePlan` validates a dependency DAG and stores a deterministic topological
order. Each node carries planner-visible capabilities rather than forcing an
optimizer to infer semantics from AST shape.

The key metadata is:

- `LookbackRequirement`: no history, period-based, fixed, or dynamic history.
- `ComputeEffect`: pure computation, variable write, named output, drawing, or
  an opaque stateful operation.
- `ComputeCapabilities`: deterministic, streaming, stateful, lookback, typed
  `dependency` shape, and effect flags.
- `DependencyShape`: `FixedLookback(rows)`, `Expanding`, `Dynamic`,
  `CrossSectional`, or `Global`. Only a proven `FixedLookback` permits dirty-range
  execution; every other shape falls back to a full recompute.
- `ExecutionPolicy`: shared NaN and warm-up policy.
- `ComputeInput`: validated borrowed `MarketFrame` plus execution policy.

```rust
use finkit::compute::{
    ComputeCapabilities, ComputeEffect, ComputeNode, ComputeNodeId, ComputePlan, DependencyShape,
    LookbackRequirement,
};

let pure = ComputeCapabilities {
    deterministic: true,
    streaming: true,
    stateful: false,
    lookback: LookbackRequirement::None,
    dependency: DependencyShape::FixedLookback(0),
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

## Semantic Graph

`finkit::semantic_graph` is the one graph model every frontend lowers into:

```text
Frontend AST  ->  SemanticGraph  ->  ComputePlan
```

`Formula`, `Factor`, `Feature`, and `Composite` are `NodeKind` labels on a node,
not four graph types. That matters beyond tidiness: every graph-level concern —
dependency folding, CSE, level scheduling, artifact identity — then has exactly
one implementation site instead of one per concept.

```rust
use finkit::semantic_graph::{NodeKind, SemanticGraph};

let mut builder = SemanticGraph::builder();
let close = builder.push_leaf(NodeKind::Input, "VARIABLE:CLOSE");
let first = builder.push(NodeKind::Factor, "MA", vec![close], pure.clone());
let second = builder.push(NodeKind::Factor, "MA", vec![close], pure.clone());
let sum = builder.push(NodeKind::Factor, "ADD", vec![first, second], pure);
builder.target(sum);
let graph = builder.build()?;

let outcome = graph.eliminate_common_subexpressions();
assert_eq!(outcome.report.merged, 1); // the two identical `MA` nodes folded
let plan = outcome.graph.lower()?;
```

Contracts worth knowing:

- **`inputs` is an ordered operand list**, not a set. `A - B` and `B - A`, and
  `X + X` versus `X`, are different graphs. Every transformation preserves
  operand order and multiplicity.
- **`NodeKind` is provenance, not semantics.** `content_hash()` deliberately
  excludes it, so a formula and a factor computing the same series share one
  artifact identity and one cached result.
- **Validation is `ComputePlan::compile`.** Unknown operands, duplicate ids,
  empty operations, and cycles are reported with the plan's own error type, so
  there is one DAG validator in the crate rather than two.
- **`dependency_shape(id)` folds the whole upward cone**, not just the node's own
  shape, using the single rule `DependencyShape::combine`. One `Dynamic`,
  `Expanding`, `CrossSectional`, or `Global` node anywhere upstream makes the
  chain — and therefore `can_execute_range()` — refuse range execution.
- **`eliminate_common_subexpressions()` merges only pure, stateless,
  deterministic nodes.** Impure nodes are observable; stateful nodes have their
  state slots keyed by node identity, so merging would renumber them for a
  marginal win. Each refusal is counted in `CseReport`.
- **`levels()` groups; it never reorders.** Flattening the levels keeps the
  graph's topological order within each level, so observable effects stay in
  their declared relative order.

### The production route: `UnifiedRuntime::compile_semantic_graph`

A graph type only the test suite can reach is not part of the product, so the
runtime exposes the whole route — optimize, lower, hot-compile — as one call:

```rust
use finkit::unified_runtime::{GraphOptimization, UnifiedRuntime};

let mut compiled = UnifiedRuntime::compile_semantic_graph(
    &graph,
    dispatcher,
    GraphOptimization::CommonSubexpressions,
)?;
if let Some(report) = compiled.cse() {
    println!("folded {} node(s)", report.merged);
}
let output = compiled.executor_mut().execute(&[&close])?;
```

- **`GraphOptimization::None` is the default.** Optimization is opted into rather
  than silently applied to a caller's plan, so a caller who has not thought about
  it gets exactly what the frontend declared.
- **`cse()` returns `Option<CseReport>`.** `None` means "no pass ran"; a report
  with `merged == 0` means "a pass ran and found nothing to fold". Collapsing the
  two would hide the difference between a frontend that emits clean DAGs and one
  that was never optimized.
- **The graph is not mutated.** The pass returns a rewritten graph and that copy
  is what lowers, so a caller can hold declaration and optimization side by side —
  which is what the numerical-equivalence gate does.
- **The executor keeps its arena and persistent state** across calls, so a scan
  reuses its working set per symbol instead of re-allocating. Drive it through
  `executor_mut()`, or take ownership with `into_executor()`.
- **`dependency()` carries the folded contract.** Folding a duplicate must not
  turn a range-eligible graph into a `Dynamic` one, and the pass must actually
  reduce work rather than only report that it did; both are asserted by
  `the_production_graph_entry_point_matches_the_direct_route`.

**Adoption status.** this section documents the supported route and its contract;
it is not a claim that every frontend already takes it. The formula and factor
frontends still lower their own plans directly (`FormulaComputePlan::compile`,
`FactorPlan::execute_borrowed`) and do not yet route through `SemanticGraph`.

### Derive once: `UnifiedRuntime::compile_semantic_graph_cached`

§20 ranks "stop re-deriving what has already been derived" above every
micro-optimization, and the factor scan is the case it was written for: one
declaration compiled, then run for every symbol in the universe. The cached
entry point shares the compiled plan through a `RuntimeContext` so the
lower-and-compile cost is paid per declaration rather than per symbol:

```rust
use finkit::runtime_context::RuntimeContext;
use finkit::unified_runtime::{GraphOptimization, UnifiedRuntime};

let mut runtime = RuntimeContext::new();
let mut compiled = UnifiedRuntime::compile_semantic_graph_cached(
    &graph,
    dispatcher,
    GraphOptimization::CommonSubexpressions,
    &mut runtime,
)?;
// Every later call with the same declaration is a cache hit.
assert_eq!(runtime.cache().stats().hits, 1);
```

- **What is cached is the plan, not the executor.** `CompiledPlanArtifact` holds
  the primitive plan plus its CSE report and dependency shape — all plain data.
  The executor is built per call because it owns a buffer arena and persistent
  kernel state; sharing one across symbols would let the second symbol overwrite
  the first one's working set.
- **The cache is keyed on the declaration, not on the optimized graph.** Running
  CSE merely to decide whether to run CSE would defeat the purpose, so the
  optimization setting selects a namespace instead
  (`FINPLAN0` as-declared / `FINPLAN1` common-subexpressions). As-declared and
  optimized plans therefore never answer each other's requests.
- **`ArtifactCache` is the one cache the runtime context owns.** Formula, factor,
  composite and research planners had each grown their own key type and eviction
  rule, so "is this already compiled?" had a different answer in every layer.
  This is the shared answer, bounded and LRU.
- **A failed build is not cached**, so a graph corrected after a
  `GraphPlanError` is not poisoned by the earlier failure.
- **Reachability is asserted, not assumed.** `check_orphan_modules.py` reported
  the whole semantic layer as test-only before a production entry point existed;
  `recompiling_the_same_declaration_is_served_from_the_artifact_cache` now fails
  if nothing writes to the cache, and
  `the_plan_cache_separates_optimization_settings` fails if the two namespaces
  ever collapse into one.

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

## Factor Engine, borrowed runtime, and FactorPlan

`FactorRegistry` stores named factor definitions and their dependencies.
`FactorEngine` preserves the existing owned `FactorContext` API while sharing
one internal raw-context abstraction with the new `BorrowedFactorContext`.
Custom `FactorFn` callbacks still receive `FactorInputs` and therefore do not
need to change when callers move from owned to borrowed market data.

The owned path remains available:

```rust
use finkit::factors::{builtin_factor_registry, FactorContext, FactorEngine};

let context = FactorContext::new()
    .with_series("close", vec![100.0, 101.0, 103.0, 104.0])?;
let engine = FactorEngine::new(builtin_factor_registry());
let momentum = engine.evaluate("momentum_5", &context)?;
assert_eq!(momentum.len(), context.len());
```

For an existing `MarketFrame`, use `BorrowedFactorContext::from_market_frame`.
The OHLCV and optional amount columns are borrowed as `&[f64]`; no second
numeric `Vec<f64>` copy is required just to enter the factor engine. Timestamps
are intentionally excluded because `MarketFrame` stores them as `i64`, while
raw factor series use `f64`.

```rust
use finkit::factors::{
    builtin_factor_registry, BorrowedFactorContext, FactorEngine,
};
use finkit::runtime::MarketFrame;

let open = [9.5, 10.0, 10.5];
let high = [10.2, 10.7, 11.2];
let low = [9.0, 9.8, 10.0];
let close = [10.0, 10.5, 11.0];
let volume = [100.0, 120.0, 140.0];
let frame = MarketFrame::new(&open, &high, &low, &close, &volume)?;
let context = BorrowedFactorContext::from_market_frame(frame)?;
let engine = FactorEngine::new(builtin_factor_registry());
let momentum = engine.evaluate_borrowed("momentum_5", &context)?;
assert_eq!(context.get("close").expect("close").as_ptr(), close.as_ptr());
assert_eq!(momentum.len(), close.len());
```

`FactorPlan` moves graph discovery into an explicit compile phase. It validates
targets, resolves dependencies, detects cycles, produces a stable dependency-first
execution order, and records the required raw-series manifest before numerical
execution starts.

```rust
use finkit::compute::FactorPlan;
use finkit::factors::{
    builtin_factor_registry, BorrowedFactorContext, FactorEngine,
};

let registry = builtin_factor_registry();
let plan = FactorPlan::compile(&registry, &["reversal_5"])?;
let engine = FactorEngine::new(registry);
let close = [100.0, 101.0, 103.0, 104.0, 105.0, 106.0];
let context = BorrowedFactorContext::new().with_series("close", &close)?;
let values = plan.execute_borrowed(&engine, &context)?;
assert!(values.contains_key("momentum_5"));
assert!(values.contains_key("reversal_5"));
```

The optimized plan path now executes directly in the precompiled topological
order instead of recursively rediscovering the dependency graph on every call.
Use `execute_precompiled` for owned input and `execute_borrowed` for borrowed
input. Both validate the raw-input manifest and reject stale plans when the
registry dependency graph no longer matches the graph that was compiled.

`FactorPlan::execute` keeps its existing source signature but now validates the
context/registry and delegates to the precompiled topology, so existing callers
receive the optimized path without migrating API calls. `execute_precompiled`
remains available when callers want to state that execution mode explicitly.

The unified `OperationRequest::Factor` path can additionally carry an explicit
`data_revision` and `cache_scope`. When both are supplied, the
`UnifiedOperationEngine` uses the same bounded LRU result-cache contract as
Composite execution; the revision must advance whenever any input affecting
the Factor changes, and the scope isolates symbols/timeframes. Omitting the
revision deliberately disables result caching while retaining compiled-plan
reuse, so callers cannot accidentally reuse a result for mutable input data.
This makes Factor and Composite cache identity explicit without reintroducing
string-based Factory creation in the hot path.

Built-in factors remain deliberately small reference factors. Register custom
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
without letting idle retained memory grow without bound. Backend-wide arena
integration remains incremental; the presence of the shared type is not a claim
that every existing indicator/formula implementation already uses it.

### Persistent output: `execute_into`

`UnifiedExecutor::execute` moves the retained result buffers *out* of the arena,
because the caller owns the results. That leaves a zero-length `Vec` in each
output slot, and `BufferArena::recycle` deliberately refuses those, so the next
execution has to allocate a replacement for every retained output — one
allocation per output per execution, which in a scan loop is one per symbol.

`UnifiedExecutor::execute_into` closes that gap: it runs the same plan, copies
each retained output into storage the caller already owns, and hands the arena
buffers straight back.

```rust
use finkit::unified_executor::UnifiedExecutor;

let mut destination = vec![0.0; rows];
let mut destinations: [&mut [f64]; 1] = [destination.as_mut_slice()];
executor.execute_into(&[&close], &mut destinations)?;
// Every buffer, outputs included, is now back in the arena: a scan loop that
// keeps this destination performs zero arena allocations after the first run.
```

The trade is one `memcpy` of `rows` `f64` per output per execution against an
allocation of the same size — and unlike the allocation, the copy is flat in the
number of symbols scanned.

Both the destination count and the destination length are validated **before the
plan runs**, so a rejected call has no side effects at all. That ordering is the
contract, not an implementation detail: `execute` drives persistent kernel state,
so a length check placed after the run would leave a rejected executor one step
ahead of its caller, and a retry with a corrected destination would compute the
second sample's value and return it as the first.
`a_rejected_execute_into_leaves_the_executor_untouched` asserts it through
`RuntimeMetrics`, whose counters only advance on the success path.

## Formula optimizer equivalence contract

The integration suite compares raw AST execution against the normal
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

Only Finkit's own language is advertised as `Native` in v0.1.15. TongDaXin,
TongHuaShun, EastMoney, and TradingView/Pine adapters are explicitly
`CommonSubset` contracts. Golden fixtures under `core/tests/fixtures/formula_compat`
exercise representative parser/semantic behavior for each external terminal.

The adapters normalize transport artifacts such as UTF-8 BOM and line endings.
They do not claim full semantic compatibility with every external terminal;
terminal-specific syntax remains an explicit future extension point.
