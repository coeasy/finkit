# Pine Script -> Finkit Migration Guide

This guide describes how to migrate indicator-style TradingView Pine Script logic to the current Finkit formula engine. Finkit is not a TradingView strategy runtime, so migration should distinguish formula/indicator semantics from broker, alert, chart-object, and strategy-execution behavior.

## 1. Choose the migration mode

Use the Pine-compatible parser when you want to validate or port an existing Pine indicator with minimal syntax changes. Use the native Finkit/terminal-style formula syntax when you want a compact production formula and explicit control over supported semantics.

Conceptually:

- existing Pine indicator -> parse through the Pine dialect -> inspect compatibility gaps;
- production formula -> rewrite to supported Finkit/terminal functions;
- strategy/order/alert logic -> move to the application layer.

## 2. Common function mappings

| Pine Script | Finkit-style equivalent |
| --- | --- |
| `ta.sma(src, len)` | `MA(src, len)` / supported SMA form |
| `ta.ema(src, len)` | `EMA(src, len)` |
| `ta.rsi(src, len)` | `RSI(src, len)` |
| `ta.atr(len)` | `ATR(...)` using the supported formula signature |
| `ta.cci(src, len)` | `CCI(...)` |
| `ta.highest(src, len)` | `HHV(src, len)` |
| `ta.lowest(src, len)` | `LLV(src, len)` |
| `ta.mom(src, len)` | `MOM(src, len)` |
| `ta.roc(src, len)` | `ROC(src, len)` |
| `math.abs(x)` | `ABS(x)` |
| `math.max(a, b)` | `MAX(a, b)` |
| `math.min(a, b)` | `MIN(a, b)` |
| `math.sqrt(x)` | `SQRT(x)` |

Use [generated/formula-functions.md](../generated/formula-functions.md) and [generated/pine-compatibility.md](../generated/pine-compatibility.md) as the current source of truth instead of assuming every Pine built-in is supported.

## 3. Market variables

Typical mappings are:

| Pine | Finkit |
| --- | --- |
| `open` | `OPEN` |
| `high` | `HIGH` |
| `low` | `LOW` |
| `close` | `CLOSE` |
| `volume` | `VOLUME` |
| `hl2` | `(HIGH + LOW) / 2` |
| `hlc3` | `(HIGH + LOW + CLOSE) / 3` |
| `ohlc4` | `(OPEN + HIGH + LOW + CLOSE) / 4` |

## 4. Historical references

Pine historical indexing should be rewritten to an explicitly supported historical-reference function when necessary.

Example:

```text
PREV_CLOSE := REF(CLOSE, 1);
```

Complex indexing/state expressions should be tested against the Pine compatibility corpus before relying on them in production.

## 5. Multi-output indicators

Pine tuple-style APIs may need to be rewritten as explicit named calculations or consumed through a native indicator API.

For MACD-style logic, an explicit formula is often easier to reason about:

```text
DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26);
DEA := EMA(DIF, 9);
MACD := 2 * (DIF - DEA);
```

Always validate the exact formula definition required by the source terminal because naming and scaling conventions can differ.

## 6. Features that belong outside the formula engine

The following Pine concepts should not be assumed to have full TradingView semantics inside Finkit:

- `strategy()` execution and broker simulation;
- order placement and fills;
- alerts;
- external Pine libraries/imports;
- TradingView chart-object lifecycle (`line`, `label`, `box`, etc.);
- complete repaint/lookahead semantics;
- symbol/session metadata unless explicitly supplied by the host application;
- unrestricted dynamic arrays/custom types;
- every plot style/color behavior.

Move those concerns to the host application or visualization layer.

## 7. Cross-timeframe/security behavior

Cross-timeframe requests are one of the highest-risk migration areas because lookahead, bar-close timing, and repaint rules materially affect results. Treat parser acceptance as separate from semantic equivalence.

Before production use:

1. test against a fixed historical corpus;
2. compare bar-by-bar outputs with the source implementation;
3. explicitly define lookahead/repaint expectations;
4. avoid claiming TradingView parity when only syntax parsing is verified.

## 8. Migration procedure

1. Inventory every Pine built-in, variable, historical reference, and stateful feature used by the script.
2. Check [generated/pine-compatibility.md](../generated/pine-compatibility.md).
3. Port pure indicator/math expressions first.
4. Rewrite historical references explicitly where needed.
5. Move unsupported strategy/alert/chart-object behavior to the application layer.
6. Build golden test data from the source script.
7. Compare outputs bar-by-bar, including warm-up regions.
8. Only then switch the production application to the Finkit implementation.

## 9. Warm-up differences

Finkit preserves series alignment and rolling indicators normally have leading warm-up `NaN` values. When comparing with Pine, make sure both sides use the same lookback and alignment rules before treating a mismatch as a numerical bug.

## 10. Related references

- [Formula guide](../formula.md)
- [Formula grammar](../formula/grammar.md)
- [Pine grammar](../formula/pine-grammar.md)
- [Generated Pine compatibility matrix](../generated/pine-compatibility.md)
- [Generated formula function catalog](../generated/formula-functions.md)
- `tests/formula_corpus/`
- `tests/pine_corpus/`
- `core/src/formula/pine/`
