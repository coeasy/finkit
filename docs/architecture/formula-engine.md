# Finkit — Formula engine internals

The formula engine compiles `MA(CLOSE, 20)`-style source into an executable
compute plan. **Production execution goes through the compiled-plan path;
the tree-walking interpreter is the reference implementation.**

## Frozen surfaces (decided 2026-09-21)

Two legacy entry points are **frozen**: they stay (four language bindings
export them as public API) but neither grows, and `eval()` never routes
into them:

- `formula-jit` / `FormulaEngine::eval_jit` — peephole-optimised bytecode
  interpretation. Despite the name there is **no native codegen**.
- `formula-simd` / `FormulaEngine::eval_simd` — an *exact alias* of
  `eval`. There is no SIMD formula-evaluation path behind this feature;
  real SIMD lives in the numeric kernels (`math::simd_kernels`) on the
  normal execution path.

There is **no hot-loop detection and no iteration-count promotion** —
earlier descriptions of a "hot loop ≥ 1M → JIT" pipeline were aspirational
and never matched the code.

## Pipeline

```mermaid
flowchart TB
  src[Source string<br/>MA(CLOSE, 20)]
  src -->|pest| tokens[Tokens]
  tokens -->|pest| ast[AST]
  ast -->|constant fold + CSE + DCE| opt[Optimised AST]
  opt -->|lower| graph[SemanticGraph]
  graph --> plan[ComputePlan]
  opt --> tree[Tree interpreter<br/>reference path]
  plan --> exec[Plan executor]
  tree --> out[Array1 result]
  exec --> out
```

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
   `ComputePlan` — the reusable production artifact. Lowering walks the
   tree recursively, so the depth is checked *while* lowering
   (`MAX_LOWER_DEPTH`); a too-deep AST fails with the typed
   `ComputePlanError::LoweringDepthExceeded` instead of exhausting the
   stack.
5. **Execute** (`formula/executor.rs`, `formula/unified_dispatch.rs`): the
   plan executor evaluates the compiled graph; the tree interpreter
   (`execute_val`) remains the reference path every differential gate
   compares against. Frozen legacy modes: bytecode VM dispatch
   (`formula/bytecode.rs`) and the frozen `eval_jit` entry point.

## Memory pool

The engine uses a per-thread `MemoryPool` (ADR-0008) keyed by the formula
hash. Intermediate `Array1<f64>` allocations are reused; LRU eviction
bounds the working set.

## Caching

A `lru` cache (formula source → CompiledFormula) gives ~23× speedup for
repeat evaluations. Cache key includes the source string + feature set
hash.

## Error model

| Stage   | Error type         | Recovery                |
|---------|--------------------|-------------------------|
| Parse   | `FormulaError::Parse` | Hard fail; return Err |
| Type    | `FormulaError::TypeMismatch` | Hard fail; return Err |
| Run     | `FormulaError::RuntimeError` | Per-call; `eval_partial` |

See [Formula runtime contract](../formula-runtime-contract.md) for the runtime execution contract.

## Cross-references

- [Overview](overview.md)
- [Data flow](dataflow.md)
- [api-reference.md](../api-reference.md) (English) · [api-reference-zh.md](../api-reference-zh.md) (中文)
