# Formula Engine

The formula engine evaluates expression-based computations such as `MA(CLOSE, 20)` or `RSI(CLOSE, 14)` with JIT compilation and SIMD acceleration. Formulas can reference OHLCV fields, nested expressions, and built-in functions.

See `docs/formula.md` for usage examples and `docs/architecture/formula-engine.md` for internal design.
