# Finkit — Formula engine internals

The formula engine compiles `MA(CLOSE, 20)`-style source into an executable
compute plan. **The tree-walking interpreter is the current default backend and
the reference implementation; the compiled-plan path is opt-in**
(`FormulaExecutionMode::Plan`). Both are real production paths and neither is
the other's fallback — see [Execution backends](#execution-backends) below.

> This page describes the *pipeline internals*. The observable execution
> contract (ownership, warm-up, range/last/append, multi-path agreement and the
> `BackendUnsupported` matrix) lives in
> [Formula runtime contract](../formula-runtime-contract.md). The numerical
> policy behind every kernel these stages call lives in
> [Numerical semantics](numerical-semantics.md). The cache contract lives in
> [Runtime and cache contract](runtime-and-cache-contract.md).

## Frozen surfaces (decided 2026-09-21)

Two legacy entry points are **frozen**: they stay (four language bindings
export them as public API) but neither grows, and `eval()` never routes
into them:

- `formula-jit` / `FormulaEngine::eval_jit` — peephole-optimised bytecode
  interpretation. Despite the name there is **no native codegen**.
- `formula-simd` / `FormulaEngine::eval_simd` — an *exact alias* of
  `eval`. There is no SIMD formula-evaluation path behind this feature;
  real SIMD lives in the numeric kernels (`math::simd_kernels`) on the
  normal execution path. (`eval_simd` therefore inherits whatever backend
  the engine is in — it does **not** pin itself to the tree.)

There is **no hot-loop detection and no iteration-count promotion** —
earlier descriptions of a "hot loop ≥ 1M → JIT" pipeline were aspirational
and never matched the code.

## Execution backends

`FormulaExecutionMode` selects the backend for the `eval*` entry points and is a
**backend contract, not a hint**: an entry the selected backend cannot serve
fails with `FormulaError::BackendUnsupported` rather than silently running the
other one.

| | `Tree` (default) | `Plan` (opt-in) |
|---|---|---|
| Coverage | everything | only what lowers to a compute plan |
| Failure | non-evaluable nodes error at runtime | unsupported formulas fail at **compile time** |
| `ctx.variables` / `ctx.output_names` / `ctx.output_modifiers` | written back | written back |
| Warm-up / streaming state | full | single-shot batch |

`Plan` is **not** the default and flipping it is a deliberate release decision
gated on coverage, numerical parity and a version-migration plan — it is not a
pending "finish the kernels" task. The historical kernel gap was closed on
2026-09-24 (`unified_dispatch.rs` routes every TA-Lib `CALL:<NAME>` kernel
through `dispatch_modern_call`); the remaining structural blocker is that a
formula containing a **string literal** cannot run under `Plan` at all, because
the tree path appends the literal to `FormulaContext::string_table` and the plan
executor receives only `&[&[f64]]` with no context to append to.

### Entry-point matrix

The split below is exhaustive and machine-checked by
`core/tests/formula_execution_mode.rs`; the tree-only set is exactly the set of
`require_tree_backend(..)` call sites in `core/src/formula/engine.rs`.

| Family | Entries | Under `Plan` |
|---|---|---|
| Governed, plan-capable | `eval`, `eval_with_dialect`, `eval_with_params`, `eval_multi`, `eval_multi_with_dialect`, `eval_incremental`, `eval_simd`, `eval_batch`, `eval_batch_shared` | run on the plan backend (the plan cache is populated) |
| Governed, tree-only | `eval_ast`, `eval_lazy`, `eval_parallel`, `eval_optimized`, `eval_with_debug`, `eval_template`, `eval_with_validation`, `eval_with_defaults`, `eval_zero_copy`, `eval_zero_copy_cached`, `eval_zero_alloc`, `eval_multi_with_dialect(Pine)`, `eval_multi_with_pine_security` | `FormulaError::BackendUnsupported { backend: "plan", entry }` |
| Backend-explicit (never governed) | `eval_plan*`, `compile_plan*`, `eval_jit`, `compile_jit`, `execute_jit`, `compile_bytecode`, `execute_bytecode`, and every entry taking a `&CompiledFormula` | runs the backend its argument type declares |

Two rules follow:

1. A *source-level* entry never silently runs a backend the caller did not
   select; a tree-only one names itself and the selected backend in the error.
2. An entry whose argument is already a tree artifact (`CompiledFormula`,
   `AstNode`, `Bytecode`) is backend-explicit: building that artifact **is** the
   backend choice. Its plan counterpart is `compile_plan` / `eval_plan`.

## Pipeline

```mermaid
flowchart TB
  src[Source string<br/>MA(CLOSE, 20)]
  src -->|pest| tokens[Tokens]
  tokens -->|pest| ast[AST]
  ast -->|constant fold + CSE + DCE| opt[Optimised AST]
  opt -->|lower| graph[SemanticGraph]
  graph --> plan[ComputePlan]
  opt -->|default backend| tree[Tree interpreter<br/>reference path]
  plan -->|eval_plan / Plan mode| exec[Plan executor<br/>UnifiedExecutor]
  tree --> out[Array1 result]
  exec --> out
```

The two right-hand branches are peers. The tree interpreter is the default and
the reference every differential gate compares against; the compiled plan
(`ComputePlan` / `FormulaHotPlan` → `UnifiedExecutor`) is the opt-in production
carrier.

## Stages

1. **Parse** (`formula/parser.rs`): pest-generated PEG grammar in
   `formula/grammar.pest`. Produces a `pest::Pair` tree. Input budgets are
   enforced here: source size (1 MiB), bracket nesting (256) and —
   iteratively, after the parse — AST depth (1024). Breaching any of them
   is a typed parse error, not a crash; rejected deep trees are torn down
   iteratively so their drop cannot overflow the stack either.
2. **AST** (`formula/ast.rs`): typed nodes (`Call`, `Ident`, `Literal`,
   `Ref`, `Binary`, `Unary`).
3. **Optimise** (`formula/optimizer.rs`): constant folding, common
   subexpression elimination, dead-code elimination, type-specialisation
   (`MA(CLOSE, 20)` → `sma_inplace`).
4. **Lower** (`formula/compute_ir.rs`): the AST is lowered into a
   `SemanticGraph` (validation, CSE, layering) and compiled into a
   `ComputePlan` — the reusable production artifact reached only through the
   plan backend. Lowering walks the tree recursively, so the depth is checked
   *while* lowering (`MAX_LOWER_DEPTH`); a too-deep AST fails with the typed
   `ComputePlanError::LoweringDepthExceeded` instead of exhausting the stack. A
   formula the plan path cannot lower is a hard error; it does **not** fall back
   to the tree.
5. **Execute** (`formula/executor.rs`, `formula/unified_dispatch.rs`): the
   plan executor evaluates the compiled graph; the tree interpreter
   (`execute_val`) remains the default and the reference path every
   differential gate compares against. Frozen legacy modes: bytecode VM
   dispatch (`formula/bytecode.rs`) and the frozen `eval_jit` entry point.

## Memory pool

The engine uses a per-thread `MemoryPool` (ADR-0008) keyed by the formula
hash. Intermediate `Array1<f64>` allocations are reused per logical length.

The pool is **bounded, not LRU**: each length retains at most
`max_buffers_per_size` buffers, and once that cap is reached the pool simply
stops retaining recycled buffers instead of evicting a live one. There is no
working-set LRU at this layer — see `formula/memory_pool.rs`.

## Caching

The engine holds **several bounded compile-artifact caches, not one LRU cache**.
They are all bounded, but their keys and eviction policies differ on purpose
(a semantic DAG, a dialect/parameter-sensitive numeric plan and an AST artifact
are not interchangeable), so they were not merged into a single container.

| Cache | Field | Key | Policy | Capacity |
|---|---|---|---|---|
| AST / compiled formula | `cache` (`FormulaCache`) | source hash + features | **LRU** | 100 (`with_cache_size`) |
| Semantic plan | `semantic_plan_cache` | exact formula source | FIFO | `ENGINE_CACHE_CAPACITY` = 1024 |
| Hot numeric plan | `plan_cache` (`FormulaPlanCache`) | source + dialect + param fingerprint | FIFO | `ENGINE_CACHE_CAPACITY` = 1024 |
| Bytecode | `bytecode_cache` | source + feature set | FIFO | `ENGINE_CACHE_CAPACITY` = 1024 |

The tree backend reads the AST cache; the plan backend reads `plan_cache`
(and reports it through `plan_cache_size()`). `cache_hit` / `cache_size` report
the cache of the **active** mode, and `clear_cache()` drops the plan cache even
from `Tree` mode, so a caller cannot leave a stale compiled plan behind. The
cross-layer `RuntimeContext::ArtifactCache` (LRU) is owned by the unified
runtime, not the formula engine — the two are bounded but do not share keys or
lifecycles. See [Runtime and cache contract](runtime-and-cache-contract.md) for
the full contract.

## Error model

| Stage   | Error type         | Recovery                |
|---------|--------------------|-------------------------|
| Parse   | `FormulaError::Parse` | Hard fail; return Err |
| Type    | `FormulaError::TypeMismatch` | Hard fail; return Err |
| Backend | `FormulaError::BackendUnsupported` | Hard fail; switch backend or use a plan-capable entry |
| Run     | `FormulaError::RuntimeError` | Per-call; `eval_partial` |

See [Formula runtime contract](../formula-runtime-contract.md) for the runtime execution contract.

## Cross-references

- [Overview](overview.md)
- [Data flow](dataflow.md)
- [Numerical semantics](numerical-semantics.md)
- [Runtime and cache contract](runtime-and-cache-contract.md)
- [api-reference.md](../api-reference.md) (English) · [api-reference-zh.md](../api-reference-zh.md) (中文)
