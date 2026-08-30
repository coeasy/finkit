# Pine Script Compatibility

> **SSOT** — auto-generated from `core/src/formula/pine/builtin_table.rs`.
> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`

Pine Script v5 built-in functions supported: **33**

Finkit supports a subset of Pine Script v5 for indicator migration from TradingView.

## Supported Built-in Functions

| Function |
|----------|
| `ta.abs` |
| `ta.aroon` |
| `ta.atr` |
| `ta.bb` |
| `ta.cci` |
| `ta.change` |
| `ta.crossover` |
| `ta.dmi` |
| `ta.ema` |
| `ta.fixnan` |
| `ta.highest` |
| `ta.log` |
| `ta.lowest` |
| `ta.macd` |
| `ta.max` |
| `ta.min` |
| `ta.mom` |
| `ta.na` |
| `ta.nz` |
| `ta.obv` |
| `ta.pow` |
| `ta.roc` |
| `ta.rsi` |
| `ta.sar` |
| `ta.security` |
| `ta.sma` |
| `ta.sqrt` |
| `ta.stoch` |
| `ta.supertrend` |
| `ta.trix` |
| `ta.vwap` |
| `ta.vwma` |
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
