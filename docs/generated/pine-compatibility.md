# Pine Script Compatibility

> **SSOT** — auto-generated from `core/src/formula/pine/builtin_table.rs`.
> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`

Pine Script v5 built-in functions supported: **45**

Finkit supports a subset of Pine Script v5 for indicator migration from TradingView.

## Supported Built-in Functions

| Function |
|----------|
| `ta.abs` |
| `ta.aroon` |
| `ta.atr` |
| `ta.barssince` |
| `ta.bb` |
| `ta.cci` |
| `ta.change` |
| `ta.correlation` |
| `ta.crossover` |
| `ta.cum` |
| `ta.dmi` |
| `ta.ema` |
| `ta.fixnan` |
| `ta.highest` |
| `ta.hma` |
| `ta.log` |
| `ta.lowest` |
| `ta.macd` |
| `ta.max` |
| `ta.median` |
| `ta.min` |
| `ta.mom` |
| `ta.na` |
| `ta.nz` |
| `ta.obv` |
| `ta.pow` |
| `ta.range` |
| `ta.rma` |
| `ta.roc` |
| `ta.rsi` |
| `ta.sar` |
| `ta.security` |
| `ta.sma` |
| `ta.sqrt` |
| `ta.stdev` |
| `ta.stoch` |
| `ta.sum` |
| `ta.supertrend` |
| `ta.tr` |
| `ta.trix` |
| `ta.variance` |
| `ta.vwap` |
| `ta.vwma` |
| `ta.wma` |
| `ta.wpr` |

## Usage Example

```pine
//@version=5
indicator("RSI Example", overlay=false)
rsi = ta.rsi(close, 14)
plot(rsi)
```

## Regenerate

```bash
python scripts/gen_ssot_docs.py --generate
python scripts/gen_ssot_docs.py --check   # CI gate
```
