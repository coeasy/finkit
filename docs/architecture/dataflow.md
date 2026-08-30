# Finkit — Data flow

End-to-end data flow for the two primary usage patterns.

## Backtest (batch)

```mermaid
sequenceDiagram
  participant U as User code
  participant C as finkit::indicators
  participant M as finkit::math
  participant S as finkit::streaming (optional)

  U->>C: sma(&close, 20)
  C->>M: moving_avg::sma_simd_avx2(close, 20)
  M-->>C: Vec<f64>
  C-->>U: Array1<f64>
```

## Live trading (streaming)

```mermaid
sequenceDiagram
  participant F as Feed
  participant U as User code
  participant S as StreamingSma
  participant CH as Checkpoint (optional)

  loop every bar
    F->>U: new OHLCV bar
    U->>S: next(&bar)
    S-->>U: Option<f64>
    U->>U: act on signal
  end

  U->>CH: save_state()
  CH-->>U: bytes
  Note over U,CH: every N bars or on shutdown

  F->>U: process restart
  U->>CH: load_state(bytes) / restore_or_recompute(data)
  CH-->>U: StreamingSma (recovered)
```

## Formula engine

```mermaid
flowchart LR
  src[Formula source] --> p[pest parser]
  p --> ast[AST]
  ast --> opt[Optimizer]
  opt --> bc[Bytecode]
  ast -.-> jit[JIT]
  bc --> vm[VM]
  jit --> vm
  vm -.-> simd[SIMD paths]
  vm --> out[Array1<f64>]
```

## Error propagation

```mermaid
flowchart LR
  ind[IndicatorError] --> ta[TaError]
  form[FormulaError] --> ta
  ffi[FfiError] --> ta
  ta --> user[user code via ?]
```

## Cross-references

- [Overview](overview.md)
- [Formula engine](formula-engine.md)
