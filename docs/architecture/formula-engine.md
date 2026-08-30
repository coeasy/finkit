# Finkit — Formula engine internals

The formula engine implements a 4-stage pipeline that compiles the
`MA(CLOSE, 20)` style source into native code paths, with on-the-fly
specialisation for hot loops.

## Pipeline

```mermaid
flowchart TB
  src[Source string<br/>MA(CLOSE, 20)]
  src -->|pest| tokens[Tokens]
  tokens -->|pest| ast[AST]
  ast -->|constant fold + DCE| opt[Optimised AST]
  opt -->|bytecode| bc[Bytecode]
  opt -.->|JIT| jit[Native code]
  bc --> vm[VM dispatch]
  jit --> vm
  vm -->|hot loop detection| simd[SIMD path]
  vm --> out[Array1 result]
```

## Stages

1. **Parse** (`formula/parser.rs`): pest-generated PEG grammar in
   `formula/grammar.pest`. Produces a `pest::Pair` tree.
2. **AST** (`formula/ast.rs`): typed nodes (`Call`, `Ident`, `Literal`,
   `Ref`, `Binary`, `Unary`).
3. **Optimise** (`formula/optimizer.rs`): constant folding, common
   subexpression elimination, dead-code elimination, type-specialisation
   (`MA(CLOSE, 20)` → `sma_inplace`).
4. **Codegen** (`formula/bytecode.rs`, `formula/jit.rs`): produces
   bytecode for the VM. Hot loops (≥ 1M iterations) are promoted to JIT.
5. **Execute** (`formula/executor.rs`): VM dispatch with inline caches.
   The first N iterations of any inner loop are profiled; if they pass
   the `HOT_LOOP_THRESHOLD`, the loop is converted to a SIMD path
   (`formula/simd.rs`).

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

See [ADR-0009](../adr/0009-error-recovery-strategy.md) for the rationale.

## Cross-references

- [Overview](overview.md)
- [Data flow](dataflow.md)
- [api-reference.md](../api-reference.md) (English) · [api-reference-zh.md](../api-reference-zh.md) (中文)
