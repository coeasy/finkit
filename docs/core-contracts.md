# Core Contracts

This page documents the stable v0.1.2 Rust contracts added around the
indicator and formula engines. They are intentionally data-source agnostic and
do not replace the existing indicator executors.

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

let close = [10.0, 10.5, 11.0];
let frame = MarketFrame::new(
    &[9.5, 10.0, 10.5],
    &[10.2, 10.7, 11.2],
    &[9.0, 9.8, 10.0],
    &close,
    &[100.0, 120.0, 140.0],
)?;
let close_view = frame.series(" CLOSE ").expect("close column");
let borrowed = close_view.normalized_cow(NanPolicy::Preserve)?;
assert_eq!(borrowed.as_ref(), &close);
```

`WarmupPolicy::Nan` preserves row alignment by filling the lookback prefix
with `NaN`; `WarmupPolicy::Trim` returns only stable rows. `NanPolicy::Error`
rejects non-finite numeric fields before execution.

## Factor Engine

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

Built-in factors are deliberately small reference factors. Register custom
factors with `FactorDefinition::new` and return one value per context row.

## Function metadata registry

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
